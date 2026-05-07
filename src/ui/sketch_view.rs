use crate::sketch::{
    default_export_file_name, export_svg_to_path, list_saved_sketches, SketchElement, SketchPoint,
    SketchState, SketchSymbolKind, SketchTool, MAX_SKETCH_ZOOM, MIN_SKETCH_ZOOM,
};
use std::path::Path;

use super::sketch_paint::{
    paint_inline_text_cursor, paint_sketch_document, paint_symbol_shape, screen_to_canvas,
};

#[derive(Clone, Copy)]
pub(crate) struct SketchAppearance {
    pub canvas_bg: egui::Color32,
    pub text_color: egui::Color32,
    pub active_btn: egui::Color32,
}

pub(crate) fn render_sketch_view(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    sketch: &mut SketchState,
    appearance: &SketchAppearance,
    project_root: &Path,
) -> egui::Rect {
    // Handle inline text input before general shortcuts so typed characters
    // are consumed by the text draft and not interpreted as shortcut keys.
    let text_input_active = sketch.text_draft.is_some();
    if text_input_active {
        handle_inline_text_input(ui, sketch);
    } else {
        sketch_shortcuts(ui, sketch);
    }

    // ── Toolbar row ──
    ui.horizontal(|ui| {
        // Title and optional active sketch name
        let title = if let Some(name) = &sketch.active_sketch_name {
            format!("Sketch - {name}")
        } else {
            "Sketch".to_string()
        };
        ui.label(
            egui::RichText::new(title)
                .size(22.0)
                .color(egui::Color32::WHITE),
        );
        ui.add_space(16.0);
        tool_button(
            ui,
            sketch,
            SketchTool::Select,
            "Select",
            appearance.active_btn,
            appearance.text_color,
        )
        .on_hover_text("Select, move, and delete elements");
        tool_button(
            ui,
            sketch,
            SketchTool::Marker,
            "Marker",
            appearance.active_btn,
            appearance.text_color,
        )
        .on_hover_text("Draw freehand strokes");
        tool_button(
            ui,
            sketch,
            SketchTool::Rectangle,
            "Rect",
            appearance.active_btn,
            appearance.text_color,
        )
        .on_hover_text("Drag to create a rectangle. Shift makes a square, Alt draws from center");
        tool_button(
            ui,
            sketch,
            SketchTool::Text,
            "Text",
            appearance.active_btn,
            appearance.text_color,
        )
        .on_hover_text("Click to place text directly on the canvas. Enter commits, Escape cancels");

        ui.separator();

        if ui.button("Image").on_hover_text("Import image").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(
                    "Images",
                    &["png", "jpg", "jpeg", "bmp", "webp", "gif", "tiff"],
                )
                .pick_file()
            {
                match sketch.add_image_from_path(&path, SketchPoint::new(72.0, 72.0)) {
                    Ok(_) => sketch.status_message = Some("Image added to sketch.".to_string()),
                    Err(err) => sketch.status_message = Some(err),
                }
            }
        }
        render_symbol_menu(ui, sketch, appearance);

        ui.separator();

        render_zoom_controls(ui, sketch, appearance);

        ui.separator();

        let mut c = sketch.style.stroke_color;
        if ui.color_edit_button_srgba_unmultiplied(&mut c).changed() {
            sketch.style.stroke_color = c;
        }

        ui.add_space(8.0);
        ui.add(egui::Slider::new(&mut sketch.style.stroke_width, 1.0..=14.0).text("Width"));

        ui.separator();

        if ui
            .add_enabled(sketch.can_undo(), egui::Button::new("Undo"))
            .clicked()
        {
            sketch.undo();
        }
        if ui
            .add_enabled(sketch.can_redo(), egui::Button::new("Redo"))
            .clicked()
        {
            sketch.redo();
        }
        if ui
            .add_enabled(
                !sketch.document.elements.is_empty(),
                egui::Button::new("Clear"),
            )
            .clicked()
        {
            sketch.clear();
        }

        ui.separator();

        // ── Sketch save/recall controls ──
        if ui.button("New").on_hover_text("New blank sketch").clicked() {
            sketch.new_sketch();
        }
        if ui
            .button("Save As")
            .on_hover_text("Save sketch with a name")
            .clicked()
        {
            sketch.save_as_open = !sketch.save_as_open;
            if sketch.save_as_open {
                // Pre-fill with current name if any
                sketch.save_as_input = sketch.active_sketch_name.clone().unwrap_or_default();
            }
        }
        if ui
            .button(if sketch.browser_open {
                "Close Browser"
            } else {
                "Browse"
            })
            .on_hover_text("Browse saved sketches")
            .clicked()
        {
            sketch.browser_open = !sketch.browser_open;
            if sketch.browser_open {
                sketch.saved_sketch_names = list_saved_sketches();
            }
        }

        ui.separator();
        render_export_menu(ui, sketch, project_root);
    });

    render_selected_image_controls(ui, sketch);
    if let Some(message) = &sketch.status_message {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(message)
                .size(12.0)
                .color(egui::Color32::from_rgb(170, 205, 180)),
        );
    }

    // ── Save-As inline prompt (below toolbar, above canvas) ──
    if sketch.save_as_open {
        render_save_as_prompt(ui, sketch);
    }

    ui.add_space(4.0);

    // ── Main area: optional browser panel + canvas ──
    let available = ui.available_size();

    if sketch.browser_open {
        // Side-by-side: browser panel on the left, canvas on the right
        let browser_width = 180.0_f32.min(available.x * 0.25);

        let resp = ui.horizontal(|ui| {
            // Browser panel
            ui.vertical(|ui| {
                ui.set_width(browser_width);
                ui.set_height(available.y);
                render_sketch_browser(ui, sketch);
            });
            ui.add_space(4.0);
            // Canvas takes remaining space
            let canvas_width = (available.x - browser_width - 12.0).max(320.0);
            let canvas_size = egui::Vec2::new(canvas_width, available.y.max(240.0));
            render_canvas(ctx, ui, sketch, appearance, canvas_size)
        });
        resp.inner
    } else {
        let canvas_size = egui::Vec2::new(available.x.max(320.0), available.y.max(240.0));
        render_canvas(ctx, ui, sketch, appearance, canvas_size)
    }
}

