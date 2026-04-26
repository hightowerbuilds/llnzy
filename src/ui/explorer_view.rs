use crate::editor::buffer::Position;
use crate::editor::EditorState;
use crate::explorer::{format_size, ExplorerState, FileContent};
use crate::lsp::{DiagSeverity, LspManager};

/// Auto-closing bracket pairs.
const PAIRS: &[(char, char)] = &[
    ('(', ')'),
    ('[', ']'),
    ('{', '}'),
    ('"', '"'),
    ('\'', '\''),
    ('`', '`'),
];

/// Persistent editor UI state — lives alongside the ExplorerState.
pub struct EditorViewState {
    pub editor: EditorState,
    pub lsp: Option<LspManager>,
    pub status_msg: Option<String>,
    pub clipboard_out: Option<String>,
    pub clipboard_in: Option<String>,
}

impl Default for EditorViewState {
    fn default() -> Self {
        Self {
            editor: EditorState::new(),
            lsp: None,
            status_msg: None,
            clipboard_out: None,
            clipboard_in: None,
        }
    }
}

impl EditorViewState {
    /// Initialize the LSP manager with the event loop proxy.
    pub fn init_lsp(&mut self, proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>) {
        if self.lsp.is_none() {
            self.lsp = Some(LspManager::new(proxy));
        }
    }

    /// Open a file, starting the LSP server if available.
    pub fn open_file(&mut self, path: std::path::PathBuf) -> Result<usize, String> {
        let idx = self.editor.open(path)?;

        // Start LSP for this language if available
        if let Some(lsp) = &mut self.lsp {
            let buf = &self.editor.buffers[idx];
            let view = &self.editor.views[idx];
            if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
                // Detect project root
                if let Some(root) = LspManager::detect_root(path) {
                    lsp.set_root(root);
                }
                // Start server (no-op if already running)
                if lsp.ensure_server(lang_id) {
                    let text = buf.text();
                    lsp.did_open(path, lang_id, &text);
                }
            }
        }

