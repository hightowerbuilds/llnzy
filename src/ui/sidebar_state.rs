use std::path::PathBuf;

use super::types::{BUMPER_WIDTH, SIDEBAR_WIDTH};

#[derive(Clone, Debug)]
pub struct SidebarDropZone {
    pub rect: egui::Rect,
    pub folder: PathBuf,
}

pub struct SidebarUiState {
    pub open: bool,
    pub actual_width: f32,
    pub recent_open: bool,
    pub native_drop_zones: Vec<SidebarDropZone>,
}

impl Default for SidebarUiState {
    fn default() -> Self {
        Self {
            open: false,
            actual_width: SIDEBAR_WIDTH,
            recent_open: false,
            native_drop_zones: Vec::new(),
        }
    }
}

impl SidebarUiState {
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn total_width(&self) -> f32 {
        if self.open {
            self.actual_width + BUMPER_WIDTH
        } else {
            BUMPER_WIDTH
        }
    }

    pub fn clear_native_drop_zones(&mut self) {
        self.native_drop_zones.clear();
    }

    pub fn register_native_drop_zone(&mut self, rect: egui::Rect, folder: PathBuf) {
        self.native_drop_zones
            .push(SidebarDropZone { rect, folder });
    }

    pub fn folder_at_logical_pos(&self, pos: egui::Pos2) -> Option<PathBuf> {
        self.native_drop_zones
            .iter()
            .rev()
            .find(|zone| zone.rect.contains(pos))
            .map(|zone| zone.folder.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_at_logical_pos_prefers_latest_matching_zone() {
        let mut state = SidebarUiState::default();
        state.register_native_drop_zone(
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0)),
            PathBuf::from("/repo"),
        );
        state.register_native_drop_zone(
            egui::Rect::from_min_max(egui::pos2(10.0, 10.0), egui::pos2(90.0, 90.0)),
            PathBuf::from("/repo/src"),
        );

        assert_eq!(
            state.folder_at_logical_pos(egui::pos2(20.0, 20.0)),
            Some(PathBuf::from("/repo/src"))
        );
    }
}
