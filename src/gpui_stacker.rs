use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    actions, div, px, rgb, size, App, Application, Bounds, Context, Entity, FocusHandle, Focusable,
    KeyBinding, Render, Window, WindowBounds, WindowOptions,
};

use crate::stacker::{
    load_saved_prompts, load_stacker_queue,
    queue::{self, QueuedPrompt},
    save_stacker_queue,
    sync::plan_prompt_refresh,
    StackerPrompt,
};

mod layout;
mod render;
mod text_input;

use render::{header, stacker_workbench, status_bar};
use text_input::StackerTextInput;

const CHROME_BG: u32 = 0x242424;
const CONTENT_BG: u32 = 0x191920;
const CONTENT_PANEL_BG: u32 = 0x1f1f28;
const BORDER: u32 = 0x33333a;
const TEXT: u32 = 0xe8e8ee;
const MUTED_TEXT: u32 = 0xa8a8b4;
const SELECTED_BG: u32 = 0x313846;
const QUEUE_GREEN: u32 = 0x6aff90;
const PROMPT_REFRESH_TICK: Duration = Duration::from_millis(1000);

#[derive(Clone, Copy, Debug)]
pub(crate) struct StackerPalette {
    chrome_bg: u32,
    content_bg: u32,
    panel_bg: u32,
    border: u32,
    text: u32,
    muted_text: u32,
    selected_bg: u32,
    button_bg: u32,
}

impl StackerPalette {
    fn for_light_mode(light_mode: bool) -> Self {
        if light_mode {
            Self {
                chrome_bg: 0xf0e3d1,
                content_bg: 0xfaf2e2,
                panel_bg: 0xfffbf2,
                border: 0xd8c6ad,
                text: 0x3e372f,
                muted_text: 0x7d7064,
                selected_bg: 0xe2d0ed,
                button_bg: 0xeadcc8,
            }
        } else {
            Self {
                chrome_bg: CHROME_BG,
                content_bg: CONTENT_BG,
                panel_bg: CONTENT_PANEL_BG,
                border: BORDER,
                text: TEXT,
                muted_text: MUTED_TEXT,
                selected_bg: SELECTED_BG,
                button_bg: 0x242632,
            }
        }
    }
}

actions!(
    stacker_gpui,
    [
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
        Paste,
        Cut,
        Copy,
        Undo,
        Redo,
        Quit,
    ]
);

pub fn run_stacker_prototype() {
    Application::new().run(|cx: &mut App| {
        bind_stacker_keys(cx);
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

        let bounds = Bounds::centered(None, size(px(1040.0), px(720.0)), cx);
        let window = match cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(StackerPrototype::new),
        ) {
            Ok(window) => window,
            Err(error) => {
                log::error!("failed to open stacker window: {error:?}");
                cx.quit();
                return;
            }
        };
        if let Err(error) = window.update(cx, |view, window, cx| {
            window.focus(&view.editor.focus_handle(cx));
        }) {
            log::error!("failed to focus stacker window: {error:?}");
            cx.quit();
            return;
        }
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.activate(true);
    });
}

pub(crate) fn bind_stacker_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, None),
        KeyBinding::new("delete", Delete, None),
        KeyBinding::new("left", Left, None),
        KeyBinding::new("right", Right, None),
        KeyBinding::new("shift-left", SelectLeft, None),
        KeyBinding::new("shift-right", SelectRight, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("cmd-z", Undo, None),
        KeyBinding::new("cmd-shift-z", Redo, None),
        KeyBinding::new("home", Home, None),
        KeyBinding::new("end", End, None),
    ]);
}

pub(crate) struct StackerPrototype {
    editor: Entity<StackerTextInput>,
    prompts: Vec<StackerPrompt>,
    queued_prompts: Vec<QueuedPrompt>,
    /// Which saved prompt (if any) the editor is currently editing. `None`
    /// means the editor is a fresh blank draft; Save will create a new
    /// prompt rather than overwrite an existing one.
    active_prompt: Option<usize>,
    show_chrome: bool,
    /// Index of the prompt the delete-confirmation modal is open for, or
    /// `None` when no modal is showing.
    pending_delete: Option<usize>,
    /// Fraction of the workbench height occupied by the prompt list.
    /// The editor takes the remainder. Dragging the divider between them
    /// updates this; clamped to a sensible range so neither pane collapses.
    prompt_list_ratio: f32,
    /// True when the "?" help modal explaining the Stacker CLI is visible.
    cli_help_open: bool,
    light_mode: bool,
    status_message: Option<String>,
}

