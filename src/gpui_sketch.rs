use std::path::{Path, PathBuf};

use gpui::prelude::*;
use gpui::{
    actions, div, fill, img, point, px, relative, rgb, rgba, size, App, Bounds, ContentMask,
    Context, Element, ElementId, Entity, FocusHandle, Focusable, GlobalElementId, KeyBinding,
    LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ObjectFit, PaintQuad,
    Path as GpuiPath, PathBuilder, Pixels, Point, Render, SharedString, Style, StyledImage,
    TextAlign, TextRun, Window, WrappedLine,
};

use crate::sketch::{
    default_jpeg_export_file_name, export_jpeg_to_path, save_appearance_settings,
    save_default_document, save_named_sketch, sketch_screenshot_drop_dir, DraftElement,
    ImageElement, RectElement, SketchCanvasBackgroundMode, SketchElement, SketchGridMode,
    SketchPoint, SketchState, SketchTool,
};

const SKETCH_BG: u32 = 0x191920;
const SKETCH_PANEL_BG: u32 = 0x1b1b22;
const SKETCH_CANVAS_BG: u32 = 0x1c1e24;
const SKETCH_BORDER: u32 = 0x30323a;
const SKETCH_TEXT: u32 = 0xe8edf6;
const SKETCH_MUTED: u32 = 0x9aa2b3;
const SKETCH_ACCENT: u32 = 0x6aff90;
const SKETCH_ACTIVE_BG: u32 = 0x183725;
const SKETCH_BUTTON_BG: u32 = 0x242632;
const SKETCH_SELECTION: u32 = 0x3c82ffff;

actions!(
    sketch_gpui,
    [
        DeleteSelected,
        EscapeSketch,
        UndoSketch,
        RedoSketch,
        SaveSketch
    ]
);

pub(crate) fn bind_sketch_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("delete", DeleteSelected, Some("SketchSurface")),
        KeyBinding::new("backspace", DeleteSelected, Some("SketchSurface")),
        KeyBinding::new("escape", EscapeSketch, Some("SketchSurface")),
        KeyBinding::new("cmd-z", UndoSketch, Some("SketchSurface")),
        KeyBinding::new("cmd-shift-z", RedoSketch, Some("SketchSurface")),
        KeyBinding::new("cmd-s", SaveSketch, Some("SketchSurface")),
    ]);
}

pub(crate) struct SketchSurface {
    focus_handle: FocusHandle,
    state: SketchState,
    workspace_root: Option<PathBuf>,
    last_bounds: Option<Bounds<Pixels>>,
    is_dragging: bool,
    status_message: Option<String>,
}

