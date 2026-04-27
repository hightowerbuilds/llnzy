use crate::sketch::{
    list_saved_sketches, DraftElement, RectElement, SketchElement, SketchPoint, SketchState,
    SketchTool,
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
                sketch.save_as_input = sketch
                    .active_sketch_name
                    .clone()
                    .unwrap_or_default();
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
    });

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
                let is_active = sketch
                    .active_sketch_name
                    .as_deref()
                    == Some(name.as_str());
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
                    if ui.add(egui::Label::new(label).sense(egui::Sense::click())).clicked() {
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

fn sketch_shortcuts(ui: &egui::Ui, sketch: &mut SketchState) {
    ui.input(|input| {
        if input.modifiers.command && input.key_pressed(egui::Key::Z) {
            if input.modifiers.shift {
                sketch.redo();
            } else {
                sketch.undo();
            }
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
        // Collect typed text from events
        for event in &input.events {
            if let egui::Event::Text(s) = event {
                text.push_str(s);
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

fn paint_sketch_document(painter: &egui::Painter, canvas_rect: egui::Rect, sketch: &SketchState) {
    for (index, element) in sketch.document.elements.iter().enumerate() {
        let is_text_draft = sketch
            .text_draft
            .as_ref()
            .is_some_and(|d| d.index == index);
        // For a text element being actively edited, we paint it with the
        // draft text (which may differ from the committed text) and with
        // a cursor. The paint_inline_text_cursor function handles the cursor,
        // so here we just paint the draft text instead of the committed text.
        if is_text_draft {
            if let SketchElement::Text(text_el) = element {
                let draft_text = sketch
                    .text_draft
                    .as_ref()
                    .map(|d| d.text.as_str())
                    .unwrap_or("");
                if !draft_text.is_empty() {
                    let pos =
                        canvas_to_screen(canvas_rect, SketchPoint::new(text_el.x, text_el.y));
                    painter.text(
                        pos,
                        egui::Align2::LEFT_TOP,
                        draft_text,
                        egui::FontId::proportional(text_el.style.font_size),
                        color32(text_el.style.stroke_color),
                    );
                }
            }
        } else {
            paint_sketch_element(
                painter,
                canvas_rect,
                element,
                sketch.selected == Some(index),
            );
        }
    }

    match &sketch.draft {
        Some(DraftElement::Stroke(stroke)) => {
            paint_stroke(
                painter,
                canvas_rect,
                &stroke.points,
                stroke.style.stroke_color,
                stroke.style.stroke_width,
            );
        }
        Some(DraftElement::Rectangle { .. }) => {
            if let Some(rect) = sketch.draft_rectangle() {
                paint_rectangle(painter, canvas_rect, &rect, false);
            }
        }
        None => {}
    }
}

fn paint_sketch_element(
    painter: &egui::Painter,
    canvas_rect: egui::Rect,
    element: &SketchElement,
    selected: bool,
) {
    match element {
        SketchElement::Stroke(stroke) => {
            paint_stroke(
                painter,
                canvas_rect,
                &stroke.points,
                stroke.style.stroke_color,
                stroke.style.stroke_width,
            );
        }
        SketchElement::Rectangle(rect) => {
            paint_rectangle(painter, canvas_rect, rect, selected);
        }
        SketchElement::Text(text) => {
            if text.text.is_empty() {
                return;
            }
            let pos = canvas_to_screen(canvas_rect, SketchPoint::new(text.x, text.y));
            let screen_rect = egui::Rect::from_min_size(pos, egui::Vec2::new(text.w, text.h));
            painter.text(
                pos,
                egui::Align2::LEFT_TOP,
                &text.text,
                egui::FontId::proportional(text.style.font_size),
                color32(text.style.stroke_color),
            );
            if selected {
                paint_selection(painter, screen_rect);
            }
        }
    }
}

/// Paint a blinking cursor at the end of the inline text draft on the canvas.
fn paint_inline_text_cursor(
    ctx: &egui::Context,
    painter: &egui::Painter,
    canvas_rect: egui::Rect,
    sketch: &SketchState,
) {
    let Some(draft) = &sketch.text_draft else {
        return;
    };
    let Some(SketchElement::Text(text_el)) = sketch.document.elements.get(draft.index) else {
        return;
    };

    let pos = canvas_to_screen(canvas_rect, SketchPoint::new(text_el.x, text_el.y));
    let font_id = egui::FontId::proportional(text_el.style.font_size);
    let text_color = color32(text_el.style.stroke_color);

    // Measure text width to place cursor after the last character
    let galley = painter.layout_no_wrap(draft.text.clone(), font_id.clone(), text_color);
    let text_width = galley.rect.width();
    let text_height = text_el.style.font_size;

    // Blinking: visible for ~500ms, hidden for ~500ms
    let time = ctx.input(|i| i.time);
    let blink_on = (time * 2.0) as u64 % 2 == 0;

    if blink_on {
        let cursor_x = pos.x + text_width;
        let cursor_top = pos.y;
        let cursor_bottom = pos.y + text_height;
        painter.line_segment(
            [
                egui::pos2(cursor_x + 1.0, cursor_top),
                egui::pos2(cursor_x + 1.0, cursor_bottom),
            ],
            egui::Stroke::new(1.5, text_color),
        );
    }

    // Request repaint for blink animation
    ctx.request_repaint();
}

fn paint_stroke(
    painter: &egui::Painter,
    canvas_rect: egui::Rect,
    points: &[SketchPoint],
    color: [u8; 4],
    width: f32,
) {
    if points.len() < 2 {
        return;
    }
    let screen_points: Vec<egui::Pos2> = points
        .iter()
        .map(|point| canvas_to_screen(canvas_rect, *point))
        .collect();
    painter.add(egui::Shape::line(
        screen_points,
        egui::Stroke::new(width, color32(color)),
    ));
}

fn paint_rectangle(
    painter: &egui::Painter,
    canvas_rect: egui::Rect,
    rect: &RectElement,
    selected: bool,
) {
    let screen_rect = egui::Rect::from_min_size(
        canvas_to_screen(canvas_rect, SketchPoint::new(rect.x, rect.y)),
        egui::Vec2::new(rect.w, rect.h),
    );
    if let Some(fill) = rect.style.fill_color {
        painter.rect_filled(screen_rect, egui::Rounding::same(2.0), color32(fill));
    }
    painter.rect_stroke(
        screen_rect,
        egui::Rounding::same(2.0),
        egui::Stroke::new(rect.style.stroke_width, color32(rect.style.stroke_color)),
    );
    if selected {
        paint_selection(painter, screen_rect);
    }
}

fn paint_selection(painter: &egui::Painter, rect: egui::Rect) {
    painter.rect_stroke(
        rect.expand(4.0),
        egui::Rounding::same(3.0),
        egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 130, 255)),
    );
}

fn screen_to_canvas(pos: egui::Pos2, canvas_rect: egui::Rect) -> SketchPoint {
    SketchPoint::new(
        (pos.x - canvas_rect.min.x).clamp(0.0, canvas_rect.width()),
        (pos.y - canvas_rect.min.y).clamp(0.0, canvas_rect.height()),
    )
}

fn canvas_to_screen(canvas_rect: egui::Rect, point: SketchPoint) -> egui::Pos2 {
    egui::pos2(canvas_rect.min.x + point.x, canvas_rect.min.y + point.y)
}

fn color32(color: [u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3])
}