impl StackerPrototype {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        Self::with_chrome(cx, true)
    }

    #[cfg(feature = "gpui-workspace")]
    pub(crate) fn queued_prompts(&self) -> &[QueuedPrompt] {
        &self.queued_prompts
    }

    #[cfg(feature = "gpui-workspace")]
    pub(crate) fn embedded(cx: &mut Context<Self>) -> Self {
        Self::with_chrome(cx, false)
    }

    pub(crate) fn with_chrome(cx: &mut Context<Self>, show_chrome: bool) -> Self {
        let prompts = load_saved_prompts();
        let mut queued_prompts = load_stacker_queue();
        queue::sanitize_prompt_queue(&mut queued_prompts);
        // Open Stacker on a blank draft. The saved-prompt list on the left
        // is the entry point for editing existing prompts; the editor
        // starts empty so a fresh prompt is always one click away.
        let editor = cx.new(|cx| StackerTextInput::new(cx, String::new()));
        start_prompt_refresh_task(cx);
        Self {
            editor,
            prompts,
            queued_prompts,
            active_prompt: None,
            show_chrome,
            pending_delete: None,
            prompt_list_ratio: 0.34,
            cli_help_open: false,
            light_mode: false,
            status_message: None,
        }
    }

    #[cfg(feature = "gpui-workspace")]
    pub(crate) fn set_light_mode(&mut self, light_mode: bool, cx: &mut Context<Self>) {
        if self.light_mode == light_mode {
            return;
        }
        self.light_mode = light_mode;
        self.editor
            .update(cx, |editor, cx| editor.set_light_mode(light_mode, cx));
        cx.notify();
    }

    pub(crate) fn toggle_cli_help(&mut self, cx: &mut Context<Self>) {
        self.cli_help_open = !self.cli_help_open;
        cx.notify();
    }

    pub(crate) fn close_cli_help(&mut self, cx: &mut Context<Self>) {
        if self.cli_help_open {
            self.cli_help_open = false;
            cx.notify();
        }
    }

    /// Copy a snippet to the clipboard from the CLI help modal — used by
    /// per-command copy affordances so users can paste the install or
    /// example commands straight into their terminal.
    pub(crate) fn copy_cli_snippet(&mut self, snippet: String, cx: &mut Context<Self>) {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(snippet));
        cx.notify();
    }

    /// Open the Stacker inbox directory in Finder so users can verify the
    /// directory exists and inspect what an agent has written. Creates
    /// the directory first if it's missing — early in a user's lifecycle
    /// nothing may have populated it yet.
    pub(crate) fn reveal_inbox_in_finder(&mut self, _cx: &mut Context<Self>) {
        let Some(paths) = crate::platform::paths::current_paths() else {
            log::error!("stacker cli help: could not resolve config paths");
            return;
        };
        let inbox = paths.prompts_inbox_dir();
        if let Err(err) = std::fs::create_dir_all(&inbox) {
            log::error!("stacker cli help: create inbox dir failed: {err}");
            return;
        }
        if let Err(err) = std::process::Command::new("open").arg(&inbox).spawn() {
            log::error!("stacker cli help: open inbox in Finder failed: {err}");
        }
    }

    /// Update the prompt-list / editor split ratio. Called by the drag
    /// handle in the divider. The clamp keeps either side from collapsing
    /// completely below a usable size.
    pub(crate) fn set_prompt_list_ratio(&mut self, ratio: f32, cx: &mut Context<Self>) {
        let clamped = ratio.clamp(0.12, 0.85);
        if (clamped - self.prompt_list_ratio).abs() > f32::EPSILON {
            self.prompt_list_ratio = clamped;
            cx.notify();
        }
    }

    /// Dispatch a formatting command (heading / list / etc.) to the active
    /// prompt editor. The toolbar buttons call this; the underlying
    /// `execute_stacker_command` already routes through the session for
    /// undo/redo coherence.
    pub(crate) fn run_stacker_command(
        &mut self,
        id: crate::stacker::commands::StackerCommandId,
        cx: &mut Context<Self>,
    ) {
        let editor_command = crate::stacker::commands::stacker_editor_command(id);
        self.editor.update(cx, |input, cx| {
            crate::stacker::commands::execute_stacker_command(&mut input.session, editor_command);
            input.session.set_selection(input.session.selection());
            cx.notify();
        });
        cx.notify();
    }

    /// Persist the current draft to disk. When a saved prompt is active,
    /// updates it in place; otherwise creates a new prompt and selects it.
    /// No-op when the editor is empty.
    pub(crate) fn save_active_prompt(&mut self, cx: &mut Context<Self>) -> Result<(), String> {
        let current_text = self.editor.read(cx).session.text().trim().to_string();
        if current_text.is_empty() {
            return Ok(());
        }

        let previous = self.prompts.clone();
        match self.active_prompt {
            Some(index) if index < self.prompts.len() => {
                if !crate::stacker::apply_prompt_edit(&mut self.prompts, index, &current_text) {
                    return Ok(());
                }
            }
            _ => {
                let Some(prompt) = crate::stacker::new_prompt(&current_text, "") else {
                    return Ok(());
                };
                self.prompts.push(prompt);
                self.active_prompt = Some(self.prompts.len() - 1);
            }
        }

        match crate::stacker::persist_prompt_library(&mut self.prompts, &previous) {
            Ok(()) => {
                self.status_message = Some("Prompt saved".to_string());
                cx.notify();
                Ok(())
            }
            Err(error) => {
                self.status_message = Some(format!("Save failed: {error}"));
                log::warn!("stacker save failed: {error}");
                cx.notify();
                Err(error)
            }
        }
    }

    /// Open the delete-confirmation modal for the prompt at `index`.
    pub(crate) fn request_delete_prompt(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.prompts.len() {
            self.pending_delete = Some(index);
            cx.notify();
        }
    }

    pub(crate) fn cancel_delete_prompt(&mut self, cx: &mut Context<Self>) {
        if self.pending_delete.take().is_some() {
            cx.notify();
        }
    }

    /// Confirm the pending delete: remove the prompt from the library,
    /// persist, and reset the editor to a blank draft if the deleted
    /// prompt was active.
    pub(crate) fn confirm_delete_prompt(&mut self, cx: &mut Context<Self>) {
        let Some(index) = self.pending_delete.take() else {
            return;
        };
        if index >= self.prompts.len() {
            cx.notify();
            return;
        }

        let previous = self.prompts.clone();
        let previous_active = self.active_prompt;
        let was_active = self.active_prompt == Some(index);
        self.prompts.remove(index);

        // Shift the active index if needed so it keeps pointing at the
        // same prompt by identity (or clears if the active one was the
        // one removed).
        self.active_prompt = match self.active_prompt {
            Some(active) if active == index => None,
            Some(active) if active > index => Some(active - 1),
            other => other,
        };

        match crate::stacker::persist_prompt_library(&mut self.prompts, &previous) {
            Ok(()) => {
                self.status_message = Some("Prompt deleted".to_string());
            }
            Err(error) => {
                self.prompts = previous;
                self.active_prompt = previous_active;
                self.status_message = Some(format!("Delete failed: {error}"));
                log::warn!("stacker delete failed: {error}");
                cx.notify();
                return;
            }
        }

        if was_active {
            self.editor.update(cx, |input, cx| {
                input.session.set_text(String::new());
                cx.notify();
            });
        }
        cx.notify();
    }

    /// Adjust the prompt editor's text size. Bounded so the toolbar can't
    /// shrink past readability or stretch past line-height sanity.
    pub(crate) fn adjust_stacker_font_size(&mut self, delta: f32, cx: &mut Context<Self>) {
        self.editor.update(cx, |input, cx| {
            let next = (input.font_size_px() + delta).clamp(11.0, 28.0);
            input.set_font_size_px(next);
            cx.notify();
        });
        cx.notify();
    }

    fn load_prompt(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(prompt) = self.prompts.get(index) else {
            return;
        };
        self.active_prompt = Some(index);
        let text = prompt.text.clone();
        self.editor.update(cx, |editor, cx| {
            editor.session.set_text(text);
            cx.notify();
        });
        cx.notify();
    }

    /// Reset the editor to a blank draft (no active prompt). Used by the
    /// "New Prompt" button so the user can start writing without first
    /// saving the current draft. The caller is responsible for saving
    /// the previous draft first if needed — typically this is called
    /// after a manual Save or after the close-event autosave hook has
    /// already persisted any pending work.
    pub(crate) fn start_new_prompt(&mut self, cx: &mut Context<Self>) {
        self.active_prompt = None;
        self.editor.update(cx, |input, cx| {
            input.session.set_text(String::new());
            cx.notify();
        });
        cx.notify();
    }

    fn toggle_prompt_queue(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(prompt) = self.prompts.get(index) else {
            return;
        };
        queue::toggle_prompt(&mut self.queued_prompts, prompt);
        queue::sanitize_prompt_queue(&mut self.queued_prompts);
        save_stacker_queue(&self.queued_prompts);
        cx.notify();
    }

    fn refresh_external_prompt_state(&mut self, cx: &mut Context<Self>) -> bool {
        let next_queue = load_stacker_queue();
        let next_prompts = load_saved_prompts();
        let editor_text = self.editor.read(cx).session.text().to_string();
        let Some(plan) = plan_prompt_refresh(
            &self.prompts,
            &self.queued_prompts,
            self.active_prompt,
            &editor_text,
            next_prompts,
            next_queue,
        ) else {
            return false;
        };

        self.prompts = plan.prompts;
        self.queued_prompts = plan.queued_prompts;
        self.active_prompt = plan.active_prompt;

        if let Some(text) = plan.editor_text {
            self.editor.update(cx, |editor, cx| {
                editor.session.set_text(text);
                cx.notify();
            });
        }
        true
    }
}

