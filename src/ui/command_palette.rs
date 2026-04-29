/// A command that can be executed from the palette.
#[derive(Clone)]
pub struct Command {
    pub name: &'static str,
    pub keybinding: &'static str,
    pub id: CommandId,
}

/// All available command identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandId {
    Save,
    Undo,
    Redo,
    SelectAll,
    Cut,
    Copy,
    Paste,
    DeleteLine,
    DuplicateLine,
    MoveLineUp,
    MoveLineDown,
    FormatDocument,
    RenameSymbol,
    GoToDefinition,
    ShowHover,
    CodeActions,
    DocumentSymbols,
    Find,
    FindReplace,
    FindReferences,
    WorkspaceSymbols,
    ProjectSearch,
    RunTask,
    OpenWorkspace,
    FindFile,
    ToggleTerminal,
    ToggleSidebar,
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    ToggleEffects,
    ToggleFps,
}

/// All registered commands.
pub fn all_commands() -> Vec<Command> {
    vec![
        Command {
            name: "Save",
            keybinding: "Cmd+S",
            id: CommandId::Save,
        },
        Command {
            name: "Undo",
            keybinding: "Cmd+Z",
            id: CommandId::Undo,
        },
        Command {
            name: "Redo",
            keybinding: "Cmd+Shift+Z",
            id: CommandId::Redo,
        },
        Command {
            name: "Select All",
            keybinding: "Cmd+A",
            id: CommandId::SelectAll,
        },
        Command {
            name: "Cut",
            keybinding: "Cmd+X",
            id: CommandId::Cut,
        },
        Command {
            name: "Copy",
            keybinding: "Cmd+C",
            id: CommandId::Copy,
        },
        Command {
            name: "Paste",
            keybinding: "Cmd+V",
            id: CommandId::Paste,
        },
        Command {
            name: "Delete Line",
            keybinding: "Cmd+Shift+K",
            id: CommandId::DeleteLine,
        },
        Command {
            name: "Duplicate Line",
            keybinding: "Cmd+Shift+D",
            id: CommandId::DuplicateLine,
        },
        Command {
            name: "Move Line Up",
            keybinding: "Alt+Up",
            id: CommandId::MoveLineUp,
        },
        Command {
            name: "Move Line Down",
            keybinding: "Alt+Down",
            id: CommandId::MoveLineDown,
        },
        Command {
            name: "Format Document",
            keybinding: "Cmd+Shift+F",
            id: CommandId::FormatDocument,
        },
        Command {
            name: "Rename Symbol",
            keybinding: "F2",
            id: CommandId::RenameSymbol,
        },
        Command {
            name: "Go to Definition",
            keybinding: "F12",
            id: CommandId::GoToDefinition,
        },
        Command {
            name: "Show Hover Info",
            keybinding: "F1",
            id: CommandId::ShowHover,
        },
        Command {
            name: "Code Actions",
            keybinding: "Cmd+.",
            id: CommandId::CodeActions,
        },
        Command {
            name: "Document Symbols",
            keybinding: "Cmd+Shift+O",
            id: CommandId::DocumentSymbols,
        },
        Command {
            name: "Find",
            keybinding: "Cmd+F",
            id: CommandId::Find,
        },
        Command {
            name: "Find and Replace",
            keybinding: "Cmd+H",
            id: CommandId::FindReplace,
        },
        Command {
            name: "Find References",
            keybinding: "Shift+F12",
            id: CommandId::FindReferences,
        },
        Command {
            name: "Workspace Symbols",
            keybinding: "Cmd+Shift+T",
            id: CommandId::WorkspaceSymbols,
        },
        Command {
            name: "Search in Project",
            keybinding: "Cmd+Shift+G",
            id: CommandId::ProjectSearch,
        },
        Command {
            name: "Run Task",
            keybinding: "Cmd+Shift+B",
            id: CommandId::RunTask,
        },
        Command {
            name: "Open Workspace",
            keybinding: "",
            id: CommandId::OpenWorkspace,
        },
        Command {
            name: "Find File",
            keybinding: "Cmd+P",
            id: CommandId::FindFile,
        },
        Command {
            name: "Toggle Terminal",
            keybinding: "Cmd+`",
            id: CommandId::ToggleTerminal,
        },
        Command {
            name: "Toggle Sidebar",
            keybinding: "Cmd+B",
            id: CommandId::ToggleSidebar,
        },
        Command {
            name: "New Tab",
            keybinding: "Cmd+T",
            id: CommandId::NewTab,
        },
        Command {
            name: "Close Tab",
            keybinding: "Cmd+W",
            id: CommandId::CloseTab,
        },
        Command {
            name: "Next Tab",
            keybinding: "Cmd+]",
            id: CommandId::NextTab,
        },
        Command {
            name: "Previous Tab",
            keybinding: "Cmd+[",
            id: CommandId::PrevTab,
        },
        Command {
            name: "Toggle Effects",
            keybinding: "Cmd+Shift+F",
            id: CommandId::ToggleEffects,
        },
        Command {
            name: "Toggle FPS Overlay",
            keybinding: "Cmd+Shift+P",
            id: CommandId::ToggleFps,
        },
    ]
}

