use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveOrigin {
    DragDrop,
    ContextMenu,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarMoveRequest {
    pub sources: Vec<PathBuf>,
    pub destination_folder: PathBuf,
    pub origin: MoveOrigin,
}

impl SidebarMoveRequest {
    pub fn new(sources: Vec<PathBuf>, destination_folder: PathBuf, origin: MoveOrigin) -> Self {
        Self {
            sources,
            destination_folder,
            origin,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarMovePlanItem {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub is_dir: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarMovePlan {
    pub destination_folder: PathBuf,
    pub items: Vec<SidebarMovePlanItem>,
}

impl SidebarMovePlan {
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
