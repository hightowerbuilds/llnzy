use super::{
    queue::{sanitize_prompt_queue, QueuedPrompt},
    StackerPrompt,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptRefreshPlan {
    pub prompts: Vec<StackerPrompt>,
    pub queued_prompts: Vec<QueuedPrompt>,
    pub active_prompt: Option<usize>,
    pub editor_text: Option<String>,
}

pub fn plan_prompt_refresh(
    current_prompts: &[StackerPrompt],
    current_queue: &[QueuedPrompt],
    current_active: Option<usize>,
    editor_text: &str,
    next_prompts: Vec<StackerPrompt>,
    mut next_queue: Vec<QueuedPrompt>,
) -> Option<PromptRefreshPlan> {
    sanitize_prompt_queue(&mut next_queue);
    if next_prompts == current_prompts && next_queue == current_queue {
        return None;
    }

    let previous_active_id = current_active
        .and_then(|index| current_prompts.get(index))
        .and_then(|prompt| prompt.id.clone());
    let previous_active_text = current_active
        .and_then(|index| current_prompts.get(index))
        .map(|prompt| prompt.text.as_str());
    let active_prompt = previous_active_id
        .as_deref()
        .and_then(|id| {
            next_prompts
                .iter()
                .position(|prompt| prompt.id.as_deref() == Some(id))
        })
        .or_else(|| (!next_prompts.is_empty()).then_some(0));

    let should_replace_editor = previous_active_text.is_none_or(|text| editor_text == text);
    let editor_text = should_replace_editor.then(|| {
        active_prompt
            .and_then(|index| next_prompts.get(index))
            .map(|prompt| prompt.text.clone())
            .unwrap_or_default()
    });

    Some(PromptRefreshPlan {
        prompts: next_prompts,
        queued_prompts: next_queue,
        active_prompt,
        editor_text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stacker::{prompt_label, queue::QueuedPrompt};

    fn prompt(id: &str, text: &str) -> StackerPrompt {
        StackerPrompt {
            id: Some(id.to_string()),
            text: text.to_string(),
            label: prompt_label(text),
            ..StackerPrompt::default()
        }
    }

    fn queued(text: &str) -> QueuedPrompt {
        QueuedPrompt::from_text(text).expect("test prompt should be queueable")
    }

    #[test]
    fn unchanged_prompt_state_needs_no_refresh() {
        let prompts = vec![prompt("one", "first")];
        let queue = vec![queued("first")];

        assert_eq!(
            plan_prompt_refresh(
                &prompts,
                &queue,
                Some(0),
                "first",
                prompts.clone(),
                queue.clone()
            ),
            None
        );
    }

    #[test]
    fn clean_editor_tracks_active_prompt_by_id_after_reorder() {
        let current = vec![prompt("one", "first"), prompt("two", "second")];
        let next = vec![prompt("two", "second updated"), prompt("one", "first")];

        let plan = plan_prompt_refresh(&current, &[], Some(1), "second", next, vec![])
            .expect("changed prompt text should refresh");

        assert_eq!(plan.active_prompt, Some(0));
        assert_eq!(plan.editor_text, Some("second updated".to_string()));
    }

    #[test]
    fn dirty_editor_keeps_local_text_while_state_refreshes() {
        let current = vec![prompt("one", "first"), prompt("two", "second")];
        let next = vec![prompt("two", "server second"), prompt("one", "first")];

        let plan = plan_prompt_refresh(&current, &[], Some(1), "dirty local edit", next, vec![])
            .expect("changed prompt text should refresh");

        assert_eq!(plan.active_prompt, Some(0));
        assert_eq!(plan.editor_text, None);
    }

    #[test]
    fn missing_active_prompt_falls_back_to_first_prompt() {
        let current = vec![prompt("one", "first")];
        let next = vec![prompt("two", "second")];

        let plan = plan_prompt_refresh(&current, &[], Some(0), "first", next, vec![])
            .expect("replacement prompt set should refresh");

        assert_eq!(plan.active_prompt, Some(0));
        assert_eq!(plan.editor_text, Some("second".to_string()));
    }

    #[test]
    fn refresh_sanitizes_incoming_queue() {
        let current = vec![prompt("one", "first")];
        let next_queue = vec![
            queued("one"),
            queued("two"),
            queued("one"),
            queued("three"),
            queued("four"),
            queued("five"),
            queued("six"),
        ];

        let plan = plan_prompt_refresh(&current, &[], None, "", current.clone(), next_queue)
            .expect("queue change should refresh");

        assert_eq!(
            plan.queued_prompts
                .iter()
                .map(|prompt| prompt.text.as_str())
                .collect::<Vec<_>>(),
            vec!["one", "two", "three", "four", "five"]
        );
    }
}
