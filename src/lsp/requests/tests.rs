use std::path::Path;

use super::*;
use crate::lsp::document::path_to_uri;
use crate::lsp::test_harness::FakeLspServer;

fn uri(path: &str) -> lsp_types::Uri {
    path_to_uri(Path::new(path)).unwrap()
}

fn diagnostic(line: u32, col: u32, message: &str) -> FileDiagnostic {
    FileDiagnostic {
        line,
        col,
        end_line: line,
        end_col: col + 4,
        severity: DiagSeverity::Warning,
        message: message.to_string(),
        source: Some("fake-lsp".to_string()),
    }
}

#[tokio::test]
async fn fake_lsp_harness_drives_completion_parsing() {
    let server = FakeLspServer::new();
    let document_uri = uri("/tmp/llnzy-fake-completion.rs");
    server.respond(
        "textDocument/completion",
        serde_json::json!([
            {
                "label": "println!",
                "detail": "macro",
                "insertText": "println!(\"$0\");",
                "kind": 3
            }
        ]),
    );

    let completions = async_completion(&server, &document_uri, 12, 4).await;

    assert_eq!(completions.len(), 1);
    assert_eq!(completions[0].label, "println!");
    assert_eq!(completions[0].detail.as_deref(), Some("macro"));
    assert_eq!(
        completions[0].insert_text.as_deref(),
        Some("println!(\"$0\");")
    );

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "textDocument/completion");
    assert_eq!(
        requests[0].params["position"],
        serde_json::json!({"line": 12, "character": 4})
    );
}

#[tokio::test]
async fn fake_lsp_harness_drives_range_formatting() {
    let server = FakeLspServer::new();
    let document_uri = uri("/tmp/llnzy-fake-range-format.rs");
    server.respond(
        "textDocument/rangeFormatting",
        serde_json::json!([
            {
                "range": {
                    "start": {"line": 2, "character": 4},
                    "end": {"line": 2, "character": 12}
                },
                "newText": "formatted"
            }
        ]),
    );

    let edits = async_range_format(&server, &document_uri, 2, 4, 2, 12).await;

    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].start_line, 2);
    assert_eq!(edits[0].start_col, 4);
    assert_eq!(edits[0].end_col, 12);
    assert_eq!(edits[0].new_text, "formatted");

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "textDocument/rangeFormatting");
    assert_eq!(
        requests[0].params["range"],
        serde_json::json!({
            "start": {"line": 2, "character": 4},
            "end": {"line": 2, "character": 12}
        })
    );
}

#[tokio::test]
async fn fake_lsp_harness_drives_workspace_symbol_parsing() {
    let server = FakeLspServer::new();
    let symbol_uri = uri("/tmp/llnzy-fake-symbol.rs");
    server.respond(
        "workspace/symbol",
        serde_json::json!([
            {
                "name": "build_fake_lsp",
                "kind": 12,
                "location": {
                    "uri": symbol_uri,
                    "range": {
                        "start": {"line": 2, "character": 8},
                        "end": {"line": 2, "character": 22}
                    }
                }
            }
        ]),
    );

    let symbols = async_workspace_symbols(&server, "fake").await;

    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "build_fake_lsp");
    assert_eq!(symbols[0].line, 2);
    assert_eq!(symbols[0].col, 8);

    let requests = server.requests();
    assert_eq!(requests[0].method, "workspace/symbol");
    assert_eq!(requests[0].params["query"], "fake");
}

#[tokio::test]
async fn fake_lsp_harness_drives_references_parsing() {
    let server = FakeLspServer::new();
    let reference_path = std::env::temp_dir().join(format!(
        "llnzy-lsp-requests-reference-{}.rs",
        std::process::id()
    ));
    std::fs::write(
        &reference_path,
        "fn main() {\n    let answer = target();\n}\n",
    )
    .unwrap();
    let document_uri = uri("/tmp/llnzy-fake-references.rs");
    let reference_uri = path_to_uri(&reference_path).unwrap();
    server.respond(
        "textDocument/references",
        serde_json::json!([
            {
                "uri": reference_uri,
                "range": {
                    "start": {"line": 1, "character": 17},
                    "end": {"line": 1, "character": 23}
                }
            }
        ]),
    );

    let references = async_references(&server, &document_uri, 4, 9).await;
    let _ = std::fs::remove_file(&reference_path);

    assert_eq!(references.len(), 1);
    assert_eq!(references[0].path, reference_path);
    assert_eq!(references[0].line, 1);
    assert_eq!(references[0].col, 17);
    assert_eq!(references[0].context, "let answer = target();");

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "textDocument/references");
    assert_eq!(
        requests[0].params["position"],
        serde_json::json!({"line": 4, "character": 9})
    );
    assert_eq!(requests[0].params["context"]["includeDeclaration"], true);
}

