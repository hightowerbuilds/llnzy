//! Native macOS text ingress bridge for Stacker.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObjectProtocol};
use objc2::{define_class, msg_send};
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSResponder, NSTextView};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use crate::{StackerNativeEdit, UserEvent};

static EVENT_PROXY: OnceLock<winit::event_loop::EventLoopProxy<UserEvent>> = OnceLock::new();
static BRIDGE_VIEW: AtomicUsize = AtomicUsize::new(0);
static WINIT_NS_VIEW: AtomicUsize = AtomicUsize::new(0);
static NS_WINDOW: AtomicUsize = AtomicUsize::new(0);
static SYNCING_TEXT: AtomicBool = AtomicBool::new(false);
static STACKER_ACTIVE: AtomicBool = AtomicBool::new(false);
static FIRST_RESPONDER_ACTIVE: AtomicBool = AtomicBool::new(false);
static BRIDGE_TEXT: OnceLock<Mutex<String>> = OnceLock::new();

define_class!(
    #[unsafe(super(NSTextView))]
    #[name = "LlnzyStackerTextBridgeView"]
    struct StackerTextBridgeView;

    impl StackerTextBridgeView {
        #[unsafe(method(didChangeText))]
        fn did_change_text(&self) {
            if SYNCING_TEXT.load(Ordering::Relaxed) || !STACKER_ACTIVE.load(Ordering::Relaxed) {
                return;
            }

            let result = unsafe {
                let string: Retained<NSString> = msg_send![self, string];
                string.to_string()
            };
            let Some(edit) = update_bridge_text_from_native(result) else {
                return;
            };
            if let Some(proxy) = EVENT_PROXY.get() {
                let _ = proxy.send_event(UserEvent::StackerNativeEdit(edit));
            }
        }
    }

    unsafe impl NSObjectProtocol for StackerTextBridgeView {}
);

pub fn setup(proxy: winit::event_loop::EventLoopProxy<UserEvent>) {
    let _ = EVENT_PROXY.set(proxy);
}

pub fn install(window: &Window) {
    if BRIDGE_VIEW.load(Ordering::Relaxed) != 0 {
        return;
    }

    let Some((window_obj, content_view_obj, winit_view_obj)) = appkit_window_parts(window) else {
        return;
    };

    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0));
    let mtm = unsafe { MainThreadMarker::new_unchecked() };
    let bridge: Retained<StackerTextBridgeView> =
        unsafe { msg_send![StackerTextBridgeView::alloc(mtm), initWithFrame: frame] };

    unsafe {
        let _: () = msg_send![&*bridge, setEditable: true];
        let _: () = msg_send![&*bridge, setSelectable: true];
        let _: () = msg_send![&*bridge, setRichText: false];
        let _: () = msg_send![&*bridge, setImportsGraphics: false];
        let _: () = msg_send![&*bridge, setAutomaticQuoteSubstitutionEnabled: false];
        let _: () = msg_send![&*bridge, setAutomaticDashSubstitutionEnabled: false];
        let _: () = msg_send![&*bridge, setAutomaticTextReplacementEnabled: false];
        let _: () = msg_send![&*bridge, setContinuousSpellCheckingEnabled: false];
        let _: () = msg_send![&*bridge, setAlphaValue: 0.0f64];
        let _: () = msg_send![content_view_obj, addSubview: &*bridge];
    }

    NS_WINDOW.store(window_obj as usize, Ordering::Relaxed);
    WINIT_NS_VIEW.store(winit_view_obj as usize, Ordering::Relaxed);
    let bridge_obj: Retained<AnyObject> = bridge.into();
    BRIDGE_VIEW.store(Retained::into_raw(bridge_obj) as usize, Ordering::Relaxed);
}