impl SketchSurface {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            state: SketchState::load_default(),
            workspace_root: None,
            last_bounds: None,
            is_dragging: false,
            status_message: None,
        }
    }

    pub(crate) fn set_workspace_root(&mut self, workspace_root: Option<PathBuf>) {
        self.workspace_root = workspace_root;
    }

    fn set_tool(&mut self, tool: SketchTool, cx: &mut Context<Self>) {
        self.state.set_tool(tool);
        self.status_message = Some(match tool {
            SketchTool::Select => "Select and move shapes".into(),
            SketchTool::Marker => "Marker ready".into(),
            SketchTool::Rectangle => "Rectangle ready".into(),
            SketchTool::Text => "Text boxes render; GPUI text editing is next".into(),
        });
        cx.notify();
    }

    fn cycle_grid(&mut self, cx: &mut Context<Self>) {
        self.state.appearance.grid_mode = match self.state.appearance.grid_mode {
            SketchGridMode::Hidden => SketchGridMode::Lines,
            SketchGridMode::Lines => SketchGridMode::Dots,
            SketchGridMode::Dots => SketchGridMode::Hidden,
        };
        let _ = save_appearance_settings(&self.state.appearance);
        cx.notify();
    }

    fn clear_canvas(&mut self, cx: &mut Context<Self>) {
        if self.state.clear() {
            self.status_message = Some("Sketch cleared".into());
            self.persist_if_dirty();
            cx.notify();
        }
    }

    pub(crate) fn save_from_workspace(&mut self, cx: &mut Context<Self>) {
        match self.persist() {
            Ok(()) => self.status_message = Some("Sketch saved".into()),
            Err(err) => self.status_message = Some(format!("Save failed: {err}")),
        }
        cx.notify();
    }

    pub(crate) fn undo_from_workspace(&mut self, cx: &mut Context<Self>) {
        if self.state.undo() {
            self.status_message = Some("Undo".into());
            self.persist_if_dirty();
            cx.notify();
        }
    }

    pub(crate) fn redo_from_workspace(&mut self, cx: &mut Context<Self>) {
        if self.state.redo() {
            self.status_message = Some("Redo".into());
            self.persist_if_dirty();
            cx.notify();
        }
    }

    fn persist_if_dirty(&mut self) {
        if self.state.is_dirty() {
            let _ = self.persist();
        }
    }

    fn persist(&mut self) -> Result<(), String> {
        save_default_document(&self.state.document)?;
        if let Some(name) = self.state.active_sketch_name.clone() {
            save_named_sketch(&name, &self.state.document)?;
        }
        self.state.mark_saved();
        Ok(())
    }

    fn import_image(&mut self, cx: &mut Context<Self>) {
        let insert_point = self.default_import_point();
        cx.spawn(
            move |surface: gpui::WeakEntity<SketchSurface>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let Some(file) = rfd::AsyncFileDialog::new()
                        .set_title("Import Sketch Image")
                        .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "webp", "gif"])
                        .pick_file()
                        .await
                    else {
                        return;
                    };
                    let path = file.path().to_path_buf();
                    let _ = surface.update(&mut cx, |surface, cx| {
                        match surface.state.add_image_from_path(&path, insert_point) {
                            Ok(_) => {
                                surface.state.set_tool(SketchTool::Select);
                                surface.status_message = Some(format!(
                                    "Imported {}",
                                    path.file_name()
                                        .and_then(|name| name.to_str())
                                        .unwrap_or("image")
                                ));
                                surface.persist_if_dirty();
                            }
                            Err(err) => {
                                surface.status_message = Some(format!("Import failed: {err}"));
                            }
                        }
                        cx.notify();
                    });
                }
            },
        )
        .detach();
    }

    fn export_jpeg(&mut self, cx: &mut Context<Self>) {
        if let Some(root) = self.workspace_root.clone() {
            self.export_jpeg_to_dir(root.join("sketches"), cx);
            return;
        }

        cx.spawn(
            |surface: gpui::WeakEntity<SketchSurface>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let Some(folder) = rfd::AsyncFileDialog::new()
                        .set_title("Export Sketch JPEG")
                        .pick_folder()
                        .await
                    else {
                        return;
                    };
                    let directory = folder.path().to_path_buf();
                    let _ = surface.update(&mut cx, |surface, cx| {
                        surface.export_jpeg_to_dir(directory, cx);
                    });
                }
            },
        )
        .detach();
    }

    fn export_jpeg_to_dir(&mut self, directory: PathBuf, cx: &mut Context<Self>) {
        let filename = default_jpeg_export_file_name(self.state.active_sketch_name.as_deref());
        let path = unique_export_path(&directory, &filename);
        let canvas_size = self.export_canvas_size();
        match export_jpeg_to_path(&self.state.document, &path, canvas_size) {
            Ok(result) => {
                self.status_message = Some(format!(
                    "Exported {} ({}x{})",
                    result.path.display(),
                    result.width,
                    result.height
                ));
            }
            Err(err) => {
                self.status_message = Some(format!("Export failed: {err}"));
            }
        }
        cx.notify();
    }

    fn open_screenshot_drop_folder(&mut self, cx: &mut Context<Self>) {
        match sketch_screenshot_drop_dir() {
            Ok(dir) => {
                self.status_message = Some(format!("Screenshot drop folder: {}", dir.display()));
                let _ = std::process::Command::new("open").arg(&dir).spawn();
            }
            Err(err) => {
                self.status_message = Some(format!("Drop folder failed: {err}"));
            }
        }
        cx.notify();
    }

    fn resize_selected_image(&mut self, scale: f32, cx: &mut Context<Self>) {
        if self.state.resize_selected_image_to_scale(scale) {
            self.status_message = Some(format!("Image {:.0}%", scale * 100.0));
            self.persist_if_dirty();
            cx.notify();
        }
    }

    fn default_import_point(&self) -> SketchPoint {
        SketchPoint::new(24.0, 24.0)
    }

    fn export_canvas_size(&self) -> [f32; 2] {
        let [width, height] = self.state.last_canvas_size;
        [width.max(1.0), height.max(1.0)]
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle(cx));
        let Some(point) = self.canvas_point(event.position) else {
            return;
        };

        self.is_dragging = true;
        match self.state.tool {
            SketchTool::Marker => self.state.begin_stroke(point),
            SketchTool::Rectangle => self.state.begin_rectangle(point),
            SketchTool::Select => self.begin_select_drag(point),
            SketchTool::Text => {
                self.state.add_text_box(point);
                self.status_message = Some("Text box created; editing is pending in GPUI".into());
                self.persist_if_dirty();
            }
        }
        cx.notify();
    }

    fn begin_select_drag(&mut self, point: SketchPoint) {
        if let Some(handle) = self.state.selected_resize_handle_at(point) {
            self.state.begin_resize_selected(handle, point);
            return;
        }

        if self.state.select_at(point).is_some() {
            self.state.begin_move_selected(point);
        }
    }

    fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.is_dragging || !event.dragging() {
            return;
        }
        let Some(point) = self.canvas_point(event.position) else {
            return;
        };

        let changed = match self.state.tool {
            SketchTool::Marker => {
                self.state.append_stroke_point(point);
                true
            }
            SketchTool::Rectangle => {
                self.state.update_rectangle(point);
                true
            }
            SketchTool::Select => {
                self.state.update_resize_selected(point) || self.state.update_move_selected(point)
            }
            SketchTool::Text => false,
        };

        if changed {
            cx.notify();
        }
    }

    fn on_mouse_up(&mut self, event: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_dragging {
            return;
        }

        if let Some(point) = self.canvas_point(event.position) {
            match self.state.tool {
                SketchTool::Marker => self.state.append_stroke_point(point),
                SketchTool::Rectangle => self.state.update_rectangle(point),
                SketchTool::Select => {
                    self.state.update_resize_selected(point);
                    self.state.update_move_selected(point);
                }
                SketchTool::Text => {}
            }
        }

        let changed = match self.state.tool {
            SketchTool::Marker => self.state.finish_stroke(),
            SketchTool::Rectangle => self.state.finish_rectangle(),
            SketchTool::Select => {
                self.state.finish_resize_selected() || self.state.finish_move_selected()
            }
            SketchTool::Text => false,
        };
        self.is_dragging = false;
        if changed {
            self.persist_if_dirty();
        }
        cx.notify();
    }

    fn canvas_point(&self, point: Point<Pixels>) -> Option<SketchPoint> {
        let bounds = self.last_bounds?;
        if !bounds.contains(&point) {
            return None;
        }
        let x = ((point.x - bounds.left()) / px(1.0)).clamp(0.0, bounds.size.width / px(1.0));
        let y = ((point.y - bounds.top()) / px(1.0)).clamp(0.0, bounds.size.height / px(1.0));
        Some(SketchPoint::new(x, y))
    }

    fn delete_selected_action(
        &mut self,
        _: &DeleteSelected,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.state.delete_selected() {
            self.status_message = Some("Deleted selection".into());
            self.persist_if_dirty();
            cx.notify();
        }
    }

    fn escape_action(&mut self, _: &EscapeSketch, _: &mut Window, cx: &mut Context<Self>) {
        self.state.cancel_text_draft();
        self.state.draft = None;
        self.state.move_draft = None;
        self.state.resize_draft = None;
        self.state.selected = None;
        self.is_dragging = false;
        cx.notify();
    }

    fn undo_action(&mut self, _: &UndoSketch, _: &mut Window, cx: &mut Context<Self>) {
        self.undo_from_workspace(cx);
    }

    fn redo_action(&mut self, _: &RedoSketch, _: &mut Window, cx: &mut Context<Self>) {
        self.redo_from_workspace(cx);
    }

    fn save_action(&mut self, _: &SaveSketch, _: &mut Window, cx: &mut Context<Self>) {
        self.save_from_workspace(cx);
    }
}

