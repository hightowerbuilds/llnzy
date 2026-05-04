use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenTarget {
    Url(String),
    File(PathBuf),
    Folder(PathBuf),
    Reveal(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenRequest {
    pub target: OpenTarget,
    pub fallback_to_parent_folder: bool,
}
