use std::path::{Path, PathBuf};

use gpui::prelude::*;
use gpui::{
    actions, div, fill, img, point, px, relative, rgb, rgba, size, App, Bounds, ContentMask,
    Context, DispatchPhase, Element, ElementId, Entity, ExternalPaths, FocusHandle, Focusable,
    GlobalElementId, KeyBinding, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ObjectFit, PaintQuad, Path as GpuiPath, PathBuilder, Pixels, Point, Render,
    ScrollWheelEvent, SharedString, Style, StyledImage, TextAlign, TextRun, Window, WrappedLine,
};

use crate::path_utils::{path_extension_matches, BACKGROUND_IMAGE_EXTS};
use crate::sketch::{
    canvas_to_sketch_point, default_jpeg_export_file_name, export_frame_size, export_jpeg_to_path,
    normalize_zoom_scale, pad_offset_for_zoom_anchor, save_appearance_settings,
    save_default_document, save_named_sketch, sketch_to_canvas_point, DraftElement, ImageElement,
    RectElement, SketchCanvasBackgroundMode, SketchElement, SketchGridMode, SketchPoint,
    SketchState, SketchTool, SketchToolbarPosition,
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
const SKETCH_EXPORT_BOUNDARY: u32 = 0x6aff9033;
const SKETCH_SCROLL_ZOOM_DIVISOR: f32 = 360.0;

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
    pad_drag: Option<SketchPadDrag>,
    status_message: Option<String>,
}

#[derive(Clone, Copy, Debug)]
struct SketchPadDrag {
    start_local: SketchPoint,
    start_offset: SketchPoint,
}

