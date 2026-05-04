use super::types::ActiveView;
use crate::workspace::TabKind;

const FOOTER_TEXT_SIZE: f32 = 14.0;
const FOOTER_BUTTON_HEIGHT: f32 = 36.0;
const QUEUE_CHIP_GREEN: egui::Color32 = egui::Color32::from_rgb(106, 255, 144);

pub struct FooterQueuePrompt {
    pub letter: char,
    pub preview: String,
    pub clipboard_text: String,
}

/// Action returned by the footer when a button is clicked.
pub enum FooterAction {
    /// Show an overlay view.
    ShowOverlay(ActiveView),
    /// Open or focus a singleton tab (Stacker, Sketch).
    OpenSingletonTab(TabKind),
    /// Create a new terminal tab.
    NewTerminalTab,
    /// Copy a queued prompt into the clipboard.
    CopyQueuedPrompt(String),
}

/// Render the footer navigation bar. Returns the action if a button was clicked.
pub fn render_footer(
    ctx: &egui::Context,
    footer_height: f32,
    _active_singleton: Option<TabKind>,
    active_tab_kind: Option<TabKind>,
    chrome_bg: egui::Color32,
    active_btn: egui::Color32,
    text_color: egui::Color32,
    queued_prompts: &[FooterQueuePrompt],
) -> Option<FooterAction> {
    let mut result: Option<FooterAction> = None;

    egui::TopBottomPanel::bottom("footer")
        .exact_height(footer_height)
        .frame(
            egui::Frame::none()
                .fill(chrome_bg)
                .inner_margin(egui::Margin::symmetric(10.0, 6.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                // Home button opens/focuses the Home singleton tab.
                let home_active = active_tab_kind == Some(TabKind::Home);
                render_button(ui, "Home", home_active, active_btn, text_color, || {
                    result = Some(FooterAction::OpenSingletonTab(TabKind::Home));
                });

                // Terminal button creates a new terminal tab.
                let terminal_active = matches!(active_tab_kind, Some(TabKind::Terminal));
                render_button(
                    ui,
                    "Terminal",
                    terminal_active,
                    active_btn,
                    text_color,
                    || {
                        result = Some(FooterAction::NewTerminalTab);
                    },
                );

                // All singleton tab buttons
                let singletons: &[(&str, TabKind)] = &[
                    ("Stacker", TabKind::Stacker),
                    ("Sketch", TabKind::Sketch),
                    ("Git", TabKind::Git),
                    ("Appearances", TabKind::Appearances),
                    ("Settings", TabKind::Settings),
                ];
                for &(name, kind) in singletons {
                    let is_active = active_tab_kind == Some(kind);
                    render_button(ui, name, is_active, active_btn, text_color, || {
                        result = Some(FooterAction::OpenSingletonTab(kind));
                    });
                }

                if matches!(active_tab_kind, Some(TabKind::Terminal)) && !queued_prompts.is_empty()
                {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        for prompt in queued_prompts.iter().rev() {
                            if render_queue_chip(ui, prompt).clicked() {
                                result = Some(FooterAction::CopyQueuedPrompt(
                                    prompt.clipboard_text.clone(),
                                ));
                            }
                        }
                    });
                }
            });
        });

    result
}

fn render_queue_chip(ui: &mut egui::Ui, prompt: &FooterQueuePrompt) -> egui::Response {
    let label = format!("{}: {}", prompt.letter, prompt.preview);
    ui.add(
        egui::Button::new(
            egui::RichText::new(label)
                .size(12.0)
                .strong()
                .color(QUEUE_CHIP_GREEN),
        )
        .fill(egui::Color32::BLACK)
        .rounding(egui::Rounding::same(3.0))
        .min_size(egui::vec2(70.0, 28.0)),
    )
    .on_hover_text("Copy queued prompt")
}

fn render_button(
    ui: &mut egui::Ui,
    name: &str,
    is_active: bool,
    active_btn: egui::Color32,
    text_color: egui::Color32,
    mut on_click: impl FnMut(),
) {
    let btn_fill = if is_active {
        active_btn
    } else {
        egui::Color32::TRANSPARENT
    };
    let btn_text = if is_active {
        egui::Color32::WHITE
    } else {
        text_color
    };
    let btn = ui.add(
        egui::Button::new(
            egui::RichText::new(name)
                .size(FOOTER_TEXT_SIZE)
                .color(btn_text),
        )
        .fill(btn_fill)
        .rounding(egui::Rounding::same(4.0))
        .min_size(egui::Vec2::new(0.0, FOOTER_BUTTON_HEIGHT)),
    );
    if btn.clicked() {
        on_click();
    }
}
