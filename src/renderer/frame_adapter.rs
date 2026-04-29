use crate::config::Config;
use crate::engine::{
    Color, EffectPass, EffectStack, EguiLayer, EngineFrame, Layer, LayerKind, Primitive, Rect,
    Size, TextRun,
};

use super::RenderRequest;

pub(super) fn engine_frame_from_request(
    request: &RenderRequest<'_>,
    config: &Config,
    viewport: Size,
    use_scene: bool,
    line_height: f32,
) -> EngineFrame {
    let mut frame = EngineFrame::new(viewport);
    frame.clear_color = color_from_rgba(config.bg());

    if use_scene {
        let mut scene_effects = EffectStack::default();
        if config.effects.bloom_enabled {
            scene_effects.passes.push(EffectPass::Bloom {
                intensity: config.effects.bloom_intensity,
            });
        }
        if config.effects.crt_enabled {
            scene_effects.passes.push(EffectPass::Crt {
                curvature: config.effects.curvature,
                scanline_strength: config.effects.scanline_intensity,
            });
        }
        if !scene_effects.passes.is_empty() {
            let mut layer = Layer::new("scene-effects", 900, LayerKind::Primitives(Vec::new()));
            layer.style.effects = scene_effects;
            frame.push_layer(layer);
        }
    }

    if request.terminal.is_some() {
        frame.push_layer(Layer::new(
            "terminal-content",
            100,
            LayerKind::CustomGpu(crate::engine::CustomGpuLayer {
                pass: crate::engine::CustomPassId::new("terminal-grid"),
                bounds: rect_from_layout_content(request),
            }),
        ));
    }

    if let Some(session) = request.terminal {
        let content_x = request.screen_layout.content.x;
        let content_y = request.screen_layout.content.y;
        let cell_w = request.screen_layout.cell_w;
        let cell_h = request.screen_layout.cell_h;
        let terminal = &session.terminal;

        let backgrounds: Vec<Primitive> = terminal
            .background_rects(config, cell_w, cell_h)
            .into_iter()
            .map(|(x, y, width, height, color)| Primitive::Rect {
                rect: Rect::new(x + content_x, y + content_y, width, height),
                color: color_from_rgba(color),
            })
            .collect();
        if !backgrounds.is_empty() {
            frame.push_layer(Layer::new(
                "terminal-cell-backgrounds",
                90,
                LayerKind::Primitives(backgrounds),
            ));
        }

        let mut decorations: Vec<Primitive> = terminal
            .decoration_rects(config, cell_w, cell_h)
            .into_iter()
            .map(|(x, y, width, height, color)| Primitive::Rect {
                rect: Rect::new(x + content_x, y + content_y, width, height),
                color: color_from_rgba(color),
            })
            .collect();
        decorations.extend(
            terminal
                .url_decoration_rects(cell_w, cell_h)
                .into_iter()
                .map(|(x, y, width, height, color)| Primitive::Rect {
                    rect: Rect::new(x + content_x, y + content_y, width, height),
                    color: color_from_rgba(color),
                }),
        );
        if !decorations.is_empty() {
            frame.push_layer(Layer::new(
                "terminal-decorations",
                105,
                LayerKind::Primitives(decorations),
            ));
        }
    }

    let mut highlight_primitives = Vec::new();
    highlight_primitives.extend(rect_primitives(
        request.search_rects,
        request.screen_layout.content.x,
        request.screen_layout.content.y,
    ));
    highlight_primitives.extend(rect_primitives(
        request.selection_rects,
        request.screen_layout.content.x,
        request.screen_layout.content.y,
    ));
    if !highlight_primitives.is_empty() {
        frame.push_layer(Layer::new(
            "terminal-highlights",
            110,
            LayerKind::Primitives(highlight_primitives),
        ));
    }

    if request.visual_bell {
        frame.push_layer(Layer::new(
            "visual-bell",
            800,
            LayerKind::Primitives(vec![Primitive::Rect {
                rect: Rect::new(0.0, 0.0, viewport.width, viewport.height),
                color: Color::rgba(1.0, 1.0, 1.0, 0.15),
            }]),
        ));
    }

    if let Some((query, status)) = request.search_bar {
        let bar_h = 28.0;
        let bar_y = viewport.height - bar_h;
        frame.push_layer(Layer::new(
            "search-bar-bg",
            850,
            LayerKind::Primitives(vec![Primitive::Rect {
                rect: Rect::new(0.0, bar_y, viewport.width, bar_h),
                color: Color::rgba(0.15, 0.15, 0.18, 0.95),
            }]),
        ));
        frame.push_layer(Layer::new(
            "search-bar-text",
            851,
            LayerKind::Text(vec![TextRun {
                text: format!("Find: {query}  {status}"),
                origin: [8.0, bar_y],
                size: config.font_size,
                color: Color::rgba(0.94, 0.94, 0.96, 1.0),
                font_family: config.font_family.clone(),
                monospace: true,
            }]),
        ));
    }

    if let Some((panel, log)) = request.error_panel {
        if panel.visible {
            let panel_h = (viewport.height * 0.4).max(line_height * 5.0);
            let panel_y = viewport.height - panel_h;
            let (bg_rects, lines) = panel.render_data(log, viewport.width, panel_h, line_height);
            let primitives: Vec<Primitive> = bg_rects
                .into_iter()
                .map(|(x, y, width, height, color)| Primitive::Rect {
                    rect: Rect::new(x, y + panel_y, width, height),
                    color: color_from_rgba(color),
                })
                .collect();
            if !primitives.is_empty() {
                frame.push_layer(Layer::new(
                    "error-panel-bg",
                    860,
                    LayerKind::Primitives(primitives),
                ));
            }

            let runs: Vec<TextRun> = lines
                .into_iter()
                .enumerate()
                .map(|(i, (line, color))| TextRun {
                    text: line,
                    origin: [0.0, panel_y + i as f32 * line_height],
                    size: config.font_size * 0.75,
                    color: color_from_rgb_u8(color),
                    font_family: config.font_family.clone(),
                    monospace: true,
                })
                .collect();
            if !runs.is_empty() {
                frame.push_layer(Layer::new("error-panel-text", 861, LayerKind::Text(runs)));
            }
        }
    }

    if request.egui_render.is_some() {
        frame.push_layer(Layer::new(
            "egui",
            1_000,
            LayerKind::Egui(EguiLayer {
                bounds: Some(Rect::new(0.0, 0.0, viewport.width, viewport.height)),
            }),
        ));
    }

    frame
}