impl Focusable for SketchSurface {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SketchSurface {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(SKETCH_BG))
            .text_color(rgb(SKETCH_TEXT))
            .font_family("Inter")
            .key_context("SketchSurface")
            .track_focus(&self.focus_handle(cx))
            .on_action(cx.listener(Self::delete_selected_action))
            .on_action(cx.listener(Self::escape_action))
            .on_action(cx.listener(Self::undo_action))
            .on_action(cx.listener(Self::redo_action))
            .on_action(cx.listener(Self::save_action))
            .child(sketch_header(self))
            .child(sketch_toolbar(
                self.state.tool,
                self.state.appearance.grid_mode,
                self.state.selected_image_scale(),
                cx,
            ))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .bg(rgb(SKETCH_PANEL_BG))
                    .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
                    .on_mouse_move(cx.listener(Self::on_mouse_move))
                    .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
                    .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
                    .child(SketchCanvasElement {
                        surface: cx.entity(),
                        layer: SketchCanvasLayer::Background,
                    })
                    .child(sketch_image_layer(&self.state))
                    .child(SketchCanvasElement {
                        surface: cx.entity(),
                        layer: SketchCanvasLayer::Foreground,
                    }),
            )
    }
}

fn sketch_header(surface: &SketchSurface) -> impl IntoElement {
    let status = surface.status_message.clone().unwrap_or_else(|| {
        if surface.state.is_dirty() {
            "Unsaved sketch".into()
        } else {
            "Scratch sketch ready".into()
        }
    });
    let title = surface
        .state
        .active_sketch_name
        .clone()
        .unwrap_or_else(|| "Sketch Pad".into());

    div()
        .h(px(42.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_3()
        .border_b_1()
        .border_color(rgb(SKETCH_BORDER))
        .bg(rgb(0x121217))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_size(px(13.0)).child(title))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(SKETCH_MUTED))
                        .child(format!(
                            "{} elements | {}",
                            surface.state.document.elements.len(),
                            if surface.state.is_dirty() {
                                "dirty"
                            } else {
                                "saved"
                            }
                        )),
                ),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(SKETCH_ACCENT))
                .child(status),
        )
}

