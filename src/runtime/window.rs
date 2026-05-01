use winit::window::Fullscreen;

use llnzy::app::window_state;
use llnzy::ui::{UiTabInfo, UiTabPaneInfo};
use llnzy::workspace::TabContent;

use crate::App;

impl App {
    pub(crate) fn toggle_effects(&mut self) {
        self.config.effects.enabled = !self.config.effects.enabled;
        if let Some(renderer) = &mut self.renderer {
            renderer.update_config(self.config.clone());
        }
        self.request_redraw();
    }

    pub(crate) fn toggle_fullscreen(&self) {
        if let Some(window) = &self.window {
            if window.fullscreen().is_some() {
                window.set_fullscreen(None);
            } else {
                window.set_fullscreen(Some(Fullscreen::Borderless(None)));
            }
        }
    }

    pub(crate) fn save_window_state(&self) {
        let Some(window) = &self.window else { return };
        let size = window.inner_size();
        let pos = window.outer_position().ok();
        window_state::save_window_placement(size.width, size.height, pos.map(|pos| (pos.x, pos.y)));

        let project_path = self.ui.as_ref().and_then(|ui| {
            if !ui.explorer.tree.is_empty() {
                Some(ui.explorer.root.clone())
            } else {
                None
            }
        });
        let tab_entries: Vec<llnzy::workspace_store::TabEntry> = self
            .tabs
            .iter()
            .filter_map(|tab| match &tab.content {
                TabContent::Home => Some(llnzy::workspace_store::TabEntry::Home),
                TabContent::Terminal(_) => Some(llnzy::workspace_store::TabEntry::Terminal),
                TabContent::CodeFile { path, .. } => {
                    Some(llnzy::workspace_store::TabEntry::CodeFile { path: path.clone() })
                }
                TabContent::Stacker => Some(llnzy::workspace_store::TabEntry::Stacker),
                TabContent::Sketch => Some(llnzy::workspace_store::TabEntry::Sketch),
                TabContent::Git => Some(llnzy::workspace_store::TabEntry::Git),
                _ => None,
            })
            .collect();
        let snapshot = llnzy::workspace_store::SessionSnapshot {
            theme: None,
            project_path,
            tabs: tab_entries,
        };
        let _ = llnzy::workspace_store::save_session(&snapshot);
    }

    pub(crate) fn tab_titles(&self) -> Vec<UiTabInfo> {
        self.tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let title = tab.display_name(i);
                let exited = tab
                    .content
                    .as_terminal()
                    .is_some_and(|session| session.exited.is_some());
                UiTabInfo {
                    title,
                    kind: tab.content.kind(),
                    exited,
                }
            })
            .collect()
    }

    pub(crate) fn tab_panes(&self) -> Vec<UiTabPaneInfo> {
        self.tabs
            .iter()
            .map(|tab| UiTabPaneInfo {
                kind: tab.content.kind(),
                buffer_idx: match &tab.content {
                    TabContent::CodeFile { buffer_idx, .. } => Some(*buffer_idx),
                    _ => None,
                },
            })
            .collect()
    }
}
