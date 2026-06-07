use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::buffer::Buffer;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecoverySnapshot {
    pub path: PathBuf,
    pub saved_at_unix_secs: u64,
    pub content: String,
}

pub fn clear_path_snapshot(path: &Path) -> Result<(), String> {
    let Some(dir) = recovery_dir() else {
        return Ok(());
    };
    clear_path_snapshot_in(&dir, path)
}

pub fn load_snapshot(path: &Path) -> Result<Option<RecoverySnapshot>, String> {
    let Some(dir) = recovery_dir() else {
        return Ok(None);
    };
    load_snapshot_in(&dir, path)
}

pub fn save_or_clear_buffer_snapshot_in(
    recovery_dir: &Path,
    buffer: &Buffer,
) -> Result<Option<PathBuf>, String> {
    let Some(path) = buffer.path() else {
        return Ok(None);
    };
    if !buffer.is_modified() {
        clear_path_snapshot_in(recovery_dir, path)?;
        return Ok(None);
    }

    fs::create_dir_all(recovery_dir)
        .map_err(|err| format!("Failed to create editor recovery dir: {err}"))?;
    let snapshot = RecoverySnapshot {
        path: path.to_path_buf(),
        saved_at_unix_secs: now_unix_secs(),
        content: buffer.text(),
    };
    let text = toml::to_string_pretty(&snapshot)
        .map_err(|err| format!("Failed to serialize editor recovery snapshot: {err}"))?;
    let path = snapshot_path_in(recovery_dir, path);
    atomic_write(&path, &text)?;
    Ok(Some(path))
}

pub fn clear_path_snapshot_in(recovery_dir: &Path, path: &Path) -> Result<(), String> {
    let snapshot_path = snapshot_path_in(recovery_dir, path);
    match fs::remove_file(&snapshot_path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "Failed to clear editor recovery snapshot {}: {err}",
            snapshot_path.display()
        )),
    }
}

pub fn load_snapshot_in(
    recovery_dir: &Path,
    path: &Path,
) -> Result<Option<RecoverySnapshot>, String> {
    let snapshot_path = snapshot_path_in(recovery_dir, path);
    let text = match fs::read_to_string(&snapshot_path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(format!(
                "Failed to read editor recovery snapshot {}: {err}",
                snapshot_path.display()
            ));
        }
    };
    let snapshot = toml::from_str::<RecoverySnapshot>(&text).map_err(|err| {
        format!(
            "Failed to parse editor recovery snapshot {}: {err}",
            snapshot_path.display()
        )
    })?;
    Ok(Some(snapshot))
}

fn recovery_dir() -> Option<PathBuf> {
    crate::platform::paths::current_paths().map(|paths| paths.data_dir.join("editor-recovery"))
}

fn snapshot_path_in(recovery_dir: &Path, path: &Path) -> PathBuf {
    recovery_dir.join(format!("{:016x}.toml", stable_path_id(path)))
}

fn stable_path_id(path: &Path) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;

    let mut hash = OFFSET;
    for byte in path.to_string_lossy().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn atomic_write(path: &Path, text: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Invalid recovery snapshot path".to_string())?;
    let tmp = parent.join(format!(
        ".{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("editor-recovery")
    ));
    fs::write(&tmp, text).map_err(|err| format!("Failed to write recovery snapshot: {err}"))?;
    fs::rename(&tmp, path).map_err(|err| {
        let _ = fs::remove_file(&tmp);
        format!("Failed to commit recovery snapshot: {err}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::buffer::{Buffer, Position};

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("llnzy-recovery-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn dirty_buffer_writes_recovery_snapshot() {
        let dir = temp_dir("dirty");
        let file = dir.join("note.txt");
        fs::write(&file, "one").unwrap();

        let mut buffer = Buffer::from_file(&file).unwrap();
        buffer.insert(Position::new(0, 3), " two");

        let recovery_dir = dir.join("recovery");
        let snapshot_path = save_or_clear_buffer_snapshot_in(&recovery_dir, &buffer)
            .unwrap()
            .unwrap();
        let snapshot = load_snapshot_in(&recovery_dir, &file).unwrap().unwrap();

        assert!(snapshot_path.exists());
        assert_eq!(snapshot.path, file);
        assert_eq!(snapshot.content, "one two");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn clean_buffer_clears_existing_recovery_snapshot() {
        let dir = temp_dir("clean");
        let file = dir.join("note.txt");
        fs::write(&file, "one").unwrap();

        let mut buffer = Buffer::from_file(&file).unwrap();
        buffer.insert(Position::new(0, 3), " two");
        let recovery_dir = dir.join("recovery");
        save_or_clear_buffer_snapshot_in(&recovery_dir, &buffer)
            .unwrap()
            .unwrap();
        buffer.save().unwrap();

        assert!(save_or_clear_buffer_snapshot_in(&recovery_dir, &buffer)
            .unwrap()
            .is_none());
        assert!(load_snapshot_in(&recovery_dir, &file).unwrap().is_none());
        let _ = fs::remove_dir_all(dir);
    }
}
