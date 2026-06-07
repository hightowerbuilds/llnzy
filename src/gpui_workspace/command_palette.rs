use std::path::{Path, PathBuf};

use gpui::{div, prelude::*, px, rgb, rgba, Action, Context, IntoElement, SharedString};

use crate::text_utils::fuzzy_match_case_insensitive_ascii;

use super::{
    MenuActivateTab1, MenuActivateTab2, MenuActivateTab3, MenuActivateTab4, MenuCloseProject,
    MenuCloseTab, MenuCopy, MenuEditorCheckDisk, MenuEditorCloseOthers, MenuEditorCloseSaved,
    MenuEditorReopenClosed, MenuFind, MenuJoinTabs, MenuLspCodeActions, MenuLspCompletion,
    MenuLspDefinition, MenuLspFormat, MenuLspHover, MenuLspReferences, MenuLspRename,
    MenuLspSignatureHelp, MenuLspSymbols, MenuMarkdownCycle, MenuMarkdownPreview,
    MenuMarkdownSource, MenuMarkdownSplit, MenuNewTab, MenuNextTab, MenuOpenProject, MenuPaste,
    MenuPreviousTab, MenuRedo, MenuSave, MenuSelectAll, MenuSeparateTabs, MenuShowAppearances,
    MenuShowEditor, MenuShowHome, MenuShowSketch, MenuShowStacker, MenuShowTerminal, MenuSwapTabs,
    MenuToggleSidebar, MenuUndo, MenuZoomIn, MenuZoomOut, MenuZoomReset, WorkspacePrototype,
};

/// What kind of list the palette is currently showing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum PaletteMode {
    #[default]
    Commands,
    Files,
}

/// Visual + behavioral state for the command palette overlay.
#[derive(Clone, Debug, Default)]
pub(super) struct CommandPaletteState {
    pub(super) open: bool,
    pub(super) query: String,
    pub(super) selected: usize,
    pub(super) mode: PaletteMode,
    /// Files indexed when the finder was opened. Empty in Commands mode.
    pub(super) files: Vec<PathBuf>,
    /// Project root the `files` list is relative to. None outside Files mode
    /// or when no project is open.
    pub(super) project_root: Option<PathBuf>,
}

impl CommandPaletteState {
    pub(super) fn reset(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected = 0;
        self.mode = PaletteMode::Commands;
        self.files.clear();
        self.project_root = None;
    }
}

/// One command shown in the palette. `build_action` is called when the user
/// hits Enter; we build a fresh action box every time so the same entry can
/// be invoked repeatedly.
pub(super) struct CommandEntry {
    pub(super) label: &'static str,
    pub(super) shortcut: Option<&'static str>,
    pub(super) build_action: fn() -> Box<dyn Action>,
}

