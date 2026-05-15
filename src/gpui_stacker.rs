use std::{ops::Range, time::Duration};

use gpui::prelude::*;
use gpui::{
    actions, div, fill, hsla, px, relative, rgb, rgba, size, App, Application, Bounds,
    ClipboardItem, ContentMask, Context, CursorStyle, DispatchPhase, Element, ElementId,
    ElementInputHandler, Entity, EntityInputHandler, FocusHandle, Focusable, GlobalElementId,
    KeyBinding, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad,
    Pixels, Point, Render, ScrollWheelEvent, SharedString, Style, TextAlign, TextRun,
    UTF16Selection, Window, WindowBounds, WindowOptions,
};

use crate::stacker::{
    input::StackerSelection,
    load_saved_prompts, load_stacker_queue,
    queue::{self, QueuedPrompt},
    save_stacker_queue,
    session::StackerSession,
    sync::plan_prompt_refresh,
    utf16::{char_index_to_utf16_index, utf16_index_to_char_index},
    StackerPrompt,
};

mod layout;
mod render;

use layout::{layout_text_lines, marked_runs_for_line, slice_chars, LayoutLine, MultilineLayout};
use render::{header, stacker_workbench, status_bar};

const CHROME_BG: u32 = 0x242424;
const CONTENT_BG: u32 = 0x191920;
const CONTENT_PANEL_BG: u32 = 0x1f1f28;
const BORDER: u32 = 0x33333a;
const TEXT: u32 = 0xe8e8ee;
const MUTED_TEXT: u32 = 0xa8a8b4;
const SELECTED_BG: u32 = 0x313846;
const QUEUE_GREEN: u32 = 0x6aff90;
const PROMPT_REFRESH_TICK: Duration = Duration::from_millis(1000);

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
}

impl StackerPrototype {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        Self::with_chrome(cx, true)
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
        }
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
            crate::stacker::commands::execute_stacker_command(
                &mut input.session,
                editor_command,
            );
            input.session.set_selection(input.session.selection());
            cx.notify();
        });
        cx.notify();
    }

    /// Persist the current draft to disk. When a saved prompt is active,
    /// updates it in place; otherwise creates a new prompt and selects it.
    /// No-op when the editor is empty.
    pub(crate) fn save_active_prompt(&mut self, cx: &mut Context<Self>) {
        let current_text = self.editor.read(cx).session.text().trim().to_string();
        if current_text.is_empty() {
            return;
        }

        let previous = self.prompts.clone();
        match self.active_prompt {
            Some(index) if index < self.prompts.len() => {
                if !crate::stacker::apply_prompt_edit(&mut self.prompts, index, &current_text) {
                    return;
                }
            }
            _ => {
                let Some(prompt) = crate::stacker::new_prompt(&current_text, "") else {
                    return;
                };
                self.prompts.push(prompt);
                self.active_prompt = Some(self.prompts.len() - 1);
            }
        }

        crate::stacker::persist_prompt_library(&mut self.prompts, &previous);
        cx.notify();
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

        crate::stacker::persist_prompt_library(&mut self.prompts, &previous);

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
        let content = stacker_workbench(
            &self.prompts,
            &self.queued_prompts,
            self.active_prompt,
            self.editor.clone(),
            self.show_chrome,
            self.prompt_list_ratio,
            cx,
        );
        let mut frame = div().size_full().flex().flex_col();
        if self.show_chrome {
            frame = frame.child(header());
        }
        frame = frame.child(content);
        if self.show_chrome {
            frame = frame.child(status_bar(self.editor.read(cx)));
        }

        let pending_delete = self.pending_delete;
        let pending_label = pending_delete
            .and_then(|index| self.prompts.get(index))
            .map(|prompt| prompt.label.clone());
        let cli_help_open = self.cli_help_open;

        div()
            .relative()
            .size_full()
            .bg(rgb(CONTENT_BG))
            .text_color(rgb(TEXT))
            .font_family("Inter")
            .child(frame)
            .when_some(pending_label, |root, label| {
                root.child(render::delete_confirmation_modal(label, cx))
            })
            .when(cli_help_open, |root| root.child(render::cli_help_modal(cx)))
    }
}

struct StackerTextInput {
    focus_handle: FocusHandle,
    session: StackerSession,
    last_layout: Option<MultilineLayout>,
    last_bounds: Option<Bounds<Pixels>>,
    scroll_y: Pixels,
    content_height: Pixels,
    is_selecting: bool,
    font_size: f32,
}

