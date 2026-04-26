use crate::editor::EditorState;
use crate::explorer::{format_size, ExplorerState, FileContent};
use crate::lsp::LspManager;

use super::editor_view;

/// Persistent editor UI state -- lives alongside the ExplorerState.
pub struct EditorViewState {
    pub editor: EditorState,
    pub lsp: Option<LspManager>,
    pub status_msg: Option<String>,
    pub clipboard_out: Option<String>,
    pub clipboard_in: Option<String>,
    /// Hover tooltip text, if any.
    pub hover_text: Option<String>,
    /// Position the hover was requested at (to dismiss when cursor moves).
    pub hover_pos: Option<(usize, usize)>,
    /// Go-to-definition result to apply next frame (path, line, col).
    pub goto_target: Option<(std::path::PathBuf, u32, u32)>,
    /// Active completion popup state.
    pub completion: Option<CompletionState>,
    /// Code actions popup: list of available actions.
    pub code_actions_popup: Option<Vec<crate::lsp::CodeAction>>,
    pub code_actions_selected: usize,
    /// Document symbols popup.
    pub symbols_popup: Option<Vec<crate::lsp::SymbolInfo>>,
    pub symbols_selected: usize,
    pub symbols_filter: String,
    /// Rename input state.
    pub rename_input: Option<String>,
}

/// State for the auto-completion popup.
pub struct CompletionState {
    pub items: Vec<crate::lsp::CompletionItem>,
    pub selected: usize,
    /// Filter text typed since the completion was triggered.
    pub filter: String,
    /// Cursor position where completion was triggered.
    pub trigger_line: usize,
    pub trigger_col: usize,
}

impl Default for EditorViewState {
    fn default() -> Self {
        Self {
            editor: EditorState::new(),
            lsp: None,
            status_msg: None,
            clipboard_out: None,
            clipboard_in: None,
            hover_text: None,
            hover_pos: None,
            goto_target: None,
            completion: None,
            code_actions_popup: None,
            code_actions_selected: 0,
            symbols_popup: None,
            symbols_selected: 0,
            symbols_filter: String::new(),
            rename_input: None,
        }
    }
}

impl EditorViewState {
    pub fn init_lsp(&mut self, proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>) {
        if self.lsp.is_none() {
            self.lsp = Some(LspManager::new(proxy));
        }
    }

    pub fn open_file(&mut self, path: std::path::PathBuf) -> Result<usize, String> {
        let idx = self.editor.open(path)?;

        if let Some(lsp) = &mut self.lsp {
            let buf = &self.editor.buffers[idx];
            let view = &self.editor.views[idx];
            if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
                if let Some(root) = LspManager::detect_root(path) {
                    lsp.set_root(root);
                }
                if lsp.ensure_server(lang_id) {
                    let text = buf.text();
                    lsp.did_open(path, lang_id, &text);
                }
            }
        }