        Ok(idx)
    }

    /// Notify LSP of a document change (call after edits).
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

    /// Notify LSP of a save.
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
    // Force white text everywhere in the explorer, overriding egui defaults
    ui.visuals_mut().override_text_color = Some(egui::Color32::WHITE);

    // ── Editor mode: show tabs + active buffer ──
    if !editor_state.editor.is_empty() {
        // Tab bar
        let tabs = editor_state.editor.tab_info();
        let mut switch_to: Option<usize> = None;
        let mut close_tab: Option<usize> = None;

        ui.horizontal(|ui| {
            // Back to browser button
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("< Files")
                            .size(13.0)
                            .color(egui::Color32::from_rgb(100, 180, 255)),
                    )
                    .fill(egui::Color32::TRANSPARENT),
                )
                .clicked()
            {
                // Don't close buffers — just go back to the file browser
                explorer.close_file();
            }

            ui.add_space(8.0);

            for (i, &(name, active, modified)) in tabs.iter().enumerate() {
                let label = if modified {
                    format!("{name} *")
                } else {
                    name.to_string()
                };
                let btn_color = if active {
                    egui::Color32::from_rgb(50, 80, 130)
                } else {
                    egui::Color32::from_rgb(35, 35, 45)
                };
                let text_color = if active {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::from_rgb(160, 160, 175)
                };
                let tab_btn = ui.add(
                    egui::Button::new(egui::RichText::new(&label).size(12.0).color(text_color))
                        .fill(btn_color)
                        .rounding(egui::Rounding {
                            nw: 4.0, ne: 4.0, sw: 0.0, se: 0.0,
                        }),
                );
                if tab_btn.clicked() {
                    switch_to = Some(i);
                }
                // Middle-click or right-click to close
                if tab_btn.middle_clicked() || (tab_btn.secondary_clicked()) {
                    close_tab = Some(i);
                }
            }

            // Status message
            if let Some(msg) = &editor_state.status_msg {
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new(msg)
                        .size(11.0)
                        .color(egui::Color32::from_rgb(150, 150, 160)),
                );
            }
        });

        if let Some(idx) = switch_to {
            editor_state.editor.switch_to(idx);
        }
        if let Some(idx) = close_tab {
            editor_state.editor.close(idx);
            if editor_state.editor.is_empty() {
                return;
            }
        }

        ui.add_space(2.0);

        // Reparse syntax tree if dirty, then render
        editor_state.editor.reparse_active();

        let active = editor_state.editor.active;
        if active < editor_state.editor.buffers.len() {
            // Get diagnostics for this file from LSP
            let diags = editor_state.lsp.as_ref().and_then(|lsp| {
                let path = editor_state.editor.buffers[active].path()?;
                let d = lsp.get_diagnostics(path);
                if d.is_empty() { None } else { Some(d.to_vec()) }
            });

            let len_before = editor_state.editor.buffers[active].len_chars();
            let was_modified = editor_state.editor.buffers[active].is_modified();

            let buf = &mut editor_state.editor.buffers[active];
            let view = &mut editor_state.editor.views[active];
            let syntax = &editor_state.editor.syntax;
            render_text_editor(
                ui, buf, view, syntax, diags.as_deref(),
                &mut editor_state.status_msg,
                &mut editor_state.clipboard_out,
                &mut editor_state.clipboard_in,
            );

            let len_after = editor_state.editor.buffers[active].len_chars();
            let is_modified = editor_state.editor.buffers[active].is_modified();

            // Notify LSP of changes
            if len_before != len_after {
                editor_state.lsp_did_change();
            }
            // Detect save (was modified, now not)
            if was_modified && !is_modified {
                editor_state.lsp_did_save();
            }
        }
    } else if explorer.open_file.is_some() {
        // ── Image viewer (non-text files stay in explorer.open_file) ──
        let file_name = explorer.open_file.as_ref().unwrap().name.clone();
        let mut close = false;
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("< Back")
                            .size(14.0)
                            .color(egui::Color32::from_rgb(100, 180, 255)),
                    )
                    .fill(egui::Color32::TRANSPARENT),
                )
                .clicked()
            {
                close = true;
            }
            ui.label(
                egui::RichText::new(&file_name)
                    .size(18.0)
                    .color(egui::Color32::WHITE)
                    .strong(),
            );
        });
        if close {
            explorer.close_file();
            return;
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        let open = explorer.open_file.as_mut().unwrap();
        match &mut open.content {
            FileContent::Text(_) => {
                // Should not happen — text files go through EditorState now
            }
            FileContent::Image {
                rgba,
                width,
                height,
                texture,
            } => {
                let handle = texture.get_or_insert_with(|| {
                    ui.ctx().load_texture(
                        "explorer_image",
                        egui::ColorImage::from_rgba_unmultiplied(
                            [*width as usize, *height as usize],
                            rgba,
                        ),
                        Default::default(),
                    )
                });

                let available = ui.available_size();
                let img_w = *width as f32;
                let img_h = *height as f32;
                let scale = (available.x / img_w).min(available.y / img_h).min(1.0);
                let display_size = egui::Vec2::new(img_w * scale, img_h * scale);

                egui::ScrollArea::both()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.image(egui::load::SizedTexture::new(handle.id(), display_size));
                    });
            }
        }
    } else {
        // ── Directory browser mode ──
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("< Up")
                            .size(14.0)
                            .color(egui::Color32::from_rgb(100, 180, 255)),
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
            ui.label(
                egui::RichText::new(err)
                    .size(14.0)
                    .color(egui::Color32::from_rgb(255, 100, 100)),
            );
            ui.add_space(8.0);
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for i in 0..explorer.entries.len() {
                    let is_dir = explorer.entries[i].is_dir;
                    let name = explorer.entries[i].name.clone();
                    let size = explorer.entries[i].size;

                    let label = if is_dir {
                        format!("/{name}")
                    } else {
                        name.clone()
                    };

                    let row = ui.horizontal(|ui| {
                        let dir_color = egui::Color32::from_rgb(100, 180, 255);
                        let text = if is_dir {
                            egui::RichText::new(&label)
                                .size(14.0)
                                .color(dir_color)
                                .strong()
                        } else {
                            egui::RichText::new(&label)
                                .size(14.0)
                                .color(egui::Color32::WHITE)
                        };

                        let response =
                            ui.add(egui::Label::new(text).sense(egui::Sense::click()));

                        if !is_dir {
                            ui.label(
                                egui::RichText::new(format_size(size))
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(120, 120, 130)),
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
                            // Images go through explorer's preview
                            explorer.open(path);
                            break;
                        } else {
                            // Text files go through the editor
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
}

fn is_image_ext(path: &std::path::Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tiff" | "tif" | "ico"
    )
}

/// Render the code editor for a text buffer.
fn render_text_editor(
    ui: &mut egui::Ui,
    buf: &mut crate::editor::buffer::Buffer,
    view: &mut crate::editor::BufferView,
    syntax: &crate::editor::syntax::SyntaxEngine,
    diagnostics: Option<&[crate::lsp::FileDiagnostic]>,
    status_msg: &mut Option<String>,
    clipboard_out: &mut Option<String>,
    clipboard_in: &mut Option<String>,
) {
    let line_count = buf.line_count();
    let gutter_digits = ((line_count as f64).log10().floor() as usize + 1).max(2);
    let char_width = 8.0; // approximate monospace char width at size 13
    let line_height = 18.0;
    let gutter_width = (gutter_digits as f32 + 1.5) * char_width;
    let text_margin = 2.0;

    // Handle keyboard input (may modify buffer)
    let content_before = buf.len_chars();
    let ctx = ui.ctx().clone();
    handle_editor_keys(&ctx, buf, view, status_msg, clipboard_out, clipboard_in, line_height);
    // Mark syntax tree for re-parse if content changed
    if buf.len_chars() != content_before {
        view.tree_dirty = true;
    }

    // Ensure cursor is in bounds
    view.cursor.clamp(buf);

    // Vertical scroll to keep cursor visible
    let available_h = ui.available_height() - 20.0; // minus status bar
    let visible_lines = (available_h / line_height).max(1.0) as usize;
    if view.cursor.pos.line < view.scroll_line {
        view.scroll_line = view.cursor.pos.line;
    } else if view.cursor.pos.line >= view.scroll_line + visible_lines {
        view.scroll_line = view.cursor.pos.line.saturating_sub(visible_lines - 1);
    }

    // Horizontal scroll to keep cursor visible
    let text_area_w = ui.available_width() - gutter_width - text_margin;
    let visible_cols = (text_area_w / char_width).max(1.0) as usize;
    let margin_cols = 4; // keep cursor this many cols from the edge
    if view.cursor.pos.col < view.scroll_col {
        view.scroll_col = view.cursor.pos.col;
    } else if view.cursor.pos.col >= view.scroll_col + visible_cols.saturating_sub(margin_cols) {
        view.scroll_col = view.cursor.pos.col.saturating_sub(visible_cols.saturating_sub(margin_cols));
    }

    let end_line = (view.scroll_line + visible_lines + 2).min(line_count);
    let h_offset = view.scroll_col as f32 * char_width;

    // Status bar text
    let indent_label = match buf.indent_style {
        crate::editor::buffer::IndentStyle::Spaces(n) => format!("Spaces: {n}"),
        crate::editor::buffer::IndentStyle::Tabs => "Tabs".to_string(),
    };
    let status_text = format!(
        "Ln {}, Col {}  |  {} lines  |  {}  |  {}",
        view.cursor.pos.line + 1,
        view.cursor.pos.col + 1,
        line_count,
        indent_label,
        if buf.is_modified() { "Modified" } else { "Saved" },
    );

    // Main editor area with custom painting
    let (response, painter) = ui.allocate_painter(
        egui::Vec2::new(ui.available_width(), available_h),
        egui::Sense::click(),
    );
    let rect = response.rect;

    // Clip text to the text area (not the gutter)
    let text_clip = egui::Rect::from_min_max(
        egui::pos2(rect.left() + gutter_width, rect.top()),
        rect.right_bottom(),
    );

    // Background
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(25, 25, 32));

    // Gutter background
    let gutter_rect = egui::Rect::from_min_size(
        rect.left_top(),
        egui::Vec2::new(gutter_width, rect.height()),
    );
    painter.rect_filled(gutter_rect, 0.0, egui::Color32::from_rgb(30, 30, 38));

    // Handle mouse clicks to position cursor
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let rel_x = pos.x - rect.left() - gutter_width - text_margin + h_offset;
            let rel_y = pos.y - rect.top();
            let click_line = view.scroll_line + (rel_y / line_height) as usize;
            let click_col = (rel_x / char_width).max(0.0) as usize;
            let click_line = click_line.min(line_count.saturating_sub(1));
            let click_col = click_col.min(buf.line_len(click_line));
            view.cursor.clear_selection();
            view.cursor.pos = Position::new(click_line, click_col);
            view.cursor.desired_col = None;
        }
    }

    // Selection highlight
    if let Some((sel_start, sel_end)) = view.cursor.selection() {
        let sel_color = egui::Color32::from_rgba_unmultiplied(60, 100, 180, 80);
        for line_idx in sel_start.line..=sel_end.line {
            if line_idx < view.scroll_line || line_idx >= end_line {
                continue;
            }
            let vis_y = (line_idx - view.scroll_line) as f32 * line_height;
            let line_len = buf.line_len(line_idx);

            let col_start = if line_idx == sel_start.line { sel_start.col } else { 0 };
            let col_end = if line_idx == sel_end.line { sel_end.col } else { line_len };

            let x1 = rect.left() + gutter_width + text_margin + col_start as f32 * char_width - h_offset;
            let x2 = rect.left() + gutter_width + text_margin + col_end as f32 * char_width - h_offset;
            let sel_rect = egui::Rect::from_min_max(
                egui::pos2(x1.max(text_clip.left()), rect.top() + vis_y),
                egui::pos2(x2.max(x1 + char_width).min(text_clip.right()), rect.top() + vis_y + line_height),
            );
            if sel_rect.width() > 0.0 {
                painter.rect_filled(sel_rect, 0.0, sel_color);
            }
        }
    }

    // Compute syntax highlights for visible lines
    let source_text = buf.text();
    let highlight_spans = match (view.lang_id, &view.tree) {
        (Some(lang_id), Some(tree)) => {
            syntax.highlights_for_range(lang_id, tree, source_text.as_bytes(), view.scroll_line, end_line)
        }
        _ => vec![Vec::new(); end_line.saturating_sub(view.scroll_line)],
    };

    // Render visible lines
    let text_color = egui::Color32::WHITE;
    let gutter_color = egui::Color32::from_rgb(100, 100, 120);
    let current_line_gutter = egui::Color32::from_rgb(180, 180, 200);
    let current_line_bg = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 8);
    let font = egui::FontId::monospace(13.0);

    for line_idx in view.scroll_line..end_line {
        let vis_y = (line_idx - view.scroll_line) as f32 * line_height;
        let y = rect.top() + vis_y;

        // Current line highlight
        if line_idx == view.cursor.pos.line {
            let line_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left() + gutter_width, y),
                egui::Vec2::new(rect.width() - gutter_width, line_height),
            );
            painter.rect_filled(line_rect, 0.0, current_line_bg);
        }

        // Line number (gutter — not affected by horizontal scroll)
        let num_str = format!("{:>width$}", line_idx + 1, width = gutter_digits);
        let num_color = if line_idx == view.cursor.pos.line {
            current_line_gutter
        } else {
            gutter_color
        };
        painter.text(
            egui::pos2(rect.left() + 4.0, y + 1.0),
            egui::Align2::LEFT_TOP,
            &num_str,
            font.clone(),
            num_color,
        );

        // Line text with syntax highlighting
        let line_text = buf.line(line_idx);
        if line_text.is_empty() {
            continue;
        }
        let text_x_base = rect.left() + gutter_width + text_margin - h_offset;
        let spans = &highlight_spans[line_idx - view.scroll_line];

        if spans.is_empty() {
            // No highlights — render as plain white
            painter.with_clip_rect(text_clip).text(
                egui::pos2(text_x_base, y + 1.0),
                egui::Align2::LEFT_TOP,
                line_text,
                font.clone(),
                text_color,
            );
        } else {
            // Render character-by-character spans with colors
            let chars: Vec<char> = line_text.chars().collect();
            let mut col = 0;
            while col < chars.len() {
                // Find the highest-priority span covering this column
                let color = spans
                    .iter()
                    .find(|s| col >= s.col_start && col < s.col_end)
                    .map(|s| {
                        let rgb = crate::editor::syntax::group_color(s.group);
                        egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
                    })
                    .unwrap_or(text_color);

                // Batch consecutive characters with the same color
                let start_col = col;
                let span_end = spans
                    .iter()
                    .find(|s| col >= s.col_start && col < s.col_end)
                    .map(|s| s.col_end.min(chars.len()))
                    .unwrap_or(chars.len());

                // Find how far this color extends
                let mut batch_end = col + 1;
                while batch_end < span_end {
                    let next_color = spans
                        .iter()
                        .find(|s| batch_end >= s.col_start && batch_end < s.col_end)
                        .map(|s| {
                            let rgb = crate::editor::syntax::group_color(s.group);
                            egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
                        })
                        .unwrap_or(text_color);
                    if next_color != color {
                        break;
                    }
                    batch_end += 1;
                }

                let chunk: String = chars[start_col..batch_end].iter().collect();
                let x = text_x_base + start_col as f32 * char_width;
                painter.with_clip_rect(text_clip).text(
                    egui::pos2(x, y + 1.0),
                    egui::Align2::LEFT_TOP,
                    &chunk,
                    font.clone(),
                    color,
                );
                col = batch_end;
            }
        }
    }

    // Diagnostic underlines
    if let Some(diags) = diagnostics {
        for diag in diags {
            if diag.line < view.scroll_line as u32 || diag.line >= end_line as u32 {
                continue;
            }
            let vis_y = (diag.line as usize - view.scroll_line) as f32 * line_height;
            let y_base = rect.top() + vis_y + line_height - 2.0;
            let x_start = rect.left() + gutter_width + text_margin
                + diag.col as f32 * char_width - h_offset;
            let x_end = rect.left() + gutter_width + text_margin
                + diag.end_col as f32 * char_width - h_offset;
            let width = (x_end - x_start).max(char_width);

            let color = match diag.severity {
                DiagSeverity::Error => egui::Color32::from_rgb(255, 80, 80),
                DiagSeverity::Warning => egui::Color32::from_rgb(230, 180, 50),
                DiagSeverity::Info => egui::Color32::from_rgb(80, 160, 255),
                DiagSeverity::Hint => egui::Color32::from_rgb(130, 130, 150),
            };

            // Draw squiggly underline (approximated with small segments)
            let segments = ((width / 4.0) as usize).max(2);
            let seg_w = width / segments as f32;
            for i in 0..segments {
                let sx = x_start + i as f32 * seg_w;
                let offset = if i % 2 == 0 { 0.0 } else { 2.0 };
                painter.with_clip_rect(text_clip).line_segment(
                    [
                        egui::pos2(sx, y_base + offset),
                        egui::pos2(sx + seg_w, y_base + 2.0 - offset),
                    ],
                    egui::Stroke::new(1.0, color),
                );
            }

            // Gutter marker
            let gutter_y = rect.top() + vis_y;
            let marker = match diag.severity {
                DiagSeverity::Error => "E",
                DiagSeverity::Warning => "W",
                DiagSeverity::Info => "i",
                DiagSeverity::Hint => ".",
            };
            painter.text(
                egui::pos2(rect.left() + 1.0, gutter_y + 1.0),
                egui::Align2::LEFT_TOP,
                marker,
                egui::FontId::monospace(10.0),
                color,
            );
        }
    }

    // Cursor (blinking beam, clipped to text area)
    if view.cursor.pos.line >= view.scroll_line && view.cursor.pos.line < end_line {
        let vis_y = (view.cursor.pos.line - view.scroll_line) as f32 * line_height;
        let cursor_x = rect.left() + gutter_width + text_margin + view.cursor.pos.col as f32 * char_width - h_offset;
        let cursor_y = rect.top() + vis_y;

        if cursor_x >= text_clip.left() && cursor_x <= text_clip.right() {
            let time = ui.ctx().input(|i| i.time);
            let blink_on = (time * 2.0) as u64 % 2 == 0;
            if blink_on {
                let cursor_color = egui::Color32::from_rgb(80, 160, 255);
                painter.with_clip_rect(text_clip).line_segment(
                    [
                        egui::pos2(cursor_x, cursor_y + 1.0),
                        egui::pos2(cursor_x, cursor_y + line_height - 1.0),
                    ],
                    egui::Stroke::new(2.0, cursor_color),
                );
            }
        }
        ui.ctx().request_repaint();
    }

    // Vertical scrollbar hint
    if line_count > visible_lines {
        let track_h = rect.height();
        let thumb_frac = visible_lines as f32 / line_count as f32;
        let thumb_h = (track_h * thumb_frac).max(20.0);
        let thumb_top = (view.scroll_line as f32 / line_count as f32) * track_h;
        let scrollbar_rect = egui::Rect::from_min_size(
            egui::pos2(rect.right() - 6.0, rect.top() + thumb_top),
            egui::Vec2::new(4.0, thumb_h),
        );
        painter.rect_filled(scrollbar_rect, 2.0, egui::Color32::from_rgba_unmultiplied(180, 180, 200, 40));
    }

    // Status bar
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(status_text)
                .size(12.0)
                .color(egui::Color32::from_rgb(130, 130, 145))
                .monospace(),
        );
    });
}

