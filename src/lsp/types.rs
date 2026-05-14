use std::path::PathBuf;

use lsp_types::DiagnosticSeverity;

/// A text edit from formatting or workspace edits.
#[derive(Clone, Debug)]
pub struct FormatEdit {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub new_text: String,
}

pub type WorkspaceEdits = Vec<(PathBuf, Vec<FormatEdit>)>;

/// A simplified code action for the UI.
#[derive(Clone, Debug)]
pub struct CodeAction {
    pub title: String,
    pub edits: WorkspaceEdits,
}

/// Simplified signature help info for the UI.
#[derive(Clone, Debug)]
pub struct SignatureInfo {
    pub label: String,
    pub parameters: Vec<String>,
    pub active_parameter: usize,
}

/// A simplified reference location for the UI.
#[derive(Clone, Debug)]
pub struct ReferenceLocation {
    pub path: PathBuf,
    pub line: u32,
    pub col: u32,
    pub context: String,
}

/// A simplified document symbol for the UI.
#[derive(Clone, Debug)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String,
    pub line: u32,
    pub col: u32,
}

/// A workspace symbol with file path.
#[derive(Clone, Debug)]
pub struct WorkspaceSymbol {
    pub name: String,
    pub kind: String,
    pub path: PathBuf,
    pub line: u32,
    pub col: u32,
}

/// A simplified inlay hint for the UI.
#[derive(Clone, Debug)]
pub struct InlayHintInfo {
    pub line: u32,
    pub col: u32,
    pub label: String,
    pub padding_left: bool,
    pub padding_right: bool,
}

/// A simplified code lens for the UI.
#[derive(Clone, Debug)]
pub struct CodeLensInfo {
    pub line: u32,
    pub title: String,
}

/// A simplified completion item for the UI.
#[derive(Clone, Debug)]
pub struct CompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub insert_text: Option<String>,
    pub kind: Option<lsp_types::CompletionItemKind>,
    pub insert_text_format: Option<lsp_types::InsertTextFormat>,
}

/// A diagnostic with position and severity.
#[derive(Clone, Debug)]
pub struct FileDiagnostic {
    pub line: u32,
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub severity: DiagSeverity,
    pub message: String,
    pub source: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl From<Option<DiagnosticSeverity>> for DiagSeverity {
    fn from(s: Option<DiagnosticSeverity>) -> Self {
        match s {
            Some(DiagnosticSeverity::ERROR) => DiagSeverity::Error,
            Some(DiagnosticSeverity::WARNING) => DiagSeverity::Warning,
            Some(DiagnosticSeverity::INFORMATION) => DiagSeverity::Info,
            Some(DiagnosticSeverity::HINT) => DiagSeverity::Hint,
            _ => DiagSeverity::Info,
        }
    }
}