fn sketch_toolbar(
    active_tool: SketchTool,
    grid_mode: SketchGridMode,
    selected_image_scale: Option<f32>,
    cx: &mut Context<SketchSurface>,
) -> impl IntoElement {
    let mut toolbar = div()
        .h(px(44.0))
        .w_full()
        .flex()
        .items_center()
        .gap_1()
        .px_3()
        .border_b_1()
        .border_color(rgb(SKETCH_BORDER))
        .bg(rgb(SKETCH_BG))
        .child(tool_button(
            "Select",
            active_tool == SketchTool::Select,
            cx,
            |this, cx| {
                this.set_tool(SketchTool::Select, cx);
            },
        ))
        .child(tool_button(
            "Marker",
            active_tool == SketchTool::Marker,
            cx,
            |this, cx| {
                this.set_tool(SketchTool::Marker, cx);
            },
        ))
        .child(tool_button(
            "Rectangle",
            active_tool == SketchTool::Rectangle,
            cx,
            |this, cx| {
                this.set_tool(SketchTool::Rectangle, cx);
            },
        ))
        .child(div().w(px(1.0)).h(px(24.0)).bg(rgb(SKETCH_BORDER)))
        .child(tool_button(
            grid_label(grid_mode),
            grid_mode != SketchGridMode::Hidden,
            cx,
            |this, cx| {
                this.cycle_grid(cx);
            },
        ))
        .child(tool_button("Undo", false, cx, |this, cx| {
            this.undo_from_workspace(cx);
        }))
        .child(tool_button("Redo", false, cx, |this, cx| {
            this.redo_from_workspace(cx);
        }))
        .child(div().w(px(1.0)).h(px(24.0)).bg(rgb(SKETCH_BORDER)))
        .child(tool_button("Import", false, cx, |this, cx| {
            this.import_image(cx);
        }))
        .child(tool_button("Drop Folder", false, cx, |this, cx| {
            this.open_screenshot_drop_folder(cx);
        }));

    if let Some(scale) = selected_image_scale {
        let smaller = (scale * 0.8).max(0.05);
        let larger = (scale * 1.25).min(2.0);
        toolbar = toolbar
            .child(div().w(px(1.0)).h(px(24.0)).bg(rgb(SKETCH_BORDER)))
            .child(tool_button("Image -", false, cx, move |this, cx| {
                this.resize_selected_image(smaller, cx);
            }))
            .child(tool_button(
                "Image 100%",
                (scale - 1.0).abs() < 0.01,
                cx,
                |this, cx| {
                    this.resize_selected_image(1.0, cx);
                },
            ))
            .child(tool_button("Image +", false, cx, move |this, cx| {
                this.resize_selected_image(larger, cx);
            }));
    }

    toolbar
        .child(div().flex_1())
        .child(tool_button("Clear", false, cx, |this, cx| {
            this.clear_canvas(cx);
        }))
        .child(tool_button("Save", false, cx, |this, cx| {
            this.save_from_workspace(cx);
        }))
        .child(tool_button("Export JPG", false, cx, |this, cx| {
            this.export_jpeg(cx);
        }))
}

