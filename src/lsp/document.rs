use std::borrow::Cow;
#[cfg(unix)]
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use lsp_types::Uri;
use rustc_hash::FxHashMap;

pub(crate) fn path_to_uri(path: &Path) -> Result<Uri, String> {
    let abs = document_key(path);
    let s = format!("file://{}", percent_encode_path(&abs)?);
    s.parse::<Uri>().map_err(|e| e.to_string())
}

/// Convert a URI back to a file path.
pub(crate) fn uri_to_path(uri: &Uri) -> Option<PathBuf> {
    let rest = uri.as_str().strip_prefix("file://")?;
    let path = if rest.starts_with('/') {
        Cow::Borrowed(rest)
    } else {
        let (host, path) = rest.split_once('/')?;
        if !host.eq_ignore_ascii_case("localhost") {
            return None;
        }
        Cow::Owned(format!("/{path}"))
    };
    if path.contains(['?', '#']) {
        return None;
    }
    decoded_path_buf(percent_decode(&path)?)
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

fn percent_encode_path(path: &Path) -> Result<String, String> {
    let bytes = path_bytes(path)?;
    let mut encoded = String::with_capacity(bytes.len());
    for &byte in bytes {
        if is_file_uri_path_byte(byte) {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push(HEX[(byte >> 4) as usize] as char);
            encoded.push(HEX[(byte & 0x0f) as usize] as char);
        }
    }
    Ok(encoded)
}

#[cfg(unix)]
fn path_bytes(path: &Path) -> Result<&[u8], String> {
    Ok(path.as_os_str().as_bytes())
}

#[cfg(not(unix))]
fn path_bytes(path: &Path) -> Result<&[u8], String> {
    path.to_str()
        .map(str::as_bytes)
        .ok_or_else(|| format!("File path is not valid UTF-8: {}", path.display()))
}

const HEX: &[u8; 16] = b"0123456789ABCDEF";

fn is_file_uri_path_byte(byte: u8) -> bool {
    matches!(
        byte,
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/'
    )
}

fn percent_decode(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut idx = 0;
    while idx < bytes.len() {
        if bytes[idx] == b'%' {
            let hi = hex_value(*bytes.get(idx + 1)?)?;
            let lo = hex_value(*bytes.get(idx + 2)?)?;
            decoded.push((hi << 4) | lo);
            idx += 3;
        } else {
            decoded.push(bytes[idx]);
            idx += 1;
        }
    }
    Some(decoded)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(unix)]
fn decoded_path_buf(bytes: Vec<u8>) -> Option<PathBuf> {
    Some(OsString::from_vec(bytes).into())
}

#[cfg(not(unix))]
fn decoded_path_buf(bytes: Vec<u8>) -> Option<PathBuf> {
    String::from_utf8(bytes).ok().map(PathBuf::from)
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
    docs: FxHashMap<PathBuf, DocumentSnapshot>,
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
    fn path_to_uri_percent_encodes_file_paths() {
        let path = Path::new("/tmp/llnzy dir/main#% β.rs");

        let uri = path_to_uri(path).unwrap();

        assert_eq!(
            uri.as_str(),
            "file:///tmp/llnzy%20dir/main%23%25%20%CE%B2.rs"
        );
        assert_eq!(uri_to_path(&uri), Some(path.to_path_buf()));
    }

    #[test]
    fn uri_to_path_decodes_encoded_file_uris() {
        let uri: Uri = "file:///tmp/llnzy%20dir/main%23%25.rs".parse().unwrap();

        assert_eq!(
            uri_to_path(&uri),
            Some(PathBuf::from("/tmp/llnzy dir/main#%.rs"))
        );
    }

    #[test]
    fn uri_to_path_accepts_localhost_file_uris() {
        let uri: Uri = "file://localhost/tmp/llnzy%20dir/main.rs".parse().unwrap();

        assert_eq!(
            uri_to_path(&uri),
            Some(PathBuf::from("/tmp/llnzy dir/main.rs"))
        );
    }

    #[test]
    fn uri_to_path_rejects_non_file_or_ambiguous_uris() {
        let https_uri: Uri = "https://example.com/main.rs".parse().unwrap();
        let remote_file_uri: Uri = "file://server/share/main.rs".parse().unwrap();
        let fragment_uri: Uri = "file:///tmp/main.rs#fragment".parse().unwrap();

        assert_eq!(uri_to_path(&https_uri), None);
        assert_eq!(uri_to_path(&remote_file_uri), None);
        assert_eq!(uri_to_path(&fragment_uri), None);
    }

    #[cfg(unix)]
    #[test]
    fn path_to_uri_round_trips_non_utf8_unix_paths() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let path = PathBuf::from(OsString::from_vec(b"/tmp/llnzy-\xFF.rs".to_vec()));

        let uri = path_to_uri(&path).unwrap();

        assert_eq!(uri.as_str(), "file:///tmp/llnzy-%FF.rs");
        assert_eq!(uri_to_path(&uri), Some(path));
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