/// All commands surfaced through the palette. Hand-curated rather than
/// reflected from the `actions!` registrations so we can give each entry a
/// user-readable label and a shortcut hint, and so we can exclude internal
/// actions that should not be palette-callable.
pub(super) fn palette_entries() -> Vec<CommandEntry> {
    vec![
        // --- Tabs ---
        CommandEntry {
            label: "Tab: New Terminal Tab",
            shortcut: Some("⌘T"),
            build_action: || Box::new(MenuNewTab),
        },
        CommandEntry {
            label: "Tab: Close Tab",
            shortcut: Some("⌘W"),
            build_action: || Box::new(MenuCloseTab),
        },
        CommandEntry {
            label: "Tab: Next Tab",
            shortcut: Some("⌘]"),
            build_action: || Box::new(MenuNextTab),
        },
        CommandEntry {
            label: "Tab: Previous Tab",
            shortcut: Some("⌘["),
            build_action: || Box::new(MenuPreviousTab),
        },
        CommandEntry {
            label: "Tab: Activate Tab 1",
            shortcut: Some("⌘1"),
            build_action: || Box::new(MenuActivateTab1),
        },
        CommandEntry {
            label: "Tab: Activate Tab 2",
            shortcut: Some("⌘2"),
            build_action: || Box::new(MenuActivateTab2),
        },
        CommandEntry {
            label: "Tab: Activate Tab 3",
            shortcut: Some("⌘3"),
            build_action: || Box::new(MenuActivateTab3),
        },
        CommandEntry {
            label: "Tab: Activate Tab 4",
            shortcut: Some("⌘4"),
            build_action: || Box::new(MenuActivateTab4),
        },
        CommandEntry {
            label: "Tab: Join Tabs",
            shortcut: None,
            build_action: || Box::new(MenuJoinTabs),
        },
        CommandEntry {
            label: "Tab: Separate Tabs",
            shortcut: None,
            build_action: || Box::new(MenuSeparateTabs),
        },
        CommandEntry {
            label: "Tab: Swap Tabs",
            shortcut: None,
            build_action: || Box::new(MenuSwapTabs),
        },
        // --- Surfaces ---
        CommandEntry {
            label: "View: Show Home",
            shortcut: None,
            build_action: || Box::new(MenuShowHome),
        },
        CommandEntry {
            label: "View: Show Terminal",
            shortcut: None,
            build_action: || Box::new(MenuShowTerminal),
        },
        CommandEntry {
            label: "View: Show Stacker",
            shortcut: None,
            build_action: || Box::new(MenuShowStacker),
        },
        CommandEntry {
            label: "View: Show Editor",
            shortcut: None,
            build_action: || Box::new(MenuShowEditor),
        },
        CommandEntry {
            label: "View: Show Sketch",
            shortcut: None,
            build_action: || Box::new(MenuShowSketch),
        },
        CommandEntry {
            label: "View: Show Settings",
            shortcut: None,
            build_action: || Box::new(MenuShowAppearances),
        },
        CommandEntry {
            label: "View: Toggle Sidebar",
            shortcut: Some("⌘B"),
            build_action: || Box::new(MenuToggleSidebar),
        },
        // --- Project ---
        CommandEntry {
            label: "Project: Open Project...",
            shortcut: None,
            build_action: || Box::new(MenuOpenProject),
        },
        CommandEntry {
            label: "Project: Close Project",
            shortcut: None,
            build_action: || Box::new(MenuCloseProject),
        },
        // --- File ---
        CommandEntry {
            label: "File: Save",
            shortcut: Some("⌘S"),
            build_action: || Box::new(MenuSave),
        },
        // --- Edit ---
        CommandEntry {
            label: "Edit: Undo",
            shortcut: Some("⌘Z"),
            build_action: || Box::new(MenuUndo),
        },
        CommandEntry {
            label: "Edit: Redo",
            shortcut: Some("⇧⌘Z"),
            build_action: || Box::new(MenuRedo),
        },
        CommandEntry {
            label: "Edit: Copy",
            shortcut: Some("⌘C"),
            build_action: || Box::new(MenuCopy),
        },
        CommandEntry {
            label: "Edit: Paste",
            shortcut: Some("⌘V"),
            build_action: || Box::new(MenuPaste),
        },
        CommandEntry {
            label: "Edit: Select All",
            shortcut: Some("⌘A"),
            build_action: || Box::new(MenuSelectAll),
        },
        CommandEntry {
            label: "Edit: Find",
            shortcut: Some("⌘F"),
            build_action: || Box::new(MenuFind),
        },
        // --- Editor ---
        CommandEntry {
            label: "Editor: Check Disk for Changes",
            shortcut: None,
            build_action: || Box::new(MenuEditorCheckDisk),
        },
        CommandEntry {
            label: "Editor: Reopen Closed File",
            shortcut: None,
            build_action: || Box::new(MenuEditorReopenClosed),
        },
        CommandEntry {
            label: "Editor: Close Other Files",
            shortcut: None,
            build_action: || Box::new(MenuEditorCloseOthers),
        },
        CommandEntry {
            label: "Editor: Close Saved Files",
            shortcut: None,
            build_action: || Box::new(MenuEditorCloseSaved),
        },
        CommandEntry {
            label: "Markdown: View Source",
            shortcut: None,
            build_action: || Box::new(MenuMarkdownSource),
        },
        CommandEntry {
            label: "Markdown: View Preview",
            shortcut: None,
            build_action: || Box::new(MenuMarkdownPreview),
        },
        CommandEntry {
            label: "Markdown: View Split",
            shortcut: None,
            build_action: || Box::new(MenuMarkdownSplit),
        },
        CommandEntry {
            label: "Markdown: Cycle View Mode",
            shortcut: None,
            build_action: || Box::new(MenuMarkdownCycle),
        },
        // --- LSP ---
        CommandEntry {
            label: "LSP: Hover",
            shortcut: None,
            build_action: || Box::new(MenuLspHover),
        },
        CommandEntry {
            label: "LSP: Completion",
            shortcut: None,
            build_action: || Box::new(MenuLspCompletion),
        },
        CommandEntry {
            label: "LSP: Go to Definition",
            shortcut: None,
            build_action: || Box::new(MenuLspDefinition),
        },
        CommandEntry {
            label: "LSP: Find References",
            shortcut: None,
            build_action: || Box::new(MenuLspReferences),
        },
        CommandEntry {
            label: "LSP: Signature Help",
            shortcut: None,
            build_action: || Box::new(MenuLspSignatureHelp),
        },
        CommandEntry {
            label: "LSP: Rename Symbol",
            shortcut: None,
            build_action: || Box::new(MenuLspRename),
        },
        CommandEntry {
            label: "LSP: Code Actions",
            shortcut: None,
            build_action: || Box::new(MenuLspCodeActions),
        },
        CommandEntry {
            label: "LSP: Format Document",
            shortcut: None,
            build_action: || Box::new(MenuLspFormat),
        },
        CommandEntry {
            label: "LSP: Document Symbols",
            shortcut: None,
            build_action: || Box::new(MenuLspSymbols),
        },
        // --- View ---
        CommandEntry {
            label: "Zoom: Zoom In",
            shortcut: Some("⌘+"),
            build_action: || Box::new(MenuZoomIn),
        },
        CommandEntry {
            label: "Zoom: Zoom Out",
            shortcut: Some("⌘-"),
            build_action: || Box::new(MenuZoomOut),
        },
        CommandEntry {
            label: "Zoom: Reset Zoom",
            shortcut: Some("⌘0"),
            build_action: || Box::new(MenuZoomReset),
        },
    ]
}

