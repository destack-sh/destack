use std::fs;

use destack_lsp_types as lsp;

use super::harness::{LspHarness, notification_with_params, temp_root, uri_for_path};

/// LSP didOpen publishes diagnostics for the document.
#[tokio::test]
async fn test_lsp_did_open_publishes_diagnostics() {
    // create a workspace root
    let root = temp_root("did_open");
    fs::create_dir_all(&root).expect("create lsp temp root");

    // initialize the server harness
    let mut harness = LspHarness::new(root.clone()).await;

    // build a valid document payload
    let path = root.join("main.ds");
    let uri = uri_for_path(&path);
    let text = "export const x: number = 1;\n".to_string();
    let params = lsp::DidOpenTextDocumentParams {
        text_document: lsp::TextDocumentItem::new(uri.clone(), "destack".to_string(), 1, text),
    };

    // notify didOpen
    let notification = notification_with_params("textDocument/didOpen", params);
    harness.notify(notification).await;

    // fetch diagnostics
    let diagnostics = harness.next_diagnostics().await;

    // assertion block: diagnostics match the opened document
    assert_eq!(diagnostics.uri, uri);
    assert!(diagnostics.diagnostics.is_empty());
}

/// LSP didChange publishes updated diagnostics.
#[tokio::test]
async fn test_lsp_did_change_updates_diagnostics() {
    // create a workspace root
    let root = temp_root("did_change");
    fs::create_dir_all(&root).expect("create lsp temp root");

    // initialize the server harness
    let mut harness = LspHarness::new(root.clone()).await;

    // open a valid document
    let path = root.join("main.ds");
    let uri = uri_for_path(&path);
    let open_params = lsp::DidOpenTextDocumentParams {
        text_document: lsp::TextDocumentItem::new(
            uri.clone(),
            "destack".to_string(),
            1,
            "export const x: number = 1;\n".to_string(),
        ),
    };
    let open_notification = notification_with_params("textDocument/didOpen", open_params);
    harness.notify(open_notification).await;

    // drain initial diagnostics
    let initial = harness.next_diagnostics().await;

    // send an invalid update
    let change = lsp::TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: "export const x: number = \"bad\";\n".to_string(),
    };
    let change_params = lsp::DidChangeTextDocumentParams {
        text_document: lsp::VersionedTextDocumentIdentifier::new(uri.clone(), 2),
        content_changes: vec![change],
    };
    let change_notification = notification_with_params("textDocument/didChange", change_params);
    harness.notify(change_notification).await;

    // fetch updated diagnostics
    let updated = harness.next_diagnostics().await;

    // assertion block: diagnostics update for the same document
    assert_eq!(initial.uri, uri);
    assert_eq!(updated.uri, uri);
    assert!(updated.diagnostics.len() >= initial.diagnostics.len());
    assert!(!updated.diagnostics.is_empty());
}
