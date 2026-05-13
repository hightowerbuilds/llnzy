use gpui::{Context, Window};

use crate::config::Config;

use super::{
    MenuCloseProject, MenuCloseTab, MenuCopy, MenuFind, MenuJoinTabs, MenuNewTab, MenuNextTab,
    MenuOpenProject, MenuPaste, MenuPreviousTab, MenuRedo, MenuSave, MenuSelectAll,
    MenuSeparateTabs, MenuShowAppearances, MenuShowEditor, MenuShowHome, MenuShowSketch,
    MenuShowStacker, MenuShowTerminal, MenuSwapTabs, MenuToggleSidebar, MenuUndo, MenuZoomIn,
    MenuZoomOut, MenuZoomReset, WorkspacePrototype, WorkspaceSurface,
};

impl WorkspacePrototype {
    pub(super) fn activate_relative_tab(
        &mut self,
        offset: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.tabs.is_empty() {
            return;
        }
        let current = self
            .tabs
            .iter()
            .position(|tab| tab.id == self.active_tab_id)
            .unwrap_or(0);
        let len = self.tabs.len() as isize;
        let next = (current as isize + offset).rem_euclid(len) as usize;
        let tab_id = self.tabs[next].id;
        self.activate_tab(tab_id, window, cx);
    }

    pub(super) fn menu_new_tab(
        &mut self,
        _: &MenuNewTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_new_terminal_tab(window, cx);
    }

    pub(super) fn menu_close_tab(
        &mut self,
        _: &MenuCloseTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_tab(self.active_tab_id, window, cx);
    }

    pub(super) fn menu_next_tab(
        &mut self,
        _: &MenuNextTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_relative_tab(1, window, cx);
    }

    pub(super) fn menu_previous_tab(
        &mut self,
        _: &MenuPreviousTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_relative_tab(-1, window, cx);
    }

    pub(super) fn menu_join_tabs(
        &mut self,
        _: &MenuJoinTabs,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.open_active_tab_context_menu(window, cx) {
            self.show_tab_join_targets(cx);
        }
    }

    pub(super) fn menu_separate_tabs(
        &mut self,
        _: &MenuSeparateTabs,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.separate_tab_by_id(self.active_tab_id, cx);
    }

    pub(super) fn menu_swap_tabs(
        &mut self,
        _: &MenuSwapTabs,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.swap_tabs_by_id(self.active_tab_id, cx);
    }

    pub(super) fn menu_open_project(
        &mut self,
        _: &MenuOpenProject,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pick_open_project(cx);
    }

    pub(super) fn menu_close_project(
        &mut self,
        _: &MenuCloseProject,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_project(cx);
    }

    pub(super) fn menu_save(&mut self, _: &MenuSave, _: &mut Window, cx: &mut Context<Self>) {
        match self.active_surface() {
            WorkspaceSurface::Editor => {
                self.editor
                    .update(cx, |editor, cx| editor.save_active_buffer(cx));
            }
            WorkspaceSurface::Sketch => {
                self.sketch
                    .update(cx, |sketch, cx| sketch.save_from_workspace(cx));
            }
            _ => {}
        }
    }

    pub(super) fn menu_undo(&mut self, _: &MenuUndo, _: &mut Window, cx: &mut Context<Self>) {
        match self.active_surface() {
            WorkspaceSurface::Editor => {
                self.editor.update(cx, |editor, cx| editor.undo_edit(cx));
            }
            WorkspaceSurface::Sketch => {
                self.sketch
                    .update(cx, |sketch, cx| sketch.undo_from_workspace(cx));
            }
            _ => {}
        }
    }

    pub(super) fn menu_redo(&mut self, _: &MenuRedo, _: &mut Window, cx: &mut Context<Self>) {
        match self.active_surface() {
            WorkspaceSurface::Editor => {
                self.editor.update(cx, |editor, cx| editor.redo_edit(cx));
            }
            WorkspaceSurface::Sketch => {
                self.sketch
                    .update(cx, |sketch, cx| sketch.redo_from_workspace(cx));
            }
            _ => {}
        }
    }

    pub(super) fn menu_copy(&mut self, _: &MenuCopy, _: &mut Window, cx: &mut Context<Self>) {
        if self.active_surface() == WorkspaceSurface::Editor {
            self.editor
                .update(cx, |editor, cx| editor.copy_selection_to_clipboard(cx));
        }
    }

    pub(super) fn menu_paste(&mut self, _: &MenuPaste, _: &mut Window, cx: &mut Context<Self>) {
        if self.active_surface() == WorkspaceSurface::Editor {
            self.editor
                .update(cx, |editor, cx| editor.paste_from_clipboard(cx));
        }
    }

    pub(super) fn menu_select_all(
        &mut self,
        _: &MenuSelectAll,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_surface() == WorkspaceSurface::Editor {
            self.editor
                .update(cx, |editor, cx| editor.select_all_text(cx));
        }
    }

    pub(super) fn menu_find(&mut self, _: &MenuFind, window: &mut Window, cx: &mut Context<Self>) {
        self.open_or_activate_surface(WorkspaceSurface::Editor, window, cx);
        self.editor
            .update(cx, |editor, cx| editor.open_find_from_workspace(window, cx));
    }

    pub(super) fn menu_toggle_sidebar(
        &mut self,
        _: &MenuToggleSidebar,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_sidebar(cx);
    }

    pub(super) fn menu_show_surface(
        &mut self,
        surface: WorkspaceSurface,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_or_activate_surface(surface, window, cx);
    }

    pub(super) fn menu_show_home(
        &mut self,
        _: &MenuShowHome,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.menu_show_surface(WorkspaceSurface::Home, window, cx);
    }

    pub(super) fn menu_show_terminal(
        &mut self,
        _: &MenuShowTerminal,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.menu_show_surface(WorkspaceSurface::Terminal, window, cx);
    }

    pub(super) fn menu_show_stacker(
        &mut self,
        _: &MenuShowStacker,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.menu_show_surface(WorkspaceSurface::Stacker, window, cx);
    }

    pub(super) fn menu_show_editor(
        &mut self,
        _: &MenuShowEditor,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.menu_show_surface(WorkspaceSurface::Editor, window, cx);
    }

    pub(super) fn menu_show_sketch(
        &mut self,
        _: &MenuShowSketch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.menu_show_surface(WorkspaceSurface::Sketch, window, cx);
    }

    pub(super) fn menu_show_appearances(
        &mut self,
        _: &MenuShowAppearances,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.menu_show_surface(WorkspaceSurface::Appearances, window, cx);
    }

    pub(super) fn menu_zoom_in(&mut self, _: &MenuZoomIn, _: &mut Window, cx: &mut Context<Self>) {
        self.adjust_font_size(1.0, cx);
    }

    pub(super) fn menu_zoom_out(
        &mut self,
        _: &MenuZoomOut,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.adjust_font_size(-1.0, cx);
    }

    pub(super) fn menu_zoom_reset(
        &mut self,
        _: &MenuZoomReset,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.appearance_config.font_size = Config::default().font_size;
        self.apply_appearance_config(cx);
    }
}