impl SketchSurface {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            state: SketchState::load_default(),
            workspace_root: None,
            last_bounds: None,
            is_dragging: false,
            pad_drag: None,
            status_message: None,
        }
    }

    pub(crate) fn set_workspace_root(&mut self, workspace_root: Option<PathBuf>) {
        self.workspace_root = workspace_root;
    }

    pub(crate) fn toolbar_position(&self) -> SketchToolbarPosition {
        self.state.appearance.toolbar_position
    }

    pub(crate) fn set_toolbar_position(
        &mut self,
        position: SketchToolbarPosition,
        cx: &mut Context<Self>,
    ) {
        if self.state.appearance.toolbar_position == position {
            return;
        }
        self.state.appearance.toolbar_position = position;
        let _ = save_appearance_settings(&self.state.appearance);
        self.status_message = Some(format!("Toolbar: {}", toolbar_position_label(position)));
        cx.notify();
    }

    fn set_tool(&mut self, tool: SketchTool, cx: &mut Context<Self>) {
        self.state.set_tool(tool);
        self.status_message = Some(match tool {
            SketchTool::Select => "Select and move shapes".into(),
            SketchTool::Grab => "Grab and move the pad".into(),
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

    fn resize_selected_image(&mut self, scale: f32, cx: &mut Context<Self>) {
        if self.state.resize_selected_image_to_scale(scale) {
            self.status_message = Some(format!("Image {:.0}%", scale * 100.0));
            self.persist_if_dirty();
            cx.notify();
        }
    }

    fn zoom_from_scroll(&mut self, position: Point<Pixels>, delta_y: f32, cx: &mut Context<Self>) {
        if self.state.tool != SketchTool::Grab || delta_y.abs() < 0.1 {
            return;
        }
        let Some(anchor_local) = self.canvas_local_point(position) else {
            return;
        };
        let current_zoom = normalize_zoom_scale(self.state.zoom_scale);
        let next_zoom =
            normalize_zoom_scale(current_zoom * (delta_y / SKETCH_SCROLL_ZOOM_DIVISOR).exp());
        if (next_zoom - current_zoom).abs() < 0.001 {
            return;
        }

        self.state.pad_offset = pad_offset_for_zoom_anchor(
            self.state.pad_offset,
            anchor_local,
            current_zoom,
            next_zoom,
        );
        self.state.zoom_scale = next_zoom;
        self.status_message = Some(format!("Zoom {:.0}%", next_zoom * 100.0));
        cx.notify();
    }

    fn default_import_point(&self) -> SketchPoint {
        canvas_to_sketch_point(
            SketchPoint::new(24.0, 24.0),
            self.state.pad_offset,
            self.state.zoom_scale,
        )
    }

    fn import_dropped_images(
        &mut self,
        paths: &ExternalPaths,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let image_paths = sketch_image_drop_paths(paths);
        if image_paths.is_empty() {
            self.status_message = Some("Drop PNG, JPEG, BMP, WEBP, or GIF images".into());
            cx.notify();
            return;
        }

        let base_point = self
            .canvas_point(position)
            .unwrap_or_else(|| self.default_import_point());
        let mut imported = 0usize;
        let mut last_error = None;

        for (index, path) in image_paths.iter().enumerate() {
            let offset = index as f32 * 24.0;
            let point = SketchPoint::new(base_point.x + offset, base_point.y + offset);
            match self.state.add_image_from_path(path, point) {
                Ok(_) => imported += 1,
                Err(err) => {
                    let name = path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("image");
                    last_error = Some(format!("{name}: {err}"));
                }
            }
        }

        if imported > 0 {
            self.state.set_tool(SketchTool::Select);
            self.persist_if_dirty();
            self.status_message = if let Some(error) = last_error {
                Some(format!("Imported {imported} image(s); skipped {error}"))
            } else {
                Some(format!("Imported {imported} image(s)"))
            };
        } else if let Some(error) = last_error {
            self.status_message = Some(format!("Image drop failed: {error}"));
        }

        cx.notify();
    }

    fn export_canvas_size(&self) -> [f32; 2] {
        export_frame_size(self.state.last_canvas_size)
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle(cx));
        let Some(local_point) = self.canvas_local_point(event.position) else {
            return;
        };
        let point =
            canvas_to_sketch_point(local_point, self.state.pad_offset, self.state.zoom_scale);

        self.is_dragging = true;
        match self.state.tool {
            SketchTool::Grab => self.begin_grab_drag(local_point),
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
        let Some(local_point) = self.canvas_local_point(event.position) else {
            return;
        };
        let point =
            canvas_to_sketch_point(local_point, self.state.pad_offset, self.state.zoom_scale);

        let changed = match self.state.tool {
            SketchTool::Grab => self.update_grab_drag(local_point),
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

        if let Some(local_point) = self.canvas_local_point(event.position) {
            let point =
                canvas_to_sketch_point(local_point, self.state.pad_offset, self.state.zoom_scale);
            match self.state.tool {
                SketchTool::Grab => {
                    self.update_grab_drag(local_point);
                }
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
            SketchTool::Grab => self.finish_grab_drag(),
            SketchTool::Marker => self.state.finish_stroke(),
            SketchTool::Rectangle => self.state.finish_rectangle(),
            SketchTool::Select => {
                self.state.finish_resize_selected() || self.state.finish_move_selected()
            }
            SketchTool::Text => false,
        };
        self.is_dragging = false;
        if changed && self.state.tool != SketchTool::Grab {
            self.persist_if_dirty();
        }
        cx.notify();
    }

    fn canvas_point(&self, point: Point<Pixels>) -> Option<SketchPoint> {
        self.canvas_local_point(point).map(|local| {
            canvas_to_sketch_point(local, self.state.pad_offset, self.state.zoom_scale)
        })
    }

    fn canvas_local_point(&self, point: Point<Pixels>) -> Option<SketchPoint> {
        let bounds = self.last_bounds?;
        if !bounds.contains(&point) {
            return None;
        }
        let x = ((point.x - bounds.left()) / px(1.0)).clamp(0.0, bounds.size.width / px(1.0));
        let y = ((point.y - bounds.top()) / px(1.0)).clamp(0.0, bounds.size.height / px(1.0));
        Some(SketchPoint::new(x, y))
    }

    fn begin_grab_drag(&mut self, local_point: SketchPoint) {
        self.pad_drag = Some(SketchPadDrag {
            start_local: local_point,
            start_offset: self.state.pad_offset,
        });
        self.status_message = Some("Moving sketch pad".into());
    }

    fn update_grab_drag(&mut self, local_point: SketchPoint) -> bool {
        let Some(drag) = self.pad_drag else {
            return false;
        };
        let next = SketchPoint::new(
            drag.start_offset.x + local_point.x - drag.start_local.x,
            drag.start_offset.y + local_point.y - drag.start_local.y,
        );
        if (next.x - self.state.pad_offset.x).abs() < 0.1
            && (next.y - self.state.pad_offset.y).abs() < 0.1
        {
            return false;
        }
        self.state.pad_offset = next;
        true
    }

    fn finish_grab_drag(&mut self) -> bool {
        let moved = self.pad_drag.take().is_some();
        if moved {
            self.status_message = Some(format!(
                "Pad offset {:.0}, {:.0}",
                self.state.pad_offset.x, self.state.pad_offset.y
            ));
        }
        moved
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
        self.pad_drag = None;
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
            .child(sketch_body(self, cx))
    }
}

fn sketch_body(surface: &SketchSurface, cx: &mut Context<SketchSurface>) -> gpui::Div {
    let position = surface.state.appearance.toolbar_position;
    let toolbar = sketch_toolbar(
        surface.state.tool,
        surface.state.appearance.grid_mode,
        surface.state.selected_image_scale(),
        position,
        cx,
    );
    let canvas = sketch_canvas(surface, cx);

    match position {
        SketchToolbarPosition::Top => div()
            .flex_1()
            .w_full()
            .flex()
            .flex_col()
            .child(toolbar)
            .child(canvas),
        SketchToolbarPosition::Left => div()
            .flex_1()
            .w_full()
            .flex()
            .overflow_hidden()
            .child(toolbar)
            .child(canvas),
        SketchToolbarPosition::Right => div()
            .flex_1()
            .w_full()
            .flex()
            .overflow_hidden()
            .child(canvas)
            .child(toolbar),
    }
}

fn sketch_canvas(surface: &SketchSurface, cx: &mut Context<SketchSurface>) -> gpui::Div {
    div()
        .relative()
        .flex_1()
        .h_full()
        .w_full()
        .overflow_hidden()
        .border_2()
        .border_color(rgba(0x00000000))
        .bg(rgb(SKETCH_PANEL_BG))
        .can_drop(|drag, _window, _cx| {
            drag.downcast_ref::<ExternalPaths>()
                .is_some_and(external_paths_contain_sketch_images)
        })
        .drag_over::<ExternalPaths>(|style, paths, _window, _cx| {
            if external_paths_contain_sketch_images(paths) {
                style.border_color(rgb(SKETCH_ACCENT))
            } else {
                style.border_color(rgb(0xff7a7a))
            }
        })
        .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
            this.import_dropped_images(paths, window.mouse_position(), cx);
        }))
        .on_mouse_down(MouseButton::Left, cx.listener(SketchSurface::on_mouse_down))
        .on_mouse_move(cx.listener(SketchSurface::on_mouse_move))
        .on_mouse_up(MouseButton::Left, cx.listener(SketchSurface::on_mouse_up))
        .on_mouse_up_out(MouseButton::Left, cx.listener(SketchSurface::on_mouse_up))
        .child(sketch_canvas_layer(
            cx.entity(),
            SketchCanvasLayer::Background,
        ))
        .child(sketch_image_layer(&surface.state))
        .child(sketch_canvas_layer(
            cx.entity(),
            SketchCanvasLayer::Foreground,
        ))
}

fn external_paths_contain_sketch_images(paths: &ExternalPaths) -> bool {
    paths.paths().iter().any(|path| is_sketch_image_path(path))
}

fn sketch_image_drop_paths(paths: &ExternalPaths) -> Vec<PathBuf> {
    paths
        .paths()
        .iter()
        .filter(|path| is_sketch_image_path(path))
        .cloned()
        .collect()
}

fn is_sketch_image_path(path: &Path) -> bool {
    path.is_file() && path_extension_matches(path, BACKGROUND_IMAGE_EXTS)
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
    position: SketchToolbarPosition,
    cx: &mut Context<SketchSurface>,
) -> impl IntoElement {
    let vertical = position != SketchToolbarPosition::Top;
    let mut toolbar = div()
        .id("sketch-toolbar")
        .flex()
        .gap_1()
        .border_color(rgb(SKETCH_BORDER))
        .bg(rgb(SKETCH_BG));

    toolbar = match position {
        SketchToolbarPosition::Top => toolbar
            .h(px(44.0))
            .w_full()
            .items_center()
            .px_3()
            .border_b_1(),
        SketchToolbarPosition::Left => toolbar
            .h_full()
            .w(px(116.0))
            .flex_col()
            .p_2()
            .border_r_1()
            .overflow_y_scroll()
            .scrollbar_width(px(6.0)),
        SketchToolbarPosition::Right => toolbar
            .h_full()
            .w(px(116.0))
            .flex_col()
            .p_2()
            .border_l_1()
            .overflow_y_scroll()
            .scrollbar_width(px(6.0)),
    };

    let mut toolbar = toolbar
        .child(tool_button(
            "Select",
            active_tool == SketchTool::Select,
            vertical,
            cx,
            |this, cx| {
                this.set_tool(SketchTool::Select, cx);
            },
        ))
        .child(tool_button(
            "Grab",
            active_tool == SketchTool::Grab,
            vertical,
            cx,
            |this, cx| {
                this.set_tool(SketchTool::Grab, cx);
            },
        ))
        .child(tool_button(
            "Marker",
            active_tool == SketchTool::Marker,
            vertical,
            cx,
            |this, cx| {
                this.set_tool(SketchTool::Marker, cx);
            },
        ))
        .child(tool_button(
            "Rectangle",
            active_tool == SketchTool::Rectangle,
            vertical,
            cx,
            |this, cx| {
                this.set_tool(SketchTool::Rectangle, cx);
            },
        ))
        .child(toolbar_separator(position))
        .child(tool_button(
            grid_label(grid_mode),
            grid_mode != SketchGridMode::Hidden,
            vertical,
            cx,
            |this, cx| {
                this.cycle_grid(cx);
            },
        ))
        .child(tool_button("Undo", false, vertical, cx, |this, cx| {
            this.undo_from_workspace(cx);
        }))
        .child(tool_button("Redo", false, vertical, cx, |this, cx| {
            this.redo_from_workspace(cx);
        }))
        .child(toolbar_separator(position))
        .child(tool_button("Import", false, vertical, cx, |this, cx| {
            this.import_image(cx);
        }));

    if let Some(scale) = selected_image_scale {
        let smaller = (scale * 0.8).max(0.05);
        let larger = (scale * 1.25).min(2.0);
        toolbar = toolbar
            .child(toolbar_separator(position))
            .child(tool_button(
                "Image -",
                false,
                vertical,
                cx,
                move |this, cx| {
                    this.resize_selected_image(smaller, cx);
                },
            ))
            .child(tool_button(
                "Image 100%",
                (scale - 1.0).abs() < 0.01,
                vertical,
                cx,
                |this, cx| {
                    this.resize_selected_image(1.0, cx);
                },
            ))
            .child(tool_button(
                "Image +",
                false,
                vertical,
                cx,
                move |this, cx| {
                    this.resize_selected_image(larger, cx);
                },
            ));
    }

    toolbar
        .child(div().flex_1())
        .child(tool_button("Clear", false, vertical, cx, |this, cx| {
            this.clear_canvas(cx);
        }))
        .child(tool_button("Save", false, vertical, cx, |this, cx| {
            this.save_from_workspace(cx);
        }))
        .child(tool_button(
            "Export JPG",
            false,
            vertical,
            cx,
            |this, cx| {
                this.export_jpeg(cx);
            },
        ))
}

fn grid_label(grid_mode: SketchGridMode) -> &'static str {
    match grid_mode {
        SketchGridMode::Hidden => "Grid",
        SketchGridMode::Lines => "Lines",
        SketchGridMode::Dots => "Dots",
    }
}

