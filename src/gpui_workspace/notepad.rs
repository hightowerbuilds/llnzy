//! Home's writing surface; one notebook shared by all app windows.
use std::{path::PathBuf, time::Duration};

use gpui::{
    div, prelude::*, px, rgb, Context, Entity, Focusable, Global, MouseButton, Render,
    Subscription, Task, Window,
};

use super::{home::home_palette, WorkspacePalette};
use crate::{
    config::Config,
    gpui_editor::EditorPrototype,
    notebook::{date_label, now_ms, Notebook},
};

struct SharedNotebook(Entity<NotebookStore>);
impl Global for SharedNotebook {}

struct NotebookStore {
    book: Option<Notebook>,
    path: Option<PathBuf>,
    dirty: bool,
    error: Option<String>,
    save_task: Option<Task<()>>,
}

impl NotebookStore {
    fn shared(cx: &mut gpui::App) -> Entity<Self> {
        if let Some(shared) = cx.try_global::<SharedNotebook>() {
            return shared.0.clone();
        }
        let entity = cx.new(|cx| {
            let path = crate::platform::paths::current_paths()
                .map(|p| p.data_dir.join("notes/notebook.json"));
            let loaded = path
                .as_ref()
                .ok_or_else(|| std::io::Error::other("Notes folder unavailable"))
                .and_then(|path| Notebook::load(path));
            let (book, error) = match loaded {
                Ok(mut book) => {
                    if book.notes.is_empty() {
                        book.new_note(now_ms());
                    }
                    (Some(book), None)
                }
                Err(error) => {
                    log::error!("notepad: {error}");
                    (None, Some(format!("Couldn't load notes: {error}")))
                }
            };
            cx.on_app_quit(|this: &mut Self, _| {
                this.flush();
                async {}
            })
            .detach();
            Self {
                book,
                path,
                dirty: false,
                error,
                save_task: None,
            }
        });
        cx.set_global(SharedNotebook(entity.clone()));
        entity
    }

    fn flush(&mut self) {
        if !self.dirty {
            return;
        }
        let result = match (&self.book, &self.path) {
            (Some(book), Some(path)) => book.save(path),
            _ => Err(std::io::Error::other("Notes folder unavailable")),
        };
        match result {
            Ok(()) => {
                self.dirty = false;
                self.error = None;
            }
            Err(error) => {
                log::error!("notepad save: {error}");
                self.error = Some("Not saved — click to retry".into());
            }
        }
    }

    fn update_fields(
        &mut self,
        id: u64,
        text: Option<String>,
        title: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let Some(book) = self.book.as_mut() else {
            return;
        };
        let body_changed = text.is_some_and(|text| book.update(id, text));
        let title_changed = title.is_some_and(|title| book.update_title(id, title));
        if !body_changed && !title_changed {
            return;
        }
        self.dirty = true;
        self.save_task = Some(
            cx.spawn(|this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    cx.background_executor()
                        .timer(Duration::from_millis(400))
                        .await;
                    let _ = this.update(&mut cx, |this, cx| {
                        this.flush();
                        cx.notify();
                    });
                }
            }),
        );
        cx.notify();
    }
}

pub(super) struct Notepad {
    store: Entity<NotebookStore>,
    pub(super) editor: Entity<EditorPrototype>,
    title_editor: Entity<EditorPrototype>,
    selected: Option<u64>,
    last_editor_text: String,
    last_title: String,
    palette: WorkspacePalette,
    _subscriptions: Vec<Subscription>,
}