impl StackerTextInput {
    fn new(cx: &mut Context<Self>, text: String) -> Self {
        let mut session = StackerSession::new();
        session.set_text(text);
        Self {
            focus_handle: cx.focus_handle(),
            session,
            last_layout: None,
            last_bounds: None,
            scroll_y: px(0.0),
            content_height: px(0.0),
            is_selecting: false,
            font_size: 16.0,
        }
    }

    pub(crate) fn font_size_px(&self) -> f32 {
        self.font_size
    }

    pub(crate) fn set_font_size_px(&mut self, size: f32) {
        self.font_size = size;
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        let selection = self.session.selection();
        if selection.is_collapsed() {
            self.move_to(selection.end.saturating_sub(1), cx);
        } else {
            self.move_to(selection.sorted().start, cx);
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        let selection = self.session.selection();
        if selection.is_collapsed() {
            self.move_to((selection.end + 1).min(self.session.char_count()), cx);
        } else {
            self.move_to(selection.sorted().end, cx);
        }
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        let selection = self.session.selection();
        self.select_to(selection.end.saturating_sub(1), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        let selection = self.session.selection();
        self.select_to((selection.end + 1).min(self.session.char_count()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.session.select_all();
        cx.notify();
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.session.char_count(), cx);
    }

    fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        self.session.delete_backward(self.session.selection());
        cx.notify();
    }

    fn delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        self.session.delete_forward(self.session.selection());
        cx.notify();
    }

    fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.session.replace_selection(&text);
            cx.notify();
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.session.selected_text(self.session.selection()) {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    fn cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.session.selected_text(self.session.selection()) {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
            self.session.delete_forward(self.session.selection());
            cx.notify();
        }
    }

    fn undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        if self.session.undo() {
            cx.notify();
        }
    }

    fn redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        if self.session.redo() {
            cx.notify();
        }
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle);
        self.is_selecting = true;
        let index = self.index_for_mouse_position(event.position);
        if event.modifiers.shift {
            self.select_to(index, cx);
        } else {
            self.move_to(index, cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _window: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.session
            .set_selection(StackerSelection::collapsed(offset));
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let current = self.session.selection();
        self.session.set_selection(StackerSelection {
            start: current.start,
            end: offset,
        });
        cx.notify();
    }

    fn scroll_by(&mut self, delta: Pixels, cx: &mut Context<Self>) {
        let Some(bounds) = self.last_bounds else {
            return;
        };
        let max_scroll = (self.content_height - bounds.size.height).max(px(0.0));
        let next = (self.scroll_y + delta).clamp(px(0.0), max_scroll);
        if next != self.scroll_y {
            self.scroll_y = next;
            cx.notify();
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        let (Some(bounds), Some(layout)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.session.char_count();
        }
        layout.char_index_for_point(position, *bounds)
    }

    fn range_from_utf16(&self, range: &Range<usize>) -> StackerSelection {
        StackerSelection {
            start: utf16_index_to_char_index(self.session.text(), range.start),
            end: utf16_index_to_char_index(self.session.text(), range.end),
        }
    }

    fn range_to_utf16(&self, selection: StackerSelection) -> Range<usize> {
        char_index_to_utf16_index(self.session.text(), selection.start)
            ..char_index_to_utf16_index(self.session.text(), selection.end)
    }
}

impl EntityInputHandler for StackerTextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let selection = self.range_from_utf16(&range_utf16).sorted();
        actual_range.replace(self.range_to_utf16(selection));
        Some(slice_chars(self.session.text(), selection).to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let selection = self.session.selection();
        Some(UTF16Selection {
            range: self.range_to_utf16(selection),
            reversed: selection.start > selection.end,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.session
            .marked_range()
            .map(|selection| self.range_to_utf16(selection))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.session.unmark_text();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selection = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.session.marked_range())
            .unwrap_or_else(|| self.session.selection());
        self.session.insert_text(selection, new_text);
        self.session.unmark_text();
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let replacement = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.session.marked_range());
        let marked_internal_selection = new_selected_range_utf16
            .as_ref()
            .map(|range| StackerSelection {
                start: utf16_index_to_char_index(new_text, range.start),
                end: utf16_index_to_char_index(new_text, range.end),
            })
            .unwrap_or_else(|| StackerSelection::collapsed(new_text.chars().count()));
        self.session
            .set_marked_text(new_text, marked_internal_selection, replacement);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let layout = self.last_layout.as_ref()?;
        let selection = self.range_from_utf16(&range_utf16).sorted();
        Some(layout.bounds_for_range(selection, bounds))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.last_bounds?;
        if point.y < bounds.top() || point.y > bounds.bottom() {
            return None;
        }
        let layout = self.last_layout.as_ref()?;
        let char_index = layout.char_index_for_point(point, bounds);
        Some(char_index_to_utf16_index(self.session.text(), char_index))
    }
}

