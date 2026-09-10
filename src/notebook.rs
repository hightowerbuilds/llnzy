//! Chronological personal notes. Kept outside projects and course content.
use std::{io, path::Path};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub id: u64,
    pub created_ms: u64,
    #[serde(default)]
    pub title: String,
    pub body: String,
}

impl Note {
    pub fn title(&self) -> String {
        if !self.title.trim().is_empty() {
            return self.title.trim().to_string();
        }
        self.body
            .lines()
            .find(|line| !line.trim().is_empty())
            .map(|line| line.trim().chars().take(64).collect())
            .unwrap_or_else(|| "Untitled note".into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Notebook {
    version: u32,
    pub notes: Vec<Note>,
}

impl Default for Notebook {
    fn default() -> Self {
        Self {
            version: 1,
            notes: Vec::new(),
        }
    }
}

impl Notebook {
    pub fn load(path: &Path) -> io::Result<Self> {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(err) => return Err(err),
        };
        let mut book: Self = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        let mut ids = std::collections::HashSet::new();
        if book.version != 1
            || book
                .notes
                .iter()
                .any(|note| note.id == u64::MAX || !ids.insert(note.id))
        {
            return Err(io::Error::other("Unsupported or invalid notebook"));
        }
        book.notes
            .sort_by_key(|note| std::cmp::Reverse((note.created_ms, note.id)));
        Ok(book)
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        crate::atomic_write::atomic_write(path, bytes).map_err(io::Error::other)
    }

    pub fn new_note(&mut self, created_ms: u64) -> u64 {
        // Reuse an empty draft instead of filling the history with empty entries.
        if let Some(note) = self
            .notes
            .iter()
            .find(|note| note.body.trim().is_empty() && note.title.trim().is_empty())
        {
            return note.id;
        }
        let id = self.notes.iter().map(|note| note.id).max().unwrap_or(0) + 1;
        self.notes.push(Note {
            id,
            created_ms,
            title: String::new(),
            body: String::new(),
        });
        self.notes
            .sort_by_key(|note| std::cmp::Reverse((note.created_ms, note.id)));
        id
    }

    pub fn update(&mut self, id: u64, body: String) -> bool {
        let Some(note) = self.notes.iter_mut().find(|note| note.id == id) else {
            return false;
        };
        if note.body == body {
            return false;
        }
        note.body = body;
        true
    }

    pub fn update_title(&mut self, id: u64, title: String) -> bool {
        let Some(note) = self.notes.iter_mut().find(|note| note.id == id) else {
            return false;
        };
        if note.title == title {
            return false;
        }
        note.title = title;
        true
    }
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn date_label(ms: u64) -> String {
    #[cfg(unix)]
    {
        let seconds = (ms / 1000) as libc::time_t;
        let mut result = std::mem::MaybeUninit::<libc::tm>::uninit();
        // SAFETY: both pointers reference valid stack allocations; only read
        // the initialized result after localtime_r reports success.
        unsafe {
            if !libc::localtime_r(&seconds, result.as_mut_ptr()).is_null() {
                let tm = result.assume_init();
                return format!(
                    "{:04}-{:02}-{:02} · {:02}:{:02}",
                    tm.tm_year + 1900,
                    tm.tm_mon + 1,
                    tm.tm_mday,
                    tm.tm_hour,
                    tm.tm_min
                );
            }
        }
    }
    format!("{} seconds since epoch", ms / 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chronology_survives_edits_and_equal_timestamps() {
        let mut book = Notebook::default();
        let first = book.new_note(100);
        book.update(first, "First lesson".into());
        let second = book.new_note(100);
        book.update(second, "Second lesson".into());
        book.update(first, "Revised first lesson".into());
        assert_eq!(
            book.notes.iter().map(|n| n.id).collect::<Vec<_>>(),
            [second, first]
        );
        assert_eq!(book.notes[1].created_ms, 100);
    }

    #[test]
    fn empty_drafts_are_reused_and_titles_are_unicode_safe() {
        let mut book = Notebook::default();
        let id = book.new_note(100);
        assert_eq!(book.new_note(200), id);
        book.update(id, format!("\n {}", "🦀".repeat(100)));
        assert_eq!(book.notes[0].title(), "🦀".repeat(64));
        assert_ne!(book.new_note(300), id);
    }

    #[test]
    fn roundtrip_and_corrupt_file_protection() {
        let dir =
            std::env::temp_dir().join(format!("llnzy-notes-{}-{}", std::process::id(), now_ms()));
        let path = dir.join("notes.json");
        let mut book = Notebook::load(&path).unwrap();
        let id = book.new_note(100);
        book.update(id, "Lesson notes\n日本語 🦀\n".into());
        book.update_title(id, "Rust — ownership 🦀".into());
        book.save(&path).unwrap();
        assert_eq!(Notebook::load(&path).unwrap(), book);
        std::fs::write(&path, "broken").unwrap();
        assert!(Notebook::load(&path).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "broken");
        std::fs::write(&path, r#"{"version":2,"notes":[]}"#).unwrap();
        assert!(Notebook::load(&path).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn titles_are_independent_and_title_only_notes_are_not_empty_drafts() {
        let mut book = Notebook::default();
        let first = book.new_note(100);
        book.update_title(first, "Questions for tomorrow".into());
        let second = book.new_note(200);
        assert_ne!(first, second);
        book.update(first, "Why does this borrow fail?".into());
        assert_eq!(book.notes[1].title(), "Questions for tomorrow");
        book.update_title(first, "Ownership".into());
        assert_eq!(book.notes[1].created_ms, 100);
        assert_eq!(book.notes[1].body, "Why does this borrow fail?");
        assert_eq!(book.notes[1].title(), "Ownership");
    }

    #[test]
    fn older_notebooks_load_without_losing_their_derived_titles() {
        let book: Notebook = serde_json::from_str(
            r#"{"version":1,"notes":[{"id":1,"created_ms":100,"body":"Existing note\nKeep this text"}]}"#
        ).unwrap();
        assert_eq!(book.notes[0].title, "");
        assert_eq!(book.notes[0].title(), "Existing note");
        assert_eq!(book.notes[0].body, "Existing note\nKeep this text");
    }
}