fn start_prompt_refresh_task(cx: &mut Context<StackerPrototype>) {
    cx.spawn(
        |stacker: gpui::WeakEntity<StackerPrototype>, cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            async move {
                loop {
                    cx.background_executor().timer(PROMPT_REFRESH_TICK).await;
                    let Ok(()) = stacker.update(&mut cx, |stacker, cx| {
                        if stacker.refresh_external_prompt_state(cx) {
                            cx.notify();
                        }
                    }) else {
                        break;
                    };
                }
            }
        },
    )
    .detach();
}

impl Focusable for StackerPrototype {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.editor.focus_handle(cx)
    }
}

impl Render for StackerPrototype {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = StackerPalette::for_light_mode(self.light_mode);
        let content = stacker_workbench(
            &self.prompts,
            &self.queued_prompts,
            self.active_prompt,
            self.editor.clone(),
            self.show_chrome,
            self.prompt_list_ratio,
            palette,
            cx,
        );
        let mut frame = div().size_full().flex().flex_col();
        if self.show_chrome {
            frame = frame.child(header(palette));
        }
        frame = frame.child(content);
        if self.show_chrome {
            frame = frame.child(status_bar(
                self.editor.read(cx),
                self.status_message.as_deref(),
                palette,
            ));
        }

        let pending_delete = self.pending_delete;
        let pending_label = pending_delete
            .and_then(|index| self.prompts.get(index))
            .map(|prompt| prompt.label.clone());
        let cli_help_open = self.cli_help_open;

        div()
            .relative()
            .size_full()
            .bg(rgb(palette.content_bg))
            .text_color(rgb(palette.text))
            .font_family("Inter")
            .child(frame)
            .when_some(pending_label, |root, label| {
                root.child(render::delete_confirmation_modal(label, cx))
            })
            .when(cli_help_open, |root| root.child(render::cli_help_modal(cx)))
    }
}
