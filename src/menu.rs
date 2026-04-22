//! Native macOS menu bar setup.

use std::sync::OnceLock;

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2::runtime::Sel;
use objc2::sel;
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
}

static EVENT_PROXY: OnceLock<winit::event_loop::EventLoopProxy<UserEvent>> = OnceLock::new();

/// Set up the native macOS menu bar. Must be called from the main thread.
pub fn setup_menu_bar(proxy: winit::event_loop::EventLoopProxy<UserEvent>) {
    let _ = EVENT_PROXY.set(proxy);

    // Safe: this function is called from `resumed()` which runs on the main thread.
    let mtm = unsafe { MainThreadMarker::new_unchecked() };

    let app = NSApplication::sharedApplication(mtm);
    let main_menu = NSMenu::new(mtm);

    // App menu
    let app_menu = NSMenu::new(mtm);
    app_menu.addItem(&make_item(mtm, "About llnzy", None, ""));
    app_menu.addItem(&NSMenuItem::separatorItem(mtm));
    app_menu.addItem(&make_item(mtm, "Quit llnzy", Some(sel!(terminate:)), "q"));
    let app_item = NSMenuItem::new(mtm);
    app_item.setSubmenu(Some(&app_menu));
    main_menu.addItem(&app_item);

    // File menu
    let file_menu = NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str("File"));
    file_menu.addItem(&make_item(mtm, "New Tab", None, "t"));
    file_menu.addItem(&make_item(mtm, "Close Tab", None, "w"));
    let file_item = NSMenuItem::new(mtm);
    file_item.setSubmenu(Some(&file_menu));
    main_menu.addItem(&file_item);

    // Edit menu
    let edit_menu = NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str("Edit"));
    edit_menu.addItem(&make_item(mtm, "Copy", Some(sel!(copy:)), "c"));
    edit_menu.addItem(&make_item(mtm, "Paste", Some(sel!(paste:)), "v"));
    edit_menu.addItem(&make_item(mtm, "Select All", Some(sel!(selectAll:)), "a"));
    edit_menu.addItem(&NSMenuItem::separatorItem(mtm));
    edit_menu.addItem(&make_item(mtm, "Find", None, "f"));
    let edit_item = NSMenuItem::new(mtm);
    edit_item.setSubmenu(Some(&edit_menu));
    main_menu.addItem(&edit_item);

    // View menu
    let view_menu = NSMenu::initWithTitle(mtm.alloc(), &NSString::from_str("View"));
    view_menu.addItem(&make_item(mtm, "Split Vertically", None, "d"));
    view_menu.addItem(&make_item(mtm, "Split Horizontally", None, "D"));
    let view_item = NSMenuItem::new(mtm);
    view_item.setSubmenu(Some(&view_menu));
    main_menu.addItem(&view_item);

    app.setMainMenu(Some(&main_menu));
}

fn make_item(mtm: MainThreadMarker, title: &str, action: Option<Sel>, key: &str) -> Retained<NSMenuItem> {
    unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            mtm.alloc(),
            &NSString::from_str(title),
            action,
            &NSString::from_str(key),
        )
    }
}