        Ok(idx)
    }

    pub fn lsp_did_change(&mut self) {
        let Some(lsp) = &mut self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            let text = buf.text();
            lsp.did_change(path, lang_id, &text);
        }
    }

    /// Request hover info at the current cursor position.
    pub fn request_hover(&mut self) {
        let Some(lsp) = &mut self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let pos = view.cursor.pos;
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            if let Some(text) = lsp.hover(path, lang_id, pos.line as u32, pos.col as u32) {
                self.hover_text = Some(text);
                self.hover_pos = Some((pos.line, pos.col));
            } else {
                self.hover_text = None;
                self.hover_pos = None;
            }
        }
    }

    /// Request go-to-definition at the current cursor position.
    pub fn request_goto_definition(&mut self) {
        let Some(lsp) = &mut self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let pos = view.cursor.pos;
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            self.goto_target = lsp.definition(path, lang_id, pos.line as u32, pos.col as u32);
        }
    }

    /// Apply a pending goto target (open file, jump to position).
    pub fn apply_goto(&mut self) {
        let Some((path, line, col)) = self.goto_target.take() else { return };
        match self.open_file(path) {
            Ok(idx) => {
                let view = &mut self.editor.views[idx];
                view.cursor.pos = crate::editor::buffer::Position::new(line as usize, col as usize);
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
                self.status_msg = None;
            }
            Err(e) => self.status_msg = Some(format!("Go to definition failed: {e}")),
        }
    }

    /// Request completions at the current cursor position.
    pub fn request_completion(&mut self) {
        let Some(lsp) = &mut self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let pos = view.cursor.pos;
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            let items = lsp.completion(path, lang_id, pos.line as u32, pos.col as u32);
            if !items.is_empty() {
                self.completion = Some(CompletionState {
                    items,
                    selected: 0,
                    filter: String::new(),
                    trigger_line: pos.line,
                    trigger_col: pos.col,
                });
            }
        }
    }

    /// Get filtered completion items for the popup.
    pub fn filtered_completions(&self) -> Vec<&crate::lsp::CompletionItem> {
        let Some(state) = &self.completion else { return Vec::new() };
        if state.filter.is_empty() {
            state.items.iter().take(20).collect()
        } else {
            let lower = state.filter.to_lowercase();
            state.items.iter()
                .filter(|i| i.label.to_lowercase().contains(&lower))
                .take(20)
                .collect()
        }
    }

    /// Format the active document via LSP.
    pub fn format_document(&mut self) {
        let Some(lsp) = &mut self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            let edits = lsp.format(path, lang_id);
            if edits.is_empty() {
                self.status_msg = Some("No formatting changes".to_string());
                return;
            }
            // Apply edits in reverse order to preserve positions
            let buf = &mut self.editor.buffers[active];
            let mut sorted = edits;
            sorted.sort_by(|a, b| b.start_line.cmp(&a.start_line).then(b.start_col.cmp(&a.start_col)));
            for edit in sorted {
                let start = crate::editor::buffer::Position::new(edit.start_line as usize, edit.start_col as usize);
                let end = crate::editor::buffer::Position::new(edit.end_line as usize, edit.end_col as usize);
                buf.replace(start, end, &edit.new_text);
            }
            self.editor.views[active].tree_dirty = true;
            self.lsp_did_change();
            self.status_msg = Some("Formatted".to_string());
        }
    }

    /// Rename the symbol at cursor. Prompts for new name via status_msg.
    pub fn rename_symbol(&mut self, new_name: &str) {
        let Some(lsp) = &mut self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let pos = view.cursor.pos;
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            let file_edits = lsp.rename(path, lang_id, pos.line as u32, pos.col as u32, new_name);
            if file_edits.is_empty() {
                self.status_msg = Some("Rename returned no changes".to_string());
                return;
            }
            let mut total = 0;
            for (file_path, edits) in &file_edits {
                // Only apply edits to the current open buffer for now
                if self.editor.buffers[active].path() == Some(file_path.as_path()) {
                    let buf = &mut self.editor.buffers[active];
                    let mut sorted = edits.clone();
                    sorted.sort_by(|a, b| b.start_line.cmp(&a.start_line).then(b.start_col.cmp(&a.start_col)));
                    for edit in &sorted {
                        let start = crate::editor::buffer::Position::new(edit.start_line as usize, edit.start_col as usize);
                        let end = crate::editor::buffer::Position::new(edit.end_line as usize, edit.end_col as usize);
                        buf.replace(start, end, &edit.new_text);
                        total += 1;
                    }
                }
            }
            self.editor.views[active].tree_dirty = true;
            self.lsp_did_change();
            self.status_msg = Some(format!("Renamed: {total} occurrence{}", if total == 1 { "" } else { "s" }));
        }
    }

    /// Request code actions at the cursor position.
    pub fn request_code_actions(&mut self) -> Vec<crate::lsp::CodeAction> {
        let Some(lsp) = &mut self.lsp else { return Vec::new() };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return Vec::new() }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let pos = view.cursor.pos;
        let (start, end) = view.cursor.selection().unwrap_or((pos, pos));
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            lsp.code_actions(path, lang_id, start.line as u32, start.col as u32, end.line as u32, end.col as u32)
        } else {
            Vec::new()
        }
    }

    /// Apply a code action's workspace edits.
    pub fn apply_code_action(&mut self, action: &crate::lsp::CodeAction) {
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return }
        let mut total = 0;
        for (file_path, edits) in &action.edits {
            if self.editor.buffers[active].path() == Some(file_path.as_path()) {
                let buf = &mut self.editor.buffers[active];
                let mut sorted = edits.clone();
                sorted.sort_by(|a, b| b.start_line.cmp(&a.start_line).then(b.start_col.cmp(&a.start_col)));
                for edit in &sorted {
                    let start = crate::editor::buffer::Position::new(edit.start_line as usize, edit.start_col as usize);
                    let end = crate::editor::buffer::Position::new(edit.end_line as usize, edit.end_col as usize);
                    buf.replace(start, end, &edit.new_text);
                    total += 1;
                }
            }
        }
        if total > 0 {
            self.editor.views[active].tree_dirty = true;
            self.lsp_did_change();
        }
        self.status_msg = Some(format!("Applied: {}", action.title));
    }

    /// Request document symbols for the active buffer.
    pub fn request_document_symbols(&mut self) -> Vec<crate::lsp::SymbolInfo> {
        let Some(lsp) = &mut self.lsp else { return Vec::new() };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return Vec::new() }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            lsp.document_symbols(path, lang_id)
        } else {
            Vec::new()
        }
    }

    pub fn lsp_did_save(&mut self) {
        let Some(lsp) = &mut self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            let text = buf.text();
            lsp.did_save(path, lang_id, &text);
        }
    }
}

