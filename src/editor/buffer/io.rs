use std::fs;
use std::path::Path;

use ropey::Rope;

use crate::editor::history::UndoHistory;
use crate::text_utils::normalize_crlf_to_lf;

use super::indent::IndentStyle;
use super::kind::BufferKind;
use super::model::content_hash;
use super::Buffer;

/// Detected line ending style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    CrLf,
}

impl LineEnding {
    pub fn as_str(self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
        }
    }

    /// Detect the dominant line ending in a string.
    pub(super) fn detect(text: &str) -> Self {
        let crlf = text.matches("\r\n").count();
        let lf = text.matches('\n').count().saturating_sub(crlf);
        if crlf > lf {
            LineEnding::CrLf
        } else {
            LineEnding::Lf
        }
    }
}

impl Buffer {
    /// Load a buffer from a file on disk.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("Cannot read file: {e}"))?;
        let line_ending = LineEnding::detect(&text);
        let indent_style = IndentStyle::detect(&text);

        // Normalize to LF internally; restore the original line ending on save.
        let normalized = normalize_crlf_to_lf(&text);
        let rope = Rope::from_str(normalized.as_ref());
        let hash = content_hash(&rope);

        Ok(Self {
            rope,
            path: Some(path.to_path_buf()),
            line_ending,
            modified: false,
            saved_hash: hash,
            history: UndoHistory::new(),
            last_edit: None,
            indent_style,
            kind: BufferKind::Code,
        })
    }

    /// Save buffer to its associated file path.
    pub fn save(&mut self) -> Result<(), String> {
        let path = self.path.clone().ok_or("No file path set")?;
        self.save_to(&path)
    }

    /// Save buffer to the given path (also updates the buffer's path).
    pub fn save_to(&mut self, path: &Path) -> Result<(), String> {
        let mut content = String::with_capacity(self.rope.len_bytes());
        for chunk in self.rope.chunks() {
            content.push_str(chunk);
        }

        // Convert internal LF to the file's original line ending.
        if self.line_ending == LineEnding::CrLf {
            content = content.replace('\n', "\r\n");
        }

        // Atomic write: write to temp file, then rename.
        let dir = path.parent().ok_or("Invalid file path")?;
        let temp_name = format!(
            ".llnzy-save-{}.tmp",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let temp_path = dir.join(temp_name);

        fs::write(&temp_path, &content).map_err(|e| format!("Write failed: {e}"))?;
        fs::rename(&temp_path, path).map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            format!("Rename failed: {e}")
        })?;

        self.path = Some(path.to_path_buf());
        self.saved_hash = content_hash(&self.rope);
        self.modified = false;
        self.history.mark_saved();
        Ok(())
    }
}
