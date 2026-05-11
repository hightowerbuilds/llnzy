//! `NSTextInputClient` sibling subview for the Stacker prompt.
//!
//! Implements the AppKit text-input protocol directly so macOS dictation,
//! Wispr Flow, and IME composition deliver into `StackerSession`.
//!
//! This file is the **input bridge only** — visual rendering happens in
//! the editor view (`editor_host::render_prose_editor` with
//! `prose_mode = true`). The view conforms to `NSTextInputClient`,
//! accepts first responder when active, and posts `UserEvent`s for
//! mutating protocol calls. Synchronous queries (`markedRange`,
//! `selectedRange`) answer from a thread-local snapshot pushed by the
//! host each frame.
//!
//! Glyph-rect anchoring (`firstRectForCharacterRange:` /
//! `characterIndexForPoint:`) reads the per-frame `input_anchor`
//! `(Arc<Galley>, Pos2)` produced by the editor view in prose mode and
//! exported to `StackerUiState::prompt_editor_anchor` by the prompt panel.
//! That galley is laid out with the same font, line-height, and wrap
//! width as the visible per-line render, so dictation lands on the
//! caret within sub-pixel rounding.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, AnyProtocol, NSObjectProtocol, Sel};
use objc2::{define_class, msg_send};
use objc2::{ClassType, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAccessibilityFocusedUIElementChangedNotification, NSAccessibilityPostNotification,
    NSAccessibilityRole, NSAccessibilitySelectedTextChangedNotification, NSAccessibilityTextAreaRole,
    NSAccessibilityValueChangedNotification, NSResponder, NSView,
};
use objc2_foundation::{NSPoint, NSRange, NSRect, NSSize, NSString};
use winit::event_loop::EventLoopProxy;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use crate::stacker::input::StackerSelection;
use crate::stacker::utf16::{char_index_to_utf16_index, utf16_index_to_char_index};
use crate::UserEvent;

// AppKit sentinel for "no range" (NSNotFound = NSUIntegerMax).
const NS_NOT_FOUND: usize = usize::MAX;

static EVENT_PROXY: OnceLock<EventLoopProxy<UserEvent>> = OnceLock::new();

struct ClientSnapshot {
    text: String,
    total_chars: usize,
    selection_start_char: usize,
    selection_end_char: usize,
    /// Cached UTF-16 offsets for the selection. Recomputed in `set_state`
    /// when text or selection changes so that `selectedRange` / `markedRange`
    /// queries from AppKit are O(1) instead of O(n).
    selection_start_u16: usize,
    selection_end_u16: usize,
    marked_range_chars: Option<(usize, usize)>,
    marked_range_u16: Option<(usize, usize)>,
    /// Galley from the most recent egui frame, plus its top-left in egui
    /// screen coordinates. Used by `firstRectForCharacterRange:` to anchor
    /// dictation and IME overlays to the actual caret position.
    galley: Option<(std::sync::Arc<egui::Galley>, egui::Pos2)>,
}

impl Default for ClientSnapshot {
    fn default() -> Self {
        Self {
            text: String::new(),
            total_chars: 0,
            selection_start_char: 0,
            selection_end_char: 0,
            selection_start_u16: 0,
            selection_end_u16: 0,
            marked_range_chars: None,
            marked_range_u16: None,
            galley: None,
        }
    }
}

static CLIENT_STATE: OnceLock<Mutex<ClientSnapshot>> = OnceLock::new();
static ACTIVE: AtomicBool = AtomicBool::new(false);

fn client_state() -> &'static Mutex<ClientSnapshot> {
    CLIENT_STATE.get_or_init(|| Mutex::new(ClientSnapshot::default()))
}

fn with_state<R>(f: impl FnOnce(&ClientSnapshot) -> R) -> R {
    let guard = client_state()
        .lock()
        .expect("stacker input client state poisoned");
    f(&guard)
}

fn post(event: UserEvent) {
    if let Some(proxy) = EVENT_PROXY.get() {
        let _ = proxy.send_event(event);
    }
}