/// Return the indexes (into `entries`) of entries that match `query`, in
/// the order they should be displayed. Empty query returns all entries.
pub(super) fn filter_entries(entries: &[CommandEntry], query: &str) -> Vec<usize> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return (0..entries.len()).collect();
    }
    let lower: String = trimmed.to_ascii_lowercase();
    entries
        .iter()
        .enumerate()
        .filter_map(|(idx, entry)| {
            if fuzzy_match_case_insensitive_ascii(&lower, entry.label) {
                Some(idx)
            } else {
                None
            }
        })
        .collect()
}

/// Cap on how many project files the file-finder will index.
pub(super) const FILE_FINDER_MAX: usize = 5000;

const FILE_FINDER_IGNORED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "__pycache__",
    "venv",
    ".venv",
    "dist",
    "build",
    ".next",
    ".cache",
];

/// Walk `root` and collect file paths, skipping common dependency and build
/// directories. Stops at `FILE_FINDER_MAX` to keep palette open instant.
pub(super) fn collect_project_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if meta.is_dir() {
                if name_str.starts_with('.') {
                    continue;
                }
                if FILE_FINDER_IGNORED_DIRS.contains(&name_str.as_ref()) {
                    continue;
                }
                stack.push(path);
            } else if meta.is_file() {
                files.push(path);
                if files.len() >= FILE_FINDER_MAX {
                    return files;
                }
            }
        }
    }
    files
}