fn grid_label(grid_mode: SketchGridMode) -> &'static str {
    match grid_mode {
        SketchGridMode::Hidden => "Grid",
        SketchGridMode::Lines => "Lines",
        SketchGridMode::Dots => "Dots",
    }
}

fn tool_button(
    label: impl Into<SharedString>,
    active: bool,
    cx: &mut Context<SketchSurface>,
    on_click: impl Fn(&mut SketchSurface, &mut Context<SketchSurface>) + 'static,
) -> impl IntoElement {
    let label = label.into();
    div()
        .h(px(30.0))
        .flex()
        .items_center()
        .justify_center()
        .px_2()
        .rounded_sm()
        .border_1()
        .border_color(rgb(if active { 0x47785f } else { SKETCH_BORDER }))
        .bg(rgb(if active {
            SKETCH_ACTIVE_BG
        } else {
            SKETCH_BUTTON_BG
        }))
        .text_size(px(12.0))
        .text_color(rgb(if active { SKETCH_ACCENT } else { SKETCH_TEXT }))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                window.focus(&this.focus_handle(cx));
                on_click(this, cx);
            }),
        )
        .child(label)
}

struct SketchCanvasElement {
    surface: Entity<SketchSurface>,
    layer: SketchCanvasLayer,
}

#[derive(Clone, Copy)]
enum SketchCanvasLayer {
    Background,
    Foreground,
}

struct SketchPrepaintState {
    quads: Vec<PaintQuad>,
    paths: Vec<(GpuiPath<Pixels>, u32)>,
    text: Vec<SketchPaintText>,
}

struct SketchPaintText {
    line: WrappedLine,
    origin: Point<Pixels>,
    line_height: Pixels,
}

impl IntoElement for SketchCanvasElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SketchCanvasElement {
    type RequestLayoutState = ();
    type PrepaintState = SketchPrepaintState;

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
        let surface = self.surface.read(cx);
        build_canvas_paint(&surface.state, bounds, self.layer, window)
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
        self.surface.update(cx, |surface, _cx| {
            surface.last_bounds = Some(bounds);
            surface.state.last_canvas_size =
                [bounds.size.width / px(1.0), bounds.size.height / px(1.0)];
        });

        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for quad in prepaint.quads.drain(..) {
                window.paint_quad(quad);
            }
            for (path, color) in prepaint.paths.drain(..) {
                window.paint_path(path, rgba(color));
            }
            for text in prepaint.text.drain(..) {
                text.line
                    .paint(
                        text.origin,
                        text.line_height,
                        TextAlign::Left,
                        Some(bounds),
                        window,
                        cx,
                    )
                    .ok();
            }
        });
    }
}