fn extract_string(id: *mut AnyObject) -> String {
    if id.is_null() {
        return String::new();
    }
    unsafe {
        // AppKit hands us either NSString or NSAttributedString. Both
        // respond to `string`, except plain NSString — for which we use
        // the value directly.
        let sel = objc2::sel!(string);
        let responds: bool = msg_send![id, respondsToSelector: sel];
        let ns_string: *const NSString = if responds {
            msg_send![id, string]
        } else {
            id as *const NSString
        };
        if ns_string.is_null() {
            String::new()
        } else {
            (*ns_string).to_string()
        }
    }
}

fn ns_range_to_utf16(range: NSRange) -> Option<(usize, usize)> {
    if range.location == NS_NOT_FOUND {
        None
    } else {
        Some((range.location, range.location + range.length))
    }
}

define_class!(
    #[unsafe(super(NSView))]
    #[name = "LlnzyStackerInputClient"]
    struct LlnzyStackerInputClient;

    impl LlnzyStackerInputClient {
        // Match the contentView's flipped (Y-down from top) coordinate system
        // so view-local coordinates line up with egui's screen coordinates.
        // Without this, `firstRectForCharacterRange:` and
        // `characterIndexForPoint:` invert the Y axis and the dictation
        // overlay anchors above the caret instead of below it.
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            ACTIVE.load(Ordering::Relaxed)
        }

        // Override `becomeFirstResponder` so we can post the AX focus
        // notification that native `NSTextView` posts whenever its focus
        // changes. Voice-dictation and accessibility tools listen for this
        // to confirm the field is the live input target; without it they
        // may defer delivery while polling.
        #[unsafe(method(becomeFirstResponder))]
        fn become_first_responder(&self) -> bool {
            let became: bool = unsafe { msg_send![super(self), becomeFirstResponder] };
            if became {
                crate::external_input_trace::trace(
                    "stacker.became_first_responder",
                    || "posting AX focus changed".to_string(),
                );
                unsafe {
                    let element: &AnyObject = self.as_ref();
                    NSAccessibilityPostNotification(
                        element,
                        NSAccessibilityFocusedUIElementChangedNotification,
                    );
                }
            }
            became
        }

        // NSAccessibility text-protocol surface. Tells AX clients
        // (Superwhisper, macOS system dictation, VoiceOver — and Wispr
        // Flow, though Wispr's slow path is unrelated; see
        // stacker-refinements.md) that this view is an editable text
        // area, so dictation injectors classify it correctly and
        // deliver text. Without these, third-party dictation tools that
        // scan AX for input targets either skip the view entirely or
        // fall through to slower / heuristic delivery paths.
        #[unsafe(method(isAccessibilityElement))]
        fn is_accessibility_element(&self) -> bool {
            true
        }

        #[unsafe(method(accessibilityRole))]
        fn accessibility_role(&self) -> *mut AnyObject {
            // Foundation constant: retain/release are no-ops on the
            // interned global. Safe to hand out as a raw pointer without
            // an autorelease balance.
            unsafe {
                NSAccessibilityTextAreaRole as *const NSAccessibilityRole as *mut AnyObject
            }
        }

        #[unsafe(method(accessibilityValue))]
        fn accessibility_value(&self) -> *mut AnyObject {
            let text = with_state(|s| s.text.clone());
            unsafe {
                let cls = AnyClass::get(c"NSString")
                    .expect("NSString class must exist on macOS");
                let mut bytes_nt = text.into_bytes();
                bytes_nt.push(0);
                // stringWithUTF8String: returns an autoreleased NSString
                // per Foundation factory convention — no extra balance
                // needed.
                let s: *mut AnyObject = msg_send![
                    cls,
                    stringWithUTF8String: bytes_nt.as_ptr() as *const i8
                ];
                s
            }
        }

        #[unsafe(method(setAccessibilityValue:))]
        fn set_accessibility_value(&self, value: *mut AnyObject) {
            let text = extract_string(value);
            let total_u16 = with_state(|s| {
                char_index_to_utf16_index(&s.text, s.total_chars)
            });
            post(UserEvent::StackerInputClientInsertText {
                text,
                replacement_utf16: Some((0, total_u16)),
            });
        }

        #[unsafe(method(accessibilitySelectedText))]
        fn accessibility_selected_text(&self) -> *mut AnyObject {
            let slice: String = with_state(|s| {
                let lo = s.selection_start_char.min(s.selection_end_char);
                let hi = s.selection_start_char.max(s.selection_end_char);
                if lo == hi {
                    String::new()
                } else {
                    s.text.chars().skip(lo).take(hi - lo).collect()
                }
            });
            unsafe {
                let cls = AnyClass::get(c"NSString")
                    .expect("NSString class must exist on macOS");
                let mut bytes_nt = slice.into_bytes();
                bytes_nt.push(0);
                let s: *mut AnyObject = msg_send![
                    cls,
                    stringWithUTF8String: bytes_nt.as_ptr() as *const i8
                ];
                s
            }
        }

        #[unsafe(method(setAccessibilitySelectedText:))]
        fn set_accessibility_selected_text(&self, value: *mut AnyObject) {
            let text = extract_string(value);
            let replacement_utf16 = with_state(|s| {
                let lo = s.selection_start_u16.min(s.selection_end_u16);
                let hi = s.selection_start_u16.max(s.selection_end_u16);
                Some((lo, hi))
            });
            post(UserEvent::StackerInputClientInsertText {
                text,
                replacement_utf16,
            });
        }

        #[unsafe(method(accessibilitySelectedTextRange))]
        fn accessibility_selected_text_range(&self) -> NSRange {
            with_state(|s| {
                let (lo, hi) = if s.selection_start_u16 <= s.selection_end_u16 {
                    (s.selection_start_u16, s.selection_end_u16)
                } else {
                    (s.selection_end_u16, s.selection_start_u16)
                };
                NSRange::new(lo, hi - lo)
            })
        }

        #[unsafe(method(accessibilityNumberOfCharacters))]
        fn accessibility_number_of_characters(&self) -> isize {
            with_state(|s| {
                char_index_to_utf16_index(&s.text, s.total_chars) as isize
            })
        }

        #[unsafe(method(hasMarkedText))]
        fn has_marked_text(&self) -> bool {
            with_state(|s| s.marked_range_chars.is_some())
        }

        #[unsafe(method(markedRange))]
        fn marked_range(&self) -> NSRange {
            with_state(|s| match s.marked_range_u16 {
                None => NSRange::new(NS_NOT_FOUND, 0),
                Some((start_u16, end_u16)) => {
                    NSRange::new(start_u16, end_u16.saturating_sub(start_u16))
                }
            })
        }

        #[unsafe(method(selectedRange))]
        fn selected_range(&self) -> NSRange {
            with_state(|s| {
                let (lo, hi) = if s.selection_start_u16 <= s.selection_end_u16 {
                    (s.selection_start_u16, s.selection_end_u16)
                } else {
                    (s.selection_end_u16, s.selection_start_u16)
                };
                NSRange::new(lo, hi - lo)
            })
        }

        #[unsafe(method(setMarkedText:selectedRange:replacementRange:))]
        fn set_marked_text(
            &self,
            string: *mut AnyObject,
            selected_range: NSRange,
            replacement_range: NSRange,
        ) {
            let text = extract_string(string);
            let internal = (
                selected_range.location,
                selected_range.location + selected_range.length,
            );
            let replacement = ns_range_to_utf16(replacement_range);
            post(UserEvent::StackerInputClientSetMarkedText {
                text,
                marked_internal_utf16: internal,
                replacement_utf16: replacement,
            });
        }

        #[unsafe(method(unmarkText))]
        fn unmark_text(&self) {
            post(UserEvent::StackerInputClientUnmarkText);
        }

        #[unsafe(method(insertText:replacementRange:))]
        fn insert_text(&self, string: *mut AnyObject, replacement_range: NSRange) {
            let text = extract_string(string);
            let replacement = ns_range_to_utf16(replacement_range);
            post(UserEvent::StackerInputClientInsertText {
                text,
                replacement_utf16: replacement,
            });
        }

        // Legacy single-argument NSTextInput `insertText:` selector. Modern
        // clients use `insertText:replacementRange:` above, but older /
        // cross-compatible voice dictation tools call this form. Forward to
        // the two-arg path with a "no replacement range" sentinel so the
        // handler resolves the target through the current selection.
        #[unsafe(method(insertText:))]
        fn insert_text_legacy(&self, string: *mut AnyObject) {
            let text = extract_string(string);
            crate::external_input_trace::trace("stacker.insert_text_legacy", || {
                format!("chars={}", text.chars().count())
            });
            post(UserEvent::StackerInputClientInsertText {
                text,
                replacement_utf16: None,
            });
        }

        #[unsafe(method(characterIndexForPoint:))]
        fn character_index_for_point(&self, point: NSPoint) -> usize {
            // Convert the screen-coordinate point to a galley-local position
            // and find the nearest character. The coordinate transform is the
            // exact inverse of the one used in `firstRectForCharacterRange:`.
            unsafe {
                let window: *mut AnyObject = msg_send![self, window];
                if window.is_null() {
                    return NS_NOT_FOUND;
                }

                let galley_result = with_state(|s| {
                    let (galley, origin) = s.galley.as_ref()?;
                    Some((galley.clone(), *origin, s.text.clone()))
                });

                let Some((galley, galley_origin, text)) = galley_result else {
                    return NS_NOT_FOUND;
                };

                // Screen → window base coordinates.
                let window_pt: NSPoint = msg_send![window, convertPointFromScreen: point];
                // Window base → view-local coordinates (nil = from window).
                let null_view = std::ptr::null::<AnyObject>();
                let local_pt: NSPoint =
                    msg_send![self, convertPoint: window_pt, fromView: null_view];

                // View-local → egui absolute (reverse of Phase 4c step).
                let frame: NSRect = msg_send![self, frame];
                let egui_x = local_pt.x as f32 + frame.origin.x as f32;
                let egui_y = local_pt.y as f32 + frame.origin.y as f32;

                // Egui absolute → galley-local.
                let galley_pos =
                    egui::vec2(egui_x - galley_origin.x, egui_y - galley_origin.y);
                let cursor = galley.cursor_from_pos(galley_pos);
                char_index_to_utf16_index(&text, cursor.ccursor.index)
            }
        }

        #[unsafe(method(firstRectForCharacterRange:actualRange:))]
        fn first_rect_for_character_range(
            &self,
            range: NSRange,
            _actual_range: *mut NSRange,
        ) -> NSRect {
            // Use the per-frame galley for accurate caret-anchored positioning.
            // Falls back to the whole view rect when no galley is cached yet.
            unsafe {
                let window: *mut AnyObject = msg_send![self, window];
                if window.is_null() {
                    let frame: NSRect = msg_send![self, frame];
                    return frame;
                }

                // Clone the galley data out of the mutex so we don't hold the
                // lock across the AppKit calls below.
                let galley_data = with_state(|s| {
                    let (galley, origin) = s.galley.as_ref()?;
                    let char_idx = match ns_range_to_utf16(range) {
                        Some((start_u16, _)) => {
                            utf16_index_to_char_index(&s.text, start_u16)
                        }
                        None => s.selection_start_char,
                    };
                    let local = galley.pos_from_ccursor(egui::text::CCursor::new(char_idx));
                    // Char rect in egui absolute screen coordinates (Y-down from window top).
                    Some((
                        egui::pos2(origin.x + local.min.x, origin.y + local.min.y),
                        egui::vec2(local.width().max(2.0), local.height()),
                    ))
                });

                if let Some((abs_pos, size)) = galley_data {
                    // Our view's frame in the content view's coordinate system.
                    // Since winit sets the content view as flipped (Y-down from
                    // top), frame.origin matches egui screen coordinates directly.
                    let frame: NSRect = msg_send![self, frame];
                    // Convert from absolute egui coords to view-local coords.
                    let local_x = abs_pos.x as f64 - frame.origin.x;
                    let local_y = abs_pos.y as f64 - frame.origin.y;
                    let local_rect = NSRect::new(
                        NSPoint::new(local_x, local_y),
                        NSSize::new(size.x as f64, size.y as f64),
                    );
                    // convertRect:toView:nil maps from view-local to window base
                    // coordinates; convertRectToScreen: maps to screen coordinates.
                    let null_view = std::ptr::null::<AnyObject>();
                    let window_rect: NSRect =
                        msg_send![self, convertRect: local_rect, toView: null_view];
                    return msg_send![window, convertRectToScreen: window_rect];
                }

                // Fallback: return the whole view in screen space.
                let frame: NSRect = msg_send![self, frame];
                msg_send![window, convertRectToScreen: frame]
            }
        }

        #[unsafe(method(validAttributesForMarkedText))]
        fn valid_attributes_for_marked_text(&self) -> *mut AnyObject {
            unsafe {
                let cls: &AnyClass =
                    AnyClass::get(c"NSArray").expect("NSArray class must exist on macOS");
                let arr: *mut AnyObject = msg_send![cls, array];
                arr
            }
        }

        #[unsafe(method(attributedSubstringForProposedRange:actualRange:))]
        fn attributed_substring_for_proposed_range(
            &self,
            range: NSRange,
            actual_range: *mut NSRange,
        ) -> *mut AnyObject {
            // Extract the document substring for the proposed UTF-16 range.
            // CJK IMEs read surrounding context from this; returning nil causes
            // composition artifacts (repeated characters, wrong candidates).
            let slice_data = with_state(|s| -> Option<(String, usize, usize)> {
                let (start_u16, end_u16) = ns_range_to_utf16(range)?;
                let total_chars = s.text.chars().count();
                let total_u16 = char_index_to_utf16_index(&s.text, total_chars);
                let start_u16 = start_u16.min(total_u16);
                let end_u16 = end_u16.min(total_u16);
                if start_u16 >= end_u16 {
                    return None;
                }
                let start_char = utf16_index_to_char_index(&s.text, start_u16);
                let end_char = utf16_index_to_char_index(&s.text, end_u16);
                let slice: String = s
                    .text
                    .chars()
                    .skip(start_char)
                    .take(end_char - start_char)
                    .collect();
                let actual_start = char_index_to_utf16_index(&s.text, start_char);
                let actual_end = char_index_to_utf16_index(&s.text, end_char);
                Some((slice, actual_start, actual_end))
            });

            let Some((slice, actual_start, actual_end)) = slice_data else {
                return std::ptr::null_mut();
            };

            unsafe {
                if !actual_range.is_null() {
                    *actual_range = NSRange::new(actual_start, actual_end - actual_start);
                }
                let ns_str_cls =
                    AnyClass::get(c"NSString").expect("NSString class must exist on macOS");
                let mut bytes_nt = slice.into_bytes();
                bytes_nt.push(0);
                let ns_str: *mut AnyObject =
                    msg_send![ns_str_cls, stringWithUTF8String: bytes_nt.as_ptr() as *const i8];
                if ns_str.is_null() {
                    return std::ptr::null_mut();
                }
                let attr_cls = AnyClass::get(c"NSAttributedString")
                    .expect("NSAttributedString class must exist on macOS");
                let attr_alloc: *mut AnyObject = msg_send![attr_cls, alloc];
                let attr_str: *mut AnyObject = msg_send![attr_alloc, initWithString: ns_str];
                if attr_str.is_null() {
                    return std::ptr::null_mut();
                }
                msg_send![attr_str, autorelease]
            }
        }

        #[unsafe(method(doCommandBySelector:))]
        fn do_command_by_selector(&self, selector: Sel) {
            let name = selector.name().to_string_lossy().into_owned();
            post(UserEvent::StackerInputClientDoCommand {
                selector_name: name,
            });
        }

    }

    unsafe impl NSObjectProtocol for LlnzyStackerInputClient {}
);

