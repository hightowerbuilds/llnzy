use crate::sketch::{
    default_export_file_name, delete_named_sketch, export_svg_to_path, list_saved_sketches,
    SketchCanvasBackgroundMode, SketchElement, SketchGridMode, SketchPoint, SketchState,
    SketchSymbolKind, SketchTool, SketchToolbarPosition,
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
    interactive: bool,
) -> egui::Rect {
    // Handle inline text input before general shortcuts so typed characters
    // are consumed by the text draft and not interpreted as shortcut keys.
    if interactive {
        let text_input_active = sketch.text_draft.is_some();
        if text_input_active {
            handle_inline_text_input(ui, sketch);
        } else {
            sketch_shortcuts(ui, sketch);
        }
    }

    match sketch.appearance.toolbar_position {
        SketchToolbarPosition::Top => {
            render_sketch_toolbar(ui, sketch, appearance, project_root, false);
            render_sketch_body(ctx, ui, sketch, appearance, interactive)
        }
        SketchToolbarPosition::Left | SketchToolbarPosition::Right => {
            render_sketch_side_layout(ctx, ui, sketch, appearance, project_root, interactive)
        }
    }
}

fn render_sketch_side_layout(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    sketch: &mut SketchState,
    appearance: &SketchAppearance,
    project_root: &Path,
    interactive: bool,
) -> egui::Rect {
    let available = ui.available_size();
    let toolbar_w = (available.x * 0.22).clamp(132.0, 178.0);
    let gap = 8.0;
    let content_size = egui::vec2(
        (available.x - toolbar_w - gap).max(320.0),
        available.y.max(1.0),
    );

    let response = ui.horizontal(|ui| {
        if sketch.appearance.toolbar_position == SketchToolbarPosition::Left {
            render_sidebar_toolbar(ui, sketch, appearance, project_root, toolbar_w, available.y);
            ui.add_space(gap);
            render_sized_sketch_body(ctx, ui, sketch, appearance, interactive, content_size)
        } else {
            let canvas_rect =
                render_sized_sketch_body(ctx, ui, sketch, appearance, interactive, content_size);
            ui.add_space(gap);
            render_sidebar_toolbar(ui, sketch, appearance, project_root, toolbar_w, available.y);
            canvas_rect
        }
    });
    response.inner
}

fn render_sized_sketch_body(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    sketch: &mut SketchState,
    appearance: &SketchAppearance,
    interactive: bool,
    size: egui::Vec2,
) -> egui::Rect {
    ui.allocate_ui_with_layout(size, egui::Layout::top_down(egui::Align::Min), |ui| {
        ui.set_width(size.x);
        ui.set_height(size.y);
        render_sketch_body(ctx, ui, sketch, appearance, interactive)
    })
    .inner
}

fn render_sidebar_toolbar(
    ui: &mut egui::Ui,
    sketch: &mut SketchState,
    appearance: &SketchAppearance,
    project_root: &Path,
    width: f32,
    height: f32,
) {
    ui.vertical(|ui| {
        ui.set_width(width);
        ui.set_height(height.max(1.0));
        egui::ScrollArea::vertical()
            .id_salt("sketch_sidebar_toolbar_scroll")
            .auto_shrink([false, false])
            .max_width(width)
            .max_height(height.max(1.0))
            .show(ui, |ui| {
                ui.set_width((width - 8.0).max(96.0));
                render_sketch_toolbar(ui, sketch, appearance, project_root, true);
            });
    });
}

fn render_sketch_toolbar(
    ui: &mut egui::Ui,
    sketch: &mut SketchState,
    appearance: &SketchAppearance,
    project_root: &Path,
    vertical: bool,
) {
    if vertical {
        ui.vertical(|ui| {
            render_sketch_toolbar_contents(ui, sketch, appearance, project_root, true)
        });
    } else {
        ui.horizontal(|ui| {
            render_sketch_toolbar_contents(ui, sketch, appearance, project_root, false);
        });
    }
}