pub(crate) fn render_explorer_view(
    ui: &mut egui::Ui,
    explorer: &mut ExplorerState,
    editor_state: &mut EditorViewState,
) {
    ui.visuals_mut().override_text_color = Some(egui::Color32::WHITE);

    // ── Editor mode: show tabs + active buffer ──
    if !editor_state.editor.is_empty() {
        render_editor_tabs(ui, editor_state);
    } else if explorer.open_file.is_some() {
        render_image_viewer(ui, explorer);
    } else {
        render_file_browser(ui, explorer, editor_state);
    }
}

/// Render the tab bar and active buffer editor.
fn render_editor_tabs(ui: &mut egui::Ui, editor_state: &mut EditorViewState) {
    let tabs = editor_state.editor.tab_info();
    let mut switch_to: Option<usize> = None;
    let mut close_tab: Option<usize> = None;

    ui.horizontal(|ui| {
        if ui
            .add(
                egui::Button::new(
                    egui::RichText::new("< Files").size(13.0).color(egui::Color32::from_rgb(100, 180, 255)),
                )
                .fill(egui::Color32::TRANSPARENT),
            )
            .clicked()
        {
            // Go back to file browser without closing buffers
        }

        ui.add_space(8.0);

        for (i, &(name, active, modified)) in tabs.iter().enumerate() {
            let label = if modified { format!("{name} *") } else { name.to_string() };
            let btn_color = if active { egui::Color32::from_rgb(50, 80, 130) } else { egui::Color32::from_rgb(35, 35, 45) };
            let text_color = if active { egui::Color32::WHITE } else { egui::Color32::from_rgb(160, 160, 175) };
            let tab_btn = ui.add(
                egui::Button::new(egui::RichText::new(&label).size(12.0).color(text_color))
                    .fill(btn_color)
                    .rounding(egui::Rounding { nw: 4.0, ne: 4.0, sw: 0.0, se: 0.0 }),
            );
            if tab_btn.clicked() { switch_to = Some(i); }
            if tab_btn.middle_clicked() || tab_btn.secondary_clicked() { close_tab = Some(i); }
        }

        if let Some(msg) = &editor_state.status_msg {
            ui.add_space(12.0);
            ui.label(egui::RichText::new(msg).size(11.0).color(egui::Color32::from_rgb(150, 150, 160)));
        }
    });

    if let Some(idx) = switch_to { editor_state.editor.switch_to(idx); }
    if let Some(idx) = close_tab {
        editor_state.editor.close(idx);
        if editor_state.editor.is_empty() { return; }
    }

    ui.add_space(2.0);

    // Reparse syntax tree if dirty
    editor_state.editor.reparse_active();

    let active = editor_state.editor.active;
    if active < editor_state.editor.buffers.len() {
        let diags = editor_state.lsp.as_ref().and_then(|lsp| {
            let path = editor_state.editor.buffers[active].path()?;
            let d = lsp.get_diagnostics(path);
            if d.is_empty() { None } else { Some(d.to_vec()) }
        });

        let len_before = editor_state.editor.buffers[active].len_chars();
        let was_modified = editor_state.editor.buffers[active].is_modified();

        let hover_text = editor_state.hover_text.as_deref().map(|s| s.to_string());
        // Clone completion items to avoid borrow conflicts
        let completion_snapshot: Option<(Vec<crate::lsp::CompletionItem>, usize)> =
            editor_state.completion.as_ref().map(|c| {
                let lower = c.filter.to_lowercase();
                let filtered: Vec<_> = if c.filter.is_empty() {
                    c.items.iter().take(20).cloned().collect()
                } else {
                    c.items.iter()
                        .filter(|i| i.label.to_lowercase().contains(&lower))
                        .take(20).cloned().collect()
                };
                (filtered, c.selected)
            });
        let completions_refs: Vec<&crate::lsp::CompletionItem> = match &completion_snapshot {
            Some((items, _)) if !items.is_empty() => items.iter().collect(),
            _ => Vec::new(),
        };
        let completions_arg = match &completion_snapshot {
            Some((_, sel)) if !completions_refs.is_empty() => {
                Some((completions_refs.as_slice(), *sel))
            }
            _ => None,
        };

        let buf = &mut editor_state.editor.buffers[active];
        let view = &mut editor_state.editor.views[active];
        let syntax = &editor_state.editor.syntax;
        let frame_result = editor_view::render_text_editor(
            ui, buf, view, syntax, diags.as_deref(),
            hover_text.as_deref(),
            completions_arg,
            &mut editor_state.status_msg,
            &mut editor_state.clipboard_out,
            &mut editor_state.clipboard_in,
        );

        let len_after = editor_state.editor.buffers[active].len_chars();
        let is_modified = editor_state.editor.buffers[active].is_modified();
        if len_before != len_after {
            editor_state.lsp_did_change();
            editor_state.hover_text = None; // Dismiss hover on edit
        }
        if was_modified && !is_modified { editor_state.lsp_did_save(); }

        // Handle LSP key actions
        if frame_result.key_action.goto_definition {
            editor_state.request_goto_definition();
            editor_state.apply_goto();
        }
        if frame_result.key_action.request_hover {
            editor_state.request_hover();
        }
        if frame_result.key_action.request_completion {
            editor_state.request_completion();
        }
        // Completion navigation
        if let Some(ref mut comp) = editor_state.completion {
            if frame_result.key_action.dismiss_completion {
                editor_state.completion = None;
            } else if frame_result.key_action.completion_down {
                comp.selected = (comp.selected + 1).min(comp.items.len().saturating_sub(1));
            } else if frame_result.key_action.completion_up {
                comp.selected = comp.selected.saturating_sub(1);
            } else if frame_result.key_action.accept_completion {
                // Clone out the insert text to avoid borrow conflicts
                let insert_text = {
                    let snapshot = &completion_snapshot;
                    snapshot.as_ref().and_then(|(items, _)| {
                        items.get(comp.selected).map(|item| {
                            item.insert_text.clone().unwrap_or_else(|| item.label.clone())
                        })
                    })
                };
                if let Some(insert) = insert_text {
                    let buf = &mut editor_state.editor.buffers[active];
                    let view = &mut editor_state.editor.views[active];
                    let start = crate::editor::buffer::Position::new(comp.trigger_line, comp.trigger_col);
                    let end = view.cursor.pos;
                    buf.replace(start, end, &insert);
                    let new_col = comp.trigger_col + insert.chars().count();
                    view.cursor.pos = crate::editor::buffer::Position::new(comp.trigger_line, new_col);
                    view.cursor.desired_col = None;
                    view.tree_dirty = true;
                    editor_state.lsp_did_change();
                }
                editor_state.completion = None;
            }
        }

        // Format document
        if frame_result.key_action.format_document {
            editor_state.format_document();
        }

        // Rename symbol: open input or apply
        if frame_result.key_action.rename_symbol && editor_state.rename_input.is_none() {
            // Get current word at cursor for prefill
            let word = {
                let buf = &editor_state.editor.buffers[active];
                let pos = editor_state.editor.views[active].cursor.pos;
                let line = buf.line(pos.line);
                let chars: Vec<char> = line.chars().collect();
                let mut start = pos.col;
                let mut end = pos.col;
                while start > 0 && chars.get(start - 1).is_some_and(|c| c.is_alphanumeric() || *c == '_') { start -= 1; }
                while end < chars.len() && chars.get(end).is_some_and(|c| c.is_alphanumeric() || *c == '_') { end += 1; }
                chars[start..end].iter().collect::<String>()
            };
            editor_state.rename_input = Some(word);
        }

        // Code actions
        if frame_result.key_action.code_actions {
            let actions = editor_state.request_code_actions();
            if actions.is_empty() {
                editor_state.status_msg = Some("No code actions available".to_string());
            } else {
                editor_state.code_actions_popup = Some(actions);
                editor_state.code_actions_selected = 0;
            }
        }

        // Document symbols
        if frame_result.key_action.document_symbols {
            let symbols = editor_state.request_document_symbols();
            if symbols.is_empty() {
                editor_state.status_msg = Some("No symbols found".to_string());
            } else {
                editor_state.symbols_popup = Some(symbols);
                editor_state.symbols_selected = 0;
                editor_state.symbols_filter.clear();
            }
        }
    }
}