pub struct StackerInputClient {
    view: Retained<LlnzyStackerInputClient>,
    window_obj: *mut AnyObject,
    winit_view_obj: *mut AnyObject,
    visible: bool,
    last_rect: Option<[i32; 4]>,
}

/// Register `NSTextInputClient` protocol conformance on
/// `LlnzyStackerInputClient` at runtime, exactly once.
///
/// macOS routes dictation / IME composition by querying
/// `conformsToProtocol:@protocol(NSTextInputClient)` on the first
/// responder. Without this registration the runtime reports the class as
/// non-conforming and dictation silently skips the view.
///
/// Done at runtime rather than via `unsafe impl NSTextInputClient for ...`
/// inside `define_class!` because objc2 0.6's macro requires return types
/// that satisfy `EncodeReturn`, and several protocol methods declare
/// `Retained<NSAttributedString>` / `Retained<NSArray<…>>` which the
/// macro can't bridge from `define_class!` method registration.
/// `class_addProtocol` adds conformance based on selector matching, which
/// is what AppKit's input system actually checks.
fn ensure_text_input_client_protocol_registered() {
    static REGISTERED: OnceLock<()> = OnceLock::new();
    REGISTERED.get_or_init(|| {
        // Foreign-function declaration kept local so we don't depend on
        // objc2's private `ffi` module.
        extern "C" {
            fn class_addProtocol(cls: *mut AnyClass, protocol: *const AnyProtocol) -> bool;
        }
        let proto = AnyProtocol::get(c"NSTextInputClient")
            .expect("NSTextInputClient protocol must be registered by AppKit");
        let cls: *const AnyClass = LlnzyStackerInputClient::class();
        unsafe {
            class_addProtocol(cls as *mut AnyClass, proto as *const AnyProtocol);
        }
    });
}