fn render_canvas(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    sketch: &mut SketchState,
    appearance: &SketchAppearance,
    canvas_size: egui::Vec2,
) -> egui::Rect {
    let (canvas_rect, response) =
        ui.allocate_exact_size(canvas_size, egui::Sense::click_and_drag());
    let painter = ui.painter_at(canvas_rect);

    painter.rect_filled(canvas_rect, egui::Rounding::same(4.0), appearance.canvas_bg);
    painter.rect_stroke(
        canvas_rect,
        egui::Rounding::same(4.0),
        egui::Stroke::new(1.0, appearance.active_btn),
    );

    sketch.last_canvas_size = [
        canvas_rect.width() / sketch.zoom.max(0.01),
        canvas_rect.height() / sketch.zoom.max(0.01),
    ];
    handle_canvas_paste(ctx, sketch, canvas_rect);
    handle_sketch_pointer(sketch, &response, canvas_rect);
    paint_sketch_document(&painter, canvas_rect, sketch);
    paint_inline_text_cursor(ctx, &painter, canvas_rect, sketch);

    canvas_rect
}

fn render_save_as_prompt(ui: &mut egui::Ui, sketch: &mut SketchState) {
    ui.add_space(4.0);
    let mut commit = false;
    let mut cancel = false;

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Name:")
                .size(14.0)
                .color(egui::Color32::WHITE),
        );
        let response = ui.add(
            egui::TextEdit::singleline(&mut sketch.save_as_input)
                .desired_width(200.0)
                .hint_text("my-sketch"),
        );
        response.request_focus();
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            commit = true;
        }
        if ui.button("Save").clicked() {
            commit = true;
        }
        if ui.button("Cancel").clicked() {
            cancel = true;
        }
    });

    if commit {
        let name = sketch.save_as_input.clone();
        if !name.trim().is_empty() {
            let _ = sketch.save_sketch_as(&name);
        }
        sketch.save_as_open = false;
        sketch.save_as_input.clear();
    } else if cancel {
        sketch.save_as_open = false;
        sketch.save_as_input.clear();
    }
}

