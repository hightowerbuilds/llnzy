use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rustc_hash::{FxHashMap, FxHashSet};

pub mod cli;
pub mod commands;
pub mod draft;
pub mod formatting;
pub mod input;
pub mod queue;
pub mod session;
pub mod storage;
pub mod sync;
#[cfg(target_os = "macos")]
pub mod utf16;

use queue::{sanitize_prompt_queue, QueuedPrompt};

/// A saved prompt in the Stacker queue.
///
/// `id` is `Some(...)` for prompts backed by a `saved/<id>.md` file on disk
/// and `None` for in-memory prompts that haven't been persisted yet (newly
/// authored, freshly imported, or migrated from a legacy `stacker.json`).
/// The first persist after creation populates the id.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StackerPrompt {
    pub text: String,
    pub label: String,
    #[serde(default)]
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// `true` for unaccepted agent suggestions (file lives in `inbox/`),
    /// `false` for the user's saved library (file lives in `saved/`).
    #[serde(default, skip_serializing_if = "is_false")]
    pub inbox: bool,
    /// Agent that produced this prompt, recorded only for inbox suggestions.
    /// Surfaced as a tooltip on the inbox row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_agent: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

pub fn stacker_path() -> Option<PathBuf> {
    crate::platform::paths::current_paths().map(|paths| paths.stacker_file())
}

pub fn stacker_queue_path() -> Option<PathBuf> {
    crate::platform::paths::current_paths().map(|paths| paths.stacker_queue_file())
}

pub fn prompt_label(text: &str) -> String {
    let trimmed = text.trim();
    let words = trimmed
        .split_whitespace()
        .take(8)
        .collect::<Vec<_>>()
        .join(" ");

    if words.len() < trimmed.len() {
        format!("{}...", words)
    } else {
        words
    }
}

pub fn new_prompt(text: &str, category: &str) -> Option<StackerPrompt> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(StackerPrompt {
        text: trimmed.to_string(),
        label: prompt_label(trimmed),
        category: category.trim().to_string(),
        ..StackerPrompt::default()
    })
}

pub fn apply_prompt_edit(prompts: &mut [StackerPrompt], idx: usize, text: &str) -> bool {
    let Some(prompt) = prompts.get_mut(idx) else {
        return false;
    };

    prompt.text = text.trim().to_string();
    prompt.label = prompt_label(&prompt.text);
    true
}

pub fn merge_unique_prompts(
    existing: &mut Vec<StackerPrompt>,
    imported: impl IntoIterator<Item = StackerPrompt>,
) -> usize {
    let mut added = 0;

    for prompt in imported {
        if !existing.iter().any(|existing| existing.text == prompt.text) {
            existing.push(prompt);
            added += 1;
        }
    }

    added
}

