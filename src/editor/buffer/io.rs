use std::fs;
use std::path::Path;

use ropey::Rope;

use crate::atomic_write::atomic_write;
use crate::editor::editorconfig::{Charset, EndOfLine};
use crate::editor::history::UndoHistory;
use crate::text_utils::normalize_crlf_and_cr_to_lf;

use super::indent::IndentStyle;
use super::kind::BufferKind;
use super::model::content_hash;
use super::Buffer;

/// Detected line ending style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    CrLf,
    Cr,
}

impl LineEnding {
    pub fn as_str(self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
            LineEnding::Cr => "\r",
        }
    }

    /// Detect the dominant line ending in a string.
    pub(super) fn detect(text: &str) -> Self {
        let crlf = text.matches("\r\n").count();
        let lf = text.matches('\n').count().saturating_sub(crlf);
        let cr = text.matches('\r').count().saturating_sub(crlf);
        if crlf > lf && crlf > cr {
            LineEnding::CrLf
        } else if cr > lf && cr > crlf {
            LineEnding::Cr
        } else {
            LineEnding::Lf
        }
    }
}

impl Buffer {
    /// Load a buffer from a file on disk.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let mut text = fs::read_to_string(path).map_err(|e| format!("Cannot read file: {e}"))?;
        if let Some(stripped) = text.strip_prefix('\u{feff}') {
            text = stripped.to_string();
        }
        let line_ending = LineEnding::detect(&text);
        let indent_style = IndentStyle::detect(&text);

        // Normalize to LF internally; restore the original line ending on save.
        let normalized = normalize_crlf_and_cr_to_lf(&text);
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
            insert_final_newline: None,
            trim_trailing_whitespace: None,
            eol_override: None,
            charset_override: None,
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

        let mut internal_content = content;
        apply_save_policies(&mut internal_content, self);
        let save_line_ending = self.save_line_ending();
        let mut file_content = internal_content.clone();

        // Convert internal LF to the file's configured line ending.
        if save_line_ending != LineEnding::Lf {
            file_content = file_content.replace('\n', save_line_ending.as_str());
        }

        let bytes = self.encode_save_bytes(&file_content);
        atomic_write(path, bytes.as_ref()).map_err(|err| err.to_string())?;

        self.rope = Rope::from_str(&internal_content);
        self.line_ending = save_line_ending;
        self.path = Some(path.to_path_buf());
        self.saved_hash = content_hash(&self.rope);
        self.modified = false;
        self.history.mark_saved();
        Ok(())
    }

    fn save_line_ending(&self) -> LineEnding {
        match self.eol_override {
            Some(EndOfLine::Lf) => LineEnding::Lf,
            Some(EndOfLine::Crlf) => LineEnding::CrLf,
            Some(EndOfLine::Cr) => LineEnding::Cr,
            None => self.line_ending,
        }
    }

    fn encode_save_bytes<'a>(&self, content: &'a str) -> std::borrow::Cow<'a, [u8]> {
        match self.charset_override {
            Some(Charset::Utf8Bom) => {
                let mut bytes = Vec::with_capacity(3 + content.len());
                bytes.extend_from_slice(b"\xEF\xBB\xBF");
                bytes.extend_from_slice(content.as_bytes());
                std::borrow::Cow::Owned(bytes)
            }
            _ => std::borrow::Cow::Borrowed(content.as_bytes()),
        }
    }
}

fn apply_save_policies(content: &mut String, buffer: &Buffer) {
    if buffer.trim_trailing_whitespace == Some(true) {
        *content = trim_trailing_whitespace(content);
    }

    match buffer.insert_final_newline {
        Some(true) if !content.ends_with('\n') => content.push('\n'),
        Some(false) => {
            while content.ends_with('\n') {
                content.pop();
            }
        }
        _ => {}
    }
}

fn trim_trailing_whitespace(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    for segment in content.split_inclusive('\n') {
        if let Some(line) = segment.strip_suffix('\n') {
            out.push_str(line.trim_end_matches([' ', '\t']));
            out.push('\n');
        } else {
            out.push_str(segment.trim_end_matches([' ', '\t']));
        }
    }
    out
}