fn render_sketch_browser(ui: &mut egui::Ui, sketch: &mut SketchState) {
    ui.label(
        egui::RichText::new("Saved Sketches")
            .size(14.0)
            .strong()
            .color(egui::Color32::WHITE),
    );
    ui.add_space(4.0);

    if sketch.saved_sketch_names.is_empty() {
        ui.label(
            egui::RichText::new("No saved sketches yet.")
                .size(12.0)
                .color(egui::Color32::GRAY),
        );
        if ui
            .small_button("Refresh")
            .on_hover_text("Reload list")
            .clicked()
        {
            sketch.saved_sketch_names = list_saved_sketches();
        }
        return;
    }

    let mut load_name: Option<String> = None;
    let mut delete_name: Option<String> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for name in &sketch.saved_sketch_names {
                let is_active = sketch.active_sketch_name.as_deref() == Some(name.as_str());
                ui.horizontal(|ui| {
                    let label = if is_active {
                        egui::RichText::new(name)
                            .size(13.0)
                            .color(egui::Color32::from_rgb(60, 180, 255))
                            .strong()
                    } else {
                        egui::RichText::new(name)
                            .size(13.0)
                            .color(egui::Color32::from_rgb(200, 200, 210))
                    };
                    if ui
                        .add(egui::Label::new(label).sense(egui::Sense::click()))
                        .clicked()
                    {
                        load_name = Some(name.clone());
                    }
                    if ui
                        .small_button("x")
                        .on_hover_text("Delete this sketch")
                        .clicked()
                    {
                        delete_name = Some(name.clone());
                    }
                });
            }
        });

    if let Some(name) = load_name {
        let _ = sketch.load_sketch(&name);
    }
    if let Some(name) = delete_name {
        let _ = crate::sketch::delete_named_sketch(&name);
        sketch.saved_sketch_names = list_saved_sketches();
        // If we deleted the active sketch, clear the name
        if sketch.active_sketch_name.as_deref() == Some(name.as_str()) {
            sketch.active_sketch_name = None;
        }
    }
}

fn tool_button(
    ui: &mut egui::Ui,
    sketch: &mut SketchState,
    tool: SketchTool,
    text: &str,
    active_btn: egui::Color32,
    text_color: egui::Color32,
) -> egui::Response {
    let selected = sketch.tool == tool;
    let response = ui.add(
        egui::Button::new(egui::RichText::new(text).size(14.0).color(if selected {
            egui::Color32::WHITE
        } else {
            text_color
        }))
        .fill(if selected {
            active_btn
        } else {
            egui::Color32::from_rgb(30, 32, 40)
        }),
    );
    if response.clicked() {
        sketch.set_tool(tool);
    }
    response
}

fn render_zoom_controls(
    ui: &mut egui::Ui,
    sketch: &mut SketchState,
    appearance: &SketchAppearance,
) {
    if ui.small_button("-").on_hover_text("Zoom out").clicked() {
        sketch.zoom_out();
    }
    let mut zoom = sketch.zoom;
    let response = ui.add(
        egui::Slider::new(&mut zoom, MIN_SKETCH_ZOOM..=MAX_SKETCH_ZOOM)
            .show_value(false)
            .fixed_decimals(0),
    );
    if response.changed() {
        sketch.set_zoom(zoom);
    }
    ui.label(
        egui::RichText::new(format!("{:.0}%", sketch.zoom * 100.0))
            .size(12.0)
            .color(appearance.text_color),
    );
    if ui.small_button("+").on_hover_text("Zoom in").clicked() {
        sketch.zoom_in();
    }
    if ui.small_button("100").on_hover_text("Reset zoom").clicked() {
        sketch.reset_zoom();
    }
}

fn render_selected_image_controls(ui: &mut egui::Ui, sketch: &mut SketchState) {
    let Some(mut scale) = sketch.selected_image_scale() else {
        return;
    };
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Image size")
                .size(12.0)
                .color(egui::Color32::from_rgb(190, 195, 205)),
        );
        if ui
            .add(egui::Slider::new(&mut scale, 0.05..=2.0).text(""))
            .changed()
        {
            sketch.resize_selected_image_to_scale(scale);
        }
        ui.label(
            egui::RichText::new(format!("{:.0}%", scale * 100.0))
                .size(12.0)
                .color(egui::Color32::from_rgb(160, 170, 185)),
        );
    });
}

fn render_symbol_menu(ui: &mut egui::Ui, sketch: &mut SketchState, appearance: &SketchAppearance) {
    ui.menu_button("Symbols", |ui| {
        ui.set_min_width(340.0);
        egui::Grid::new("sketch_symbol_grid")
            .num_columns(4)
            .spacing(egui::vec2(8.0, 8.0))
            .show(ui, |ui| {
                for (index, kind) in SKETCH_SYMBOLS.iter().copied().enumerate() {
                    if symbol_button(ui, kind, appearance).clicked() {
                        sketch.add_symbol(kind, SketchPoint::new(96.0, 96.0));
                        ui.close_menu();
                    }
                    if index % 4 == 3 {
                        ui.end_row();
                    }
                }
            });
    });
}

