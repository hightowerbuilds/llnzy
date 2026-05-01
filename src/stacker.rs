use std::path::{Path, PathBuf};

pub mod formatting;
pub mod queue;

/// A saved prompt in the Stacker queue.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StackerPrompt {
    pub text: String,
    pub label: String,
    #[serde(default)]
    pub category: String,
}

pub fn stacker_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("llnzy").join("stacker.json"))
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

pub fn save_prompts_to_path(prompts: &[StackerPrompt], path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let json = serde_json::to_string_pretty(prompts).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

pub fn load_stacker_prompts() -> Vec<StackerPrompt> {
    let Some(path) = stacker_path() else {
        return Vec::new();
    };

    load_prompts_from_path(&path).unwrap_or_default()
}

pub fn save_stacker_prompts(prompts: &[StackerPrompt]) {
    let Some(path) = stacker_path() else { return };
    let _ = save_prompts_to_path(prompts, &path);
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
}
