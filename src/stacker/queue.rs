use super::{prompt_label, StackerPrompt};

pub const MAX_QUEUE_PROMPTS: usize = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueuedPrompt {
    pub text: String,
    pub label: String,
}

impl QueuedPrompt {
    pub fn from_prompt(prompt: &StackerPrompt) -> Self {
        Self {
            text: prompt.text.clone(),
            label: prompt.label.clone(),
        }
    }

    pub fn from_text(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        Some(Self {
            text: text.to_string(),
            label: prompt_label(text),
        })
    }
}

pub fn add_prompt(queue: &mut Vec<QueuedPrompt>, prompt: &StackerPrompt) -> bool {
    if queue.len() >= MAX_QUEUE_PROMPTS || contains_prompt(queue, prompt) {
        return false;
    }
    queue.push(QueuedPrompt::from_prompt(prompt));
    true
}

pub fn contains_prompt(queue: &[QueuedPrompt], prompt: &StackerPrompt) -> bool {
    queue.iter().any(|queued| queued.text == prompt.text)
}

pub fn sanitize_prompt_queue(queue: &mut Vec<QueuedPrompt>) {
    let mut seen = Vec::<String>::new();
    queue.retain(|queued| {
        let text = queued.text.trim();
        if text.is_empty() || seen.iter().any(|existing| existing == text) {
            return false;
        }
        seen.push(text.to_string());
        true
    });
    queue.truncate(MAX_QUEUE_PROMPTS);
}

pub fn footer_preview(text: &str) -> String {
    let words = text
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ");
    if words.is_empty() {
        "Prompt...".to_string()
    } else {
        format!("{}...", words)
    }
}

pub fn clipboard_markdown(prompt: &QueuedPrompt) -> String {
    prompt.text.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prompt(text: &str) -> StackerPrompt {
        StackerPrompt {
            text: text.to_string(),
            label: prompt_label(text),
            category: String::new(),
        }
    }

    #[test]
    fn add_prompt_caps_at_five_and_blocks_duplicates() {
        let mut queue = Vec::new();
        for i in 0..5 {
            assert!(add_prompt(&mut queue, &prompt(&format!("prompt {i}"))));
        }

        assert!(!add_prompt(&mut queue, &prompt("prompt 1")));
        assert!(!add_prompt(&mut queue, &prompt("prompt 5")));
        assert_eq!(queue.len(), 5);
    }

    #[test]
    fn sanitizer_removes_blank_duplicate_and_over_limit_entries() {
        let mut queue = vec![
            QueuedPrompt::from_text("one").unwrap(),
            QueuedPrompt::from_text("two").unwrap(),
            QueuedPrompt::from_text("one").unwrap(),
            QueuedPrompt {
                text: "   ".to_string(),
                label: String::new(),
            },
            QueuedPrompt::from_text("three").unwrap(),
            QueuedPrompt::from_text("four").unwrap(),
            QueuedPrompt::from_text("five").unwrap(),
            QueuedPrompt::from_text("six").unwrap(),
        ];

        sanitize_prompt_queue(&mut queue);

        assert_eq!(
            queue
                .iter()
                .map(|prompt| prompt.text.as_str())
                .collect::<Vec<_>>(),
            vec!["one", "two", "three", "four", "five"]
        );
    }

    #[test]
    fn footer_preview_uses_first_three_words() {
        assert_eq!(footer_preview("one two three four"), "one two three...");
        assert_eq!(footer_preview(""), "Prompt...");
    }

    #[test]
    fn clipboard_markdown_preserves_prompt_markdown_markers() {
        let prompt = QueuedPrompt::from_text(
            "Use **strong framing** here\n- first point\n- second point\n1. numbered point",
        )
        .unwrap();

        assert_eq!(
            clipboard_markdown(&prompt),
            "Use **strong framing** here\n- first point\n- second point\n1. numbered point"
        );
    }
}
