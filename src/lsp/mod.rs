pub mod client;
mod diagnostics;
pub mod document;
mod lifecycle;
mod manager;
pub mod registry;
mod requests;
mod symbols;
#[cfg(test)]
pub(crate) mod test_harness;
pub mod transport;
mod types;
mod workspace_edit;

pub use lifecycle::{LspEnsureStatus, LspLifecycleState, LspLifecycleStatus};
pub use manager::LspManager;
pub use types::{
    CodeAction, CodeLensInfo, CompletionItem, DiagSeverity, FileDiagnostic, FormatEdit,
    InlayHintInfo, ReferenceLocation, SignatureInfo, SymbolInfo, WorkspaceSymbol,
};