/// Handle keyboard input for the editor.
fn handle_editor_keys(
    ctx: &egui::Context,
    buf: &mut crate::editor::buffer::Buffer,
    view: &mut crate::editor::BufferView,
    status_msg: &mut Option<String>,
    clipboard_out: &mut Option<String>,
    clipboard_in: &mut Option<String>,
    line_height: f32,
) {
    ctx.input(|input| {
        let cmd = input.modifiers.command;
        let shift = input.modifiers.shift;
        let alt = input.modifiers.alt;

        // ── Cmd shortcuts ──

        // Cmd+S: save
        if cmd && !shift && input.key_pressed(egui::Key::S) {
            match buf.save() {
                Ok(()) => *status_msg = Some("Saved".to_string()),
                Err(e) => *status_msg = Some(format!("Save failed: {e}")),
            }
            return;
        }

        // Cmd+Z: undo
        if cmd && !shift && input.key_pressed(egui::Key::Z) {
            if let Some(pos) = buf.undo() {
                view.cursor.clear_selection();
                view.cursor.pos = pos;
                view.cursor.desired_col = None;
            }
            return;
        }

        // Cmd+Shift+Z: redo
        if cmd && shift && input.key_pressed(egui::Key::Z) {
            if let Some(pos) = buf.redo() {
                view.cursor.clear_selection();
                view.cursor.pos = pos;
                view.cursor.desired_col = None;
            }
            return;
        }

        // Cmd+A: select all
        if cmd && input.key_pressed(egui::Key::A) {
            view.cursor.select_all(buf);
            return;
        }

        // Cmd+C: copy
        if cmd && !shift && input.key_pressed(egui::Key::C) {
            let text = if let Some((start, end)) = view.cursor.selection() {
                buf.text_range(start, end)
            } else {
                buf.line_text_for_copy(view.cursor.pos.line)
            };
            *clipboard_out = Some(text);
            return;
        }

        // Cmd+X: cut
        if cmd && !shift && input.key_pressed(egui::Key::X) {
            *status_msg = None;
            if let Some((start, end)) = view.cursor.selection() {
                *clipboard_out = Some(buf.text_range(start, end));
                buf.delete(start, end);
                view.cursor.clear_selection();
                view.cursor.pos = start;
            } else {
                *clipboard_out = Some(buf.line_text_for_copy(view.cursor.pos.line));
                view.cursor.pos = buf.delete_line(view.cursor.pos.line);
                view.cursor.clear_selection();
            }
            view.cursor.desired_col = None;
            return;
        }

        // Cmd+V: paste
        if cmd && !shift && input.key_pressed(egui::Key::V) {
            if let Some(text) = clipboard_in.take() {
                *status_msg = None;
                if let Some((start, end)) = view.cursor.selection() {
                    buf.delete(start, end);
                    view.cursor.clear_selection();
                    view.cursor.pos = start;
                }
                let end_pos = buf.compute_end_pos_pub(view.cursor.pos, &text);
                buf.insert(view.cursor.pos, &text);
                view.cursor.pos = end_pos;
                view.cursor.desired_col = None;
            }
            return;
        }

        // Cmd+Shift+K: delete line
        if cmd && shift && input.key_pressed(egui::Key::K) {
            *status_msg = None;
            view.cursor.pos = buf.delete_line(view.cursor.pos.line);
            view.cursor.clear_selection();
            view.cursor.desired_col = None;
            return;
        }

        // Cmd+Shift+D: duplicate line
        if cmd && shift && input.key_pressed(egui::Key::D) {
            *status_msg = None;
            view.cursor.pos = buf.duplicate_line(view.cursor.pos.line);
            view.cursor.clear_selection();
            view.cursor.desired_col = None;
            return;
        }

        // Alt+Up: move line up
        if alt && !cmd && !shift && input.key_pressed(egui::Key::ArrowUp) {
            *status_msg = None;
            if let Some(pos) = buf.move_line_up(view.cursor.pos.line) {
                view.cursor.pos = pos;
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
            }
            return;
        }

        // Alt+Down: move line down
        if alt && !cmd && !shift && input.key_pressed(egui::Key::ArrowDown) {
            *status_msg = None;
            if let Some(pos) = buf.move_line_down(view.cursor.pos.line) {
                view.cursor.pos = pos;
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
            }
            return;
        }

        // ── Arrow keys ──

        if input.key_pressed(egui::Key::ArrowRight) {
            if cmd {
                view.cursor.move_end(buf, shift);
            } else if alt {
                view.cursor.move_word_right(buf, shift);
            } else {
                view.cursor.move_right(buf, shift);
            }
            *status_msg = None;
            return;
        }
        if input.key_pressed(egui::Key::ArrowLeft) {
            if cmd {
                view.cursor.move_home(buf, shift);
            } else if alt {
                view.cursor.move_word_left(buf, shift);
            } else {
                view.cursor.move_left(buf, shift);
            }
            *status_msg = None;
            return;
        }
        if input.key_pressed(egui::Key::ArrowUp) {
            if cmd {
                view.cursor.move_to_start(shift);
            } else {
                view.cursor.move_up(buf, shift);
            }
            *status_msg = None;
            return;
        }
        if input.key_pressed(egui::Key::ArrowDown) {
            if cmd {
                view.cursor.move_to_end(buf, shift);
            } else {
                view.cursor.move_down(buf, shift);
            }
            *status_msg = None;
            return;
        }

        // Home/End
        if input.key_pressed(egui::Key::Home) {
            view.cursor.move_home(buf, shift);
            return;
        }
        if input.key_pressed(egui::Key::End) {
            view.cursor.move_end(buf, shift);
            return;
        }

        // Page Up/Down
        let page_lines = (300.0 / line_height) as usize;
        if input.key_pressed(egui::Key::PageUp) {
            view.cursor.move_page_up(buf, page_lines, shift);
            return;
        }
        if input.key_pressed(egui::Key::PageDown) {
            view.cursor.move_page_down(buf, page_lines, shift);
            return;
        }

        // ── Editing keys ──

        // Backspace
        if input.key_pressed(egui::Key::Backspace) {
            *status_msg = None;
            if let Some((start, end)) = view.cursor.selection() {
                buf.delete(start, end);
                view.cursor.clear_selection();
                view.cursor.pos = start;
            } else if view.cursor.pos.col > 0 {
                // Check for auto-pair deletion
                let before = buf.char_at(Position::new(view.cursor.pos.line, view.cursor.pos.col - 1));
                let after = buf.char_at(view.cursor.pos);
                let is_pair = before.is_some() && after.is_some() && PAIRS.iter().any(|&(o, c)| {
                    Some(o) == before && Some(c) == after
                });
                if is_pair {
                    // Delete both characters
                    let del_start = Position::new(view.cursor.pos.line, view.cursor.pos.col - 1);
                    let del_end = Position::new(view.cursor.pos.line, view.cursor.pos.col + 1);
                    buf.delete(del_start, del_end);
                    view.cursor.pos = del_start;
                } else {
                    let del_start = Position::new(view.cursor.pos.line, view.cursor.pos.col - 1);
                    buf.delete(del_start, view.cursor.pos);
                    view.cursor.pos = del_start;
                }
            } else if view.cursor.pos.line > 0 {
                let prev_len = buf.line_len(view.cursor.pos.line - 1);
                let join_pos = Position::new(view.cursor.pos.line - 1, prev_len);
                buf.delete(join_pos, view.cursor.pos);
                view.cursor.pos = join_pos;
            }
            view.cursor.desired_col = None;
            return;
        }

        // Delete
        if input.key_pressed(egui::Key::Delete) {
            *status_msg = None;
            if let Some((start, end)) = view.cursor.selection() {
                buf.delete(start, end);
                view.cursor.clear_selection();
                view.cursor.pos = start;
            } else {
                let line_len = buf.line_len(view.cursor.pos.line);
                if view.cursor.pos.col < line_len {
                    let del_end = Position::new(view.cursor.pos.line, view.cursor.pos.col + 1);
                    buf.delete(view.cursor.pos, del_end);
                } else if view.cursor.pos.line + 1 < buf.line_count() {
                    let next_start = Position::new(view.cursor.pos.line + 1, 0);
                    buf.delete(view.cursor.pos, next_start);
                }
            }
            view.cursor.desired_col = None;
            return;
        }

        // Enter
        if input.key_pressed(egui::Key::Enter) {
            *status_msg = None;
            if let Some((start, end)) = view.cursor.selection() {
                buf.delete(start, end);
                view.cursor.clear_selection();
                view.cursor.pos = start;
            }
            let indent = buf.line_indent(view.cursor.pos.line).to_string();
            // Smart indent: add extra level after { or (
            let line_before = buf.line(view.cursor.pos.line);
            let before_cursor = &line_before[..line_before.len().min(view.cursor.pos.col)];
            let extra = if before_cursor.trim_end().ends_with('{')
                || before_cursor.trim_end().ends_with('(')
                || before_cursor.trim_end().ends_with('[')
            {
                buf.indent_style.as_str()
            } else {
                ""
            };
            let insert_text = format!("\n{indent}{extra}");
            let new_col = indent.chars().count() + extra.chars().count();
            buf.insert(view.cursor.pos, &insert_text);
            view.cursor.pos = Position::new(view.cursor.pos.line + 1, new_col);
            view.cursor.desired_col = None;
            return;
        }

        // Tab / Shift+Tab
        if input.key_pressed(egui::Key::Tab) {
            *status_msg = None;
            if let Some((start, end)) = view.cursor.selection() {
                if shift {
                    buf.dedent_lines(start.line, end.line);
                } else {
                    buf.indent_lines(start.line, end.line);
                }
                // Keep selection spanning the same lines
                view.cursor.anchor = Some(Position::new(start.line, 0));
                let end_line_len = buf.line_len(end.line);
                view.cursor.pos = Position::new(end.line, end_line_len);
            } else if shift {
                buf.dedent_lines(view.cursor.pos.line, view.cursor.pos.line);
                view.cursor.pos.col = view.cursor.pos.col.min(buf.line_len(view.cursor.pos.line));
            } else {
                let indent = buf.indent_style.as_str();
                buf.insert(view.cursor.pos, indent);
                view.cursor.pos.col += buf.indent_style.width();
            }
            view.cursor.desired_col = None;
            return;
        }

        // ── Text input (regular characters) ──
        for event in &input.events {
            if let egui::Event::Text(text) = event {
                if !cmd {
                    *status_msg = None;
                    let text = text.clone();
                    // Delete selection first
                    if let Some((start, end)) = view.cursor.selection() {
                        buf.delete(start, end);
                        view.cursor.clear_selection();
                        view.cursor.pos = start;
                    }
                    for ch in text.chars() {
                        // Auto-closing: if typing a closing bracket, skip over it
                        if PAIRS.iter().any(|&(_, c)| c == ch) {
                            let next = buf.char_at(view.cursor.pos);
                            if next == Some(ch) {
                                view.cursor.pos.col += 1;
                                continue;
                            }
                        }

                        buf.insert_char(view.cursor.pos, ch);
                        view.cursor.pos.col += 1;

                        // Auto-pair: insert closing bracket
                        if let Some(&(_, close)) = PAIRS.iter().find(|&&(o, _)| o == ch) {
                            // Only auto-pair if next char is whitespace, closing bracket, or end of line
                            let next = buf.char_at(view.cursor.pos);
                            let should_pair = next.is_none()
                                || next.is_some_and(|c| c.is_whitespace() || ")]}\"'`".contains(c));
                            if should_pair {
                                buf.insert_char(view.cursor.pos, close);
                                // Don't advance cursor — stay between the pair
                            }
                        }
                    }
                    view.cursor.desired_col = None;
                }
            }
        }
    });
}