fn render_sketch_toolbar_contents(
    ui: &mut egui::Ui,
    sketch: &mut SketchState,
    appearance: &SketchAppearance,
    project_root: &Path,
    vertical: bool,
) {
    let title = if let Some(name) = &sketch.active_sketch_name {
        format!("Sketch - {name}")
    } else {
        "Sketch".to_string()
    };
    ui.label(
        egui::RichText::new(title)
            .size(if vertical { 18.0 } else { 22.0 })
            .color(egui::Color32::WHITE),
    );
    add_toolbar_gap(ui, vertical);
    render_tool_buttons(ui, sketch, appearance, vertical);
    add_toolbar_separator(ui, vertical);
    render_insert_controls(ui, sketch, appearance);
    add_toolbar_separator(ui, vertical);
    render_style_controls(ui, sketch, vertical);
    add_toolbar_separator(ui, vertical);
    render_history_controls(ui, sketch, vertical);
    add_toolbar_separator(ui, vertical);
    render_document_controls(ui, sketch, vertical);
    add_toolbar_separator(ui, vertical);
    render_export_menu(ui, sketch, project_root);
}

fn add_toolbar_gap(ui: &mut egui::Ui, vertical: bool) {
    if vertical {
        ui.add_space(8.0);
    } else {
        ui.add_space(16.0);
    }
}

fn add_toolbar_separator(ui: &mut egui::Ui, vertical: bool) {
    if vertical {
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);
    } else {
        ui.separator();
    }
}

fn render_tool_buttons(
    ui: &mut egui::Ui,
    sketch: &mut SketchState,
    appearance: &SketchAppearance,
    vertical: bool,
) {
    let add_buttons = |ui: &mut egui::Ui, sketch: &mut SketchState| {
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
    };
    if vertical {
        ui.vertical(|ui| add_buttons(ui, sketch));
    } else {
        add_buttons(ui, sketch);
    }
}

fn render_insert_controls(
    ui: &mut egui::Ui,
    sketch: &mut SketchState,
    appearance: &SketchAppearance,
) {
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
}

fn render_style_controls(ui: &mut egui::Ui, sketch: &mut SketchState, vertical: bool) {
    let mut c = sketch.style.stroke_color;
    if ui.color_edit_button_srgba_unmultiplied(&mut c).changed() {
        sketch.style.stroke_color = c;
    }

    ui.add_space(if vertical { 4.0 } else { 8.0 });
    let slider_width = if vertical { 118.0 } else { 160.0 };
    ui.add_sized(
        egui::vec2(slider_width, 18.0),
        egui::Slider::new(&mut sketch.style.stroke_width, 1.0..=14.0).text("Width"),
    );
}

fn render_history_controls(ui: &mut egui::Ui, sketch: &mut SketchState, vertical: bool) {
    let add_buttons = |ui: &mut egui::Ui, sketch: &mut SketchState| {
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
    };
    if vertical {
        ui.vertical(|ui| add_buttons(ui, sketch));
    } else {
        add_buttons(ui, sketch);
    }
}

fn render_document_controls(ui: &mut egui::Ui, sketch: &mut SketchState, vertical: bool) {
    let add_buttons = |ui: &mut egui::Ui, sketch: &mut SketchState| {
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
    };
    if vertical {
        ui.vertical(|ui| add_buttons(ui, sketch));
    } else {
        add_buttons(ui, sketch);
    }
}

fn render_sketch_body(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    sketch: &mut SketchState,
    appearance: &SketchAppearance,
    interactive: bool,
) -> egui::Rect {
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
    if interactive && sketch.save_as_open {
        render_save_as_prompt(ui, sketch);
    }
    if interactive {
        render_delete_sketch_modal(ctx, sketch);
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
            render_canvas(ctx, ui, sketch, appearance, canvas_size, interactive)
        });
        resp.inner
    } else {
        let canvas_size = egui::Vec2::new(available.x.max(320.0), available.y.max(240.0));
        render_canvas(ctx, ui, sketch, appearance, canvas_size, interactive)
    }
}