fn toolbar_position_label(position: SketchToolbarPosition) -> &'static str {
    match position {
        SketchToolbarPosition::Top => "Top",
        SketchToolbarPosition::Left => "Left",
        SketchToolbarPosition::Right => "Right",
    }
}

fn toolbar_separator(position: SketchToolbarPosition) -> gpui::Div {
    if position == SketchToolbarPosition::Top {
        div().w(px(1.0)).h(px(24.0)).bg(rgb(SKETCH_BORDER))
    } else {
        div().h(px(1.0)).w_full().bg(rgb(SKETCH_BORDER))
    }
}

fn tool_button(
    label: impl Into<SharedString>,
    active: bool,
    full_width: bool,
    cx: &mut Context<SketchSurface>,
    on_click: impl Fn(&mut SketchSurface, &mut Context<SketchSurface>) + 'static,
) -> gpui::Div {
    let label = label.into();
    let mut button = div()
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
        .child(label);

    if full_width {
        button = button.w_full();
    }

    button
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

fn sketch_canvas_layer(
    surface: Entity<SketchSurface>,
    layer: SketchCanvasLayer,
) -> impl IntoElement {
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .child(SketchCanvasElement { surface, layer })
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

#[derive(Clone, Copy)]
struct SketchCanvasFrame {
    bounds: Bounds<Pixels>,
    pad_offset: SketchPoint,
    zoom_scale: f32,
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

        if matches!(self.layer, SketchCanvasLayer::Foreground) {
            let surface = self.surface.clone();
            window.on_mouse_event(move |event: &ScrollWheelEvent, phase, _window, cx| {
                if phase == DispatchPhase::Bubble && bounds.contains(&event.position) {
                    let delta = event.delta.pixel_delta(px(16.0));
                    surface.update(cx, |surface, cx| {
                        surface.zoom_from_scroll(event.position, delta.y / px(1.0), cx);
                    });
                }
            });
        }
    }
}

fn build_canvas_paint(
    state: &SketchState,
    bounds: Bounds<Pixels>,
    layer: SketchCanvasLayer,
    window: &mut Window,
) -> SketchPrepaintState {
    let frame = SketchCanvasFrame {
        bounds,
        pad_offset: state.pad_offset,
        zoom_scale: normalize_zoom_scale(state.zoom_scale),
    };
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
            paint_grid(&mut paint, frame, state);
            return paint;
        }
        SketchCanvasLayer::Foreground => {}
    }

    for element in &state.document.elements {
        if matches!(element, SketchElement::Image(_)) {
            continue;
        }
        paint_element(&mut paint, element, frame, window);
    }

    if let Some(draft) = &state.draft {
        paint_draft(&mut paint, draft, frame);
    }
    if let Some(rect) = state.draft_rectangle() {
        paint_rect_element(&mut paint, &rect, frame);
    }

    if let Some(index) = state.selected {
        if let Some(element) = state.document.elements.get(index) {
            paint_selection(&mut paint, element, frame, state);
        }
    }

    paint_export_boundary(&mut paint, frame, state);

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
            |layer, image| {
                layer.child(sketch_image_element(
                    image,
                    state.pad_offset,
                    state.zoom_scale,
                ))
            },
        )
}