/// Render the image viewer for non-text files.
fn render_image_viewer(ui: &mut egui::Ui, explorer: &mut ExplorerState) {
    let file_name = explorer.open_file.as_ref().unwrap().name.clone();
    let mut close = false;
    ui.horizontal(|ui| {
        if ui
            .add(
                egui::Button::new(
                    egui::RichText::new("< Back").size(14.0).color(egui::Color32::from_rgb(100, 180, 255)),
                )
                .fill(egui::Color32::TRANSPARENT),
            )
            .clicked()
        {
            close = true;
        }
        ui.label(egui::RichText::new(&file_name).size(18.0).color(egui::Color32::WHITE).strong());
    });
    if close { explorer.close_file(); return; }

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    let open = explorer.open_file.as_mut().unwrap();
    match &mut open.content {
        FileContent::Text(_) => {} // Text files go through editor
        FileContent::Image { rgba, width, height, texture } => {
            let handle = texture.get_or_insert_with(|| {
                ui.ctx().load_texture(
                    "explorer_image",
                    egui::ColorImage::from_rgba_unmultiplied([*width as usize, *height as usize], rgba),
                    Default::default(),
                )
            });
            let available = ui.available_size();
            let scale = (available.x / *width as f32).min(available.y / *height as f32).min(1.0);
            let display_size = egui::Vec2::new(*width as f32 * scale, *height as f32 * scale);
            egui::ScrollArea::both().auto_shrink([false; 2]).show(ui, |ui| {
                ui.image(egui::load::SizedTexture::new(handle.id(), display_size));
            });
        }
    }
}

