use std::fs;
use std::path::{Path, PathBuf};

use crate::editor::buffer::Buffer;

const MAX_FILE_SIZE: u64 = 10_485_760; // 10 MB (increased for editor)
const MAX_IMAGE_SIZE: u64 = 20_971_520; // 20 MB

pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
}

pub enum FileContent {
    Text(Buffer),
    Image {
        rgba: Vec<u8>,
        width: u32,
        height: u32,
        texture: Option<egui::TextureHandle>,
    },
}

pub struct OpenFile {
    pub path: PathBuf,
    pub name: String,
    pub content: FileContent,
}

pub struct ExplorerState {
    pub current_dir: PathBuf,
    pub entries: Vec<DirEntry>,
    pub open_file: Option<OpenFile>,
    pub error: Option<String>,
}

impl Default for ExplorerState {
    fn default() -> Self {
        Self::new()
    }
}

impl ExplorerState {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let mut state = ExplorerState {
            current_dir: home,
            entries: Vec::new(),
            open_file: None,
            error: None,
        };
        state.refresh();
        state
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        self.error = None;

        let read_dir = match fs::read_dir(&self.current_dir) {
            Ok(rd) => rd,
            Err(e) => {
                self.error = Some(format!("Cannot read directory: {e}"));
                return;
            }
        };

        let mut dirs = Vec::new();
        let mut files = Vec::new();

        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let meta = entry.metadata().ok();
            let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());
            let size = meta.as_ref().map_or(0, |m| m.len());

            let de = DirEntry {
                name,
                path,
                is_dir,
                size,
            };

            if is_dir {
                dirs.push(de);
            } else {
                files.push(de);
            }
        }

        dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        self.entries.reserve(dirs.len() + files.len());
        self.entries.append(&mut dirs);
        self.entries.append(&mut files);
    }

    pub fn navigate(&mut self, path: PathBuf) {
        self.current_dir = path;
        self.open_file = None;
        self.refresh();
    }

    pub fn go_up(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            self.navigate(parent.to_path_buf());
        }
    }

    pub fn open(&mut self, path: PathBuf) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".to_string());

        if is_image(&path) {
            self.open_image(path, name);
        } else {
            self.open_text(path, name);
        }
    }

    fn open_text(&mut self, path: PathBuf, name: String) {
        match fs::metadata(&path) {
            Ok(meta) if meta.len() > MAX_FILE_SIZE => {
                self.error = Some(format!(
                    "File too large to edit ({:.0} MB limit)",
                    MAX_FILE_SIZE as f64 / 1_048_576.0
                ));
                return;
            }
            Err(e) => {
                self.error = Some(format!("Cannot read file: {e}"));
                return;
            }
            _ => {}
        }

        match Buffer::from_file(&path) {
            Ok(buf) => {
                self.error = None;
                self.open_file = Some(OpenFile {
                    path,
                    name,
                    content: FileContent::Text(buf),
                });
            }
            Err(e) => {
                self.error = Some(e);
            }
        }
    }

    fn open_image(&mut self, path: PathBuf, name: String) {
        match fs::metadata(&path) {
            Ok(meta) if meta.len() > MAX_IMAGE_SIZE => {
                self.error = Some(format!(
                    "Image too large to preview ({:.0} MB limit)",
                    MAX_IMAGE_SIZE as f64 / 1_048_576.0
                ));
                return;
            }
            Err(e) => {
                self.error = Some(format!("Cannot read file: {e}"));
                return;
            }
            _ => {}
        }

        match image::open(&path) {
            Ok(img) => {
                let rgba_image = img.to_rgba8();
                let width = rgba_image.width();
                let height = rgba_image.height();
                let rgba = rgba_image.into_raw();
                self.error = None;
                self.open_file = Some(OpenFile {
                    path,
                    name,
                    content: FileContent::Image {
                        rgba,
                        width,
                        height,
                        texture: None,
                    },
                });
            }
            Err(e) => {
                self.error = Some(format!("Cannot decode image: {e}"));
            }
        }
    }

    pub fn close_file(&mut self) {
        self.open_file = None;
        self.error = None;
    }
}

fn is_image(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tiff" | "tif" | "ico"
    )
}

/// Format a byte count for display.
pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1_048_576 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    }
}