fn build_canvas_paint(
    state: &SketchState,
    bounds: Bounds<Pixels>,
    layer: SketchCanvasLayer,
    window: &mut Window,
) -> SketchPrepaintState {
    let mut paint = SketchPrepaintState {
        quads: Vec::new(),
        paths: Vec::new(),
        text: Vec::new(),
    };

    match layer {
        SketchCanvasLayer::Background => {
            let canvas_bg = match state.appearance.canvas_background_mode {
                SketchCanvasBackgroundMode::Theme => rgb(SKETCH_CANVAS_BG),
                SketchCanvasBackgroundMode::Solid => {
                    rgba(rgba_u32(state.appearance.canvas_background_color))
                }
            };
            paint.quads.push(fill(bounds, canvas_bg));
            paint_grid(&mut paint, bounds, state);
            return paint;
        }
        SketchCanvasLayer::Foreground => {}
    }

    for element in &state.document.elements {
        if matches!(element, SketchElement::Image(_)) {
            continue;
        }
        paint_element(&mut paint, element, bounds, window);
    }

    if let Some(draft) = &state.draft {
        paint_draft(&mut paint, draft, bounds);
    }
    if let Some(rect) = state.draft_rectangle() {
        paint_rect_element(&mut paint, &rect, bounds);
    }

    if let Some(index) = state.selected {
        if let Some(element) = state.document.elements.get(index) {
            paint_selection(&mut paint, element, bounds, state);
        }
    }

    if state.appearance.canvas_border_visible {
        paint_rect_outline(&mut paint.quads, bounds, 1.0, SKETCH_BORDER);
    }

    paint
}

fn sketch_image_layer(state: &SketchState) -> gpui::Div {
    state
        .document
        .elements
        .iter()
        .filter_map(|element| match element {
            SketchElement::Image(image) => Some(image.clone()),
            _ => None,
        })
        .fold(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .overflow_hidden(),
            |layer, image| layer.child(sketch_image_element(image)),
        )
}

fn sketch_image_element(image: ImageElement) -> impl IntoElement {
    div()
        .absolute()
        .left(px(image.x))
        .top(px(image.y))
        .w(px(image.w.max(1.0)))
        .h(px(image.h.max(1.0)))
        .overflow_hidden()
        .border_1()
        .border_color(rgb(0x111722))
        .child(
            img(PathBuf::from(image.path))
                .size_full()
                .object_fit(ObjectFit::Fill),
        )
}

fn unique_export_path(directory: &Path, filename: &str) -> PathBuf {
    let source = Path::new(filename);
    let stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.trim().is_empty())
        .unwrap_or("sketch");
    let extension = source
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| !extension.trim().is_empty())
        .unwrap_or("jpg");
    let mut candidate = directory.join(format!("{stem}.{extension}"));
    let mut index = 1;
    while candidate.exists() {
        candidate = directory.join(format!("{stem}-{index}.{extension}"));
        index += 1;
    }
    candidate
}

fn paint_grid(paint: &mut SketchPrepaintState, bounds: Bounds<Pixels>, state: &SketchState) {
    if !state.appearance.grid_visible() {
        return;
    }
    let spacing = state.appearance.effective_grid_spacing().max(4.0);
    let alpha = (state.appearance.effective_grid_opacity() * 255.0).round() as u8;
    let color = rgba_u32([130, 140, 160, alpha]);
    let width = bounds.size.width / px(1.0);
    let height = bounds.size.height / px(1.0);

    match state.appearance.grid_mode {
        SketchGridMode::Hidden => {}
        SketchGridMode::Lines => {
            let mut x = 0.0;
            while x <= width {
                paint.quads.push(fill(
                    Bounds::new(
                        point(bounds.left() + px(x), bounds.top()),
                        size(px(1.0), bounds.size.height),
                    ),
                    rgba(color),
                ));
                x += spacing;
            }
            let mut y = 0.0;
            while y <= height {
                paint.quads.push(fill(
                    Bounds::new(
                        point(bounds.left(), bounds.top() + px(y)),
                        size(bounds.size.width, px(1.0)),
                    ),
                    rgba(color),
                ));
                y += spacing;
            }
        }
        SketchGridMode::Dots => {
            let mut y = 0.0;
            while y <= height {
                let mut x = 0.0;
                while x <= width {
                    paint.quads.push(fill(
                        Bounds::new(
                            point(bounds.left() + px(x), bounds.top() + px(y)),
                            size(px(2.0), px(2.0)),
                        ),
                        rgba(color),
                    ));
                    x += spacing;
                }
                y += spacing;
            }
        }
    }
}