/// Render the directory file browser.
fn render_file_browser(
    ui: &mut egui::Ui,
    explorer: &mut ExplorerState,
    editor_state: &mut EditorViewState,
) {
    ui.horizontal(|ui| {
        if ui
            .add(
                egui::Button::new(
                    egui::RichText::new("< Up").size(14.0).color(egui::Color32::from_rgb(100, 180, 255)),
                )
                .fill(egui::Color32::TRANSPARENT),
            )
            .clicked()
        {
            explorer.go_up();
        }
        ui.label(
            egui::RichText::new(explorer.current_dir.display().to_string())
                .size(14.0)
                .color(egui::Color32::WHITE),
        );
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

    if let Some(err) = &explorer.error {
        ui.label(egui::RichText::new(err).size(14.0).color(egui::Color32::from_rgb(255, 100, 100)));
        ui.add_space(8.0);
    }

    egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
        for i in 0..explorer.entries.len() {
            let is_dir = explorer.entries[i].is_dir;
            let name = explorer.entries[i].name.clone();
            let size = explorer.entries[i].size;

            let label = if is_dir { format!("/{name}") } else { name.clone() };

            let row = ui.horizontal(|ui| {
                let dir_color = egui::Color32::from_rgb(100, 180, 255);
                let text = if is_dir {
                    egui::RichText::new(&label).size(14.0).color(dir_color).strong()
                } else {
                    egui::RichText::new(&label).size(14.0).color(egui::Color32::WHITE)
                };
                let response = ui.add(egui::Label::new(text).sense(egui::Sense::click()));
                if !is_dir {
                    ui.label(
                        egui::RichText::new(format_size(size)).size(12.0).color(egui::Color32::from_rgb(120, 120, 130)),
                    );
                }
                response
            });

            if row.inner.clicked() {
                let path = explorer.entries[i].path.clone();
                if is_dir {
                    explorer.navigate(path);
                    break;
                } else if is_image_ext(&path) {
                    explorer.open(path);
                    break;
                } else {
                    match editor_state.open_file(path) {
                        Ok(_) => editor_state.status_msg = None,
                        Err(e) => editor_state.status_msg = Some(e),
                    }
                    break;
                }
            }
        }
    });
}

fn is_image_ext(path: &std::path::Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tiff" | "tif" | "ico")
}