impl Notepad {
    pub(super) fn new(config: Config, cx: &mut Context<Self>) -> Self {
        let store = NotebookStore::shared(cx);
        let selected = store
            .read(cx)
            .book
            .as_ref()
            .and_then(|book| book.notes.first())
            .map(|n| n.id);
        let text = store
            .read(cx)
            .book
            .as_ref()
            .and_then(|book| book.notes.first())
            .map(|n| n.body.clone())
            .unwrap_or_default();
        let title = store
            .read(cx)
            .book
            .as_ref()
            .and_then(|book| book.notes.first())
            .map(|note| note.title.clone())
            .unwrap_or_default();
        let config = writing_config(config);
        let editor = cx.new(EditorPrototype::notepad);
        editor.update(cx, |editor, cx| {
            editor.set_note_text(&text, cx);
            editor.set_appearance_config(config.clone(), cx);
        });
        let body_focus = editor.focus_handle(cx);
        let title_editor = cx.new(|cx| EditorPrototype::note_title(body_focus, cx));
        title_editor.update(cx, |editor, cx| {
            editor.set_note_text(&title, cx);
            editor.set_appearance_config(config.clone(), cx);
        });
        let title_edits = cx.observe(&title_editor, |this: &mut Self, _, cx| this.capture(cx));
        let edits = cx.observe(&editor, |this: &mut Self, _, cx| {
            this.capture(cx);
        });
        let updates = cx.observe(&store, |this: &mut Self, _, cx| {
            let note = this
                .store
                .read(cx)
                .book
                .as_ref()
                .and_then(|book| book.notes.iter().find(|n| Some(n.id) == this.selected))
                .cloned();
            if let Some(note) = note {
                if this.editor.read(cx).note_text() != note.body {
                    this.last_editor_text = note.body.clone();
                    this.editor
                        .update(cx, |editor, cx| editor.set_note_text(&note.body, cx));
                }
                if this.title_editor.read(cx).note_text() != note.title {
                    this.last_title = note.title.clone();
                    this.title_editor
                        .update(cx, |editor, cx| editor.set_note_text(&note.title, cx));
                }
            }
            cx.notify();
        });
        let quit = cx.on_app_quit(|this: &mut Self, cx| {
            this.capture(cx);
            this.store.update(cx, |store, _| store.flush());
            async {}
        });
        let release = cx.on_release(|this: &mut Self, cx| {
            // Capture the final edit even when the debounce hasn't fired.
            this.capture(cx);
            this.store.update(cx, |store, _| store.flush());
        });
        Self {
            store,
            editor,
            selected,
            last_editor_text: text,
            title_editor,
            last_title: title,
            palette: home_palette(),
            _subscriptions: vec![edits, title_edits, updates, quit, release],
        }
    }

    pub(super) fn set_config(&mut self, config: Config, cx: &mut Context<Self>) {
        let config = writing_config(config);
        self.palette = home_palette();
        self.title_editor.update(cx, |editor, cx| {
            editor.set_appearance_config(config.clone(), cx)
        });
        self.editor
            .update(cx, |editor, cx| editor.set_appearance_config(config, cx));
        cx.notify();
    }

    pub(super) fn focused_editor(
        &self,
        window: &Window,
        cx: &gpui::App,
    ) -> Entity<EditorPrototype> {
        if self.title_editor.focus_handle(cx).is_focused(window) {
            self.title_editor.clone()
        } else {
            self.editor.clone()
        }
    }

    fn capture(&mut self, cx: &mut gpui::App) {
        if let Some(id) = self.selected {
            let text = self.editor.read(cx).note_text();
            let title = self.title_editor.read(cx).note_text();
            // Publish only fields edited here; an idle field may be stale
            // while another window's notification is still being delivered.
            let body_change = (text != self.last_editor_text).then_some(text.clone());
            let title_change = (title != self.last_title).then_some(title.clone());
            if body_change.is_none() && title_change.is_none() {
                return;
            }
            self.last_editor_text = text;
            self.last_title = title;
            self.store.update(cx, |store, cx| {
                store.update_fields(id, body_change, title_change, cx)
            });
        }
    }

    fn select(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.capture(cx);
        if self.selected != Some(id) {
            let text = self
                .store
                .read(cx)
                .book
                .as_ref()
                .and_then(|book| book.notes.iter().find(|n| n.id == id))
                .cloned();
            if let Some(note) = text {
                self.selected = Some(id);
                self.last_editor_text = note.body.clone();
                self.last_title = note.title.clone();
                self.title_editor
                    .update(cx, |editor, cx| editor.set_note_text(&note.title, cx));
                self.editor
                    .update(cx, |editor, cx| editor.set_note_text(&note.body, cx));
            }
        }
        window.focus(&self.editor.focus_handle(cx));
        cx.notify();
    }
}

