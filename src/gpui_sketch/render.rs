use gpui::prelude::*;
use gpui::{div, px, rgb, rgba, Context, MouseButton, MouseDownEvent, SharedString, Window};

use crate::sketch::{SketchGridMode, SketchTool, SketchToolbarPosition};

use super::canvas_element::{sketch_canvas_layer, sketch_image_layer, SketchCanvasLayer};
use super::{external_paths_contain_sketch_images, SketchPalette, SketchSurface};

use gpui::ExternalPaths;
use gpui::Focusable;
use gpui::Render;

impl Render for SketchSurface {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = SketchPalette::for_light_mode(self.light_mode);
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(palette.bg))
            .text_color(rgb(palette.text))
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
    let palette = SketchPalette::for_light_mode(surface.light_mode);
    let toolbar = sketch_toolbar(
        surface.state.tool,
        surface.state.appearance.grid_mode,
        surface.state.selected_image_scale(),
        position,
        palette,
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
    let palette = SketchPalette::for_light_mode(surface.light_mode);
    div()
        .relative()
        .flex_1()
        .h_full()
        .w_full()
        .overflow_hidden()
        .border_2()
        .border_color(rgba(0x00000000))
        .bg(rgb(palette.panel_bg))
        .can_drop(|drag, _window, _cx| {
            drag.downcast_ref::<ExternalPaths>()
                .is_some_and(external_paths_contain_sketch_images)
        })
        .drag_over::<ExternalPaths>(move |style, paths, _window, _cx| {
            if external_paths_contain_sketch_images(paths) {
                style.border_color(rgb(palette.accent))
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

fn sketch_header(surface: &SketchSurface) -> impl IntoElement {
    let palette = SketchPalette::for_light_mode(surface.light_mode);
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
        .border_color(rgb(palette.border))
        .bg(rgb(palette.header_bg))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_size(px(13.0)).child(title))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(palette.muted))
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
                .text_color(rgb(palette.accent))
                .child(status),
        )
}

fn sketch_toolbar(
    active_tool: SketchTool,
    grid_mode: SketchGridMode,
    selected_image_scale: Option<f32>,
    position: SketchToolbarPosition,
    palette: SketchPalette,
    cx: &mut Context<SketchSurface>,
) -> impl IntoElement {
    let vertical = position != SketchToolbarPosition::Top;
    let mut toolbar = div()
        .id("sketch-toolbar")
        .flex()
        .gap_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.bg));

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
            palette,
            cx,
            |this, cx| {
                this.set_tool(SketchTool::Select, cx);
            },
        ))
        .child(tool_button(
            "Grab",
            active_tool == SketchTool::Grab,
            vertical,
            palette,
            cx,
            |this, cx| {
                this.set_tool(SketchTool::Grab, cx);
            },
        ))
        .child(tool_button(
            "Marker",
            active_tool == SketchTool::Marker,
            vertical,
            palette,
            cx,
            |this, cx| {
                this.set_tool(SketchTool::Marker, cx);
            },
        ))
        .child(tool_button(
            "Rectangle",
            active_tool == SketchTool::Rectangle,
            vertical,
            palette,
            cx,
            |this, cx| {
                this.set_tool(SketchTool::Rectangle, cx);
            },
        ))
        .child(toolbar_separator(position, palette))
        .child(tool_button(
            grid_toggle_label(grid_mode),
            grid_mode != SketchGridMode::Hidden,
            vertical,
            palette,
            cx,
            |this, cx| {
                this.toggle_grid(cx);
            },
        ))
        .child(tool_button(
            "Undo",
            false,
            vertical,
            palette,
            cx,
            |this, cx| {
                this.undo_from_workspace(cx);
            },
        ))
        .child(tool_button(
            "Redo",
            false,
            vertical,
            palette,
            cx,
            |this, cx| {
                this.redo_from_workspace(cx);
            },
        ))
        .child(toolbar_separator(position, palette))
        .child(tool_button(
            "Import",
            false,
            vertical,
            palette,
            cx,
            |this, cx| {
                this.import_image(cx);
            },
        ));

    if let Some(scale) = selected_image_scale {
        let smaller = (scale * 0.8).max(0.05);
        let larger = (scale * 1.25).min(2.0);
        toolbar = toolbar
            .child(toolbar_separator(position, palette))
            .child(tool_button(
                "Image -",
                false,
                vertical,
                palette,
                cx,
                move |this, cx| {
                    this.resize_selected_image(smaller, cx);
                },
            ))
            .child(tool_button(
                "Image 100%",
                (scale - 1.0).abs() < 0.01,
                vertical,
                palette,
                cx,
                |this, cx| {
                    this.resize_selected_image(1.0, cx);
                },
            ))
            .child(tool_button(
                "Image +",
                false,
                vertical,
                palette,
                cx,
                move |this, cx| {
                    this.resize_selected_image(larger, cx);
                },
            ));
    }

    toolbar
        .child(div().flex_1())
        .child(tool_button(
            "Clear",
            false,
            vertical,
            palette,
            cx,
            |this, cx| {
                this.clear_canvas(cx);
            },
        ))
        .child(tool_button(
            "Save",
            false,
            vertical,
            palette,
            cx,
            |this, cx| {
                this.save_from_workspace(cx);
            },
        ))
        .child(tool_button(
            "Export JPG",
            false,
            vertical,
            palette,
            cx,
            |this, cx| {
                this.export_jpeg(cx);
            },
        ))
}

fn grid_toggle_label(grid_mode: SketchGridMode) -> &'static str {
    match grid_mode {
        SketchGridMode::Hidden => "Grid Off",
        SketchGridMode::Lines | SketchGridMode::Dots => "Grid On",
    }
}

fn toolbar_separator(position: SketchToolbarPosition, palette: SketchPalette) -> gpui::Div {
    if position == SketchToolbarPosition::Top {
        div().w(px(1.0)).h(px(24.0)).bg(rgb(palette.border))
    } else {
        div().h(px(1.0)).w_full().bg(rgb(palette.border))
    }
}

fn tool_button(
    label: impl Into<SharedString>,
    active: bool,
    full_width: bool,
    palette: SketchPalette,
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
        .border_color(rgb(if active {
            palette.accent
        } else {
            palette.border
        }))
        .bg(rgb(if active {
            palette.active_bg
        } else {
            palette.button_bg
        }))
        .text_size(px(12.0))
        .text_color(rgb(if active { palette.accent } else { palette.text }))
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