fn sketch_image_element(
    image: ImageElement,
    pad_offset: SketchPoint,
    zoom_scale: f32,
) -> impl IntoElement {
    let zoom_scale = normalize_zoom_scale(zoom_scale);
    let origin = sketch_to_canvas_point(SketchPoint::new(image.x, image.y), pad_offset, zoom_scale);
    div()
        .absolute()
        .left(px(origin.x))
        .top(px(origin.y))
        .w(px((image.w.max(1.0) * zoom_scale).max(1.0)))
        .h(px((image.h.max(1.0) * zoom_scale).max(1.0)))
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

fn paint_grid(paint: &mut SketchPrepaintState, frame: SketchCanvasFrame, state: &SketchState) {
    if !state.appearance.grid_visible() {
        return;
    }
    let spacing = state.appearance.effective_grid_spacing().max(4.0);
    let alpha = (state.appearance.effective_grid_opacity() * 255.0).round() as u8;
    let color = rgba_u32([130, 140, 160, alpha]);
    let bounds = frame.bounds;
    let width = bounds.size.width / px(1.0);
    let height = bounds.size.height / px(1.0);
    let spacing = spacing * frame.zoom_scale;
    let start_x = positive_mod(frame.pad_offset.x, spacing);
    let start_y = positive_mod(frame.pad_offset.y, spacing);

    match state.appearance.grid_mode {
        SketchGridMode::Hidden => {}
        SketchGridMode::Lines => {
            let mut x = start_x;
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
            let mut y = start_y;
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
            let mut y = start_y;
            while y <= height {
                let mut x = start_x;
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

fn positive_mod(value: f32, modulus: f32) -> f32 {
    ((value % modulus) + modulus) % modulus
}

fn paint_element(
    paint: &mut SketchPrepaintState,
    element: &SketchElement,
    frame: SketchCanvasFrame,
    window: &mut Window,
) {
    match element {
        SketchElement::Stroke(stroke) => {
            if let Some(path) = stroke_path(&stroke.points, stroke.style.stroke_width, frame) {
                paint
                    .paths
                    .push((path, rgba_u32(stroke.style.stroke_color)));
            }
        }
        SketchElement::Rectangle(rect) => paint_rect_element(paint, rect, frame),
        SketchElement::Text(text) => {
            let text_bounds = local_bounds(frame, text.x, text.y, text.w, text.h);
            paint.quads.push(fill(text_bounds, rgba(0x10131a70)));
            paint_rect_outline(
                &mut paint.quads,
                text_bounds,
                frame.zoom_scale.max(1.0),
                rgba_u32(text.style.stroke_color),
            );
            paint_text_box(
                &mut paint.text,
                &text.text,
                text_bounds,
                text.style.font_size * frame.zoom_scale,
                rgba_u32(text.style.stroke_color),
                window,
            );
        }
        SketchElement::Image(image) => {
            let image_bounds = local_bounds(frame, image.x, image.y, image.w, image.h);
            paint.quads.push(fill(image_bounds, rgba(0x111722cc)));
            paint_rect_outline(
                &mut paint.quads,
                image_bounds,
                frame.zoom_scale.max(1.0),
                0x475569ff,
            );
            let label = Path::new(&image.path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Image");
            paint_text_box(
                &mut paint.text,
                label,
                image_bounds,
                12.0 * frame.zoom_scale,
                0xcbd5e1ff,
                window,
            );
        }
        SketchElement::Symbol(symbol) => {
            let symbol_bounds = local_bounds(frame, symbol.x, symbol.y, symbol.w, symbol.h);
            paint.quads.push(fill(symbol_bounds, rgba(0x132018cc)));
            paint_rect_outline(
                &mut paint.quads,
                symbol_bounds,
                (2.0 * frame.zoom_scale).max(1.0),
                rgba_u32(symbol.style.stroke_color),
            );
            paint_text_box(
                &mut paint.text,
                symbol.kind.label(),
                symbol_bounds,
                13.0 * frame.zoom_scale,
                rgba_u32(symbol.style.stroke_color),
                window,
            );
        }
    }
}

fn paint_rect_element(
    paint: &mut SketchPrepaintState,
    rect: &RectElement,
    frame: SketchCanvasFrame,
) {
    let rect_bounds = local_bounds(frame, rect.x, rect.y, rect.w, rect.h);
    if let Some(fill_color) = rect.style.fill_color {
        paint
            .quads
            .push(fill(rect_bounds, rgba(rgba_u32(fill_color))));
    }
    paint_rect_outline(
        &mut paint.quads,
        rect_bounds,
        (rect.style.stroke_width.max(1.0) * frame.zoom_scale).max(1.0),
        rgba_u32(rect.style.stroke_color),
    );
}

fn paint_draft(paint: &mut SketchPrepaintState, draft: &DraftElement, frame: SketchCanvasFrame) {
    match draft {
        DraftElement::Stroke(stroke) => {
            if let Some(path) = stroke_path(&stroke.points, stroke.style.stroke_width, frame) {
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
    frame: SketchCanvasFrame,
    state: &SketchState,
) {
    let Some(bounds) = element_bounds(element, frame) else {
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

fn paint_export_boundary(
    paint: &mut SketchPrepaintState,
    frame: SketchCanvasFrame,
    state: &SketchState,
) {
    let [width, height] = export_frame_size(state.last_canvas_size);
    let bounds = local_bounds(frame, 0.0, 0.0, width, height);
    paint_dashed_rect_outline(
        &mut paint.quads,
        bounds,
        2.0,
        14.0,
        8.0,
        SKETCH_EXPORT_BOUNDARY,
    );
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

fn paint_dashed_rect_outline(
    quads: &mut Vec<PaintQuad>,
    bounds: Bounds<Pixels>,
    width: f32,
    dash: f32,
    gap: f32,
    color: u32,
) {
    let width = width.max(1.0);
    let dash = dash.max(1.0);
    let gap = gap.max(1.0);
    let rect_w = bounds.size.width / px(1.0);
    let rect_h = bounds.size.height / px(1.0);

    push_dashed_horizontal(quads, bounds, 0.0, rect_w, width, dash, gap, color);
    push_dashed_horizontal(
        quads,
        bounds,
        rect_h - width,
        rect_w,
        width,
        dash,
        gap,
        color,
    );
    push_dashed_vertical(quads, bounds, 0.0, rect_h, width, dash, gap, color);
    push_dashed_vertical(
        quads,
        bounds,
        rect_w - width,
        rect_h,
        width,
        dash,
        gap,
        color,
    );
}

fn push_dashed_horizontal(
    quads: &mut Vec<PaintQuad>,
    bounds: Bounds<Pixels>,
    y: f32,
    total_w: f32,
    width: f32,
    dash: f32,
    gap: f32,
    color: u32,
) {
    let mut x = 0.0;
    while x < total_w {
        let segment_w = dash.min(total_w - x).max(0.0);
        if segment_w > 0.0 {
            quads.push(fill(
                Bounds::new(
                    point(bounds.left() + px(x), bounds.top() + px(y.max(0.0))),
                    size(px(segment_w), px(width)),
                ),
                rgba(color),
            ));
        }
        x += dash + gap;
    }
}

fn push_dashed_vertical(
    quads: &mut Vec<PaintQuad>,
    bounds: Bounds<Pixels>,
    x: f32,
    total_h: f32,
    width: f32,
    dash: f32,
    gap: f32,
    color: u32,
) {
    let mut y = 0.0;
    while y < total_h {
        let segment_h = dash.min(total_h - y).max(0.0);
        if segment_h > 0.0 {
            quads.push(fill(
                Bounds::new(
                    point(bounds.left() + px(x.max(0.0)), bounds.top() + px(y)),
                    size(px(width), px(segment_h)),
                ),
                rgba(color),
            ));
        }
        y += dash + gap;
    }
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
    frame: SketchCanvasFrame,
) -> Option<GpuiPath<Pixels>> {
    let first = points.first()?;
    let mut builder = PathBuilder::stroke(px((width.max(1.0) * frame.zoom_scale).max(0.5)));
    builder.move_to(local_point(frame, *first));
    for point in points.iter().skip(1) {
        builder.line_to(local_point(frame, *point));
    }
    builder.build().ok()
}

fn element_bounds(element: &SketchElement, frame: SketchCanvasFrame) -> Option<Bounds<Pixels>> {
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
                frame,
                min_x - pad,
                min_y - pad,
                (max_x - min_x) + pad * 2.0,
                (max_y - min_y) + pad * 2.0,
            ))
        }
        SketchElement::Rectangle(rect) => Some(local_bounds(frame, rect.x, rect.y, rect.w, rect.h)),
        SketchElement::Text(text) => Some(local_bounds(frame, text.x, text.y, text.w, text.h)),
        SketchElement::Image(image) => {
            Some(local_bounds(frame, image.x, image.y, image.w, image.h))
        }
        SketchElement::Symbol(symbol) => {
            Some(local_bounds(frame, symbol.x, symbol.y, symbol.w, symbol.h))
        }
    }
}

fn local_bounds(frame: SketchCanvasFrame, x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
    let origin = sketch_to_canvas_point(SketchPoint::new(x, y), frame.pad_offset, frame.zoom_scale);
    Bounds::new(
        point(
            frame.bounds.left() + px(origin.x),
            frame.bounds.top() + px(origin.y),
        ),
        size(
            px((w.max(1.0) * frame.zoom_scale).max(1.0)),
            px((h.max(1.0) * frame.zoom_scale).max(1.0)),
        ),
    )
}

fn local_point(frame: SketchCanvasFrame, sketch_point: SketchPoint) -> Point<Pixels> {
    let local = sketch_to_canvas_point(sketch_point, frame.pad_offset, frame.zoom_scale);
    point(
        frame.bounds.left() + px(local.x),
        frame.bounds.top() + px(local.y),
    )
}

fn rgba_u32(color: [u8; 4]) -> u32 {
    ((color[0] as u32) << 24)
        | ((color[1] as u32) << 16)
        | ((color[2] as u32) << 8)
        | color[3] as u32
}