fn symbol_button(
    ui: &mut egui::Ui,
    kind: SketchSymbolKind,
    appearance: &SketchAppearance,
) -> egui::Response {
    let desired = egui::vec2(76.0, 64.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    let painter = ui.painter_at(rect);
    painter.rect_filled(
        rect,
        egui::Rounding::same(5.0),
        egui::Color32::from_rgb(28, 31, 38),
    );
    painter.rect_stroke(
        rect,
        egui::Rounding::same(5.0),
        egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 64, 78)),
    );
    let symbol_rect =
        egui::Rect::from_min_size(rect.min + egui::vec2(18.0, 8.0), egui::vec2(40.0, 28.0));
    paint_symbol_shape(&painter, symbol_rect, kind, appearance.active_btn, 1.6);
    painter.text(
        egui::pos2(rect.center().x, rect.bottom() - 8.0),
        egui::Align2::CENTER_BOTTOM,
        kind.label(),
        egui::FontId::proportional(10.5),
        appearance.text_color,
    );
    response
}

fn render_export_menu(ui: &mut egui::Ui, sketch: &mut SketchState, project_root: &Path) {
    ui.menu_button("Export", |ui| {
        if ui.button("Repo root SVG").clicked() {
            let path = project_root.join(default_export_file_name(
                sketch.active_sketch_name.as_deref(),
            ));
            export_sketch_to_path(sketch, &path);
            ui.close_menu();
        }
        if ui.button("Choose folder SVG").clicked() {
            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                let path = folder.join(default_export_file_name(
                    sketch.active_sketch_name.as_deref(),
                ));
                export_sketch_to_path(sketch, &path);
            }
            ui.close_menu();
        }
    });
}

fn export_sketch_to_path(sketch: &mut SketchState, path: &Path) {
    match export_svg_to_path(&sketch.document, path, sketch.last_canvas_size) {
        Ok(()) => {
            sketch.status_message = Some(format!("Exported {}", path.display()));
        }
        Err(err) => sketch.status_message = Some(err),
    }
}

const SKETCH_SYMBOLS: &[SketchSymbolKind] = &[
    SketchSymbolKind::Database,
    SketchSymbolKind::Table,
    SketchSymbolKind::Api,
    SketchSymbolKind::Server,
    SketchSymbolKind::Queue,
    SketchSymbolKind::Cache,
    SketchSymbolKind::Cloud,
    SketchSymbolKind::Lock,
    SketchSymbolKind::User,
    SketchSymbolKind::Component,
    SketchSymbolKind::Decision,
    SketchSymbolKind::Flow,
];

fn sketch_shortcuts(ui: &egui::Ui, sketch: &mut SketchState) {
    ui.input(|input| {
        if input.modifiers.command && input.key_pressed(egui::Key::Z) {
            if input.modifiers.shift {
                sketch.redo();
            } else {
                sketch.undo();
            }
        }
        if input.modifiers.command && input.key_pressed(egui::Key::Y) {
            sketch.redo();
        }
        if (input.key_pressed(egui::Key::Delete) || input.key_pressed(egui::Key::Backspace))
            && sketch.text_draft.is_none()
        {
            sketch.delete_selected();
        }
        if input.key_pressed(egui::Key::Escape) {
            if sketch.text_draft.is_some() {
                sketch.cancel_text_draft();
            } else {
                sketch.selected = None;
            }
        }
    });
}

