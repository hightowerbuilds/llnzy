//! Native NSTextView overlay for the Stacker prompt editor.
//!
//! Replaces the previous WKWebView-backed editor. The visible NSTextView is a
//! real NSTextInputClient, so macOS dictation, IME composition, and tools like
//! Wispr Flow deliver text directly into it. The host treats this as an input
//! bridge: text/selection changes flow back through
//! `UserEvent::StackerNativeTextChanged` into `StackerDocumentEditor`, which
//! remains the durable owner of the prompt document.

use std::sync::atomic::{AtomicBool, Ordering};

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObjectProtocol};
use objc2::ClassType;
use objc2::{define_class, msg_send};
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSColor, NSFont, NSResponder, NSScrollView, NSTextView};
use objc2_foundation::{NSPoint, NSRange, NSRect, NSSize, NSString};
use winit::event_loop::EventLoopProxy;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use crate::stacker::input::StackerSelection;
use crate::UserEvent;

// NSAutoresizingMaskOptions::ViewWidthSizable; AppKit constant value.
const NS_VIEW_WIDTH_SIZABLE: u64 = 2;

static EVENT_PROXY: std::sync::OnceLock<EventLoopProxy<UserEvent>> = std::sync::OnceLock::new();

// Suppresses the change/selection callbacks while the host is pushing a new
// document into the text view. Mirrors the pattern from the prior hidden
// macos_text_bridge.
static SYNCING_TEXT: AtomicBool = AtomicBool::new(false);

define_class!(
    #[unsafe(super(NSTextView))]
    #[name = "LlnzyStackerTextView"]
    struct LlnzyStackerTextView;

    impl LlnzyStackerTextView {
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true
        }

        #[unsafe(method(didChangeText))]
        fn did_change_text(&self) {
            if SYNCING_TEXT.load(Ordering::Relaxed) {
                return;
            }
            post_state(self, "textChanged");
        }

        #[unsafe(method(didChangeSelection:))]
        fn did_change_selection(&self, _notification: *mut AnyObject) {
            if SYNCING_TEXT.load(Ordering::Relaxed) {
                return;
            }
            post_state(self, "selectionChanged");
        }
    }

    unsafe impl NSObjectProtocol for LlnzyStackerTextView {}
);

pub struct StackerNativeView {
    text_view: Retained<LlnzyStackerTextView>,
    scroll_view: Retained<NSScrollView>,
    window_obj: *mut AnyObject,
    winit_view_obj: *mut AnyObject,
    visible: bool,
    last_text: String,
    last_selection: StackerSelection,
    last_rect: Option<[i32; 4]>,
    last_font_size: f32,
    pending_focus: bool,
}