/// Filter file paths by fuzzy-matching against `query` on the path's display
/// string relative to `root`. Empty query returns all paths. Results are
/// sorted by (display length, then alphabetical) so shorter / closer-to-root
/// matches win.
pub(super) fn filter_files(files: &[PathBuf], root: &Path, query: &str) -> Vec<usize> {
    let trimmed = query.trim();
    let mut matches: Vec<(usize, String)> = if trimmed.is_empty() {
        files
            .iter()
            .enumerate()
            .map(|(idx, path)| (idx, file_display(path, root)))
            .collect()
    } else {
        let lower: String = trimmed.to_ascii_lowercase();
        files
            .iter()
            .enumerate()
            .filter_map(|(idx, path)| {
                let display = file_display(path, root);
                if fuzzy_match_case_insensitive_ascii(&lower, &display) {
                    Some((idx, display))
                } else {
                    None
                }
            })
            .collect()
    };
    matches.sort_by(|a, b| a.1.len().cmp(&b.1.len()).then_with(|| a.1.cmp(&b.1)));
    matches.into_iter().map(|(idx, _)| idx).collect()
}

pub(super) fn file_display(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

const PALETTE_BG: u32 = 0x1d1f25;
const PALETTE_BORDER: u32 = 0x3a3f4a;
const PALETTE_INPUT_BG: u32 = 0x14161a;
const PALETTE_ROW_HOVER_BG: u32 = 0x262a31;
const PALETTE_ROW_SELECTED_BG: u32 = 0x344056;
const PALETTE_LABEL: u32 = 0xe6e8ee;
const PALETTE_SHORTCUT: u32 = 0x9097a3;
const PALETTE_PLACEHOLDER: u32 = 0x6b727f;
const PALETTE_BACKDROP: u32 = 0x000000;

const PALETTE_WIDTH: f32 = 540.0;
const PALETTE_TOP_OFFSET: f32 = 96.0;
const PALETTE_LIST_MAX_HEIGHT: f32 = 360.0;
const PALETTE_ROW_HEIGHT: f32 = 30.0;
const PALETTE_FILE_ROW_LIMIT: usize = 200;

/// A single row to render: the visible label plus an optional muted hint
/// (shortcut for commands, parent directory for files).
struct PaletteRow {
    label: String,
    hint: Option<String>,
}

fn command_rows(entries: &[CommandEntry], visible: &[usize]) -> Vec<PaletteRow> {
    visible
        .iter()
        .map(|&idx| {
            let entry = &entries[idx];
            PaletteRow {
                label: entry.label.to_string(),
                hint: entry.shortcut.map(|s| s.to_string()),
            }
        })
        .collect()
}

fn file_rows(files: &[PathBuf], root: &Path, visible: &[usize]) -> Vec<PaletteRow> {
    visible
        .iter()
        .take(PALETTE_FILE_ROW_LIMIT)
        .map(|&idx| {
            let path = &files[idx];
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| file_display(path, root));
            let parent = path.parent().and_then(|parent| {
                let rel = parent.strip_prefix(root).unwrap_or(parent);
                let display = rel.to_string_lossy().into_owned();
                if display.is_empty() {
                    None
                } else {
                    Some(display)
                }
            });
            PaletteRow {
                label: file_name,
                hint: parent,
            }
        })
        .collect()
}

