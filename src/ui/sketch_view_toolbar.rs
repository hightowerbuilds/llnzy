use std::path::Path;

use crate::sketch::{
    default_export_file_name, export_svg_to_path, list_saved_sketches, SketchPoint, SketchState,
    SketchSymbolKind, SketchTool,
};

use super::sketch_paint::paint_symbol_shape;
use super::sketch_view::SketchAppearance;

pub(super) fn render_sidebar_toolbar(
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

pub(super) fn render_sketch_toolbar(
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
