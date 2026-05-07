//! Native macOS menu bar setup.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use objc2::rc::Retained;
use objc2::runtime::Sel;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol};
use objc2::MainThreadMarker;
use objc2::{define_class, msg_send, sel, AnyThread};
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};
use objc2_foundation::NSString;

use crate::platform::menu::{command_id_for_native_action, PlatformMenuAction};
use crate::UserEvent;

static EVENT_PROXY: OnceLock<winit::event_loop::EventLoopProxy<UserEvent>> = OnceLock::new();
static MENU_TARGET: OnceLock<usize> = OnceLock::new();
static SAVE_MENU_ITEM: AtomicUsize = AtomicUsize::new(0);

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "LlnzyMenuTarget"]
    struct MenuTarget;

    impl MenuTarget {
        #[unsafe(method(llnzyNewWindow:))]
        fn new_window(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::NewWindow);
        }

        #[unsafe(method(llnzyNewTab:))]
        fn new_tab(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::NewTab);
        }

        #[unsafe(method(llnzySave:))]
        fn save(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::Save);
        }

        #[unsafe(method(llnzyCloseTab:))]
        fn close_tab(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::CloseTab);
        }

        #[unsafe(method(llnzyTabJoin:))]
        fn tab_join(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::TabJoin);
        }

        #[unsafe(method(llnzyTabSeparate:))]
        fn tab_separate(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::TabSeparate);
        }

        #[unsafe(method(llnzyTabSplit:))]
        fn tab_split(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::TabSplit);
        }

        #[unsafe(method(llnzyTabRename:))]
        fn tab_rename(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::TabRename);
        }

        #[unsafe(method(llnzyUndo:))]
        fn undo(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::Undo);
        }

        #[unsafe(method(llnzyRedo:))]
        fn redo(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::Redo);
        }

        #[unsafe(method(copy:))]
        fn copy(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::Copy);
        }

        #[unsafe(method(paste:))]
        fn paste(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::Paste);
        }

        #[unsafe(method(selectAll:))]
        fn select_all(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::SelectAll);
        }

        #[unsafe(method(llnzyFind:))]
        fn find(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::Find);
        }

        #[unsafe(method(llnzySplitVertical:))]
        fn split_vertical(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::SplitVertical);
        }

        #[unsafe(method(llnzySplitHorizontal:))]
        fn split_horizontal(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::SplitHorizontal);
        }

        #[unsafe(method(llnzyToggleWordWrap:))]
        fn toggle_word_wrap(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::ToggleWordWrap);
        }

        #[unsafe(method(llnzyZoomIn:))]
        fn zoom_in(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::ZoomIn);
        }

        #[unsafe(method(llnzyZoomOut:))]
        fn zoom_out(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::ZoomOut);
        }

        #[unsafe(method(llnzyZoomReset:))]
        fn zoom_reset(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::ZoomReset);
        }

        #[unsafe(method(llnzyOpenProject:))]
        fn open_project(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::OpenProject);
        }

        #[unsafe(method(llnzyCloseProject:))]
        fn close_project(&self, _sender: &AnyObject) {
            send_action(PlatformMenuAction::CloseProject);
        }
    }

    unsafe impl NSObjectProtocol for MenuTarget {}
);

fn send_action(action: PlatformMenuAction) {
    if let Some(proxy) = EVENT_PROXY.get() {
        let _ = proxy.send_event(UserEvent::MenuCommand(
            command_id_for_native_action(action).to_string(),
        ));
    }
}

fn menu_target() -> &'static AnyObject {
    let ptr = *MENU_TARGET.get_or_init(|| {
        let target: Retained<MenuTarget> = unsafe { msg_send![MenuTarget::alloc(), init] };
        let target: Retained<AnyObject> = target.into();
        Retained::into_raw(target) as usize
    });
    unsafe { &*(ptr as *const AnyObject) }
}

