use gpui::{Context, Window};

use crate::config::Config;
use crate::editor::MarkdownViewMode;

use super::{
    MenuActivateTab1, MenuActivateTab2, MenuActivateTab3, MenuActivateTab4, MenuActivateTab5,
    MenuActivateTab6, MenuActivateTab7, MenuActivateTab8, MenuActivateTab9, MenuCloseProject,
    MenuCloseTab, MenuCopy, MenuEditorCheckDisk, MenuEditorCloseOthers, MenuEditorCloseSaved,
    MenuEditorReopenClosed, MenuFind, MenuJoinTabs, MenuLspCodeActions, MenuLspCompletion,
    MenuLspDefinition, MenuLspFormat, MenuLspHover, MenuLspReferences, MenuLspRename,
    MenuLspSignatureHelp, MenuLspSymbols, MenuMarkdownCycle, MenuMarkdownPreview, MenuMarkdownSource,
    MenuMarkdownSplit, MenuNewTab, MenuNextTab, MenuOpenProject, MenuPaste, MenuPreviousTab,
    MenuRedo, MenuSave, MenuSelectAll, MenuSeparateTabs, MenuShowAppearances, MenuShowEditor,
    MenuShowHome, MenuShowSketch, MenuShowStacker, MenuShowTerminal, MenuSwapTabs, MenuToggleSidebar,
    MenuUndo, MenuZoomIn, MenuZoomOut, MenuZoomReset, WorkspacePrototype, WorkspaceSurface,
};

impl WorkspacePrototype {
    fn with_editor_menu_action(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        action: impl FnOnce(
            &mut crate::gpui_editor::EditorPrototype,
            &mut Context<crate::gpui_editor::EditorPrototype>,
        ),
    ) {
        if self.active_surface() != WorkspaceSurface::Editor {
            self.open_or_activate_surface(WorkspaceSurface::Editor, window, cx);
        }
        self.active_editor_entity().update(cx, action);
    }

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

    /// Activate the tab at the given 0-based position. No-op if there is
    /// no tab at that position.
    fn activate_tab_at_index(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(tab) = self.tabs.get(index) {
            let id = tab.id;
            self.activate_tab(id, window, cx);
        }
    }

    pub(super) fn menu_activate_tab_1(
        &mut self,
        _: &MenuActivateTab1,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_tab_at_index(0, window, cx);
    }

    pub(super) fn menu_activate_tab_2(
        &mut self,
        _: &MenuActivateTab2,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_tab_at_index(1, window, cx);
    }

    pub(super) fn menu_activate_tab_3(
        &mut self,
        _: &MenuActivateTab3,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_tab_at_index(2, window, cx);
    }

    pub(super) fn menu_activate_tab_4(
        &mut self,
        _: &MenuActivateTab4,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_tab_at_index(3, window, cx);
    }

    pub(super) fn menu_activate_tab_5(
        &mut self,
        _: &MenuActivateTab5,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_tab_at_index(4, window, cx);
    }

    pub(super) fn menu_activate_tab_6(
        &mut self,
        _: &MenuActivateTab6,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_tab_at_index(5, window, cx);
    }

    pub(super) fn menu_activate_tab_7(
        &mut self,
        _: &MenuActivateTab7,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_tab_at_index(6, window, cx);
    }

    pub(super) fn menu_activate_tab_8(
        &mut self,
        _: &MenuActivateTab8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_tab_at_index(7, window, cx);
    }

