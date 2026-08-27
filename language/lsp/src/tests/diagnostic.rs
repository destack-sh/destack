use destack_lsp_types as lsp;

use super::tests::{MANIFEST, TestServer, range, replace, replace_document};

/// Move diagnostic labels with their source edits.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_move_diagnostic_labels_after_source_edits() {
    let library = "export const helper: int32 = 1;\nexport const sibling: int32 = 2;\n";
    let source = r#"import { helper } from "./library.ds";

const first = helper;
const second = sibling;
"#;
    let mut server = TestServer::new("cross-file-diagnostic-label");
    server.write("destack.json", MANIFEST);
    let library_document = server.write("src/library.ds", library);
    let document = server.write("src/main.ds", source);
    let capabilities = lsp::ClientCapabilities {
        text_document: Some(lsp::TextDocumentClientCapabilities {
            diagnostic: Some(lsp::DiagnosticClientCapabilities::default()),
            ..lsp::TextDocumentClientCapabilities::default()
        }),
        ..lsp::ClientCapabilities::default()
    };
    server.initialize(capabilities, None).await.unwrap();
    server.initialized().await;

    // publish the unresolved reference with its imported declaration
    server.open(&document, 1, source).await;
    let mut expected = document.error(
        range(3, 15, 3, 22),
        "unresolved-reference",
        "cannot find 'sibling'",
    );
    expected.related_information = Some(vec![
        library_document.related(range(1, 13, 1, 20), "'sibling' is declared here"),
    ]);
    server
        .assert_document_diagnostics(&document, vec![expected.clone()])
        .await;

    // shift the primary label with its source
    server
        .change(&document, 2, [replace(range(3, 0, 3, 0), "// shifted\n")])
        .await;
    expected.range = range(4, 15, 4, 22);
    server
        .assert_document_diagnostics(&document, vec![expected.clone()])
        .await;

    // shift the foreign declaration without changing it
    server.open(&library_document, 1, library).await;
    server
        .change(
            &library_document,
            2,
            [replace(range(1, 0, 1, 0), "// shifted\n")],
        )
        .await;
    let related = expected
        .related_information
        .as_mut()
        .and_then(|related| related.first_mut())
        .unwrap();
    related.location.range = range(2, 13, 2, 20);
    server
        .assert_document_diagnostics(&document, vec![expected.clone()])
        .await;

    // remove the foreign declaration and its related label
    server
        .change(
            &library_document,
            3,
            [replace(range(2, 13, 2, 20), "other")],
        )
        .await;
    expected.related_information = None;
    server
        .assert_document_diagnostics(&document, vec![expected])
        .await;
}

/// Publish parser diagnostics when checking fails.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_publish_parser_diagnostics_before_check_failure() {
    let valid = "struct Report {}\nclass Foo {}\n";
    let invalid = "struct Report {}\nclass Foo {\n    @;\n    like\n}\n";
    let mut server = TestServer::new("parser-diagnostics-before-check-failure");
    let document = server.write("main.ds", valid);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // begin from one valid checked revision
    server.open(&document, 1, valid).await;
    server.assert_diagnostics(&document, 1, Vec::new()).await;

    // retain parser diagnostics when the edited field also fails checking
    server
        .change(&document, 2, [replace_document(invalid)])
        .await;
    server
        .assert_diagnostics(
            &document,
            2,
            vec![
                document.error(
                    range(3, 4, 3, 8),
                    "missing-type-annotation",
                    "missing type annotation",
                ),
                document.error(range(2, 5, 2, 6), "unexpected-token", "unexpected ;"),
                document.error(range(2, 5, 2, 6), "expected-member", "expected member"),
            ],
        )
        .await;
}

/// Return one complete workspace diagnostic report.
#[tokio::test]
async fn test_return_workspace_diagnostics() {
    let source = "const value = missing;\n";
    let mut server = TestServer::new("workspace-diagnostics");
    let document = server.write("main.ds", source);
    let capabilities = lsp::ClientCapabilities {
        text_document: Some(lsp::TextDocumentClientCapabilities {
            diagnostic: Some(lsp::DiagnosticClientCapabilities::default()),
            ..lsp::TextDocumentClientCapabilities::default()
        }),
        ..lsp::ClientCapabilities::default()
    };
    server.initialize(capabilities, None).await.unwrap();
    server.initialized().await;

    // return the exact diagnostic for every semantic workspace
    let request = server.workspace_diagnostics(Vec::new());
    let report = server.request(request).await.unwrap();
    let lsp::WorkspaceDiagnosticReportResult::Report(mut report) = report else {
        panic!("workspace diagnostics returned a partial report");
    };
    let [lsp::WorkspaceDocumentDiagnosticReport::Full(full)] = report.items.as_mut_slice() else {
        panic!("workspace diagnostics did not return one full report");
    };
    let Some(result_id) = full.full_document_diagnostic_report.result_id.clone() else {
        panic!("workspace diagnostics omitted its result ID");
    };
    let [diagnostic] = full.full_document_diagnostic_report.items.as_mut_slice() else {
        panic!("workspace diagnostics did not return one diagnostic");
    };
    diagnostic.data = None;
    let expected = lsp::WorkspaceDiagnosticReport {
        items: vec![lsp::WorkspaceDocumentDiagnosticReport::Full(
            lsp::WorkspaceFullDocumentDiagnosticReport {
                uri: document.uri().clone(),
                version: None,
                full_document_diagnostic_report: lsp::FullDocumentDiagnosticReport {
                    result_id: Some(result_id.clone()),
                    items: vec![document.error(
                        range(0, 14, 0, 21),
                        "unresolved-reference",
                        "cannot find 'missing'",
                    )],
                },
            },
        )],
    };
    assert_eq!(report, expected);

    // return an unchanged report for the exact previous result
    let previous = lsp::PreviousResultId {
        uri: document.uri().clone(),
        value: result_id.clone(),
    };
    let expected = lsp::WorkspaceDiagnosticReportResult::Report(lsp::WorkspaceDiagnosticReport {
        items: vec![lsp::WorkspaceDocumentDiagnosticReport::Unchanged(
            lsp::WorkspaceUnchangedDocumentDiagnosticReport {
                uri: document.uri().clone(),
                version: None,
                unchanged_document_diagnostic_report: lsp::UnchangedDocumentDiagnosticReport {
                    result_id,
                },
            },
        )],
    });
    let request = server.workspace_diagnostics(vec![previous]);
    server.assert_request(request, Ok(expected)).await;
}
