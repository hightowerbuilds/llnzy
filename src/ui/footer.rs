use super::types::ActiveView;

/// Render the footer navigation bar. Returns the view the user clicked, if any.
pub fn render_footer(
    ctx: &egui::Context,
    footer_height: f32,
    current_view: ActiveView,
    chrome_bg: egui::Color32,
    active_btn: egui::Color32,
    text_color: egui::Color32,
) -> Option<ActiveView> {
    let mut nav_target: Option<ActiveView> = None;

    egui::TopBottomPanel::bottom("footer")
        .exact_height(footer_height)
        .frame(
            egui::Frame::none()
                .fill(chrome_bg)
                .inner_margin(egui::Margin::symmetric(8.0, 2.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                let views: &[(&str, ActiveView)] = &[
                    ("Home", ActiveView::Home),
                    ("Terminal", ActiveView::Shells),
                    ("Stacker", ActiveView::Stacker),
                    ("Sketch", ActiveView::Sketch),
                    ("Appearances", ActiveView::Appearances),
                    ("Settings", ActiveView::Settings),
                ];
                for &(name, view) in views {
                    let is_active = current_view == view;
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
                            egui::RichText::new(name).size(12.0).color(btn_text),
                        )
                        .fill(btn_fill)
                        .rounding(egui::Rounding::same(4.0))
                        .min_size(egui::Vec2::new(0.0, 28.0)),
                    );
                    if btn.clicked() && current_view != view {
                        nav_target = Some(view);
                    }
                }
            });
        });

    nav_target
}