fn paint_element(
    paint: &mut SketchPrepaintState,
    element: &SketchElement,
    bounds: Bounds<Pixels>,
    window: &mut Window,
) {
    match element {
        SketchElement::Stroke(stroke) => {
            if let Some(path) = stroke_path(&stroke.points, stroke.style.stroke_width, bounds) {
                paint
                    .paths
                    .push((path, rgba_u32(stroke.style.stroke_color)));
            }
        }
        SketchElement::Rectangle(rect) => paint_rect_element(paint, rect, bounds),
        SketchElement::Text(text) => {
            let text_bounds = local_bounds(bounds, text.x, text.y, text.w, text.h);
            paint.quads.push(fill(text_bounds, rgba(0x10131a70)));
            paint_rect_outline(
                &mut paint.quads,
                text_bounds,
                1.0,
                rgba_u32(text.style.stroke_color),
            );
            paint_text_box(
                &mut paint.text,
                &text.text,
                text_bounds,
                text.style.font_size,
                rgba_u32(text.style.stroke_color),
                window,
            );
        }
        SketchElement::Image(image) => {
            let image_bounds = local_bounds(bounds, image.x, image.y, image.w, image.h);
            paint.quads.push(fill(image_bounds, rgba(0x111722cc)));
            paint_rect_outline(&mut paint.quads, image_bounds, 1.0, 0x475569ff);
            let label = Path::new(&image.path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Image");
            paint_text_box(
                &mut paint.text,
                label,
                image_bounds,
                12.0,
                0xcbd5e1ff,
                window,
            );
        }
        SketchElement::Symbol(symbol) => {
            let symbol_bounds = local_bounds(bounds, symbol.x, symbol.y, symbol.w, symbol.h);
            paint.quads.push(fill(symbol_bounds, rgba(0x132018cc)));
            paint_rect_outline(
                &mut paint.quads,
                symbol_bounds,
                2.0,
                rgba_u32(symbol.style.stroke_color),
            );
            paint_text_box(
                &mut paint.text,
                symbol.kind.label(),
                symbol_bounds,
                13.0,
                rgba_u32(symbol.style.stroke_color),
                window,
            );
        }
    }
}

fn paint_rect_element(paint: &mut SketchPrepaintState, rect: &RectElement, canvas: Bounds<Pixels>) {
    let rect_bounds = local_bounds(canvas, rect.x, rect.y, rect.w, rect.h);
    if let Some(fill_color) = rect.style.fill_color {
        paint
            .quads
            .push(fill(rect_bounds, rgba(rgba_u32(fill_color))));
    }
    paint_rect_outline(
        &mut paint.quads,
        rect_bounds,
        rect.style.stroke_width.max(1.0),
        rgba_u32(rect.style.stroke_color),
    );
}

fn paint_draft(paint: &mut SketchPrepaintState, draft: &DraftElement, bounds: Bounds<Pixels>) {
    match draft {
        DraftElement::Stroke(stroke) => {
            if let Some(path) = stroke_path(&stroke.points, stroke.style.stroke_width, bounds) {
                paint
                    .paths
                    .push((path, rgba_u32(stroke.style.stroke_color)));
            }
        }
        DraftElement::Rectangle { .. } => {}
    }
}

fn paint_selection(
    paint: &mut SketchPrepaintState,
    element: &SketchElement,
    canvas: Bounds<Pixels>,
    state: &SketchState,
) {
    let Some(bounds) = element_bounds(element, canvas) else {
        return;
    };
    let color = rgba_u32(state.appearance.selection_outline_color);
    paint_rect_outline(&mut paint.quads, bounds, 1.0, color);

    if matches!(
        element,
        SketchElement::Rectangle(_) | SketchElement::Image(_) | SketchElement::Symbol(_)
    ) {
        let handle = state.appearance.effective_handle_size();
        for corner in [
            point(bounds.left(), bounds.top()),
            point(bounds.right(), bounds.top()),
            point(bounds.left(), bounds.bottom()),
            point(bounds.right(), bounds.bottom()),
        ] {
            paint.quads.push(fill(
                Bounds::new(
                    point(corner.x - px(handle), corner.y - px(handle)),
                    size(px(handle * 2.0), px(handle * 2.0)),
                ),
                rgba(SKETCH_SELECTION),
            ));
        }
    }
}

fn paint_rect_outline(quads: &mut Vec<PaintQuad>, bounds: Bounds<Pixels>, width: f32, color: u32) {
    let width = px(width.max(1.0));
    quads.push(fill(
        Bounds::new(
            point(bounds.left(), bounds.top()),
            size(bounds.size.width, width),
        ),
        rgba(color),
    ));
    quads.push(fill(
        Bounds::new(
            point(bounds.left(), bounds.bottom() - width),
            size(bounds.size.width, width),
        ),
        rgba(color),
    ));
    quads.push(fill(
        Bounds::new(
            point(bounds.left(), bounds.top()),
            size(width, bounds.size.height),
        ),
        rgba(color),
    ));
    quads.push(fill(
        Bounds::new(
            point(bounds.right() - width, bounds.top()),
            size(width, bounds.size.height),
        ),
        rgba(color),
    ));
}

fn paint_text_box(
    output: &mut Vec<SketchPaintText>,
    text: &str,
    bounds: Bounds<Pixels>,
    font_size: f32,
    color: u32,
    window: &mut Window,
) {
    let text = if text.trim().is_empty() { "Text" } else { text };
    let mut font = window.text_style().font();
    font.family = "Inter".into();
    let run = TextRun {
        len: text.len(),
        font,
        color: rgba(color).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let wrap_width = (bounds.size.width - px(14.0)).max(px(24.0));
    if let Ok(lines) = window.text_system().shape_text(
        SharedString::from(text.to_string()),
        px(font_size),
        &[run],
        Some(wrap_width),
        Some(3),
    ) {
        let line_height = px(font_size * 1.25);
        for (index, line) in lines.into_iter().enumerate() {
            output.push(SketchPaintText {
                line,
                origin: point(
                    bounds.left() + px(7.0),
                    bounds.top() + px(7.0) + line_height * index as f32,
                ),
                line_height,
            });
        }
    }
}

fn stroke_path(
    points: &[SketchPoint],
    width: f32,
    canvas: Bounds<Pixels>,
) -> Option<GpuiPath<Pixels>> {
    let first = points.first()?;
    let mut builder = PathBuilder::stroke(px(width.max(1.0)));
    builder.move_to(local_point(canvas, *first));
    for point in points.iter().skip(1) {
        builder.line_to(local_point(canvas, *point));
    }
    builder.build().ok()
}

fn element_bounds(element: &SketchElement, canvas: Bounds<Pixels>) -> Option<Bounds<Pixels>> {
    match element {
        SketchElement::Stroke(stroke) => {
            let first = stroke.points.first()?;
            let mut min_x = first.x;
            let mut max_x = first.x;
            let mut min_y = first.y;
            let mut max_y = first.y;
            for point in &stroke.points {
                min_x = min_x.min(point.x);
                max_x = max_x.max(point.x);
                min_y = min_y.min(point.y);
                max_y = max_y.max(point.y);
            }
            let pad = stroke.style.stroke_width.max(6.0);
            Some(local_bounds(
                canvas,
                min_x - pad,
                min_y - pad,
                (max_x - min_x) + pad * 2.0,
                (max_y - min_y) + pad * 2.0,
            ))
        }
        SketchElement::Rectangle(rect) => {
            Some(local_bounds(canvas, rect.x, rect.y, rect.w, rect.h))
        }
        SketchElement::Text(text) => Some(local_bounds(canvas, text.x, text.y, text.w, text.h)),
        SketchElement::Image(image) => {
            Some(local_bounds(canvas, image.x, image.y, image.w, image.h))
        }
        SketchElement::Symbol(symbol) => {
            Some(local_bounds(canvas, symbol.x, symbol.y, symbol.w, symbol.h))
        }
    }
}

fn local_bounds(canvas: Bounds<Pixels>, x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
    Bounds::new(
        point(canvas.left() + px(x), canvas.top() + px(y)),
        size(px(w.max(1.0)), px(h.max(1.0))),
    )
}

fn local_point(canvas: Bounds<Pixels>, sketch_point: SketchPoint) -> Point<Pixels> {
    point(
        canvas.left() + px(sketch_point.x),
        canvas.top() + px(sketch_point.y),
    )
}

fn rgba_u32(color: [u8; 4]) -> u32 {
    ((color[0] as u32) << 24)
        | ((color[1] as u32) << 16)
        | ((color[2] as u32) << 8)
        | color[3] as u32
}