pub(super) fn render_command_palette(
    state: &CommandPaletteState,
    entries: &[CommandEntry],
    visible: &[usize],
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let (rows, placeholder, empty_message) = match state.mode {
        PaletteMode::Commands => (
            command_rows(entries, visible),
            "Type a command…",
            "No matching commands",
        ),
        PaletteMode::Files => {
            let root = state
                .project_root
                .clone()
                .unwrap_or_else(|| PathBuf::from(""));
            (
                file_rows(&state.files, &root, visible),
                "Open file…",
                if state.files.is_empty() {
                    "No project open"
                } else {
                    "No matching files"
                },
            )
        }
    };

    let query: SharedString = if state.query.is_empty() {
        placeholder.to_string().into()
    } else {
        state.query.clone().into()
    };
    let query_color = if state.query.is_empty() {
        PALETTE_PLACEHOLDER
    } else {
        PALETTE_LABEL
    };

    let mut list = div()
        .flex()
        .flex_col()
        .w_full()
        .max_h(px(PALETTE_LIST_MAX_HEIGHT))
        .overflow_hidden();

    if rows.is_empty() {
        list = list.child(
            div()
                .px_3()
                .py_2()
                .text_size(px(12.0))
                .text_color(rgb(PALETTE_PLACEHOLDER))
                .child(empty_message),
        );
    } else {
        for (display_idx, row) in rows.into_iter().enumerate() {
            let selected = display_idx == state.selected;
            let bg = if selected {
                PALETTE_ROW_SELECTED_BG
            } else {
                PALETTE_BG
            };
            let row_el = div()
                .id(("palette-row", display_idx))
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .px_3()
                .h(px(PALETTE_ROW_HEIGHT))
                .bg(rgb(bg))
                .hover(|style| style.bg(rgb(PALETTE_ROW_HOVER_BG)))
                .cursor_pointer()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(move |this, _, window, cx| {
                        this.invoke_palette_at(display_idx, window, cx);
                    }),
                )
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(rgb(PALETTE_LABEL))
                        .child(SharedString::from(row.label)),
                )
                .when_some(row.hint, |row_el, hint| {
                    row_el.child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(PALETTE_SHORTCUT))
                            .child(SharedString::from(hint)),
                    )
                });
            list = list.child(row_el);
        }
    }

    // Full-window backdrop catches outside clicks to dismiss the palette.
    div()
        .id("command-palette-backdrop")
        .absolute()
        .size_full()
        .top_0()
        .left_0()
        .bg(rgba(rgba_u32(PALETTE_BACKDROP, 0.35)))
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(|this, _, _window, cx| {
                this.close_command_palette(cx);
            }),
        )
        .child(
            div()
                .absolute()
                .top(px(PALETTE_TOP_OFFSET))
                .left_1_2()
                .w(px(PALETTE_WIDTH))
                .ml(px(-(PALETTE_WIDTH / 2.0)))
                .flex()
                .flex_col()
                .bg(rgb(PALETTE_BG))
                .border_1()
                .border_color(rgb(PALETTE_BORDER))
                .rounded_md()
                .overflow_hidden()
                // Stop click events on the panel itself from reaching the
                // backdrop (which would dismiss the palette).
                .on_mouse_down(gpui::MouseButton::Left, |_, _, _| {})
                .child(
                    div()
                        .px_3()
                        .py_2()
                        .bg(rgb(PALETTE_INPUT_BG))
                        .border_b_1()
                        .border_color(rgb(PALETTE_BORDER))
                        .text_size(px(13.0))
                        .text_color(rgb(query_color))
                        .child(query),
                )
                .child(list),
        )
}

fn rgba_u32(rgb: u32, alpha: f32) -> u32 {
    let alpha_byte = (alpha.clamp(0.0, 1.0) * 255.0) as u32;
    (rgb << 8) | alpha_byte
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_files_ranks_shorter_paths_first() {
        let root = PathBuf::from("/root");
        let files = vec![
            PathBuf::from("/root/a/b/c/main.rs"),
            PathBuf::from("/root/main.rs"),
            PathBuf::from("/root/a/main.rs"),
        ];
        let visible = filter_files(&files, &root, "main");
        assert_eq!(visible.len(), 3);
        assert_eq!(files[visible[0]], PathBuf::from("/root/main.rs"));
        assert_eq!(files[visible[1]], PathBuf::from("/root/a/main.rs"));
        assert_eq!(files[visible[2]], PathBuf::from("/root/a/b/c/main.rs"));
    }

    #[test]
    fn filter_files_drops_non_matching_paths() {
        let root = PathBuf::from("/root");
        let files = vec![
            PathBuf::from("/root/main.rs"),
            PathBuf::from("/root/README.md"),
        ];
        let visible = filter_files(&files, &root, "xyz");
        assert!(visible.is_empty());
    }
}
