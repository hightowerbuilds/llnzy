use super::command_palette;

struct HotkeyEntry {
    action: &'static str,
    keys: &'static str,
    group: &'static str,
}

pub(super) fn render_hotkey_legend(ui: &mut egui::Ui, open: &mut bool) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Shortcuts")
                .size(22.0)
                .color(egui::Color32::WHITE),
        );
        ui.add_space(8.0);
        let label = if *open {
            "Hide Hotkey Legend"
        } else {
            "Show Hotkey Legend"
        };
        if ui
            .add(
                egui::Button::new(
                    egui::RichText::new(label)
                        .size(13.0)
                        .color(egui::Color32::from_rgb(230, 235, 245)),
                )
                .fill(egui::Color32::from_rgb(42, 48, 62))
                .rounding(egui::Rounding::same(4.0))
                .min_size(egui::vec2(148.0, 28.0)),
            )
            .clicked()
        {
            *open = !*open;
        }
    });

    if !*open {
        ui.add_space(16.0);
        return;
    }

    ui.add_space(10.0);
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(28, 30, 38))
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::symmetric(14.0, 12.0))
        .show(ui, |ui| {
            let entries = hotkey_entries();
            for group in [
                "Editor",
                "Navigation",
                "LSP",
                "Markdown",
                "Terminal",
                "System",
            ] {
                render_group(
                    ui,
                    group,
                    entries.iter().filter(|entry| entry.group == group),
                );
            }
        });
    ui.add_space(18.0);
}

fn render_group<'a>(
    ui: &mut egui::Ui,
    title: &str,
    entries: impl Iterator<Item = &'a HotkeyEntry>,
) {
    let entries: Vec<&HotkeyEntry> = entries.collect();
    if entries.is_empty() {
        return;
    }

    ui.label(
        egui::RichText::new(title)
            .size(15.0)
            .strong()
            .color(egui::Color32::from_rgb(210, 220, 245)),
    );
    ui.add_space(4.0);
    egui::Grid::new(format!("hotkey_legend_{title}"))
        .num_columns(2)
        .spacing([26.0, 6.0])
        .striped(true)
        .show(ui, |ui| {
            for entry in entries {
                ui.label(
                    egui::RichText::new(entry.action)
                        .size(13.0)
                        .color(egui::Color32::from_rgb(218, 222, 232)),
                );
                ui.label(
                    egui::RichText::new(entry.keys)
                        .size(12.0)
                        .monospace()
                        .color(egui::Color32::from_rgb(112, 190, 255)),
                );
                ui.end_row();
            }
        });
    ui.add_space(14.0);
}

fn hotkey_entries() -> Vec<HotkeyEntry> {
    let mut entries: Vec<HotkeyEntry> = command_palette::all_commands()
        .into_iter()
        .filter(|command| !command.keybinding.is_empty())
        .map(|command| HotkeyEntry {
            action: command.name,
            keys: command.keybinding,
            group: group_for_command(command.id),
        })
        .collect();

    entries.extend([
        HotkeyEntry {
            action: "Trigger Completion",
            keys: "Cmd+Space",
            group: "LSP",
        },
        HotkeyEntry {
            action: "Fold Current",
            keys: "Cmd+Shift+[",
            group: "Editor",
        },
        HotkeyEntry {
            action: "Unfold Current",
            keys: "Cmd+Shift+]",
            group: "Editor",
        },
        HotkeyEntry {
            action: "Fold All",
            keys: "Cmd+K, Cmd+0",
            group: "Editor",
        },
        HotkeyEntry {
            action: "Unfold All",
            keys: "Cmd+K, Cmd+J",
            group: "Editor",
        },
        HotkeyEntry {
            action: "Add Cursor To Next Match",
            keys: "Cmd+D",
            group: "Editor",
        },
        HotkeyEntry {
            action: "Select All Occurrences",
            keys: "Cmd+Shift+L",
            group: "Editor",
        },
        HotkeyEntry {
            action: "Toggle Line Comment",
            keys: "Cmd+/",
            group: "Editor",
        },
        HotkeyEntry {
            action: "Toggle Block Comment",
            keys: "Cmd+Shift+/",
            group: "Editor",
        },
        HotkeyEntry {
            action: "Jump To Matching Bracket",
            keys: "Cmd+Shift+\\",
            group: "Editor",
        },
    ]);

    entries
}

fn group_for_command(id: command_palette::CommandId) -> &'static str {
    match id {
        command_palette::CommandId::Save
        | command_palette::CommandId::Undo
        | command_palette::CommandId::Redo
        | command_palette::CommandId::SelectAll
        | command_palette::CommandId::Cut
        | command_palette::CommandId::Copy
        | command_palette::CommandId::Paste
        | command_palette::CommandId::DeleteLine
        | command_palette::CommandId::DuplicateLine
        | command_palette::CommandId::MoveLineUp
        | command_palette::CommandId::MoveLineDown
        | command_palette::CommandId::Find
        | command_palette::CommandId::FindReplace => "Editor",
        command_palette::CommandId::FormatDocument
        | command_palette::CommandId::RenameSymbol
        | command_palette::CommandId::GoToDefinition
        | command_palette::CommandId::ShowHover
        | command_palette::CommandId::CodeActions
        | command_palette::CommandId::DocumentSymbols
        | command_palette::CommandId::FindReferences
        | command_palette::CommandId::WorkspaceSymbols => "LSP",
        command_palette::CommandId::ProjectSearch
        | command_palette::CommandId::FindFile
        | command_palette::CommandId::NewTab
        | command_palette::CommandId::CloseTab
        | command_palette::CommandId::NextTab
        | command_palette::CommandId::PrevTab
        | command_palette::CommandId::ToggleSidebar => "Navigation",
        command_palette::CommandId::RunTask | command_palette::CommandId::ToggleTerminal => {
            "Terminal"
        }
        command_palette::CommandId::ToggleMarkdownMode
        | command_palette::CommandId::MarkdownSource
        | command_palette::CommandId::MarkdownPreview
        | command_palette::CommandId::MarkdownSplit => "Markdown",
        command_palette::CommandId::OpenWorkspace
        | command_palette::CommandId::ToggleEffects
        | command_palette::CommandId::ToggleFps => "System",
        command_palette::CommandId::Stacker(_) => "Stacker",
    }
}