fn render_canvas(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    sketch: &mut SketchState,
    appearance: &SketchAppearance,
    canvas_size: egui::Vec2,
    interactive: bool,
) -> egui::Rect {
    let (canvas_rect, response) =
        ui.allocate_exact_size(canvas_size, egui::Sense::click_and_drag());
    let painter = ui.painter_at(canvas_rect);

    if sketch.appearance.canvas_shadow_visible {
        painter.rect_filled(
            canvas_rect.translate(egui::vec2(5.0, 5.0)),
            egui::Rounding::same(5.0),
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 70),
        );
    }

    let canvas_bg = match sketch.appearance.canvas_background_mode {
        SketchCanvasBackgroundMode::Theme => appearance.canvas_bg,
        SketchCanvasBackgroundMode::Solid => rgba32(sketch.appearance.canvas_background_color),
    };
    painter.rect_filled(canvas_rect, egui::Rounding::same(4.0), canvas_bg);
    if sketch.appearance.canvas_border_visible {
        painter.rect_stroke(
            canvas_rect,
            egui::Rounding::same(4.0),
            egui::Stroke::new(1.0, appearance.active_btn),
        );
    }
    paint_canvas_grid(&painter, canvas_rect, sketch);

    sketch.last_canvas_size = [canvas_rect.width(), canvas_rect.height()];
    if interactive {
        handle_canvas_paste(ctx, sketch, canvas_rect);
        handle_sketch_pointer(sketch, &response, canvas_rect);
    }
    paint_sketch_document(&painter, canvas_rect, sketch);
    if interactive {
        paint_inline_text_cursor(ctx, &painter, canvas_rect, sketch);
    }

    canvas_rect
}

fn paint_canvas_grid(painter: &egui::Painter, canvas_rect: egui::Rect, sketch: &SketchState) {
    if !sketch.appearance.grid_visible() {
        return;
    }

    let spacing = sketch.appearance.effective_grid_spacing().max(4.0);
    let opacity = (sketch.appearance.effective_grid_opacity() * 255.0).clamp(0.0, 255.0) as u8;
    let color = egui::Color32::from_rgba_unmultiplied(180, 190, 210, opacity);
    let clipped = painter.with_clip_rect(canvas_rect);

    match sketch.appearance.grid_mode {
        SketchGridMode::Hidden => {}
        SketchGridMode::Lines => {
            let mut x = canvas_rect.left();
            while x <= canvas_rect.right() {
                clipped.line_segment(
                    [
                        egui::pos2(x, canvas_rect.top()),
                        egui::pos2(x, canvas_rect.bottom()),
                    ],
                    egui::Stroke::new(1.0, color),
                );
                x += spacing;
            }

            let mut y = canvas_rect.top();
            while y <= canvas_rect.bottom() {
                clipped.line_segment(
                    [
                        egui::pos2(canvas_rect.left(), y),
                        egui::pos2(canvas_rect.right(), y),
                    ],
                    egui::Stroke::new(1.0, color),
                );
                y += spacing;
            }
        }
        SketchGridMode::Dots => {
            let mut x = canvas_rect.left();
            while x <= canvas_rect.right() {
                let mut y = canvas_rect.top();
                while y <= canvas_rect.bottom() {
                    clipped.circle_filled(egui::pos2(x, y), 1.2, color);
                    y += spacing;
                }
                x += spacing;
            }
        }
    }
}

fn rgba32(color: [u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3])
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
        sketch.pending_delete_sketch_name = Some(name);
    }
}