impl Focusable for StackerTextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for StackerTextInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .key_context("StackerTextInput")
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .line_height(px(self.font_size * 1.6))
            .text_size(px(self.font_size))
            .text_color(rgb(0xf4f4f8))
            .child(
                div()
                    .size_full()
                    .p(px(16.0))
                    .bg(rgb(CONTENT_BG))
                    .child(StackerTextElement { input: cx.entity() }),
            )
    }
}

struct StackerTextElement {
    input: Entity<StackerTextInput>,
}

struct TextPrepaintState {
    layout: Option<MultilineLayout>,
    cursor: Option<PaintQuad>,
    selection: Vec<PaintQuad>,
}

impl IntoElement for StackerTextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for StackerTextElement {
    type RequestLayoutState = ();
    type PrepaintState = TextPrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let text = input.session.text();
        let selection = input.session.selection();
        let style = window.text_style();
        let text_color = if text.is_empty() {
            hsla(0.0, 0.0, 1.0, 0.35)
        } else {
            style.color
        };
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line_height = window.line_height();
        let wrap_width = Some(bounds.size.width.max(px(24.0)));
        let mut visual_start = 0;
        let layout_lines = layout_text_lines(text)
            .into_iter()
            .filter_map(|text_line| {
                let display_text: SharedString = text_line.text.clone().into();
                let run = TextRun {
                    len: display_text.len(),
                    font: style.font(),
                    color: text_color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let runs = marked_runs_for_line(
                    &input.session,
                    text_line.char_start,
                    text_line.char_end,
                    &display_text,
                    run,
                );
                let mut wrapped = window
                    .text_system()
                    .shape_text(display_text, font_size, &runs, wrap_width, None)
                    .ok()?;
                let line = wrapped.pop()?;
                let line_visual_start = visual_start;
                visual_start += line.wrap_boundaries().len() + 1;
                Some(LayoutLine {
                    line,
                    text: text_line.text,
                    char_start: text_line.char_start,
                    char_end: text_line.char_end,
                    visual_start: line_visual_start,
                })
            })
            .collect();
        let content_height = line_height * visual_start.max(1) as f32;
        let max_scroll = (content_height - bounds.size.height).max(px(0.0));
        let mut scroll_y = input.scroll_y.clamp(px(0.0), max_scroll);
        let layout = MultilineLayout {
            lines: layout_lines,
            line_height,
            scroll_y,
            content_height,
        };

        let sorted = selection.sorted();
        if sorted.is_collapsed() {
            scroll_y = layout.scroll_y_for_caret(selection.end, bounds, scroll_y);
        }
        let layout = MultilineLayout { scroll_y, ..layout };
        let (selection_quads, cursor_quad) = if sorted.is_collapsed() {
            let cursor_bounds = layout.caret_bounds(selection.end, bounds);
            (Vec::new(), Some(fill(cursor_bounds, rgb(0x7dd3fc))))
        } else {
            (
                layout
                    .selection_bounds(sorted, bounds)
                    .into_iter()
                    .map(|bounds| fill(bounds, rgba(0x38bdf860)))
                    .collect(),
                None,
            )
        };

        TextPrepaintState {
            layout: Some(layout),
            cursor: cursor_quad,
            selection: selection_quads,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        let Some(layout) = prepaint.layout.take() else {
            return;
        };
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for selection in prepaint.selection.drain(..) {
                window.paint_quad(selection);
            }
            for line in &layout.lines {
                if let Err(error) = line.line.paint(
                    layout.line_origin(bounds, line),
                    layout.line_height,
                    TextAlign::Left,
                    Some(bounds),
                    window,
                    cx,
                ) {
                    log::error!("failed to paint stacker text line: {error:?}");
                }
            }
            if focus_handle.is_focused(window) {
                if let Some(cursor) = prepaint.cursor.take() {
                    window.paint_quad(cursor);
                }
            }
        });
        let input = self.input.clone();
        let line_height = layout.line_height;
        window.on_mouse_event(move |event: &ScrollWheelEvent, phase, _window, cx| {
            if phase == DispatchPhase::Bubble && bounds.contains(&event.position) {
                let delta = event.delta.pixel_delta(line_height);
                input.update(cx, |input, cx| input.scroll_by(-delta.y, cx));
            }
        });
        self.input.update(cx, |input, _cx| {
            input.scroll_y = layout.scroll_y;
            input.content_height = layout.content_height;
            input.last_layout = Some(layout);
            input.last_bounds = Some(bounds);
        });
    }
}