/// Command palette state.
pub struct PaletteState {
    pub open: bool,
    pub query: String,
    pub selected: usize,
    pub filtered: Vec<Command>,
}

impl Default for PaletteState {
    fn default() -> Self {
        Self {
            open: false,
            query: String::new(),
            selected: 0,
            filtered: all_commands(),
        }
    }
}

impl PaletteState {
    pub fn open(&mut self) {
        self.open = true;
        self.query.clear();
        self.selected = 0;
        self.filtered = all_commands();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
    }

    pub fn update_filter(&mut self) {
        let q = self.query.to_lowercase();
        if q.is_empty() {
            self.filtered = all_commands();
        } else {
            self.filtered = all_commands()
                .into_iter()
                .filter(|c| fuzzy_match(&q, &c.name.to_lowercase()))
                .collect();
        }
        self.selected = 0;
    }

    /// Get the selected command ID, if any.
    pub fn selected_command(&self) -> Option<CommandId> {
        self.filtered.get(self.selected).map(|c| c.id)
    }
}

fn fuzzy_match(query: &str, target: &str) -> bool {
    let mut target_chars = target.chars();
    for qc in query.chars() {
        loop {
            match target_chars.next() {
                Some(tc) if tc == qc => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

/// Render the command palette overlay. Returns the selected CommandId if the user confirms.
pub fn render_palette(ui: &mut egui::Ui, state: &mut PaletteState) -> Option<CommandId> {
    if !state.open {
        return None;
    }

    let mut result: Option<CommandId> = None;

    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new("Command Palette")
                .size(16.0)
                .color(egui::Color32::WHITE)
                .strong(),
        );
        ui.add_space(4.0);

        let mut query = state.query.clone();
        let resp = ui.add(
            egui::TextEdit::singleline(&mut query)
                .hint_text("Type a command...")
                .desired_width(ui.available_width() - 20.0)
                .text_color(egui::Color32::WHITE)
                .font(egui::TextStyle::Monospace),
        );
        resp.request_focus();

        if query != state.query {
            state.query = query;
            state.update_filter();
        }

        // Key handling
        let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
        let down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
        let up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));

        if escape {
            state.close();
            return;
        }
        if down {
            state.selected = (state.selected + 1).min(state.filtered.len().saturating_sub(1));
        }
        if up {
            state.selected = state.selected.saturating_sub(1);
        }
        if enter {
            result = state.selected_command();
            state.close();
            return;
        }

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        let selected_bg = egui::Color32::from_rgb(50, 80, 130);
        let key_color = egui::Color32::from_rgb(100, 105, 120);

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                for (i, cmd) in state.filtered.iter().enumerate() {
                    let bg = if i == state.selected {
                        selected_bg
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    let text_color = if i == state.selected {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgb(200, 205, 215)
                    };

                    egui::Frame::none()
                        .fill(bg)
                        .inner_margin(egui::Margin::symmetric(6.0, 3.0))
                        .rounding(2.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(cmd.name).size(13.0).color(text_color),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(cmd.keybinding)
                                                .size(11.0)
                                                .color(key_color)
                                                .monospace(),
                                        );
                                    },
                                );
                            });
                        });
                }
            });
    });

    result
}