fn rect_from_layout_content(request: &RenderRequest<'_>) -> Rect {
    Rect::new(
        request.screen_layout.content.x,
        request.screen_layout.content.y,
        request.screen_layout.content.w,
        request.screen_layout.content.h,
    )
}

fn rect_primitives(
    rects: &[(f32, f32, f32, f32, [f32; 4])],
    offset_x: f32,
    offset_y: f32,
) -> impl Iterator<Item = Primitive> + '_ {
    rects
        .iter()
        .map(move |&(x, y, width, height, color)| Primitive::Rect {
            rect: Rect::new(x + offset_x, y + offset_y, width, height),
            color: color_from_rgba(color),
        })
}

fn color_from_rgba(color: [f32; 4]) -> Color {
    Color::rgba(color[0], color[1], color[2], color[3])
}

fn color_from_rgb_u8(color: [u8; 3]) -> Color {
    Color::rgba(
        color[0] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[2] as f32 / 255.0,
        1.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{ScreenLayout, Zone};

    fn layout() -> ScreenLayout {
        ScreenLayout {
            window_w: 800.0,
            window_h: 600.0,
            sidebar_w: 0.0,
            tab_bar: Zone {
                x: 0.0,
                y: 0.0,
                w: 800.0,
                h: 28.0,
            },
            content: Zone {
                x: 10.0,
                y: 40.0,
                w: 780.0,
                h: 520.0,
            },
            cell_w: 8.0,
            cell_h: 16.0,
            grid_cols: 97,
            grid_rows: 32,
            show_tab_bar: true,
        }
    }

    #[test]
    fn adapter_builds_valid_frame_without_terminal() {
        let layout = layout();
        let request = RenderRequest {
            terminal: None,
            tab_id: 1,
            tab_titles: &[],
            selection_rects: &[],
            search_rects: &[],
            search_bar: None,
            error_panel: None,
            visual_bell: false,
            screen_layout: &layout,
            egui_render: None,
            apply_effects_to_ui: false,
            effects_mask: None,
        };

        let frame = engine_frame_from_request(
            &request,
            &Config::default(),
            Size::new(800.0, 600.0),
            false,
            16.0,
        );

        assert!(frame.validate().is_ok());
        assert!(frame.layers.is_empty());
    }

    #[test]
    fn adapter_offsets_terminal_highlight_rects_into_content_area() {
        let layout = layout();
        let search_rects = [(2.0, 3.0, 16.0, 16.0, [1.0, 0.0, 0.0, 0.5])];
        let request = RenderRequest {
            terminal: None,
            tab_id: 1,
            tab_titles: &[],
            selection_rects: &[],
            search_rects: &search_rects,
            search_bar: None,
            error_panel: None,
            visual_bell: false,
            screen_layout: &layout,
            egui_render: None,
            apply_effects_to_ui: false,
            effects_mask: None,
        };

        let frame = engine_frame_from_request(
            &request,
            &Config::default(),
            Size::new(800.0, 600.0),
            false,
            16.0,
        );

        let Some(Layer {
            kind: LayerKind::Primitives(primitives),
            ..
        }) = frame
            .layers
            .iter()
            .find(|layer| layer.id.as_str() == "terminal-highlights")
        else {
            panic!("missing terminal-highlights layer");
        };
        assert_eq!(
            primitives[0],
            Primitive::Rect {
                rect: Rect::new(12.0, 43.0, 16.0, 16.0),
                color: Color::rgba(1.0, 0.0, 0.0, 0.5),
            }
        );
    }

    #[test]
    fn adapter_adds_effect_layer_when_scene_effects_are_active() {
        let layout = layout();
        let mut config = Config::default();
        config.effects.bloom_enabled = true;
        config.effects.bloom_intensity = 0.8;
        let request = RenderRequest {
            terminal: None,
            tab_id: 1,
            tab_titles: &[],
            selection_rects: &[],
            search_rects: &[],
            search_bar: None,
            error_panel: None,
            visual_bell: false,
            screen_layout: &layout,
            egui_render: None,
            apply_effects_to_ui: false,
            effects_mask: None,
        };

        let frame =
            engine_frame_from_request(&request, &config, Size::new(800.0, 600.0), true, 16.0);

        assert!(frame
            .layers
            .iter()
            .any(|layer| layer.id.as_str() == "scene-effects"));
    }

    #[test]
    fn adapter_represents_search_bar_as_primitive_and_text_layers() {
        let layout = layout();
        let request = RenderRequest {
            terminal: None,
            tab_id: 1,
            tab_titles: &[],
            selection_rects: &[],
            search_rects: &[],
            search_bar: Some(("needle", "1/3")),
            error_panel: None,
            visual_bell: false,
            screen_layout: &layout,
            egui_render: None,
            apply_effects_to_ui: false,
            effects_mask: None,
        };

        let frame = engine_frame_from_request(
            &request,
            &Config::default(),
            Size::new(800.0, 600.0),
            false,
            16.0,
        );

        assert!(frame
            .layers
            .iter()
            .any(|layer| layer.id.as_str() == "search-bar-bg"));
        let Some(Layer {
            kind: LayerKind::Text(runs),
            ..
        }) = frame
            .layers
            .iter()
            .find(|layer| layer.id.as_str() == "search-bar-text")
        else {
            panic!("missing search-bar-text layer");
        };
        assert_eq!(runs[0].text, "Find: needle  1/3");
    }

    #[test]
    fn adapter_represents_visible_error_panel_as_primitive_and_text_layers() {
        let layout = layout();
        let log = crate::error_log::ErrorLog::new();
        log.error("boom");
        let mut panel = crate::error_log::ErrorPanel::new();
        panel.visible = true;
        let request = RenderRequest {
            terminal: None,
            tab_id: 1,
            tab_titles: &[],
            selection_rects: &[],
            search_rects: &[],
            search_bar: None,
            error_panel: Some((&panel, &log)),
            visual_bell: false,
            screen_layout: &layout,
            egui_render: None,
            apply_effects_to_ui: false,
            effects_mask: None,
        };

        let frame = engine_frame_from_request(
            &request,
            &Config::default(),
            Size::new(800.0, 600.0),
            false,
            16.0,
        );

        assert!(frame
            .layers
            .iter()
            .any(|layer| layer.id.as_str() == "error-panel-bg"));
        let Some(Layer {
            kind: LayerKind::Text(runs),
            ..
        }) = frame
            .layers
            .iter()
            .find(|layer| layer.id.as_str() == "error-panel-text")
        else {
            panic!("missing error-panel-text layer");
        };
        assert!(runs.iter().any(|run| run.text.contains("boom")));
    }
}