#[tokio::test]
async fn fake_lsp_harness_drives_signature_help_parsing() {
    let server = FakeLspServer::new();
    let document_uri = uri("/tmp/llnzy-fake-signature-help.rs");
    server.respond(
        "textDocument/signatureHelp",
        serde_json::json!({
            "signatures": [
                {
                    "label": "target(first: i32, second: &str)",
                    "parameters": [
                        {"label": [7, 17]},
                        {"label": "second: &str"}
                    ],
                    "activeParameter": 0
                }
            ],
            "activeSignature": 0,
            "activeParameter": 1
        }),
    );

    let signature = async_signature_help(&server, &document_uri, 8, 21)
        .await
        .unwrap();

    assert_eq!(signature.label, "target(first: i32, second: &str)");
    assert_eq!(signature.parameters, vec!["first: i32", "second: &str"]);
    assert_eq!(signature.active_parameter, 1);

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "textDocument/signatureHelp");
    assert_eq!(
        requests[0].params["position"],
        serde_json::json!({"line": 8, "character": 21})
    );
}

#[tokio::test]
async fn fake_lsp_harness_drives_inlay_hint_parsing() {
    let server = FakeLspServer::new();
    let document_uri = uri("/tmp/llnzy-fake-inlay-hints.rs");
    server.respond(
        "textDocument/inlayHint",
        serde_json::json!([
            {
                "position": {"line": 2, "character": 13},
                "label": [
                    {"value": ": "},
                    {"value": "usize"}
                ],
                "paddingLeft": true
            },
            {
                "position": {"line": 4, "character": 5},
                "label": ": bool",
                "paddingRight": true
            }
        ]),
    );

    let hints = async_inlay_hints(&server, &document_uri, 2, 5).await;

    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0].line, 2);
    assert_eq!(hints[0].col, 13);
    assert_eq!(hints[0].label, ": usize");
    assert!(hints[0].padding_left);
    assert!(!hints[0].padding_right);
    assert_eq!(hints[1].line, 4);
    assert_eq!(hints[1].col, 5);
    assert_eq!(hints[1].label, ": bool");
    assert!(!hints[1].padding_left);
    assert!(hints[1].padding_right);

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "textDocument/inlayHint");
    assert_eq!(
        requests[0].params["range"],
        serde_json::json!({
            "start": {"line": 2, "character": 0},
            "end": {"line": 5, "character": 0}
        })
    );
}

#[tokio::test]
async fn fake_lsp_harness_drives_code_lens_parsing() {
    let server = FakeLspServer::new();
    let document_uri = uri("/tmp/llnzy-fake-code-lens.rs");
    server.respond(
        "textDocument/codeLens",
        serde_json::json!([
            {
                "range": {
                    "start": {"line": 6, "character": 0},
                    "end": {"line": 6, "character": 12}
                },
                "command": {
                    "title": "Run test",
                    "command": "rust-analyzer.runSingle"
                }
            },
            {
                "range": {
                    "start": {"line": 8, "character": 0},
                    "end": {"line": 8, "character": 12}
                }
            }
        ]),
    );

    let lenses = async_code_lens(&server, &document_uri).await;

    assert_eq!(lenses.len(), 1);
    assert_eq!(lenses[0].line, 6);
    assert_eq!(lenses[0].title, "Run test");

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "textDocument/codeLens");
    assert_eq!(
        requests[0].params["textDocument"]["uri"],
        serde_json::json!(document_uri)
    );
}

#[tokio::test]
async fn code_actions_send_overlapping_diagnostics_for_quick_fixes() {
    let server = FakeLspServer::new();
    let document_uri = uri("/tmp/llnzy-fake-code-actions.rs");
    server.respond(
        "textDocument/codeAction",
        serde_json::json!([
            {
                "title": "Apply quick fix",
                "kind": "quickfix"
            }
        ]),
    );

    let actions = async_code_actions(
        &server,
        &document_uri,
        4,
        10,
        4,
        14,
        vec![diagnostic(4, 11, "unused value")],
    )
    .await;

    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].title, "Apply quick fix");

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "textDocument/codeAction");
    let sent_diagnostics = requests[0].params["context"]["diagnostics"]
        .as_array()
        .unwrap();
    assert_eq!(sent_diagnostics.len(), 1);
    assert_eq!(sent_diagnostics[0]["message"], "unused value");
    assert_eq!(sent_diagnostics[0]["source"], "fake-lsp");
}

#[test]
fn diagnostic_range_filter_keeps_only_overlapping_diagnostics() {
    let diagnostics = vec![
        diagnostic(1, 0, "before"),
        diagnostic(3, 4, "inside"),
        diagnostic(9, 0, "after"),
    ];

    let filtered = diagnostics_for_range(Some(&diagnostics), 3, 0, 3, 10);

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].message, "inside");
}
