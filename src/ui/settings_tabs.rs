mod appearance;
mod background;
mod components;
mod editor;
mod effects;
mod workspace;

pub(crate) use appearance::{render_text_tab, render_themes_tab};
pub(crate) use background::render_background_tab;
pub(crate) use editor::{render_editor_appearance_tab, render_editor_tab};
pub(crate) use workspace::render_workspace_tab;
pub use workspace::WorkspaceAction;
