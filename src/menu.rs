//! Native macOS menu bar setup.

use std::sync::OnceLock;

use objc2::rc::Retained;
use objc2::runtime::Sel;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol};
use objc2::MainThreadMarker;
use objc2::{define_class, msg_send, sel, AnyThread};
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};
use objc2_foundation::NSString;

use crate::UserEvent;

/// Menu action identifiers, sent via UserEvent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    NewTab,
    CloseTab,
    Copy,
    Paste,
    SelectAll,
    Find,
    ToggleFullscreen,
    SplitVertical,
    SplitHorizontal,
    ToggleEffects,
    OpenProject,
    CloseProject,
}

static EVENT_PROXY: OnceLock<winit::event_loop::EventLoopProxy<UserEvent>> = OnceLock::new();
static MENU_TARGET: OnceLock<usize> = OnceLock::new();

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "LlnzyMenuTarget"]
    struct MenuTarget;

    impl MenuTarget {
        #[unsafe(method(llnzyNewTab:))]
        fn new_tab(&self, _sender: &AnyObject) {
            send_action(MenuAction::NewTab);
        }

        #[unsafe(method(llnzyCloseTab:))]
        fn close_tab(&self, _sender: &AnyObject) {
            send_action(MenuAction::CloseTab);
        }

        #[unsafe(method(copy:))]
        fn copy(&self, _sender: &AnyObject) {
            send_action(MenuAction::Copy);
        }

        #[unsafe(method(paste:))]
        fn paste(&self, _sender: &AnyObject) {
            send_action(MenuAction::Paste);
        }

        #[unsafe(method(selectAll:))]
        fn select_all(&self, _sender: &AnyObject) {
            send_action(MenuAction::SelectAll);
        }

        #[unsafe(method(llnzyFind:))]
        fn find(&self, _sender: &AnyObject) {
            send_action(MenuAction::Find);
        }

        #[unsafe(method(llnzySplitVertical:))]
        fn split_vertical(&self, _sender: &AnyObject) {
            send_action(MenuAction::SplitVertical);
        }

        #[unsafe(method(llnzySplitHorizontal:))]
        fn split_horizontal(&self, _sender: &AnyObject) {
            send_action(MenuAction::SplitHorizontal);
        }

        #[unsafe(method(llnzyOpenProject:))]
        fn open_project(&self, _sender: &AnyObject) {
            send_action(MenuAction::OpenProject);
        }

        #[unsafe(method(llnzyCloseProject:))]
        fn close_project(&self, _sender: &AnyObject) {
            send_action(MenuAction::CloseProject);
        }
    }

    unsafe impl NSObjectProtocol for MenuTarget {}
);

fn send_action(action: MenuAction) {
    if let Some(proxy) = EVENT_PROXY.get() {
        let _ = proxy.send_event(UserEvent::MenuAction(action));
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
    file_menu.addItem(&make_app_item(
        mtm,
        target,
        "New Tab",
        sel!(llnzyNewTab:),
        "t",
    ));
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
    let view_item = NSMenuItem::new(mtm);
    view_item.setSubmenu(Some(&view_menu));
    main_menu.addItem(&view_item);

    app.setMainMenu(Some(&main_menu));
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