impl StackerInputClient {
    pub fn new(window: &Window, proxy: EventLoopProxy<UserEvent>) -> Result<Self, String> {
        let _ = EVENT_PROXY.set(proxy);

        ensure_text_input_client_protocol_registered();

        let (window_obj, content_view_obj, winit_view_obj) = appkit_window_parts(window)
            .ok_or_else(|| "Stacker input client requires an AppKit window".to_string())?;

        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0));
        let view: Retained<LlnzyStackerInputClient> =
            unsafe { msg_send![LlnzyStackerInputClient::alloc(mtm), initWithFrame: frame] };

        unsafe {
            // Layer-back the input view so it composites cleanly above the
            // wgpu Metal layer. The view never paints anything itself, so
            // the layer stays transparent.
            let _: () = msg_send![&*view, setWantsLayer: true];
            let _: () = msg_send![&*view, setHidden: true];
            let _: () = msg_send![content_view_obj, addSubview: &*view];
        }

        Ok(Self {
            view,
            window_obj,
            winit_view_obj,
            visible: false,
            last_rect: None,
        })
    }

    pub fn set_visible(&mut self, visible: bool) -> bool {
        if self.visible == visible {
            return false;
        }
        self.visible = visible;
        ACTIVE.store(visible, Ordering::Relaxed);
        unsafe {
            let _: () = msg_send![&*self.view, setHidden: !visible];
        }
        if !visible {
            self.resign_first_responder_if_focused();
        }
        true
    }

    pub fn set_bounds(&mut self, rect: egui::Rect) {
        let left = rect.left().round() as i32;
        let top = rect.top().round() as i32;
        let width = rect.width().round().max(1.0) as i32;
        let height = rect.height().round().max(1.0) as i32;
        let next = [left, top, width, height];
        if self.last_rect == Some(next) {
            return;
        }
        self.last_rect = Some(next);

        let frame = NSRect::new(
            NSPoint::new(left as f64, top as f64),
            NSSize::new(width as f64, height as f64),
        );
        unsafe {
            let _: () = msg_send![&*self.view, setFrame: frame];
        }
    }

    /// Push the current document state into the snapshot the protocol
    /// query methods read from. Called every frame from the host so
    /// `markedRange` / `selectedRange` answer with the up-to-date model.
    ///
    /// `char_count` must equal `text.chars().count()`. Pass it from
    /// `StackerSession::char_count()`, which delegates to the rope and is O(1).
    pub fn set_state(
        &mut self,
        text: &str,
        char_count: usize,
        selection: StackerSelection,
        marked_range: Option<StackerSelection>,
    ) {
        let (text_changed, sel_changed_for_notify) = {
            let mut state = client_state()
                .lock()
                .expect("stacker input client state poisoned");

            let text_changed = state.text != text;
            if text_changed {
                state.text.clear();
                state.text.push_str(text);
                state.total_chars = char_count;
            }
            let total = state.total_chars;

            let new_sel_start = selection.start.min(total);
            let new_sel_end = selection.end.min(total);
            let sel_position_changed = state.selection_start_char != new_sel_start
                || state.selection_end_char != new_sel_end;
            let sel_changed = text_changed || sel_position_changed;
            state.selection_start_char = new_sel_start;
            state.selection_end_char = new_sel_end;
            if sel_changed {
                state.selection_start_u16 =
                    char_index_to_utf16_index(&state.text, new_sel_start);
                state.selection_end_u16 =
                    char_index_to_utf16_index(&state.text, new_sel_end);
            }

            let new_marked = marked_range.map(|r| {
                let sorted = r.sorted();
                (sorted.start.min(total), sorted.end.min(total))
            });
            let marked_changed = text_changed || state.marked_range_chars != new_marked;
            state.marked_range_chars = new_marked;
            if marked_changed {
                state.marked_range_u16 = new_marked.map(|(start, end)| {
                    let start_u16 = char_index_to_utf16_index(&state.text, start);
                    let end_u16 = char_index_to_utf16_index(&state.text, end);
                    (start_u16, end_u16)
                });
            }

            // Return-only the notify flags. Drop the lock before posting AX
            // notifications — `NSAccessibilityPostNotification` may dispatch
            // observer callbacks synchronously, and any of those could call
            // back into `accessibilityValue` / `accessibilitySelectedText`
            // which re-acquire this same mutex.
            (text_changed, sel_position_changed)
        };

        // Post AX change notifications outside the snapshot lock. Native
        // `NSTextView` posts these whenever its content or selection
        // changes; voice-dictation / accessibility tools rely on them to
        // confirm the field is actively writable and to trigger their
        // "instant delivery" path instead of falling through to the slower
        // clipboard-paste timeout fallback.
        if text_changed || sel_changed_for_notify {
            unsafe {
                let element: &AnyObject = &*self.view as &AnyObject;
                if text_changed {
                    NSAccessibilityPostNotification(
                        element,
                        NSAccessibilityValueChangedNotification,
                    );
                }
                if sel_changed_for_notify {
                    NSAccessibilityPostNotification(
                        element,
                        NSAccessibilitySelectedTextChangedNotification,
                    );
                }
            }

        }
    }

    /// Push the latest input anchor (single-galley layout + screen
    /// origin) from the editor view's prose render so
    /// `firstRectForCharacterRange:` and `characterIndexForPoint:` can
    /// return accurate screen rects. Call this every frame alongside
    /// `set_state`. `None` when the prose buffer is empty.
    pub fn set_galley(
        &mut self,
        galley: Option<(std::sync::Arc<egui::Galley>, egui::Pos2)>,
    ) {
        let mut state = client_state()
            .lock()
            .expect("stacker input client state poisoned");
        state.galley = galley;
    }

    pub fn focus(&mut self) {
        if !self.visible {
            return;
        }
        unsafe {
            let responder = &*self.view as *const LlnzyStackerInputClient as *const NSResponder;
            let _: bool = msg_send![self.window_obj, makeFirstResponder: Some(&*responder)];
        }
    }

    pub fn is_focused(&self) -> bool {
        unsafe {
            let first_responder: *mut AnyObject = msg_send![self.window_obj, firstResponder];
            if first_responder.is_null() {
                return false;
            }
            let view_ptr = &*self.view as *const LlnzyStackerInputClient as *const AnyObject;
            std::ptr::eq(first_responder as *const AnyObject, view_ptr)
        }
    }

    pub fn ensure_focused(&mut self) {
        if !self.visible || self.is_focused() {
            return;
        }
        self.focus();
    }

    fn resign_first_responder_if_focused(&self) {
        unsafe {
            let first_responder: *mut AnyObject = msg_send![self.window_obj, firstResponder];
            if first_responder.is_null() {
                return;
            }
            let view_ptr = &*self.view as *const LlnzyStackerInputClient as *const AnyObject;
            if std::ptr::eq(first_responder as *const AnyObject, view_ptr) {
                let responder = self.winit_view_obj as *const AnyObject as *const NSResponder;
                let _: bool = msg_send![self.window_obj, makeFirstResponder: Some(&*responder)];
            }
        }
    }
}

/// Convert a UTF-16 range pair from an event payload into a char-indexed
/// `StackerSelection` against the supplied text. Helper for the event
/// handler that translates protocol payloads into session edits.
pub fn utf16_pair_to_selection(text: &str, pair: (usize, usize)) -> StackerSelection {
    let start = utf16_index_to_char_index(text, pair.0);
    let end = utf16_index_to_char_index(text, pair.1);
    StackerSelection { start, end }
}

fn appkit_window_parts(
    window: &Window,
) -> Option<(*mut AnyObject, *mut AnyObject, *mut AnyObject)> {
    let handle = window.window_handle().ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    let ns_view = handle.ns_view.as_ptr() as *mut AnyObject;
    let ns_window: *mut AnyObject = unsafe { msg_send![ns_view, window] };
    if ns_window.is_null() {
        return None;
    }
    let content_view: *mut AnyObject = unsafe { msg_send![ns_window, contentView] };
    if content_view.is_null() {
        return None;
    }
    Some((ns_window, content_view, ns_view))
}