/// Set up the native macOS menu bar. Must be called from the main thread.
pub fn setup_menu_bar(proxy: winit::event_loop::EventLoopProxy<UserEvent>) {
    let _ = EVENT_PROXY.set(proxy);

    // Safe: this function is called from `resumed()` which runs on the main thread.
    let mtm = unsafe { MainThreadMarker::new_unchecked() };

    let app = NSApplication::sharedApplication(mtm);
    let main_menu = NSMenu::new(mtm);
    let target = menu_target();

    // App menu
    let app_menu = NSMenu::new(mtm);
    app_menu.addItem(&make_system_item(mtm, "About llnzy", None, ""));
    app_menu.addItem(&NSMenuItem::separatorItem(mtm));
    app_menu.addItem(&make_system_item(
        mtm,
        "Quit llnzy",
        Some(sel!(terminate:)),
        "q",
    ));
    let app_item = NSMenuItem::new(mtm);
    app_item.setSubmenu(Some(&app_menu));
    main_menu.addItem(&app_item);

    // File menu
    let file_menu = NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str("File"));
    file_menu.setAutoenablesItems(false);
    file_menu.addItem(&make_app_item(
        mtm,
        target,
        "New Window",
        sel!(llnzyNewWindow:),
        "n",
    ));
    file_menu.addItem(&make_app_item(
        mtm,
        target,
        "New Tab",
        sel!(llnzyNewTab:),
        "t",
    ));
    let save_item = make_app_item(mtm, target, "Save", sel!(llnzySave:), "s");
    save_item.setEnabled(false);
    SAVE_MENU_ITEM.store(&*save_item as *const NSMenuItem as usize, Ordering::Relaxed);
    file_menu.addItem(&save_item);
    file_menu.addItem(&make_app_item(
        mtm,
        target,
        "Close Tab",
        sel!(llnzyCloseTab:),
        "w",
    ));
    file_menu.addItem(&NSMenuItem::separatorItem(mtm));
    file_menu.addItem(&make_app_item(
        mtm,
        target,
        "Open Project...",
        sel!(llnzyOpenProject:),
        "o",
    ));
    file_menu.addItem(&make_app_item(
        mtm,
        target,
        "Close Project",
        sel!(llnzyCloseProject:),
        "",
    ));
    let file_item = NSMenuItem::new(mtm);
    file_item.setSubmenu(Some(&file_menu));
    main_menu.addItem(&file_item);

    // Edit menu
    let edit_menu = NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str("Edit"));
    edit_menu.addItem(&make_app_item(mtm, target, "Undo", sel!(llnzyUndo:), "z"));
    edit_menu.addItem(&make_app_item(mtm, target, "Redo", sel!(llnzyRedo:), "Z"));
    edit_menu.addItem(&NSMenuItem::separatorItem(mtm));
    edit_menu.addItem(&make_app_item(mtm, target, "Copy", sel!(copy:), "c"));
    edit_menu.addItem(&make_app_item(mtm, target, "Paste", sel!(paste:), "v"));
    edit_menu.addItem(&make_app_item(
        mtm,
        target,
        "Select All",
        sel!(selectAll:),
        "a",
    ));
    edit_menu.addItem(&NSMenuItem::separatorItem(mtm));
    edit_menu.addItem(&make_app_item(mtm, target, "Find", sel!(llnzyFind:), "f"));
    let edit_item = NSMenuItem::new(mtm);
    edit_item.setSubmenu(Some(&edit_menu));
    main_menu.addItem(&edit_item);

    // Tab menu
    let tab_menu = NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str("Tab"));
    tab_menu.setAutoenablesItems(false);
    tab_menu.addItem(&make_app_item(mtm, target, "New", sel!(llnzyNewTab:), ""));
    tab_menu.addItem(&make_app_item(mtm, target, "Join", sel!(llnzyTabJoin:), ""));
    tab_menu.addItem(&make_app_item(
        mtm,
        target,
        "Separate",
        sel!(llnzyTabSeparate:),
        "",
    ));
    tab_menu.addItem(&make_app_item(
        mtm,
        target,
        "Split",
        sel!(llnzyTabSplit:),
        "",
    ));
    tab_menu.addItem(&NSMenuItem::separatorItem(mtm));
    tab_menu.addItem(&make_app_item(
        mtm,
        target,
        "Close",
        sel!(llnzyCloseTab:),
        "w",
    ));
    tab_menu.addItem(&make_app_item(
        mtm,
        target,
        "Rename",
        sel!(llnzyTabRename:),
        "",
    ));
    let tab_item = NSMenuItem::new(mtm);
    tab_item.setSubmenu(Some(&tab_menu));
    main_menu.addItem(&tab_item);

    // View menu
    let view_menu = NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str("View"));
    view_menu.addItem(&make_app_item(
        mtm,
        target,
        "Split Vertically",
        sel!(llnzySplitVertical:),
        "d",
    ));
    view_menu.addItem(&make_app_item(
        mtm,
        target,
        "Split Horizontally",
        sel!(llnzySplitHorizontal:),
        "D",
    ));
    view_menu.addItem(&NSMenuItem::separatorItem(mtm));
    view_menu.addItem(&make_app_item(
        mtm,
        target,
        "Toggle Word Wrap",
        sel!(llnzyToggleWordWrap:),
        "",
    ));
    view_menu.addItem(&NSMenuItem::separatorItem(mtm));
    view_menu.addItem(&make_app_item(
        mtm,
        target,
        "Increase Font Size",
        sel!(llnzyZoomIn:),
        "+",
    ));
    view_menu.addItem(&make_app_item(
        mtm,
        target,
        "Decrease Font Size",
        sel!(llnzyZoomOut:),
        "-",
    ));
    view_menu.addItem(&make_app_item(
        mtm,
        target,
        "Reset Font Size",
        sel!(llnzyZoomReset:),
        "0",
    ));
    let view_item = NSMenuItem::new(mtm);
    view_item.setSubmenu(Some(&view_menu));
    main_menu.addItem(&view_item);

    app.setMainMenu(Some(&main_menu));
}

/// Enable Save only when the active surface can handle a file save.
pub fn set_save_enabled(enabled: bool) {
    let ptr = SAVE_MENU_ITEM.load(Ordering::Relaxed);
    if ptr == 0 {
        return;
    }

    unsafe {
        (&*(ptr as *const NSMenuItem)).setEnabled(enabled);
    }
}

fn make_system_item(
    mtm: MainThreadMarker,
    title: &str,
    action: Option<Sel>,
    key: &str,
) -> Retained<NSMenuItem> {
    unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            mtm.alloc(),
            &NSString::from_str(title),
            action,
            &NSString::from_str(key),
        )
    }
}

fn make_app_item(
    mtm: MainThreadMarker,
    target: &AnyObject,
    title: &str,
    action: Sel,
    key: &str,
) -> Retained<NSMenuItem> {
    let item = make_system_item(mtm, title, Some(action), key);
    unsafe {
        item.setTarget(Some(target));
    }
    item
}
