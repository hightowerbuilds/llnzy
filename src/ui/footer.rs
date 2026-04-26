use super::types::ActiveView;
use crate::workspace::TabKind;

/// Action returned by the footer when a button is clicked.
pub enum FooterAction {
    /// Show an overlay view (Home, Appearances, Settings).
    ShowOverlay(ActiveView),
    /// Open or focus a singleton tab (Stacker, Sketch).
    OpenSingletonTab(TabKind),
    /// Create a new terminal tab.
    NewTerminalTab,
}

/// Render the footer navigation bar. Returns the action if a button was clicked.
pub fn render_footer(
    ctx: &egui::Context,
    footer_height: f32,
    current_view: ActiveView,
    _active_singleton: Option<TabKind>,
    active_tab_kind: Option<TabKind>,
    chrome_bg: egui::Color32,
    active_btn: egui::Color32,
    text_color: egui::Color32,
) -> Option<FooterAction> {
    let mut result: Option<FooterAction> = None;

    egui::TopBottomPanel::bottom("footer")
        .exact_height(footer_height)
        .frame(
            egui::Frame::none()
                .fill(chrome_bg)
                .inner_margin(egui::Margin::symmetric(8.0, 2.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                // Home button (overlay)
                let home_active = current_view == ActiveView::Home;
                render_button(ui, "Home", home_active, active_btn, text_color, || {
                    result = Some(FooterAction::ShowOverlay(ActiveView::Home));
                });

                // Terminal button — creates a new terminal tab
                let terminal_active = matches!(active_tab_kind, Some(TabKind::Terminal));
                render_button(ui, "Terminal", terminal_active, active_btn, text_color, || {
                    result = Some(FooterAction::NewTerminalTab);
                });

                // All singleton tab buttons
                let singletons: &[(&str, TabKind)] = &[
                    ("Stacker", TabKind::Stacker),
                    ("Sketch", TabKind::Sketch),
                    ("Appearances", TabKind::Appearances),
                    ("Settings", TabKind::Settings),
                ];
                for &(name, kind) in singletons {
                    let is_active = active_tab_kind == Some(kind);
                    render_button(ui, name, is_active, active_btn, text_color, || {
                        result = Some(FooterAction::OpenSingletonTab(kind));
                    });
                }
            });
        });

    result
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
        egui::Button::new(egui::RichText::new(name).size(12.0).color(btn_text))
            .fill(btn_fill)
            .rounding(egui::Rounding::same(4.0))
            .min_size(egui::Vec2::new(0.0, 28.0)),
    );
    if btn.clicked() {
        on_click();
    }
}
