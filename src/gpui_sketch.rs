use std::path::{Path, PathBuf};

use gpui::{
    actions, px, App, Bounds, Context, ExternalPaths, FocusHandle, Focusable, KeyBinding,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, Window,
};

use crate::path_utils::{path_extension_matches, BACKGROUND_IMAGE_EXTS};
use crate::sketch::{
    canvas_to_sketch_point, default_jpeg_export_file_name, export_frame_size, export_jpeg_to_path,
    normalize_zoom_scale, pad_offset_for_zoom_anchor, save_appearance_settings,
    save_default_document, save_named_sketch, SketchGridMode, SketchPoint, SketchState, SketchTool,
    SketchToolbarPosition,
};

mod canvas_element;
mod paint;
mod render;

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

#[derive(Clone, Copy, Debug)]
struct SketchPalette {
    bg: u32,
    panel_bg: u32,
    canvas_bg: u32,
    header_bg: u32,
    border: u32,
    text: u32,
    muted: u32,
    accent: u32,
    active_bg: u32,
    button_bg: u32,
}

impl SketchPalette {
    fn for_light_mode(light_mode: bool) -> Self {
        if light_mode {
            Self {
                bg: 0xf0e3d1,
                panel_bg: 0xfffbf2,
                canvas_bg: 0xfaf2e2,
                header_bg: 0xf0e3d1,
                border: 0xd8c6ad,
                text: 0x3e372f,
                muted: 0x7d7064,
                accent: 0x5f9f79,
                active_bg: 0xe2d0ed,
                button_bg: 0xeadcc8,
            }
        } else {
            Self {
                bg: SKETCH_BG,
                panel_bg: SKETCH_PANEL_BG,
                canvas_bg: SKETCH_CANVAS_BG,
                header_bg: 0x121217,
                border: SKETCH_BORDER,
                text: SKETCH_TEXT,
                muted: SKETCH_MUTED,
                accent: SKETCH_ACCENT,
                active_bg: SKETCH_ACTIVE_BG,
                button_bg: SKETCH_BUTTON_BG,
            }
        }
    }
}

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
    light_mode: bool,
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
            light_mode: false,
        }
    }

    #[cfg(feature = "gpui-workspace")]
    pub(crate) fn set_light_mode(&mut self, light_mode: bool, cx: &mut Context<Self>) {
        if self.light_mode != light_mode {
            self.light_mode = light_mode;
            cx.notify();
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

    fn toggle_grid(&mut self, cx: &mut Context<Self>) {
        self.state.appearance.grid_mode = self.state.appearance.grid_mode.toggled_visibility();
        let _ = save_appearance_settings(&self.state.appearance);
        self.status_message = Some(match self.state.appearance.grid_mode {
            SketchGridMode::Hidden => "Grid off".into(),
            SketchGridMode::Lines | SketchGridMode::Dots => "Grid on".into(),
        });
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

pub(super) fn external_paths_contain_sketch_images(paths: &ExternalPaths) -> bool {
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

fn toolbar_position_label(position: SketchToolbarPosition) -> &'static str {
    match position {
        SketchToolbarPosition::Top => "Top",
        SketchToolbarPosition::Left => "Left",
        SketchToolbarPosition::Right => "Right",
    }
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
