use crate::sketch::{
    DraftElement, RectElement, SketchElement, SketchPoint, SketchState, SketchTool,
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
    sketch_shortcuts(ui, sketch);

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Sketch")
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
        .on_hover_text("Click to place a text box. Double-click existing text to edit");

        ui.separator();

        for color in [
            [235, 238, 245, 255],
            [92, 160, 255, 255],
            [84, 220, 150, 255],
            [255, 205, 92, 255],
            [255, 105, 150, 255],
            [28, 30, 38, 255],
        ] {
            let (rect, response) =
                ui.allocate_exact_size(egui::Vec2::splat(22.0), egui::Sense::click());
            ui.painter()
                .rect_filled(rect.shrink(3.0), egui::Rounding::same(3.0), color32(color));
            if sketch.style.stroke_color == color {
                ui.painter().rect_stroke(
                    rect,
                    egui::Rounding::same(4.0),
                    egui::Stroke::new(1.5, egui::Color32::WHITE),
                );
            }
            if response.clicked() {
                sketch.style.stroke_color = color;
            }
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
    });

    ui.add_space(12.0);

    let available = ui.available_size();
    let canvas_size = egui::Vec2::new(available.x.max(320.0), available.y.max(240.0));
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
    render_text_editor(ctx, canvas_rect, sketch);

    canvas_rect
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
        paint_sketch_element(
            painter,
            canvas_rect,
            element,
            sketch.selected == Some(index),
        );
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

fn render_text_editor(ctx: &egui::Context, canvas_rect: egui::Rect, sketch: &mut SketchState) {
    let Some(draft) = sketch.text_draft.clone() else {
        return;
    };
    let Some(SketchElement::Text(text_box)) = sketch.document.elements.get(draft.index) else {
        return;
    };

    let pos = canvas_to_screen(canvas_rect, SketchPoint::new(text_box.x, text_box.y));
    let width = text_box.w;
    let mut draft_text = draft.text;
    let mut commit = false;
    let mut cancel = false;

    egui::Area::new(egui::Id::new(("sketch_text_editor", draft.index)))
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(egui::Color32::from_rgba_premultiplied(250, 250, 245, 235))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgb(60, 130, 255),
                ))
                .inner_margin(egui::Margin::same(4.0))
                .show(ui, |ui| {
                    ui.set_width(width);
                    let response = ui.add(
                        egui::TextEdit::multiline(&mut draft_text)
                            .desired_rows(2)
                            .desired_width(width)
                            .font(egui::TextStyle::Body),
                    );
                    response.request_focus();
                    if response.changed() {
                        sketch.update_text_draft(draft_text.clone());
                    }
                    ui.horizontal(|ui| {
                        if ui.small_button("Done").clicked() {
                            commit = true;
                        }
                        if ui.small_button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                    ui.input(|input| {
                        if input.key_pressed(egui::Key::Escape) {
                            cancel = true;
                        }
                        if input.modifiers.command && input.key_pressed(egui::Key::Enter) {
                            commit = true;
                        }
                    });
                });
        });

    if cancel {
        sketch.cancel_text_draft();
    } else if commit {
        sketch.update_text_draft(draft_text);
        sketch.commit_text_draft();
    }
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