/// Handle keyboard input when a text draft is active (inline text tool).
fn handle_inline_text_input(ui: &egui::Ui, sketch: &mut SketchState) {
    let Some(draft) = &sketch.text_draft else {
        return;
    };
    let mut text = draft.text.clone();
    let mut commit = false;
    let mut cancel = false;

    ui.input(|input| {
        // Escape cancels
        if input.key_pressed(egui::Key::Escape) {
            cancel = true;
            return;
        }
        // Enter commits (without modifiers)
        if input.key_pressed(egui::Key::Enter) && !input.modifiers.shift {
            commit = true;
            return;
        }
        // Backspace removes last char
        if input.key_pressed(egui::Key::Backspace) {
            text.pop();
            return;
        }
        if input.modifiers.command && input.key_pressed(egui::Key::Z) {
            if input.modifiers.shift {
                sketch.redo();
            } else {
                sketch.cancel_text_draft();
                sketch.undo();
            }
            return;
        }
        if input.modifiers.command && input.key_pressed(egui::Key::Y) {
            sketch.redo();
            return;
        }
        // Collect typed text from events
        let mut pasted_from_event = false;
        for event in &input.events {
            match event {
                egui::Event::Text(s) => text.push_str(s),
                egui::Event::Paste(s) => {
                    text.push_str(s);
                    pasted_from_event = true;
                }
                _ => {}
            }
        }
        if !pasted_from_event && input.modifiers.command && input.key_pressed(egui::Key::V) {
            if let Some(paste) = sketch.clipboard_in.as_deref() {
                text.push_str(paste);
            }
        }
    });

    if cancel {
        sketch.cancel_text_draft();
    } else if commit {
        sketch.update_text_draft(text);
        sketch.commit_text_draft();
    } else {
        sketch.update_text_draft(text);
    }
}

fn handle_canvas_paste(ctx: &egui::Context, sketch: &mut SketchState, canvas_rect: egui::Rect) {
    if sketch.text_draft.is_some() || ctx.wants_keyboard_input() {
        return;
    }
    let paste = ctx.input(|input| {
        let pasted_event = input.events.iter().find_map(|event| {
            if let egui::Event::Paste(text) = event {
                Some(text.clone())
            } else {
                None
            }
        });
        if pasted_event.is_some() {
            pasted_event
        } else if input.modifiers.command && input.key_pressed(egui::Key::V) {
            sketch.clipboard_in.clone()
        } else {
            None
        }
    });
    let Some(text) = paste.filter(|text| !text.trim().is_empty()) else {
        return;
    };
    let point = ctx.input(|input| input.pointer.hover_pos()).map_or_else(
        || SketchPoint::new(canvas_rect.width() * 0.5 / sketch.zoom, 72.0),
        |pos| {
            if canvas_rect.contains(pos) {
                screen_to_canvas(pos, canvas_rect, sketch.zoom)
            } else {
                SketchPoint::new(canvas_rect.width() * 0.5 / sketch.zoom, 72.0)
            }
        },
    );
    sketch.paste_text_box(&text, point);
}

fn handle_sketch_pointer(
    sketch: &mut SketchState,
    response: &egui::Response,
    canvas_rect: egui::Rect,
) {
    let Some(pointer_pos) = response.interact_pointer_pos() else {
        return;
    };
    let point = screen_to_canvas(pointer_pos, canvas_rect, sketch.zoom);
    let modifiers = response.ctx.input(|input| input.modifiers);

    match sketch.tool {
        SketchTool::Marker => {
            if response.drag_started() {
                sketch.begin_stroke(point);
            } else if response.dragged() {
                sketch.append_stroke_point(point);
            }
            if response.drag_stopped() {
                sketch.finish_stroke();
            }
        }
        SketchTool::Rectangle => {
            if response.drag_started() {
                sketch.begin_rectangle(point);
                sketch.update_rectangle_with_modifiers(point, modifiers.shift, modifiers.alt);
            } else if response.dragged() {
                sketch.update_rectangle_with_modifiers(point, modifiers.shift, modifiers.alt);
            }
            if response.drag_stopped() {
                sketch.update_rectangle_with_modifiers(point, modifiers.shift, modifiers.alt);
                sketch.finish_rectangle();
            }
        }
        SketchTool::Text => {
            if response.clicked() {
                // If there's already a text draft active, commit it first
                if sketch.text_draft.is_some() {
                    let draft_text = sketch.text_draft.as_ref().map(|d| d.text.clone());
                    if let Some(text) = draft_text {
                        sketch.update_text_draft(text);
                    }
                    sketch.commit_text_draft();
                }
                sketch.add_text_box(point);
            }
        }
        SketchTool::Select => {
            if response.double_clicked() {
                if let Some(index) = sketch.hit_test(point) {
                    if matches!(
                        sketch.document.elements.get(index),
                        Some(SketchElement::Text(_))
                    ) {
                        sketch.edit_text_box(index);
                        return;
                    }
                }
            }
            if response.drag_started() {
                sketch.select_at(point);
                sketch.begin_move_selected(point);
            } else if response.dragged() {
                sketch.update_move_selected(point);
            }
            if response.drag_stopped() {
                sketch.finish_move_selected();
            }
            if response.clicked() {
                sketch.select_at(point);
            }
        }
    }
}
