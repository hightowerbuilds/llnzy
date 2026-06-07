use gpui::prelude::*;
use gpui::{div, px, rgb, Context, FontWeight, MouseButton, MouseDownEvent, SharedString};

use super::super::{
    StackerPrototype, BORDER, CONTENT_PANEL_BG, MUTED_TEXT, QUEUE_GREEN, TEXT,
};

pub(super) fn cli_help_button(cx: &mut Context<StackerPrototype>) -> impl IntoElement {
    div()
        .id("stacker-cli-help")
        .h(px(24.0))
        .px_2()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(0x242632))
        .text_size(px(11.0))
        .text_color(rgb(TEXT))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                this.toggle_cli_help(cx);
            }),
        )
        .child("What is Stacker CLI?")
}

pub(super) fn cli_help_header(cx: &mut Context<StackerPrototype>) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .text_size(px(16.0))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(TEXT))
                .child("Stacker CLI"),
        )
        .child(
            div()
                .id("stacker-cli-help-close")
                .w(px(24.0))
                .h(px(24.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .text_size(px(16.0))
                .text_color(rgb(MUTED_TEXT))
                .cursor_pointer()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                        this.close_cli_help(cx);
                    }),
                )
                .child("×"),
        )
}

pub(super) fn cli_help_section_title(label: &'static str) -> impl IntoElement {
    div()
        .pt_2()
        .text_size(px(11.0))
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(MUTED_TEXT))
        .child(label.to_uppercase())
}

pub(super) fn cli_help_paragraph(body: &'static str) -> impl IntoElement {
    div()
        .text_size(px(13.0))
        .line_height(px(20.0))
        .text_color(rgb(TEXT))
        .child(body)
}

/// Paste-ready Markdown block a user can drop into a fresh agent's
/// context. The agent reads this and knows enough to use the Stacker
/// CLI: the inbox path, the four mutation commands, the JSON list
/// shape, the field limits, and the exit-code map.
const AGENT_INSTRUCTIONS_TEMPLATE: &str = r##"# Stacker CLI instructions

You can queue work for me through the Stacker CLI. Each item becomes a
Markdown file in my inbox that I'll review and edit.

## Inbox
~/Library/Application Support/llnzy/prompts/inbox/

Files are plain Markdown with YAML frontmatter. One prompt per file.

## Commands

Add a prompt (body on stdin OR via --file):
  echo "<body>" | llnzy stacker add --label "<title>"
  llnzy stacker add --label "<title>" --file <path>

List inbox in machine-readable form:
  llnzy stacker list --state inbox --format json

Each list item has `id`, `label`, `category`, `body_path`, `created_at`,
and other frontmatter fields. Use `id` for edit/delete.

Edit (any subset of flags is fine):
  llnzy stacker edit <id> --state inbox --label "<new title>"
  llnzy stacker edit <id> --state inbox --body "<new body>"

Delete:
  llnzy stacker delete <id> --state inbox

## Limits
- Body: max 256 KB
- Label: max 256 chars
- Category: max 64 chars
- Inbox quota: 1000 files, 50 MB total

## Exit codes
0 = success, 1 = usage error, 2 = bad input, 3 = quota exceeded
"##;

pub(super) fn cli_help_inbox_button_row(cx: &mut Context<StackerPrototype>) -> impl IntoElement {
    div().flex().gap_2().child(
        div()
            .id("stacker-cli-reveal-inbox")
            .h(px(28.0))
            .px_3()
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .border_1()
            .border_color(rgb(BORDER))
            .bg(rgb(0x242632))
            .text_size(px(11.0))
            .text_color(rgb(TEXT))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                    this.reveal_inbox_in_finder(cx);
                }),
            )
            .child("Reveal in Finder"),
    )
}

pub(super) fn cli_help_agent_instructions_button(cx: &mut Context<StackerPrototype>) -> impl IntoElement {
    div().flex().child(
        div()
            .id("stacker-cli-copy-agent-instructions")
            .h(px(28.0))
            .px_3()
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .border_1()
            .border_color(rgb(0x3fd663))
            .bg(rgb(0x183a20))
            .text_size(px(11.0))
            .text_color(rgb(QUEUE_GREEN))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                    this.copy_cli_snippet(AGENT_INSTRUCTIONS_TEMPLATE.to_string(), cx);
                }),
            )
            .child("Copy Agent Instructions"),
    )
}

pub(super) fn cli_help_command_block(
    id: &'static str,
    snippet: String,
    cx: &mut Context<StackerPrototype>,
) -> impl IntoElement {
    let snippet_for_copy = snippet.clone();
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .flex_1()
                .px_3()
                .py_2()
                .rounded_sm()
                .border_1()
                .border_color(rgb(BORDER))
                .bg(rgb(CONTENT_PANEL_BG))
                .font_family("Menlo")
                .text_size(px(12.0))
                .text_color(rgb(TEXT))
                .child(snippet),
        )
        .child(
            div()
                .id(SharedString::from(format!("stacker-cli-copy-{id}")))
                .h(px(28.0))
                .px_3()
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .border_1()
                .border_color(rgb(BORDER))
                .bg(rgb(0x242632))
                .text_size(px(11.0))
                .text_color(rgb(MUTED_TEXT))
                .cursor_pointer()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                        this.copy_cli_snippet(snippet_for_copy.clone(), cx);
                    }),
                )
                .child("Copy"),
        )
}