pub fn load_prompts_from_path(path: &Path) -> Result<Vec<StackerPrompt>, String> {
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

/// Load the user's saved-prompt library from `$config/prompts/saved/`.
///
/// On first call after upgrade, runs the one-shot migration from a legacy
/// `stacker.json` at `$config/stacker.json` into `saved/<id>.md` files. The
/// migration is idempotent — repeated calls are no-ops once `saved/` is
/// populated.
pub fn load_saved_prompts() -> Vec<StackerPrompt> {
    let Some(paths) = crate::platform::paths::current_paths() else {
        return Vec::new();
    };
    load_saved_prompts_at(
        &paths.stacker_file(),
        &paths.prompts_saved_dir(),
        &paths.prompts_tmp_dir(),
    )
}

/// Load pending agent suggestions from `$config/prompts/inbox/`.
pub fn load_inbox_prompts() -> Vec<StackerPrompt> {
    let Some(paths) = crate::platform::paths::current_paths() else {
        return Vec::new();
    };
    load_inbox_prompts_filtered_at(
        &paths.prompts_inbox_dir(),
        &[paths.prompts_saved_dir(), paths.prompts_archive_dir()],
    )
}

pub fn load_inbox_prompts_at(inbox_dir: &Path) -> Vec<StackerPrompt> {
    load_inbox_prompts_filtered_at(inbox_dir, &[])
}

pub fn load_inbox_prompts_filtered_at(
    inbox_dir: &Path,
    suppressed_hash_dirs: &[PathBuf],
) -> Vec<StackerPrompt> {
    let suppressed_hashes = prompt_hashes_in_dirs(suppressed_hash_dirs);
    let records = storage::list(inbox_dir).unwrap_or_else(|err| {
        log::warn!(
            "failed to list inbox prompts at {}: {err}",
            inbox_dir.display()
        );
        Vec::new()
    });
    records
        .into_iter()
        .map(record_to_inbox_prompt)
        .filter(|prompt| !suppressed_hashes.contains(&storage::body_hash(&prompt.text)))
        .collect()
}

fn prompt_hashes_in_dirs(dirs: &[PathBuf]) -> FxHashSet<String> {
    let mut hashes = FxHashSet::default();
    for dir in dirs {
        match storage::list(dir) {
            Ok(records) => {
                hashes.extend(
                    records
                        .into_iter()
                        .map(|record| record.frontmatter.body_hash),
                );
            }
            Err(err) => log::warn!("failed to list prompt hashes at {}: {err}", dir.display()),
        }
    }
    hashes
}

/// Path-explicit variant of [`load_saved_prompts`] for tests and any future
/// caller that needs to point at a non-default config root.
pub fn load_saved_prompts_at(
    legacy_path: &Path,
    saved_dir: &Path,
    tmp_dir: &Path,
) -> Vec<StackerPrompt> {
    if let Err(err) = storage::migrate_legacy_json(legacy_path, saved_dir, tmp_dir) {
        log::warn!("legacy stacker.json migration failed: {err}");
    }
    let records = storage::list(saved_dir).unwrap_or_else(|err| {
        log::warn!(
            "failed to list saved prompts at {}: {err}",
            saved_dir.display()
        );
        Vec::new()
    });
    records.into_iter().map(record_to_prompt).collect()
}

/// Persist the in-memory prompt library against its previous on-disk shape.
///
/// `current` is the live list (may contain prompts with `id == None` that
/// haven't been written yet); `previous` is the snapshot returned by the
/// last successful `load_saved_prompts` / `persist_prompt_library` call.
/// Diff against `previous` decides which files to write, leave alone, or
/// archive. New prompts have an id assigned on first write.
pub fn persist_prompt_library(current: &mut [StackerPrompt], previous: &[StackerPrompt]) {
    let Some(paths) = crate::platform::paths::current_paths() else {
        return;
    };
    persist_prompt_library_at(
        current,
        previous,
        &paths.prompts_saved_dir(),
        &paths.prompts_archive_dir(),
        &paths.prompts_tmp_dir(),
    );
}

/// Path-explicit variant of [`persist_prompt_library`].
pub fn persist_prompt_library_at(
    current: &mut [StackerPrompt],
    previous: &[StackerPrompt],
    saved_dir: &Path,
    archive_dir: &Path,
    tmp_dir: &Path,
) {
    let prev_by_id: FxHashMap<&str, &StackerPrompt> = previous
        .iter()
        .filter_map(|p| p.id.as_deref().map(|id| (id, p)))
        .collect();

    let mut kept_ids: FxHashSet<String> = FxHashSet::default();

    for prompt in current.iter_mut() {
        match prompt.id.clone() {
            Some(id) => {
                kept_ids.insert(id.clone());
                let unchanged = prev_by_id.get(id.as_str()).is_some_and(|prev| {
                    prev.text == prompt.text
                        && prev.label == prompt.label
                        && prev.category == prompt.category
                });
                if unchanged {
                    continue;
                }
                let record = record_for_persist(id.clone(), prompt, saved_dir);
                if let Err(err) = storage::write_atomic(&record, saved_dir, tmp_dir) {
                    log::warn!("failed to update saved prompt {id}: {err}");
                }
            }
            None => {
                let id = storage::new_id();
                let record = record_for_persist(id.clone(), prompt, saved_dir);
                match storage::write_atomic(&record, saved_dir, tmp_dir) {
                    Ok(_) => {
                        prompt.id = Some(id.clone());
                        kept_ids.insert(id);
                    }
                    Err(err) => log::warn!("failed to write new saved prompt: {err}"),
                }
            }
        }
    }

    for prev in previous {
        let Some(id) = prev.id.as_deref() else {
            continue;
        };
        if kept_ids.contains(id) {
            continue;
        }
        let saved_path = saved_dir.join(format!("{id}.md"));
        if !saved_path.exists() {
            continue;
        }
        let archive_path = archive_dir.join(format!("{id}.md"));
        if let Err(err) = storage::rename_state(&saved_path, &archive_path) {
            log::warn!("failed to archive deleted prompt {id}: {err}");
        }
    }
}

fn record_to_prompt(record: storage::PromptRecord) -> StackerPrompt {
    StackerPrompt {
        text: record.body,
        label: record.frontmatter.label,
        category: record.frontmatter.category,
        id: Some(record.frontmatter.id),
        inbox: false,
        source_agent: None,
    }
}

fn record_to_inbox_prompt(record: storage::PromptRecord) -> StackerPrompt {
    StackerPrompt {
        text: record.body,
        label: record.frontmatter.label,
        category: record.frontmatter.category,
        id: Some(record.frontmatter.id),
        inbox: true,
        source_agent: record.frontmatter.source_agent,
    }
}

/// Promote one inbox prompt into the saved library, preserving its id and
/// creation timestamp while clearing inbox-only provenance from the saved row.
pub fn promote_inbox_prompt(prompt: &StackerPrompt) -> Result<StackerPrompt, String> {
    let Some(paths) = crate::platform::paths::current_paths() else {
        return Err("could not resolve config paths".to_string());
    };
    promote_inbox_prompt_at(
        prompt,
        &paths.prompts_inbox_dir(),
        &paths.prompts_saved_dir(),
        &paths.prompts_tmp_dir(),
    )
}

pub fn promote_inbox_prompt_at(
    prompt: &StackerPrompt,
    inbox_dir: &Path,
    saved_dir: &Path,
    tmp_dir: &Path,
) -> Result<StackerPrompt, String> {
    let id = prompt
        .id
        .as_deref()
        .ok_or_else(|| "inbox prompt is missing an id".to_string())?;
    let inbox_path = inbox_dir.join(format!("{id}.md"));
    let created = storage::read(&inbox_path)
        .map(|record| record.frontmatter.created)
        .unwrap_or_else(|_| storage::rfc3339_utc(SystemTime::now()));
    let mut saved = prompt.clone();
    saved.inbox = false;
    saved.source_agent = None;
    saved.id = Some(id.to_string());
    let record = storage::PromptRecord {
        frontmatter: storage::PromptFrontmatter {
            id: id.to_string(),
            state: storage::PromptState::Saved,
            label: saved.label.clone(),
            category: saved.category.clone(),
            created,
            source_agent: None,
            session_id: None,
            workspace: None,
            related_files: Vec::new(),
            body_hash: storage::body_hash(&saved.text),
        },
        body: saved.text.clone(),
    };
    storage::write_atomic(&record, saved_dir, tmp_dir)
        .map_err(|err| format!("failed to write saved prompt: {err}"))?;
    if let Err(err) = std::fs::remove_file(&inbox_path) {
        log::warn!(
            "failed to remove promoted inbox prompt {}: {err}",
            inbox_path.display()
        );
    }
    Ok(saved)
}

pub fn archive_inbox_prompt(prompt: &StackerPrompt) -> Result<(), String> {
    let Some(paths) = crate::platform::paths::current_paths() else {
        return Err("could not resolve config paths".to_string());
    };
    archive_inbox_prompt_at(
        prompt,
        &paths.prompts_inbox_dir(),
        &paths.prompts_archive_dir(),
        &paths.prompts_tmp_dir(),
    )
}

pub fn archive_saved_prompt(prompt: &StackerPrompt) -> Result<(), String> {
    let Some(paths) = crate::platform::paths::current_paths() else {
        return Err("could not resolve config paths".to_string());
    };
    archive_saved_prompt_at(
        prompt,
        &paths.prompts_saved_dir(),
        &paths.prompts_archive_dir(),
        &paths.prompts_tmp_dir(),
    )
}

pub fn archive_saved_prompt_at(
    prompt: &StackerPrompt,
    saved_dir: &Path,
    archive_dir: &Path,
    tmp_dir: &Path,
) -> Result<(), String> {
    let id = prompt
        .id
        .as_deref()
        .ok_or_else(|| "saved prompt is missing an id".to_string())?;
    let saved_path = saved_dir.join(format!("{id}.md"));
    let mut record = storage::read(&saved_path).unwrap_or_else(|_| {
        let mut record = record_for_persist(id.to_string(), prompt, saved_dir);
        record.frontmatter.state = storage::PromptState::Saved;
        record
    });
    record.frontmatter.state = storage::PromptState::Archived;
    record.frontmatter.body_hash = storage::body_hash(&record.body);
    storage::write_atomic(&record, archive_dir, tmp_dir)
        .map_err(|err| format!("failed to write archived prompt: {err}"))?;
    if saved_path.exists() {
        std::fs::remove_file(&saved_path).map_err(|err| {
            format!(
                "failed to remove saved prompt {}: {err}",
                saved_path.display()
            )
        })?;
    }
    Ok(())
}

pub fn archive_inbox_prompt_at(
    prompt: &StackerPrompt,
    inbox_dir: &Path,
    archive_dir: &Path,
    tmp_dir: &Path,
) -> Result<(), String> {
    let id = prompt
        .id
        .as_deref()
        .ok_or_else(|| "inbox prompt is missing an id".to_string())?;
    let inbox_path = inbox_dir.join(format!("{id}.md"));
    let mut record = storage::read(&inbox_path).map_err(|err| {
        format!(
            "failed to read inbox prompt {} before archive: {err}",
            inbox_path.display()
        )
    })?;
    record.frontmatter.state = storage::PromptState::Archived;
    record.frontmatter.body_hash = storage::body_hash(&record.body);
    storage::write_atomic(&record, archive_dir, tmp_dir)
        .map_err(|err| format!("failed to write archived prompt: {err}"))?;
    std::fs::remove_file(&inbox_path).map_err(|err| {
        format!(
            "failed to remove inbox prompt {}: {err}",
            inbox_path.display()
        )
    })
}

fn record_for_persist(
    id: String,
    prompt: &StackerPrompt,
    saved_dir: &Path,
) -> storage::PromptRecord {
    // Preserve `created` when overwriting; fall back to "now" for fresh writes.
    let target = saved_dir.join(format!("{id}.md"));
    let created = match storage::read(&target) {
        Ok(existing) => existing.frontmatter.created,
        Err(_) => storage::rfc3339_utc(SystemTime::now()),
    };
    storage::PromptRecord {
        frontmatter: storage::PromptFrontmatter {
            id,
            state: storage::PromptState::Saved,
            label: prompt.label.clone(),
            category: prompt.category.clone(),
            created,
            source_agent: None,
            session_id: None,
            workspace: None,
            related_files: Vec::new(),
            body_hash: storage::body_hash(&prompt.text),
        },
        body: prompt.text.clone(),
    }
}

pub fn load_queue_from_path(path: &Path) -> Result<Vec<QueuedPrompt>, String> {
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut queue: Vec<QueuedPrompt> = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    sanitize_prompt_queue(&mut queue);
    Ok(queue)
}

pub fn save_queue_to_path(queue: &[QueuedPrompt], path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut queue = queue.to_vec();
    sanitize_prompt_queue(&mut queue);
    let json = serde_json::to_string_pretty(&queue).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

pub fn load_stacker_queue() -> Vec<QueuedPrompt> {
    let Some(path) = stacker_queue_path() else {
        return Vec::new();
    };

    load_queue_from_path(&path).unwrap_or_default()
}

pub fn save_stacker_queue(queue: &[QueuedPrompt]) {
    let Some(path) = stacker_queue_path() else {
        return;
    };
    let _ = save_queue_to_path(queue, &path);
}

/// Import prompts from a JSON file, returning the loaded prompts.
pub fn import_prompts(path: &Path) -> Result<Vec<StackerPrompt>, String> {
    load_prompts_from_path(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prompt(text: &str, category: &str) -> StackerPrompt {
        new_prompt(text, category).expect("prompt should be valid")
    }

    fn temp_path(name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("llnzy-{name}-{nonce}.json"))
    }

    fn temp_dir(name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("llnzy-library-{name}-{nonce}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    struct LibraryDirs {
        legacy: PathBuf,
        inbox: PathBuf,
        saved: PathBuf,
        archive: PathBuf,
        tmp: PathBuf,
        root: PathBuf,
    }

    impl LibraryDirs {
        fn new(name: &str) -> Self {
            let root = temp_dir(name);
            Self {
                legacy: root.join("stacker.json"),
                inbox: root.join("inbox"),
                saved: root.join("saved"),
                archive: root.join("archive"),
                tmp: root.join(".tmp"),
                root,
            }
        }
    }

    impl Drop for LibraryDirs {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn new_prompt_trims_text_and_category() {
        let prompt = new_prompt("  write a useful test  ", "  dev  ").unwrap();
        assert_eq!(prompt.text, "write a useful test");
        assert_eq!(prompt.label, "write a useful test");
        assert_eq!(prompt.category, "dev");
    }

    #[test]
    fn new_prompt_rejects_blank_text() {
        assert!(new_prompt("   ", "dev").is_none());
    }

    #[test]
    fn prompt_label_truncates_after_eight_words() {
        assert_eq!(
            prompt_label("one two three four five six seven eight nine ten"),
            "one two three four five six seven eight..."
        );
    }

    #[test]
    fn apply_prompt_edit_updates_text_and_label() {
        let mut prompts = vec![prompt("old text", "dev")];
        assert!(apply_prompt_edit(
            &mut prompts,
            0,
            "one two three four five six seven eight nine"
        ));
        assert_eq!(
            prompts[0].text,
            "one two three four five six seven eight nine"
        );
        assert_eq!(
            prompts[0].label,
            "one two three four five six seven eight..."
        );
    }

    #[test]
    fn apply_prompt_edit_ignores_missing_index() {
        let mut prompts = vec![prompt("old text", "dev")];
        assert!(!apply_prompt_edit(&mut prompts, 1, "new text"));
        assert_eq!(prompts[0].text, "old text");
    }

    #[test]
    fn merge_unique_prompts_dedupes_by_text() {
        let mut existing = vec![prompt("keep this", "dev")];
        let imported = vec![prompt("keep this", "other"), prompt("add this", "ops")];

        let added = merge_unique_prompts(&mut existing, imported);

        assert_eq!(added, 1);
        assert_eq!(existing.len(), 2);
        assert_eq!(existing[1].text, "add this");
    }

    #[test]
    fn load_prompts_from_path_reports_invalid_json() {
        let path = temp_path("invalid");
        std::fs::write(&path, "not json").unwrap();

        let result = load_prompts_from_path(&path);
        let _ = std::fs::remove_file(&path);

        assert!(result.is_err());
    }

    #[test]
    fn load_saved_prompts_returns_empty_when_no_legacy_and_no_saved_dir() {
        let dirs = LibraryDirs::new("empty");
        let prompts = load_saved_prompts_at(&dirs.legacy, &dirs.saved, &dirs.tmp);
        assert!(prompts.is_empty());
    }

    #[test]
    fn load_inbox_prompts_suppresses_saved_or_archived_body_hashes() {
        let dirs = LibraryDirs::new("inbox-dedup");
        let body = "same prompt body";
        let inbox_record = storage::PromptRecord {
            frontmatter: storage::PromptFrontmatter {
                id: storage::new_id(),
                state: storage::PromptState::Pending,
                label: "from agent".to_string(),
                category: String::new(),
                created: "2026-05-09T00:00:00Z".to_string(),
                source_agent: Some("agent".to_string()),
                session_id: None,
                workspace: None,
                related_files: Vec::new(),
                body_hash: storage::body_hash(body),
            },
            body: body.to_string(),
        };
        let archived_record = storage::PromptRecord {
            frontmatter: storage::PromptFrontmatter {
                id: storage::new_id(),
                state: storage::PromptState::Archived,
                label: "archived".to_string(),
                category: String::new(),
                created: "2026-05-09T00:00:00Z".to_string(),
                source_agent: None,
                session_id: None,
                workspace: None,
                related_files: Vec::new(),
                body_hash: storage::body_hash(" same   prompt\nbody "),
            },
            body: body.to_string(),
        };
        storage::write_atomic(&inbox_record, &dirs.inbox, &dirs.tmp).unwrap();
        storage::write_atomic(&archived_record, &dirs.archive, &dirs.tmp).unwrap();

        let prompts = load_inbox_prompts_filtered_at(
            &dirs.inbox,
            &[dirs.saved.clone(), dirs.archive.clone()],
        );

        assert!(prompts.is_empty());
    }

    #[test]
    fn load_saved_prompts_migrates_legacy_json_on_first_call() {
        let dirs = LibraryDirs::new("migrate-on-load");
        let legacy = r#"[
            {"text":"first","label":"first","category":"dev"},
            {"text":"second","label":"second","category":""}
        ]"#;
        std::fs::write(&dirs.legacy, legacy).unwrap();

        let prompts = load_saved_prompts_at(&dirs.legacy, &dirs.saved, &dirs.tmp);
        assert_eq!(prompts.len(), 2);
        for prompt in &prompts {
            assert!(prompt.id.is_some(), "loaded prompts must carry an id");
        }
        let bodies: Vec<&str> = prompts.iter().map(|p| p.text.as_str()).collect();
        assert!(bodies.contains(&"first"));
        assert!(bodies.contains(&"second"));

        // Legacy file is renamed; the .migrated backup is left behind.
        assert!(!dirs.legacy.exists());
        assert!(dirs.root.join("stacker.json.migrated").exists());

        // Second call is idempotent — still returns the same library.
        let again = load_saved_prompts_at(&dirs.legacy, &dirs.saved, &dirs.tmp);
        let mut a: Vec<_> = prompts.iter().map(|p| p.id.clone().unwrap()).collect();
        let mut b: Vec<_> = again.iter().map(|p| p.id.clone().unwrap()).collect();
        a.sort();
        b.sort();
        assert_eq!(a, b);
    }

    #[test]
    fn persist_prompt_library_writes_new_prompts_and_assigns_ids() {
        let dirs = LibraryDirs::new("persist-new");
        let mut current = vec![prompt("hello world", "dev")];
        assert!(current[0].id.is_none());

        persist_prompt_library_at(&mut current, &[], &dirs.saved, &dirs.archive, &dirs.tmp);

        assert!(
            current[0].id.is_some(),
            "id should be assigned after first persist"
        );
        let id = current[0].id.clone().unwrap();
        let path = dirs.saved.join(format!("{id}.md"));
        assert!(path.exists());

        // A round-trip through load should return the same library shape.
        let loaded = load_saved_prompts_at(&dirs.legacy, &dirs.saved, &dirs.tmp);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id.as_deref(), Some(id.as_str()));
        assert_eq!(loaded[0].text, "hello world");
    }

    #[test]
    fn persist_prompt_library_overwrites_edited_prompts_at_same_id() {
        let dirs = LibraryDirs::new("persist-edit");
        let mut current = vec![prompt("v1 text", "dev")];
        persist_prompt_library_at(&mut current, &[], &dirs.saved, &dirs.archive, &dirs.tmp);
        let id = current[0].id.clone().unwrap();
        let previous = current.clone();

        // Read the originally written `created` so we can verify it survives the edit.
        let original_created = storage::read(&dirs.saved.join(format!("{id}.md")))
            .unwrap()
            .frontmatter
            .created;

        // Edit the prompt and persist again with the snapshot from the prior persist.
        current[0].text = "v2 text".to_string();
        current[0].label = "v2 text".to_string();
        persist_prompt_library_at(
            &mut current,
            &previous,
            &dirs.saved,
            &dirs.archive,
            &dirs.tmp,
        );

        assert_eq!(current[0].id.as_deref(), Some(id.as_str()));
        let record = storage::read(&dirs.saved.join(format!("{id}.md"))).unwrap();
        assert_eq!(record.body, "v2 text");
        assert_eq!(record.frontmatter.label, "v2 text");
        assert_eq!(
            record.frontmatter.created, original_created,
            "edits should preserve the original `created` timestamp"
        );
    }

    #[test]
    fn persist_prompt_library_archives_removed_prompts() {
        let dirs = LibraryDirs::new("persist-delete");
        let mut current = vec![prompt("keep me", "dev"), prompt("delete me", "dev")];
        persist_prompt_library_at(&mut current, &[], &dirs.saved, &dirs.archive, &dirs.tmp);
        let previous = current.clone();
        let removed_id = current[1].id.clone().unwrap();

        // Remove the second prompt and persist.
        current.pop();
        persist_prompt_library_at(
            &mut current,
            &previous,
            &dirs.saved,
            &dirs.archive,
            &dirs.tmp,
        );

        let saved_path = dirs.saved.join(format!("{removed_id}.md"));
        let archive_path = dirs.archive.join(format!("{removed_id}.md"));
        assert!(!saved_path.exists(), "deleted prompt should leave saved/");
        assert!(
            archive_path.exists(),
            "deleted prompt should land in archive/"
        );
    }

    #[test]
    fn persist_prompt_library_skips_unchanged_prompts() {
        // A no-op persist (no diff against previous) should not rewrite files,
        // so file mtimes stay stable for tools like the watcher.
        let dirs = LibraryDirs::new("persist-noop");
        let mut current = vec![prompt("stable", "dev")];
        persist_prompt_library_at(&mut current, &[], &dirs.saved, &dirs.archive, &dirs.tmp);
        let id = current[0].id.clone().unwrap();
        let path = dirs.saved.join(format!("{id}.md"));
        let mtime_before = std::fs::metadata(&path).unwrap().modified().unwrap();

        // Wait briefly so we'd notice a rewrite via mtime.
        std::thread::sleep(std::time::Duration::from_millis(20));

        let previous = current.clone();
        persist_prompt_library_at(
            &mut current,
            &previous,
            &dirs.saved,
            &dirs.archive,
            &dirs.tmp,
        );
        let mtime_after = std::fs::metadata(&path).unwrap().modified().unwrap();
        assert_eq!(mtime_before, mtime_after);
    }

    #[test]
    fn save_and_load_queue_round_trips_sanitized_prompts() {
        let path = temp_path("queue");
        let queue = vec![
            QueuedPrompt::from_text("one").unwrap(),
            QueuedPrompt::from_text("two").unwrap(),
            QueuedPrompt::from_text("one").unwrap(),
            QueuedPrompt::from_text("three").unwrap(),
            QueuedPrompt::from_text("four").unwrap(),
            QueuedPrompt::from_text("five").unwrap(),
            QueuedPrompt::from_text("six").unwrap(),
        ];

        save_queue_to_path(&queue, &path).unwrap();
        let loaded = load_queue_from_path(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            loaded
                .iter()
                .map(|prompt| prompt.text.as_str())
                .collect::<Vec<_>>(),
            vec!["one", "two", "three", "four", "five"]
        );
    }
}