impl StackerNativeView {
    pub fn new(window: &Window, proxy: EventLoopProxy<UserEvent>) -> Result<Self, String> {
        let _ = EVENT_PROXY.set(proxy);

        let (window_obj, content_view_obj, winit_view_obj) = appkit_window_parts(window)
            .ok_or_else(|| "Stacker native view requires an AppKit window".to_string())?;

        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0));

        let scroll_view: Retained<NSScrollView> =
            unsafe { msg_send![NSScrollView::alloc(mtm), initWithFrame: frame] };

        let text_view: Retained<LlnzyStackerTextView> =
            unsafe { msg_send![LlnzyStackerTextView::alloc(mtm), initWithFrame: frame] };

        unsafe {
            // Behavior: editable plain-text, no AppleScript-friendly substitutions
            // that would diverge from what users actually typed.
            let _: () = msg_send![&*text_view, setEditable: true];
            let _: () = msg_send![&*text_view, setSelectable: true];
            let _: () = msg_send![&*text_view, setRichText: false];
            let _: () = msg_send![&*text_view, setImportsGraphics: false];
            let _: () = msg_send![&*text_view, setAllowsUndo: false];
            let _: () = msg_send![&*text_view, setAutomaticQuoteSubstitutionEnabled: false];
            let _: () = msg_send![&*text_view, setAutomaticDashSubstitutionEnabled: false];
            let _: () = msg_send![&*text_view, setAutomaticTextReplacementEnabled: false];
            let _: () = msg_send![&*text_view, setAutomaticSpellingCorrectionEnabled: false];
            let _: () = msg_send![&*text_view, setContinuousSpellCheckingEnabled: false];
            let _: () = msg_send![&*text_view, setGrammarCheckingEnabled: false];
            let _: () = msg_send![&*text_view, setSmartInsertDeleteEnabled: false];
            let _: () = msg_send![&*text_view, setUsesFindBar: false];

            // Visual styling matches the previous WebView textarea: dark
            // background, soft white text, and a green caret.
            let bg = ns_color(0.110, 0.110, 0.110, 1.0);
            let fg = ns_color(0.941, 0.973, 1.000, 1.0);
            let caret = ns_color(0.416, 1.000, 0.565, 1.0);
            let _: () = msg_send![&*text_view, setBackgroundColor: &*bg];
            let _: () = msg_send![&*text_view, setTextColor: &*fg];
            let _: () = msg_send![&*text_view, setInsertionPointColor: &*caret];
            let _: () = msg_send![&*text_view, setDrawsBackground: true];
            let _: () = msg_send![&*scroll_view, setBackgroundColor: &*bg];
            let _: () = msg_send![&*scroll_view, setDrawsBackground: true];

            let inset = NSSize::new(34.0, 34.0);
            let _: () = msg_send![&*text_view, setTextContainerInset: inset];

            let font: Retained<NSFont> = msg_send![NSFont::class(), systemFontOfSize: 16.0f64];
            let _: () = msg_send![&*text_view, setFont: &*font];

            // Make the text view fill the scroll view horizontally; vertical
            // growth is handled by the scroll view's content size when wrap is
            // disabled. We leave the default text container width-tracks-text
            // behavior so wrapping respects the scroll view bounds.
            let _: () = msg_send![&*text_view, setAutoresizingMask: NS_VIEW_WIDTH_SIZABLE];

            // Wire the scroll view: vertical scroller only, no border.
            let _: () = msg_send![&*scroll_view, setHasVerticalScroller: true];
            let _: () = msg_send![&*scroll_view, setHasHorizontalScroller: false];
            let _: () = msg_send![&*scroll_view, setBorderType: 0u64]; // NSNoBorder
            let _: () = msg_send![&*scroll_view, setDocumentView: &*text_view];

            let _: () = msg_send![&*scroll_view, setHidden: true];
            let _: () = msg_send![content_view_obj, addSubview: &*scroll_view];
        }

        Ok(Self {
            text_view,
            scroll_view,
            window_obj,
            winit_view_obj,
            visible: false,
            last_text: String::new(),
            last_selection: StackerSelection::collapsed(0),
            last_rect: None,
            last_font_size: 16.0,
            pending_focus: false,
        })
    }

    pub fn set_visible(&mut self, visible: bool) -> bool {
        if self.visible == visible {
            return false;
        }
        self.visible = visible;
        unsafe {
            let _: () = msg_send![&*self.scroll_view, setHidden: !visible];
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

        // egui hands us window-coordinate rects with origin at the top-left in
        // physical pixels. AppKit content views use a flipped coordinate space
        // by default for subviews of the window content view, so we can use
        // these values directly. NSScrollView::setFrame: takes points in the
        // parent view's coordinates.
        let frame = NSRect::new(
            NSPoint::new(left as f64, top as f64),
            NSSize::new(width as f64, height as f64),
        );
        unsafe {
            let _: () = msg_send![&*self.scroll_view, setFrame: frame];
        }
    }

    pub fn set_document(&mut self, text: &str, selection: StackerSelection) {
        let selection = clamp_selection(text, selection);
        if self.last_text == text && self.last_selection == selection {
            return;
        }

        SYNCING_TEXT.store(true, Ordering::Relaxed);
        if self.last_text != text {
            self.last_text.clear();
            self.last_text.push_str(text);
            unsafe {
                let ns = NSString::from_str(text);
                let _: () = msg_send![&*self.text_view, setString: &*ns];
            }
            // setString: re-applies the default attributes; reapply the font
            // and color the view was configured with so dictation/paste keep
            // matching the surrounding chrome.
            self.reapply_text_attributes();
        }
        self.last_selection = selection;
        let utf16_start = char_index_to_utf16_index(text, selection.start);
        let utf16_end = char_index_to_utf16_index(text, selection.end);
        let length = utf16_end.saturating_sub(utf16_start);
        unsafe {
            let range = NSRange::new(utf16_start, length);
            let _: () = msg_send![&*self.text_view, setSelectedRange: range];
        }
        SYNCING_TEXT.store(false, Ordering::Relaxed);
    }

    pub fn note_view_document(&mut self, text: &str, selection: StackerSelection) {
        self.last_text.clear();
        self.last_text.push_str(text);
        self.last_selection = clamp_selection(text, selection);
    }

    pub fn set_font_size(&mut self, font_size: f32) {
        if (self.last_font_size - font_size).abs() < 0.1 {
            return;
        }
        self.last_font_size = font_size;
        unsafe {
            let font: Retained<NSFont> =
                msg_send![NSFont::class(), systemFontOfSize: font_size as f64];
            let _: () = msg_send![&*self.text_view, setFont: &*font];
        }
    }

    pub fn focus(&mut self) {
        if !self.visible {
            self.pending_focus = true;
            return;
        }
        unsafe {
            let responder = &*self.text_view as *const LlnzyStackerTextView as *const NSResponder;
            let _: bool = msg_send![self.window_obj, makeFirstResponder: Some(&*responder)];
        }
        self.pending_focus = false;
    }

    fn resign_first_responder_if_focused(&self) {
        unsafe {
            let first_responder: *mut AnyObject = msg_send![self.window_obj, firstResponder];
            // If the text view (or one of its field editors) is first responder,
            // hand the focus back to the winit content view so keyboard input
            // resumes flowing through the standard event loop.
            if first_responder.is_null() {
                return;
            }
            let text_view_ptr = &*self.text_view as *const LlnzyStackerTextView as *const AnyObject;
            if std::ptr::eq(first_responder as *const AnyObject, text_view_ptr) {
                let responder = self.winit_view_obj as *const AnyObject as *const NSResponder;
                let _: bool = msg_send![self.window_obj, makeFirstResponder: Some(&*responder)];
            }
        }
    }

    fn reapply_text_attributes(&self) {
        unsafe {
            let font: Retained<NSFont> =
                msg_send![NSFont::class(), systemFontOfSize: self.last_font_size as f64];
            let fg = ns_color(0.941, 0.973, 1.000, 1.0);
            let _: () = msg_send![&*self.text_view, setFont: &*font];
            let _: () = msg_send![&*self.text_view, setTextColor: &*fg];
        }
    }
}

