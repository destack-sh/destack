use std::collections::HashMap;

use destack_lsp_types as lsp;

use super::tests::{MANIFEST, TestServer, position, range};

/// Rename an export through its import selector chain.
#[tokio::test]
async fn test_rename_an_export_through_its_import_selector() {
    let library = r#"export const answer: int32 = 42;
"#;
    let source = r#"import { answer } from "./library.ds";

const doubled = answer + answer;
"#;
    let mut server = TestServer::new("selector-chain-rename");
    server.write("destack.json", MANIFEST);
    let library = server.write("src/library.ds", library);
    let document = server.write("src/main.ds", source);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;
    server.open(&document, 1, source).await;
    server.assert_diagnostics(&document, 1, Vec::new()).await;

    // identify the exact editable range and current name
    let prepared = Some(lsp::PrepareRenameResponse::RangeWithPlaceholder {
        range: range(2, 16, 2, 22),
        placeholder: "answer".to_string(),
    });
    server
        .assert_request(document.prepare_rename(position(2, 16)), Ok(prepared))
        .await;

    // rename the declaration, import selector, and uses from one use site
    let expected = lsp::WorkspaceEdit {
        changes: Some(HashMap::from([
            (
                library.uri().clone(),
                vec![lsp::TextEdit {
                    range: range(0, 13, 0, 19),
                    new_text: "result".to_string(),
                }],
            ),
            (
                document.uri().clone(),
                vec![
                    lsp::TextEdit {
                        range: range(0, 9, 0, 15),
                        new_text: "result".to_string(),
                    },
                    lsp::TextEdit {
                        range: range(2, 16, 2, 22),
                        new_text: "result".to_string(),
                    },
                    lsp::TextEdit {
                        range: range(2, 25, 2, 31),
                        new_text: "result".to_string(),
                    },
                ],
            ),
        ])),
        ..lsp::WorkspaceEdit::default()
    };
    server
        .assert_request(
            document.rename(position(2, 16), "result"),
            Ok(Some(expected)),
        )
        .await;
}

/// Defer and resolve one auto import code action.
#[tokio::test]
async fn test_resolve_auto_import_code_action() {
    let library = "export function greet(): void {}\n";
    let source = "\nfunction main(): void {\n    greet();\n}\n";
    let mut server = TestServer::new("resolved-code-action");
    server.write("destack.json", MANIFEST);
    server.write("src/library.ds", library);
    let document = server.write("src/main.ds", source);
    let capabilities = lsp::ClientCapabilities {
        text_document: Some(lsp::TextDocumentClientCapabilities {
            code_action: Some(lsp::CodeActionClientCapabilities {
                data_support: Some(true),
                resolve_support: Some(lsp::CodeActionCapabilityResolveSupport {
                    properties: vec!["edit".to_string()],
                }),
                ..lsp::CodeActionClientCapabilities::default()
            }),
            diagnostic: Some(lsp::DiagnosticClientCapabilities::default()),
            ..lsp::TextDocumentClientCapabilities::default()
        }),
        ..lsp::ClientCapabilities::default()
    };
    server.initialize(capabilities, None).await.unwrap();
    server.initialized().await;
    server.open(&document, 1, source).await;

    // retain the diagnostic reference required by the action request
    let report = server.request(document.diagnostics()).await.unwrap();
    let lsp::DocumentDiagnosticReportResult::Report(lsp::DocumentDiagnosticReport::Full(report)) =
        report
    else {
        panic!("document diagnostics did not return a full report");
    };
    let [diagnostic] = report.full_document_diagnostic_report.items.as_slice() else {
        panic!("document diagnostics did not return one diagnostic");
    };
    let diagnostic = diagnostic.clone();

    // return an unresolved action carrying one opaque continuation
    let context = lsp::CodeActionContext {
        diagnostics: vec![diagnostic.clone()],
        only: Some(vec![lsp::CodeActionKind::QUICKFIX]),
        trigger_kind: Some(lsp::CodeActionTriggerKind::INVOKED),
    };
    let response = server
        .request(document.code_actions(range(2, 4, 2, 9), context))
        .await
        .unwrap()
        .unwrap();
    let [lsp::CodeActionOrCommand::CodeAction(action)] = response.as_slice() else {
        panic!("code actions did not return one action");
    };
    let continuation = action.data.clone();
    assert!(continuation.is_some());
    assert_eq!(
        action,
        &lsp::CodeAction {
            title: "Import greet from \"./library\"".to_string(),
            kind: Some(lsp::CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic]),
            edit: None,
            command: None,
            is_preferred: Some(true),
            disabled: None,
            data: continuation,
        },
    );

    // resolve the continuation to its exact workspace edit
    let resolved = server
        .request(server.resolve_code_action(action.clone()))
        .await
        .unwrap();
    assert_eq!(
        resolved.edit,
        Some(lsp::WorkspaceEdit {
            changes: Some(HashMap::from([(
                document.uri().clone(),
                vec![lsp::TextEdit {
                    range: range(0, 0, 0, 0),
                    new_text: "import { greet } from \"./library\";\n".to_string(),
                }],
            )])),
            ..lsp::WorkspaceEdit::default()
        }),
    );
}

/// Update relative imports before one workspace file rename.
#[tokio::test]
async fn test_rename_imported_file() {
    let library = "export const value = 1;\n";
    let source = "import { value } from \"./source/value\";\n\nconst result = value;\n";
    let mut server = TestServer::new("rename-imported-file");
    server.write("destack.json", MANIFEST);
    let old_document = server.write("src/source/value.ds", library);
    let new_document = server.document("src/source/result.ds");
    let document = server.write("src/main.ds", source);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;
    server.open(&document, 1, source).await;
    server.assert_diagnostics(&document, 1, Vec::new()).await;

    // rewrite the exact authored module specifier before the physical rename
    let expected = Some(lsp::WorkspaceEdit {
        changes: Some(HashMap::from([(
            document.uri().clone(),
            vec![lsp::TextEdit {
                range: range(0, 22, 0, 38),
                new_text: "\"./source/result\"".to_string(),
            }],
        )])),
        ..lsp::WorkspaceEdit::default()
    });
    let request = server.rename_file(&old_document, &new_document);
    server.assert_request(request, Ok(expected)).await;
}