impl Render for Notepad {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.palette;
        let store = self.store.read(cx);
        let status = store.error.clone().unwrap_or_else(|| {
            if store.dirty {
                "Saving…"
            } else {
                "Saved on this device"
            }
            .into()
        });
        let notes = store.book.as_ref().map(|book| book.notes.clone());
        let mut root = div().w_full().text_size(px(16.0)).flex().flex_col().gap_3();
        let Some(notes) = notes else {
            return root.child(div().text_color(rgb(palette.muted_text)).child(status));
        };
        let selected = notes.iter().find(|note| Some(note.id) == self.selected);
        root = root
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(16.0))
                            .text_color(rgb(palette.active_text))
                            .child("Notepad"),
                    )
                    .child(
                        div()
                            .id("new-note")
                            .cursor_pointer()
                            .px_3()
                            .py_1()
                            .rounded_sm()
                            .text_size(px(16.0))
                            .text_color(rgb(palette.sidebar_text))
                            .hover(|style| style.bg(rgb(palette.panel_bg)))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, window, cx| {
                                    this.capture(cx);
                                    let id = this.store.update(cx, |store, cx| {
                                        let id =
                                            store.book.as_mut().map(|book| book.new_note(now_ms()));
                                        cx.notify();
                                        id
                                    });
                                    if let Some(id) = id {
                                        this.select(id, window, cx);
                                        window.focus(&this.title_editor.focus_handle(cx));
                                    }
                                }),
                            )
                            .child("+ New note"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap_2()
                    .text_size(px(16.0))
                    .text_color(rgb(palette.muted_text))
                    .child(
                        selected
                            .map(|n| date_label(n.created_ms))
                            .unwrap_or_default(),
                    )
                    .child(
                        div()
                            .id("note-save-status")
                            .cursor_pointer()
                            .child(status)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.capture(cx);
                                    this.store.update(cx, |store, cx| {
                                        store.flush();
                                        cx.notify();
                                    });
                                }),
                            ),
                    ),
            )
            .child(div().text_color(rgb(palette.muted_text)).child("Title"))
            .child(
                div()
                    .h(px(56.0))
                    .w_full()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.editor_bg))
                    .rounded_sm()
                    .overflow_hidden()
                    .px_3()
                    .child(self.title_editor.clone()),
            )
            .child(
                div()
                    .h(px(360.0))
                    .w_full()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .rounded_sm()
                    .overflow_hidden()
                    .bg(rgb(palette.editor_bg))
                    .p_4()
                    .child(self.editor.clone()),
            );
        let mut history = div()
            .id("notes-history")
            .max_h(px(160.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_1();
        for note in notes {
            let id = note.id;
            let active = Some(id) == self.selected;
            history = history.child(
                div()
                    .id(("note", id))
                    .flex()
                    .justify_between()
                    .items_center()
                    .gap_3()
                    .px_2()
                    .py_2()
                    .rounded_sm()
                    .when(active, |row| row.bg(rgb(palette.panel_bg)))
                    .hover(|row| row.bg(rgb(palette.panel_bg)))
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window, cx| this.select(id, window, cx)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .truncate()
                            .text_size(px(16.0))
                            .text_color(rgb(palette.sidebar_text))
                            .child(note.title()),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(px(16.0))
                            .text_color(rgb(palette.muted_text))
                            .child(date_label(note.created_ms)),
                    ),
            );
        }
        root.child(history)
    }
}

fn writing_config(mut config: Config) -> Config {
    config.colors.background = [27, 27, 27];
    config.colors.foreground = [220, 220, 220];
    config.colors.cursor = [220, 220, 220];
    config.cursor_style = crate::config::CursorStyle::Beam;
    config
}
