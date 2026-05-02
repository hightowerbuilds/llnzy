use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lsp_types::Uri;

pub(crate) fn path_to_uri(path: &Path) -> Result<Uri, String> {
    let abs = document_key(path);
    let s = format!("file://{}", abs.display());
    s.parse::<Uri>().map_err(|e| e.to_string())
}

fn document_key(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DocumentSnapshot {
    pub uri: Uri,
    pub version: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenAction {
    Open,
    Change,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DocumentOpen {
    pub action: OpenAction,
    pub document: DocumentSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DocumentMove {
    pub close_old: DocumentSnapshot,
    pub open_new: DocumentSnapshot,
}

/// Owns client-side LSP text document lifecycle state.
#[derive(Debug, Default)]
pub(crate) struct DocumentStore {
    docs: HashMap<PathBuf, DocumentSnapshot>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, path: &Path, uri: Uri) -> DocumentOpen {
        let key = document_key(path);
        match self.docs.get_mut(&key) {
            Some(doc) => {
                doc.uri = uri;
                doc.version += 1;
                DocumentOpen {
                    action: OpenAction::Change,
                    document: doc.clone(),
                }
            }
            None => {
                let document = DocumentSnapshot { uri, version: 1 };
                self.docs.insert(key, document.clone());
                DocumentOpen {
                    action: OpenAction::Open,
                    document,
                }
            }
        }
    }

    pub fn change(&mut self, path: &Path) -> Option<DocumentSnapshot> {
        let doc = self.docs.get_mut(&document_key(path))?;
        doc.version += 1;
        Some(doc.clone())
    }

    pub fn save(&self, path: &Path) -> Option<DocumentSnapshot> {
        self.docs.get(&document_key(path)).cloned()
    }

    pub fn close(&mut self, path: &Path) -> Option<DocumentSnapshot> {
        self.docs.remove(&document_key(path))
    }

    pub fn move_path(
        &mut self,
        old_path: &Path,
        new_path: &Path,
        new_uri: Uri,
    ) -> Option<DocumentMove> {
        let close_old = self.docs.remove(&document_key(old_path))?;
        let open_new = DocumentSnapshot {
            uri: new_uri,
            version: 1,
        };
        self.docs.insert(document_key(new_path), open_new.clone());
        Some(DocumentMove {
            close_old,
            open_new,
        })
    }

    pub fn uri(&self, path: &Path) -> Option<Uri> {
        self.docs
            .get(&document_key(path))
            .map(|doc| doc.uri.clone())
    }

    #[cfg(test)]
    fn version(&self, path: &Path) -> Option<i32> {
        self.docs.get(&document_key(path)).map(|doc| doc.version)
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.docs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uri(name: &str) -> Uri {
        format!("file:///tmp/{name}").parse().unwrap()
    }

    #[test]
    fn duplicate_open_updates_state_as_change() {
        let mut docs = DocumentStore::new();
        let path = Path::new("/tmp/main.rs");

        let first = docs.open(path, uri("main.rs"));
        assert_eq!(first.action, OpenAction::Open);
        assert_eq!(first.document.version, 1);

        let duplicate = docs.open(path, uri("main.rs"));
        assert_eq!(duplicate.action, OpenAction::Change);
        assert_eq!(duplicate.document.version, 2);
        assert_eq!(docs.version(path), Some(2));
        assert_eq!(docs.len(), 1);
    }

    #[test]
    fn change_increments_version_only_for_open_documents() {
        let mut docs = DocumentStore::new();
        let path = Path::new("/tmp/main.rs");

        assert_eq!(docs.change(path), None);
        docs.open(path, uri("main.rs"));

        let changed = docs.change(path).unwrap();
        assert_eq!(changed.version, 2);
        assert_eq!(docs.version(path), Some(2));
    }

    #[test]
    fn save_and_close_noop_after_close() {
        let mut docs = DocumentStore::new();
        let path = Path::new("/tmp/main.rs");

        assert_eq!(docs.save(path), None);
        assert_eq!(docs.close(path), None);

        docs.open(path, uri("main.rs"));
        assert!(docs.save(path).is_some());
        assert!(docs.close(path).is_some());

        assert_eq!(docs.save(path), None);
        assert_eq!(docs.change(path), None);
        assert_eq!(docs.close(path), None);
        assert_eq!(docs.len(), 0);
    }

    #[test]
    fn move_path_closes_old_uri_and_tracks_new_uri() {
        let mut docs = DocumentStore::new();
        let old_path = Path::new("/tmp/old.rs");
        let new_path = Path::new("/tmp/new.rs");
        let old_uri = uri("old.rs");
        let new_uri = uri("new.rs");

        docs.open(old_path, old_uri.clone());
        docs.change(old_path);

        let moved = docs.move_path(old_path, new_path, new_uri.clone()).unwrap();
        assert_eq!(moved.close_old.uri, old_uri);
        assert_eq!(moved.close_old.version, 2);
        assert_eq!(moved.open_new.uri, new_uri.clone());
        assert_eq!(moved.open_new.version, 1);

        assert_eq!(docs.uri(old_path), None);
        assert_eq!(docs.uri(new_path), Some(new_uri));
        assert_eq!(docs.version(new_path), Some(1));
        assert_eq!(docs.len(), 1);
    }
}