pub fn set_stacker_active(window: &Window, active: bool, text: &str) {
    install(window);
    let bridge = bridge_view();
    let window_obj = ns_window();
    let winit_view = winit_view();
    let (Some(bridge), Some(window_obj), Some(winit_view)) = (bridge, window_obj, winit_view)
    else {
        return;
    };

    STACKER_ACTIVE.store(active, Ordering::Relaxed);
    unsafe {
        if active {
            sync_bridge_text(bridge, text);
            if !FIRST_RESPONDER_ACTIVE.swap(true, Ordering::Relaxed) {
                let responder = bridge as *const AnyObject as *const NSResponder;
                let _: bool = msg_send![window_obj, makeFirstResponder: Some(&*responder)];
            }
        } else if FIRST_RESPONDER_ACTIVE.swap(false, Ordering::Relaxed) {
            let responder = winit_view as *const AnyObject as *const NSResponder;
            let _: bool = msg_send![window_obj, makeFirstResponder: Some(&*responder)];
        }
    }
}

fn sync_bridge_text(bridge: &AnyObject, text: &str) {
    {
        let mut cached = bridge_text().lock().unwrap();
        if cached.as_str() == text {
            return;
        }
        *cached = text.to_string();
    }
    unsafe {
        SYNCING_TEXT.store(true, Ordering::Relaxed);
        let text = NSString::from_str(text);
        let _: () = msg_send![bridge, setString: &*text];
        SYNCING_TEXT.store(false, Ordering::Relaxed);
    }
}

fn update_bridge_text_from_native(result: String) -> Option<StackerNativeEdit> {
    let mut cached = bridge_text().lock().unwrap();
    if *cached == result {
        return None;
    }
    let (start, end, text) = replacement_delta(&cached, &result);
    *cached = result.clone();
    Some(StackerNativeEdit {
        start,
        end,
        text,
        result,
    })
}

fn bridge_text() -> &'static Mutex<String> {
    BRIDGE_TEXT.get_or_init(|| Mutex::new(String::new()))
}

fn replacement_delta(old: &str, new: &str) -> (usize, usize, String) {
    let old_chars: Vec<char> = old.chars().collect();
    let new_chars: Vec<char> = new.chars().collect();

    let mut prefix = 0usize;
    while prefix < old_chars.len()
        && prefix < new_chars.len()
        && old_chars[prefix] == new_chars[prefix]
    {
        prefix += 1;
    }

    let mut suffix = 0usize;
    while suffix < old_chars.len().saturating_sub(prefix)
        && suffix < new_chars.len().saturating_sub(prefix)
        && old_chars[old_chars.len() - 1 - suffix] == new_chars[new_chars.len() - 1 - suffix]
    {
        suffix += 1;
    }

    let end = old_chars.len() - suffix;
    let new_end = new_chars.len() - suffix;
    let text = new_chars[prefix..new_end].iter().collect();
    (prefix, end, text)
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

fn bridge_view() -> Option<&'static AnyObject> {
    let ptr = BRIDGE_VIEW.load(Ordering::Relaxed);
    (ptr != 0).then(|| unsafe { &*(ptr as *const AnyObject) })
}

fn ns_window() -> Option<&'static AnyObject> {
    let ptr = NS_WINDOW.load(Ordering::Relaxed);
    (ptr != 0).then(|| unsafe { &*(ptr as *const AnyObject) })
}

fn winit_view() -> Option<&'static AnyObject> {
    let ptr = WINIT_NS_VIEW.load(Ordering::Relaxed);
    (ptr != 0).then(|| unsafe { &*(ptr as *const AnyObject) })
}

#[cfg(test)]
mod tests {
    use super::replacement_delta;

    #[test]
    fn replacement_delta_detects_append() {
        assert_eq!(
            replacement_delta("hello", "hello world"),
            (5, 5, " world".to_string())
        );
    }

    #[test]
    fn replacement_delta_detects_middle_replacement() {
        assert_eq!(
            replacement_delta("hello world", "hello llnzy"),
            (6, 11, "llnzy".to_string())
        );
    }

    #[test]
    fn replacement_delta_handles_deletion() {
        assert_eq!(
            replacement_delta("hello world", "hello"),
            (5, 11, String::new())
        );
    }
}
