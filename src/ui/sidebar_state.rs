use super::types::{BUMPER_WIDTH, SIDEBAR_WIDTH};

pub struct SidebarUiState {
    pub open: bool,
    pub actual_width: f32,
}

impl Default for SidebarUiState {
    fn default() -> Self {
        Self {
            open: false,
            actual_width: SIDEBAR_WIDTH,
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
}
