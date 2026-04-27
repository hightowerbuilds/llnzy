use crate::workspace::WorkspaceTab;

/// Result of rendering the tab bar.
pub struct TabBarAction {
    /// Switch to this tab index.
    pub switch_to: Option<usize>,
    /// Close this tab index.
    pub close_tab: Option<usize>,
    /// Split this tab to the right of the active tab.
    pub split_right: Option<usize>,
    /// Remove the split view.
    pub unsplit: bool,
    /// Close all tabs except this one.
    pub close_others: Option<usize>,
    /// Close all tabs to the right of this one.
    pub close_to_right: Option<usize>,
}

/// Render the workspace tab bar at the top of the content area.
pub fn render_tab_bar(
    ctx: &egui::Context,
    tabs: &[WorkspaceTab],
    active_tab: usize,
    chrome_bg: egui::Color32,
) -> TabBarAction {
    let mut action = TabBarAction {
        switch_to: None,
        close_tab: None,
        split_right: None,
        unsplit: false,
        close_others: None,
        close_to_right: None,
    };

    if tabs.is_empty() {
        return action;
    }

    egui::TopBottomPanel::top("workspace_tab_bar")
        .exact_height(30.0)
        .frame(
            egui::Frame::none()
                .fill(chrome_bg)
                .inner_margin(egui::Margin::symmetric(4.0, 0.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;

                for (i, tab) in tabs.iter().enumerate() {
                    let is_active = i == active_tab;
                    let name = tab.display_name(i);

                    let tab_bg = if is_active {
                        egui::Color32::from_rgb(50, 80, 140)
                    } else {
                        egui::Color32::from_rgb(30, 32, 40)
                    };
                    let text_color = if is_active {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgb(160, 165, 180)
                    };

                    let frame = egui::Frame::none()
                        .fill(tab_bg)
                        .rounding(egui::Rounding { nw: 4.0, ne: 4.0, sw: 0.0, se: 0.0 })
                        .inner_margin(egui::Margin::symmetric(10.0, 4.0));

                    frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Tab name — click to switch
                            let label = ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&name)
                                        .size(12.0)
                                        .color(text_color),
                                )
                                .sense(egui::Sense::click()),
                            );
                            if label.clicked() {
                                action.switch_to = Some(i);
                            }

                            ui.add_space(6.0);

                            // X close button
                            let x_color = if is_active {
                                egui::Color32::from_rgb(200, 200, 210)
                            } else {
                                egui::Color32::from_rgb(100, 105, 115)
                            };
                            let x_btn = ui.add(
                                egui::Label::new(
                                    egui::RichText::new("x")
                                        .size(11.0)
                                        .color(x_color),
                                )
                                .sense(egui::Sense::click()),
                            );
                            if x_btn.clicked() {
                                action.close_tab = Some(i);
                            }
                            if x_btn.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                        });
                    });
                }
            });
        });

    action
}