fn render_delete_sketch_modal(ctx: &egui::Context, sketch: &mut SketchState) {
    let Some(name) = sketch.pending_delete_sketch_name.clone() else {
        return;
    };

    let mut confirm = false;
    let mut cancel = false;
    egui::Window::new("Delete saved sketch?")
        .id(egui::Id::new("sketch_delete_saved_modal"))
        .fixed_pos(egui::pos2(
            ctx.screen_rect().center().x - 180.0,
            ctx.screen_rect().center().y - 64.0,
        ))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_width(360.0);
            ui.label(
                egui::RichText::new(format!("Delete \"{name}\"? This cannot be undone."))
                    .size(13.0)
                    .color(egui::Color32::from_rgb(210, 215, 225)),
            );
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Delete sketch")
                                .size(12.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(180, 50, 50)),
                    )
                    .clicked()
                {
                    confirm = true;
                }
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Cancel")
                                .size(12.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(50, 52, 62)),
                    )
                    .clicked()
                {
                    cancel = true;
                }
            });
            if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
        });

    if confirm {
        match delete_named_sketch(&name) {
            Ok(()) => {
                sketch.status_message = Some(format!("Deleted sketch \"{name}\"."));
                if sketch.active_sketch_name.as_deref() == Some(name.as_str()) {
                    sketch.active_sketch_name = None;
                }
            }
            Err(err) => {
                sketch.status_message = Some(format!("Delete failed: {err}"));
            }
        }
        sketch.saved_sketch_names = list_saved_sketches();
        sketch.pending_delete_sketch_name = None;
    } else if cancel {
        sketch.pending_delete_sketch_name = None;
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
        if let Some(shortcut) = sketch_history_shortcut(
            input.modifiers,
            input.key_pressed(egui::Key::Z),
            input.key_pressed(egui::Key::Y),
        ) {
            apply_sketch_history_shortcut(sketch, shortcut, false);
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
        if let Some(shortcut) = sketch_history_shortcut(
            input.modifiers,
            input.key_pressed(egui::Key::Z),
            input.key_pressed(egui::Key::Y),
        ) {
            apply_sketch_history_shortcut(sketch, shortcut, true);
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SketchHistoryShortcut {
    Undo,
    Redo,
}

fn sketch_history_shortcut(
    modifiers: egui::Modifiers,
    z_pressed: bool,
    y_pressed: bool,
) -> Option<SketchHistoryShortcut> {
    if !modifiers.command {
        return None;
    }

    if z_pressed {
        return Some(if modifiers.shift {
            SketchHistoryShortcut::Redo
        } else {
            SketchHistoryShortcut::Undo
        });
    }

    y_pressed.then_some(SketchHistoryShortcut::Redo)
}

fn apply_sketch_history_shortcut(
    sketch: &mut SketchState,
    shortcut: SketchHistoryShortcut,
    cancel_text_draft_for_undo: bool,
) {
    match shortcut {
        SketchHistoryShortcut::Undo => {
            if cancel_text_draft_for_undo {
                sketch.cancel_text_draft();
            }
            sketch.undo();
        }
        SketchHistoryShortcut::Redo => {
            sketch.redo();
        }
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
        || SketchPoint::new(canvas_rect.width() * 0.5, 72.0),
        |pos| {
            if canvas_rect.contains(pos) {
                screen_to_canvas(pos, canvas_rect)
            } else {
                SketchPoint::new(canvas_rect.width() * 0.5, 72.0)
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
    let point = screen_to_canvas(pointer_pos, canvas_rect);
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
            if let Some(handle) = sketch.selected_resize_handle_at(point) {
                response
                    .ctx
                    .set_cursor_icon(cursor_icon_for_resize_handle(handle));
            }
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
                if let Some(handle) = sketch.selected_resize_handle_at(point) {
                    sketch.begin_resize_selected(handle, point);
                } else {
                    sketch.select_at(point);
                    sketch.begin_move_selected(point);
                }
            } else if response.dragged() {
                if sketch.resize_draft.is_some() {
                    sketch.update_resize_selected(point);
                } else {
                    sketch.update_move_selected(point);
                }
            }
            if response.drag_stopped() {
                sketch.finish_resize_selected();
                sketch.finish_move_selected();
            }
            if response.clicked() {
                if sketch.selected_resize_handle_at(point).is_none() {
                    sketch.select_at(point);
                }
            }
        }
    }
}

fn cursor_icon_for_resize_handle(handle: crate::sketch::ResizeHandle) -> egui::CursorIcon {
    match handle {
        crate::sketch::ResizeHandle::TopLeft | crate::sketch::ResizeHandle::BottomRight => {
            egui::CursorIcon::ResizeNwSe
        }
        crate::sketch::ResizeHandle::TopRight | crate::sketch::ResizeHandle::BottomLeft => {
            egui::CursorIcon::ResizeNeSw
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command_modifiers(shift: bool) -> egui::Modifiers {
        egui::Modifiers {
            command: true,
            shift,
            ..Default::default()
        }
    }

    #[test]
    fn sketch_history_shortcuts_map_command_z_to_undo() {
        assert_eq!(
            sketch_history_shortcut(command_modifiers(false), true, false),
            Some(SketchHistoryShortcut::Undo)
        );
    }

    #[test]
    fn sketch_history_shortcuts_map_command_shift_z_to_redo() {
        assert_eq!(
            sketch_history_shortcut(command_modifiers(true), true, false),
            Some(SketchHistoryShortcut::Redo)
        );
    }

    #[test]
    fn sketch_history_shortcuts_map_command_y_to_redo() {
        assert_eq!(
            sketch_history_shortcut(command_modifiers(false), false, true),
            Some(SketchHistoryShortcut::Redo)
        );
    }

    #[test]
    fn sketch_history_shortcuts_ignore_z_without_command_modifier() {
        assert_eq!(
            sketch_history_shortcut(egui::Modifiers::default(), true, false),
            None
        );
    }
}