fn post_state(view: &LlnzyStackerTextView, kind: &'static str) {
    let Some(proxy) = EVENT_PROXY.get() else {
        return;
    };
    let (text, range) = unsafe {
        let string: Retained<NSString> = msg_send![view, string];
        let text = string.to_string();
        let range: NSRange = msg_send![view, selectedRange];
        (text, range)
    };
    let utf16_start = range.location;
    let utf16_end = range.location + range.length;
    let _ = proxy.send_event(UserEvent::StackerNativeTextChanged {
        kind,
        text,
        utf16_start,
        utf16_end,
    });
}

fn ns_color(red: f64, green: f64, blue: f64, alpha: f64) -> Retained<NSColor> {
    unsafe {
        msg_send![
            NSColor::class(),
            colorWithSRGBRed: red,
            green: green,
            blue: blue,
            alpha: alpha
        ]
    }
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

pub fn utf16_index_to_char_index(text: &str, utf16_index: usize) -> usize {
    let mut units = 0;
    for (char_index, ch) in text.chars().enumerate() {
        if units >= utf16_index {
            return char_index;
        }
        units += ch.len_utf16();
        if units > utf16_index {
            return char_index + 1;
        }
    }
    text.chars().count()
}

pub fn char_index_to_utf16_index(text: &str, char_index: usize) -> usize {
    text.chars().take(char_index).map(|ch| ch.len_utf16()).sum()
}

fn clamp_selection(text: &str, selection: StackerSelection) -> StackerSelection {
    let char_count = text.chars().count();
    StackerSelection {
        start: selection.start.min(char_count),
        end: selection.end.min(char_count),
    }
}