    pub(super) fn menu_activate_tab_9(
        &mut self,
        _: &MenuActivateTab9,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_tab_at_index(8, window, cx);
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
                self.active_editor_entity()
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
                self.active_editor_entity()
                    .update(cx, |editor, cx| editor.undo_edit(cx));
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
                self.active_editor_entity()
                    .update(cx, |editor, cx| editor.redo_edit(cx));
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
            self.active_editor_entity()
                .update(cx, |editor, cx| editor.copy_selection_to_clipboard(cx));
        }
    }

    pub(super) fn menu_paste(&mut self, _: &MenuPaste, _: &mut Window, cx: &mut Context<Self>) {
        if self.active_surface() == WorkspaceSurface::Editor {
            self.active_editor_entity()
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
            self.active_editor_entity()
                .update(cx, |editor, cx| editor.select_all_text(cx));
        }
    }

    pub(super) fn menu_find(&mut self, _: &MenuFind, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_surface() != WorkspaceSurface::Editor {
            self.open_or_activate_surface(WorkspaceSurface::Editor, window, cx);
        }
        self.active_editor_entity()
            .update(cx, |editor, cx| editor.open_find_from_workspace(window, cx));
    }

    pub(super) fn menu_editor_check_disk(
        &mut self,
        _: &MenuEditorCheckDisk,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.with_editor_menu_action(window, cx, |editor, cx| {
            editor.check_active_external_change_from_workspace(cx);
        });
    }

    pub(super) fn menu_editor_reopen_closed(
        &mut self,
        _: &MenuEditorReopenClosed,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.with_editor_menu_action(window, cx, |editor, cx| {
            editor.reopen_recent_buffer_tab_from_workspace(cx);
        });
    }

    pub(super) fn menu_editor_close_others(
        &mut self,
        _: &MenuEditorCloseOthers,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.with_editor_menu_action(window, cx, |editor, cx| {
            editor.close_other_buffer_tabs_from_workspace(cx);
        });
    }

    pub(super) fn menu_editor_close_saved(
        &mut self,
        _: &MenuEditorCloseSaved,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.with_editor_menu_action(window, cx, |editor, cx| {
            editor.close_saved_buffer_tabs_from_workspace(cx);
        });
    }

    pub(super) fn menu_markdown_source(
        &mut self,
        _: &MenuMarkdownSource,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.with_editor_menu_action(window, cx, |editor, cx| {
            editor.set_markdown_mode_from_workspace(MarkdownViewMode::Source, cx);
        });
    }

    pub(super) fn menu_markdown_preview(
        &mut self,
        _: &MenuMarkdownPreview,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.with_editor_menu_action(window, cx, |editor, cx| {
            editor.set_markdown_mode_from_workspace(MarkdownViewMode::Preview, cx);
        });
    }

    pub(super) fn menu_markdown_split(
        &mut self,
        _: &MenuMarkdownSplit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.with_editor_menu_action(window, cx, |editor, cx| {
            editor.set_markdown_mode_from_workspace(MarkdownViewMode::Split, cx);
        });
    }

    pub(super) fn menu_markdown_cycle(
        &mut self,
        _: &MenuMarkdownCycle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.with_editor_menu_action(window, cx, |editor, cx| {
            editor.cycle_markdown_preview_from_workspace(cx);
        });
    }

    pub(super) fn menu_lsp_hover(
        &mut self,
        _: &MenuLspHover,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.with_editor_menu_action(window, cx, |editor, cx| {
            editor.request_lsp_hover_from_workspace(cx);
        });
    }

    pub(super) fn menu_lsp_completion(
        &mut self,
        _: &MenuLspCompletion,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.with_editor_menu_action(window, cx, |editor, cx| {
            editor.request_lsp_completion_from_workspace(cx);
        });
    }

    pub(super) fn menu_lsp_definition(
        &mut self,
        _: &MenuLspDefinition,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.with_editor_menu_action(window, cx, |editor, cx| {
            editor.request_lsp_definition_from_workspace(cx);
        });
    }

    pub(super) fn menu_lsp_references(
        &mut self,
        _: &MenuLspReferences,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.with_editor_menu_action(window, cx, |editor, cx| {
            editor.request_lsp_references_from_workspace(cx);
        });
    }

    pub(super) fn menu_lsp_signature_help(
        &mut self,
        _: &MenuLspSignatureHelp,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.with_editor_menu_action(window, cx, |editor, cx| {
            editor.request_lsp_signature_help_from_workspace(cx);
        });
    }

    pub(super) fn menu_lsp_rename(
        &mut self,
        _: &MenuLspRename,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.with_editor_menu_action(window, cx, |editor, cx| {
            editor.open_lsp_rename_from_workspace(cx);
        });
    }

    pub(super) fn menu_lsp_code_actions(
        &mut self,
        _: &MenuLspCodeActions,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.with_editor_menu_action(window, cx, |editor, cx| {
            editor.request_lsp_code_actions_from_workspace(cx);
        });
    }

    pub(super) fn menu_lsp_format(
        &mut self,
        _: &MenuLspFormat,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.with_editor_menu_action(window, cx, |editor, cx| {
            editor.request_lsp_format_from_workspace(cx);
        });
    }

    pub(super) fn menu_lsp_symbols(
        &mut self,
        _: &MenuLspSymbols,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.with_editor_menu_action(window, cx, |editor, cx| {
            editor.request_lsp_symbols_from_workspace(cx);
        });
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
