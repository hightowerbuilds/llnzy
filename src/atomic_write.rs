use std::ffi::OsString;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub enum AtomicWriteError {
    InvalidPath,
    Write(io::Error),
    Permissions(io::Error),
    Sync(io::Error),
    Rename(io::Error),
    SyncParent(io::Error),
}

impl fmt::Display for AtomicWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AtomicWriteError::InvalidPath => write!(f, "Invalid file path"),
            AtomicWriteError::Write(err) => write!(f, "Write failed: {err}"),
            AtomicWriteError::Permissions(err) => write!(f, "Set permissions failed: {err}"),
            AtomicWriteError::Sync(err) => write!(f, "Sync failed: {err}"),
            AtomicWriteError::Rename(err) => write!(f, "Rename failed: {err}"),
            AtomicWriteError::SyncParent(err) => write!(f, "Sync parent failed: {err}"),
        }
    }
}

impl std::error::Error for AtomicWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AtomicWriteError::InvalidPath => None,
            AtomicWriteError::Write(err)
            | AtomicWriteError::Permissions(err)
            | AtomicWriteError::Sync(err)
            | AtomicWriteError::Rename(err)
            | AtomicWriteError::SyncParent(err) => Some(err),
        }
    }
}

pub fn atomic_write(path: &Path, contents: impl AsRef<[u8]>) -> Result<(), AtomicWriteError> {
    let dir = parent_dir(path)?;
    let temp_path = temp_path_for(path, dir)?;
    let existing_permissions = fs::metadata(path)
        .ok()
        .map(|metadata| metadata.permissions());

    let result = write_temp_and_rename(
        path,
        dir,
        &temp_path,
        contents.as_ref(),
        existing_permissions,
    );
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn parent_dir(path: &Path) -> Result<&Path, AtomicWriteError> {
    let dir = path.parent().ok_or(AtomicWriteError::InvalidPath)?;
    if dir.as_os_str().is_empty() {
        Ok(Path::new("."))
    } else {
        Ok(dir)
    }
}

fn temp_path_for(path: &Path, dir: &Path) -> Result<PathBuf, AtomicWriteError> {
    let file_name = path.file_name().ok_or(AtomicWriteError::InvalidPath)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(format!(".llnzy-{}-{now}-{counter}.tmp", process::id()));
    Ok(dir.join(temp_name))
}

fn write_temp_and_rename(
    path: &Path,
    dir: &Path,
    temp_path: &Path,
    contents: &[u8],
    existing_permissions: Option<fs::Permissions>,
) -> Result<(), AtomicWriteError> {
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(temp_path)
            .map_err(AtomicWriteError::Write)?;
        file.write_all(contents).map_err(AtomicWriteError::Write)?;
        if let Some(permissions) = existing_permissions {
            fs::set_permissions(temp_path, permissions).map_err(AtomicWriteError::Permissions)?;
        }
        file.sync_all().map_err(AtomicWriteError::Sync)?;
    }

    fs::rename(temp_path, path).map_err(AtomicWriteError::Rename)?;
    sync_parent_dir(dir).map_err(AtomicWriteError::SyncParent)
}

fn sync_parent_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_replaces_existing_contents() {
        let dir = test_temp_dir("atomic-write-replace");
        let path = dir.join("file.txt");
        fs::write(&path, "old").unwrap();

        atomic_write(&path, "new").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_preserves_existing_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = test_temp_dir("atomic-write-permissions");
        let path = dir.join("file.txt");
        fs::write(&path, "old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();

        atomic_write(&path, "new").unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640);
        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn atomic_write_rejects_path_without_file_name() {
        assert!(matches!(
            atomic_write(Path::new(""), "contents"),
            Err(AtomicWriteError::InvalidPath)
        ));
    }

    fn test_temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("{name}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
