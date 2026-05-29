use std::sync::Arc;
use std::time::Duration;

use destack_lsp_server::UriExt;
use destack_lsp_server::jsonrpc::{ErrorCode, Response};
use destack_lsp_types as lsp;
use destack_query as query;
use destack_service::{FileChange, LanguageService, QueryRevision};
use destack_session::open_repository_from_fs;
use destack_source::{
    FileSystem, OverlayFileSystem, PhysicalFileSystem, Span, TargetId, TemporaryPhysicalFileSystem,
};
use destack_workspace::{DestackLayoutOverride, Environment, Settings};

use super::fixture::TestLsp;
use super::harness::{LspHarness, harness_for_fs, request_with_params, test_fs, uri_for_path};
/// LSP didOpen publishes diagnostics for the document.
#[tokio::test]
async fn test_lsp_did_open_publishes_diagnostics() {
    let mut test = TestLsp::new("did_open").await;
    let path = test.path_for("main.ds");
    let uri = test.uri_for_path(&path);
    let diagnostics = test
        .open_and_get_diagnostics(&path, "export const x: number = 1;\n")
        .await;

    // check that the diagnostics match the opened document
    assert_eq!(diagnostics.uri, uri);
    assert_eq!(diagnostics.version, Some(1));
    assert!(diagnostics.diagnostics.is_empty());
}

/// LSP language service emits diagnostics for file updates.
#[test]
fn test_lsp_language_service_file_update_emits_diagnostics() {
    let fs = TemporaryPhysicalFileSystem::new_with_prefix("lsp_language_service_update");
    let overlay = Arc::new(OverlayFileSystem::with_inner(Arc::new(
        PhysicalFileSystem::new(),
    )));
    let root = fs.root().to_path_buf();
    let repository = Arc::new(
        open_repository_from_fs(
            root.clone(),
            overlay.clone(),
            Environment::capture_process(),
            Settings::default(),
            DestackLayoutOverride::default(),
        )
        .expect("failed to import repository from overlay fs"),
    );
    let language_service = LanguageService::new(
        repository.clone(),
        Some(overlay),
        vec![root.clone()],
        1,
        None,
    )
    .expect("expected language service");

    let path = root.join("main.ds");
    let _ = fs.write_text("main.ds", "export const x: number = 1;\n");
    let initial = language_service
        .apply_file(
            &path,
            FileChange::Text {
                content: "export const x: number = 1;\n".to_string(),
            },
        )
        .expect("expected initial update");
    assert!(
        initial
            .updates
            .iter()
            .all(|update| update.diagnostics.is_empty()),
        "expected no diagnostics for valid content"
    );

    let updated = language_service
        .apply_file(
            &path,
            FileChange::Text {
                content: "export const x = ;\n".to_string(),
            },
        )
        .expect("expected updated diagnostics");
    assert!(
        updated
            .updates
            .iter()
            .any(|update| !update.diagnostics.is_empty()),
        "expected diagnostics for invalid content"
    );
}

/// LSP language service queries use current file updates.
#[test]
fn test_lsp_language_service_query_uses_current_file_update() {
    let fs = TemporaryPhysicalFileSystem::new_with_prefix("lsp_language_service_query");
    let overlay = Arc::new(OverlayFileSystem::with_inner(Arc::new(
        PhysicalFileSystem::new(),
    )));
    let root = fs.root().to_path_buf();
    let repository = Arc::new(
        open_repository_from_fs(
            root.clone(),
            overlay.clone(),
            Environment::capture_process(),
            Settings::default(),
            DestackLayoutOverride::default(),
        )
        .expect("failed to import repository from overlay fs"),
    );
    let language_service = LanguageService::new(
        repository.clone(),
        Some(overlay),
        vec![root.clone()],
        1,
        None,
    )
    .expect("expected language service");

    let source_a = r#"function greet(name: string, greeting: string): string {
    return greeting + ", " + name;
}
const msg: string = greet("World", "Hello");
"#;
    let source_b = r#"function greet(name: string, greeting: string): string {
    return greeting + ", " + name;
}
const msg = greet("World", "Hello");
"#;
    let path = fs
        .write_text("main.ds", source_a)
        .expect("failed to write main.ds");
    let _ = language_service
        .apply_file(
            &path,
            FileChange::Text {
                content: source_a.to_string(),
            },
        )
        .expect("expected initial update");
    let _ = language_service
        .apply_file(
            &path,
            FileChange::Text {
                content: source_b.to_string(),
            },
        )
        .expect("expected second update");
    let view = language_service
        .file_view(&path)
        .expect("expected query file view");
    let repository = view.repository();
    let module_id = repository
        .module_id_for_file(view.revision(), view.file_id)
        .expect("expected module lookup")
        .expect("expected query module");
    let module_entry = repository
        .module(view.revision(), module_id)
        .expect("expected module entry lookup")
        .expect("expected query module entry");
    let target_id = TargetId::new(module_entry.package_id, "native");
    let profile = repository
        .module_target_profile(view.revision(), module_id, target_id)
        .expect("expected module target profile lookup")
        .expect("expected module target profile");
    let module = query::QueryModule {
        module_id,
        profile_id: profile.id(),
    };

    let response = language_service
        .query(
            &path,
            query::QueryRequest::InlayHints(query::InlayHintsRequest {
                range: query::QueryRange {
                    module,
                    span: Span::new(view.file_id, 0, source_b.len() as u32),
                },
            }),
            QueryRevision::Exact(view.revision()),
        )
        .expect("expected inlay hints query response");
    let query::QueryResponse::InlayHints(query::InlayHintsResponse { hints }) = response.response
    else {
        panic!("expected inlay hints query response payload");
    };

    let type_hint = hints
        .iter()
        .any(|hint| hint.kind == query::InlayHintKind::Type);
    assert!(
        type_hint,
        "expected one inferred type hint after the file update"
    );
}

/// LSP didChange publishes updated diagnostics.
#[tokio::test]
async fn test_lsp_did_change_updates_diagnostics() {
    let mut test = TestLsp::new("did_change").await;
    let path = test.path_for("main.ds");
    let uri = test.uri_for_path(&path);
    let initial = test
        .open_and_get_diagnostics(&path, "export const x: number = 1;\n")
        .await;

    test.harness
        .did_change(uri.clone(), "export const x = ;\n", 2)
        .await;

    let updated = test.harness.next_diagnostics_for(&uri).await;

    // check that the diagnostics update for the same document
    assert_eq!(initial.uri, uri);
    assert_eq!(initial.version, Some(1));
    assert_eq!(updated.uri, uri);
    assert_eq!(updated.version, Some(2));
    assert!(updated.diagnostics.len() >= initial.diagnostics.len());
    assert!(!updated.diagnostics.is_empty());
}

/// LSP didChange applies incremental range edits.
#[tokio::test]
async fn test_lsp_did_change_incremental_updates_diagnostics() {
    // set up a clean document and open it
    let fs = test_fs("did_change_incremental");
    let mut harness = harness_for_fs(&fs).await;

    let path = fs.path_for("main.ds");
    let uri = uri_for_path(&path);
    harness
        .did_open(uri.clone(), "export const x: number = 1;\n")
        .await;
    let initial = harness.next_diagnostics_for(&uri).await;

    // apply an incremental edit that removes the initializer value
    let change = lsp::TextDocumentContentChangeEvent {
        range: Some(lsp::Range::new(
            lsp::Position::new(0, 25),
            lsp::Position::new(0, 26),
        )),
        range_length: None,
        text: String::new(),
    };
    harness
        .did_change_incremental(uri.clone(), vec![change], 2)
        .await;
    let updated = harness.next_diagnostics_for(&uri).await;

    // verify diagnostics are produced for the edited document
    assert_eq!(initial.uri, uri);
    assert_eq!(initial.version, Some(1));
    assert_eq!(updated.uri, uri);
    assert_eq!(updated.version, Some(2));
    assert!(!updated.diagnostics.is_empty());
}

/// LSP didChange recovers when incremental edits cannot be applied.
#[tokio::test]
async fn test_lsp_did_change_incremental_recovery_publishes_diagnostics() {
    // set up a clean document and open it
    let fs = test_fs("did_change_incremental_recovery");
    let mut harness = harness_for_fs(&fs).await;

    let path = fs.path_for("main.ds");
    let uri = uri_for_path(&path);
    harness
        .did_open(uri.clone(), "export const x: number = 1;\n")
        .await;
    let initial = harness.next_diagnostics_for(&uri).await;
    assert!(initial.diagnostics.is_empty());

    // send an oversized incremental range through the service patch path
    let invalid_change = lsp::TextDocumentContentChangeEvent {
        range: Some(lsp::Range::new(
            lsp::Position::new(99, 0),
            lsp::Position::new(99, 1),
        )),
        range_length: None,
        text: String::new(),
    };
    harness
        .did_change_incremental(uri.clone(), vec![invalid_change], 2)
        .await;
    let recovered = harness.next_diagnostics_for(&uri).await;
    assert_eq!(recovered.uri, uri);
    assert_eq!(recovered.version, Some(2));
    assert!(recovered.diagnostics.is_empty());

    // verify subsequent changes continue to work after recovery
    harness
        .did_change(uri.clone(), "export const x = ;\n", 3)
        .await;
    let updated = harness.next_diagnostics_for(&uri).await;
    assert_eq!(updated.version, Some(3));
    assert!(!updated.diagnostics.is_empty());
}

/// LSP inlay hints refresh against the coherent post-change semantic state.
#[tokio::test]
async fn test_lsp_inlay_hints_refresh_after_annotation_removal() {
    let fs = test_fs("did_change_inlay_hint_refresh");
    let mut harness = harness_for_fs(&fs).await;

    let source_a = r#"function greet(name: string, greeting: string): string {
    return greeting + ", " + name;
}
const msg: string = greet("World", "Hello");
"#;
    let source_b = r#"function greet(name: string, greeting: string): string {
    return greeting + ", " + name;
}
const msg = greet("World", "Hello");
"#;
    let path = fs
        .write_text("main.ds", source_a)
        .expect("failed to write main.ds");
    let uri = uri_for_path(&path);

    // open the annotated source and drain initial diagnostics
    harness.did_open(uri.clone(), source_a).await;
    harness.wait_for_mutation_idle().await;

    // remove the annotation and wait for the semantic mutation lane
    harness.did_change(uri.clone(), source_b, 2).await;
    harness.wait_for_mutation_idle().await;

    // request inlay hints from the updated open document
    let last_line_length = source_b.lines().last().expect("expected final line").len() as u32;
    let range = lsp::Range::new(
        lsp::Position::new(0, 0),
        lsp::Position::new(3, last_line_length),
    );
    let hints: Option<Vec<lsp::InlayHint>> = harness
        .request_result(
            "textDocument/inlayHint",
            lsp::InlayHintParams {
                text_document: lsp::TextDocumentIdentifier::new(uri),
                range,
                work_done_progress_params: lsp::WorkDoneProgressParams {
                    work_done_token: None,
                },
            },
        )
        .await;
    let hints = hints.expect("expected inlay hints response");

    // keep the inferred binding type after the annotation disappears
    assert!(
        hints.iter().any(|hint| {
            hint.kind == Some(lsp::InlayHintKind::TYPE)
                && matches!(&hint.label, lsp::InlayHintLabel::String(label) if label == ": string")
        }),
        "expected one inferred type hint after the didChange update, actual hints: {hints:#?}"
    );
}

/// Definition and hover resolve for simple local symbols.
#[tokio::test]
async fn test_lsp_navigation_resolves_local_symbols() {
    let mut test = TestLsp::new("navigation_local").await;
    let path = test.write_text(
        "main.ds",
        "export function foo(x: number): number { return x; }\nconst y = foo(1);\n",
    );
    let text = "export function foo(x: number): number { return x; }\nconst y = foo(1);\n";
    let diagnostics = test.open_and_get_diagnostics(&path, text).await;
    let uri = diagnostics.uri;

    let definition = test
        .goto_definition(
            &uri,
            lsp::Position {
                line: 1,
                character: 11,
            },
        )
        .await;
    let definition = definition.expect("expected goto definition result");
    let location = first_definition_location(&definition).expect("expected definition location");
    assert_eq!(location.uri, uri);
    assert_eq!(location.range.start.line, 0);
    assert_eq!(location.range.start.character, 16);

    let hover = test
        .hover(
            &uri,
            lsp::Position {
                line: 1,
                character: 11,
            },
        )
        .await;
    assert!(hover.is_some());
}

/// Definition resolves from an import specifier to the exported symbol.
#[tokio::test]
async fn test_lsp_navigation_resolves_import_specifier_symbols() {
    let mut test = TestLsp::new("navigation_import_specifier").await;
    let lib_text = "export const value: number = 1;\n";
    let main_text = "import { value } from \"./lib\";\nconst output: number = value + 1;\n";
    let lib_path = test.write_text("lib.ds", lib_text);
    let main_path = test.write_text("main.ds", main_text);
    let lib_uri = uri_for_path(&lib_path);
    let main_uri = uri_for_path(&main_path);

    // open both documents and drain initial diagnostics
    test.harness.did_open(lib_uri.clone(), lib_text).await;
    test.harness.did_open(main_uri.clone(), main_text).await;
    let _ = test.harness.next_diagnostics_for(&lib_uri).await;
    let _ = test.harness.next_diagnostics_for(&main_uri).await;

    // request definition on the imported symbol within the import clause
    let definition = test
        .goto_definition(
            &main_uri,
            lsp::Position {
                line: 0,
                character: 10,
            },
        )
        .await
        .expect("expected goto definition result");
    let location = first_definition_location(&definition).expect("expected definition location");
    assert_location_matches_path(&location, &lib_path);
    assert_eq!(location.range.start.line, 0);
    assert_eq!(location.range.start.character, 13);
}

/// Definition resolves across nested workspace files with host-like source layout.
#[tokio::test]
async fn test_lsp_navigation_resolves_nested_import_symbol() {
    let mut test = TestLsp::new("navigation_nested_import_symbol").await;
    let main_text = "import { value } from \"./lib\";\nfunction compute(input: number): number {\n    return input + value;\n}\nconst output = compute(1);\n";
    let lib_text = "export const value = 1;\n";
    let main_path = test.write_text("nested/main.ds", main_text);
    let lib_path = test.write_text("nested/lib.ds", lib_text);
    let main_uri = uri_for_path(&main_path);
    let lib_uri = uri_for_path(&lib_path);

    // open both files and drain diagnostics before querying definitions
    test.harness.did_open(main_uri.clone(), main_text).await;
    test.harness.did_open(lib_uri.clone(), lib_text).await;
    let _ = test.harness.next_diagnostics_for(&main_uri).await;
    let _ = test.harness.next_diagnostics_for(&lib_uri).await;

    // resolve the imported value reference in the function body
    let definition = test
        .goto_definition(
            &main_uri,
            lsp::Position {
                line: 2,
                character: 20,
            },
        )
        .await
        .expect("expected goto definition result");
    let location = first_definition_location(&definition).expect("expected definition location");
    assert_location_matches_path(&location, &lib_path);
    assert_eq!(location.range.start.line, 0);
    assert_eq!(location.range.start.character, 13);
    assert_eq!(location.range.end.line, 0);
    assert_eq!(location.range.end.character, 18);
}

/// Definition resolves for files created after initialize in a nested project.
#[tokio::test]
async fn test_lsp_navigation_resolves_nested_import_symbol_after_create() {
    let mut test = TestLsp::new("navigation_nested_import_after_create").await;
    let main_text = "import { value } from \"./lib\";\nfunction compute(input: number): number {\n    return input + value;\n}\nconst output = compute(1);\n";
    let lib_text = "export const value = 1;\n";

    // write files after server initialize to mirror real host workflow
    let main_path = test.write_text("nested-create/main.ds", main_text);
    let lib_path = test.write_text("nested-create/lib.ds", lib_text);
    let main_uri = uri_for_path(&main_path);
    let lib_uri = uri_for_path(&lib_path);

    // open both files and drain diagnostics before querying definitions
    test.harness.did_open(main_uri.clone(), main_text).await;
    test.harness.did_open(lib_uri.clone(), lib_text).await;
    let _ = test.harness.next_diagnostics_for(&main_uri).await;
    let _ = test.harness.next_diagnostics_for(&lib_uri).await;

    // resolve the imported value reference in the function body
    let definition = test
        .goto_definition(
            &main_uri,
            lsp::Position {
                line: 2,
                character: 20,
            },
        )
        .await
        .expect("expected goto definition result");
    let location = first_definition_location(&definition).expect("expected definition location");
    assert_location_matches_path(&location, &lib_path);
    assert_eq!(location.range.start.line, 0);
    assert_eq!(location.range.start.character, 13);
    assert_eq!(location.range.end.line, 0);
    assert_eq!(location.range.end.character, 18);
}

/// Execute command requests must not starve definition requests.
#[tokio::test]
async fn test_lsp_execute_command_keeps_definition_requests_responsive() {
    // create a multi-file fixture with many extra modules to keep rescans busy
    let mut test = TestLsp::new("command_definition_responsive").await;
    let lib_text = "export const value: number = 1;\n";
    let main_text =
        "import { value } from \"./lib\";\nconst output: number = value + 1;\nexport { output };\n";
    let lib_path = test.write_text("lib.ds", lib_text);
    let main_path = test.write_text("main.ds", main_text);
    for index in 0..256 {
        let file_name = format!("extra_{index}.ds");
        let _ = test.write_text(&file_name, "export const marker: number = 1;\n");
    }

    // open both source files and drain initial diagnostics
    let lib_uri = uri_for_path(&lib_path);
    let main_uri = uri_for_path(&main_path);
    test.harness.did_open(lib_uri.clone(), lib_text).await;
    test.harness.did_open(main_uri.clone(), main_text).await;
    let _ = test.harness.next_diagnostics_for(&lib_uri).await;
    let _ = test.harness.next_diagnostics_for(&main_uri).await;

    // enqueue a workspace reload command
    let command_response: Option<lsp::LSPAny> = test
        .harness
        .request_result(
            "workspace/executeCommand",
            lsp::ExecuteCommandParams {
                command: "destack.reload".to_string(),
                arguments: Vec::new(),
                work_done_progress_params: lsp::WorkDoneProgressParams {
                    work_done_token: None,
                },
            },
        )
        .await;
    assert!(command_response.is_none());

    // repeatedly request definitions while the command work is in flight
    for _ in 0..8 {
        let definition = test
            .goto_definition(
                &main_uri,
                lsp::Position {
                    line: 1,
                    character: 24,
                },
            )
            .await
            .expect("expected goto definition result");
        let location =
            first_definition_location(&definition).expect("expected definition location");
        assert_location_matches_path(&location, &lib_path);
        assert_eq!(location.range.start.line, 0);
        assert_eq!(location.range.start.character, 13);
    }
}

/// Workspace diagnostics and definitions remain responsive during multi-file edits.
#[tokio::test]
async fn test_lsp_workspace_diagnostic_and_definition_remain_responsive_during_edits() {
    // set up a two-file workspace with import resolution through lib.ds
    let mut test = TestLsp::new("workspace_definition_responsive").await;
    let initial_lib_text = "export const value: number = 1;\n";
    let initial_main_text =
        "import { value } from \"./lib\";\nconst output: number = value + 1;\nexport { output };\n";
    let lib_path = test.write_text("lib.ds", initial_lib_text);
    let main_path = test.write_text("main.ds", initial_main_text);
    let lib_uri = uri_for_path(&lib_path);
    let main_uri = uri_for_path(&main_path);

    // open and drain initial diagnostics
    test.harness
        .did_open(lib_uri.clone(), initial_lib_text)
        .await;
    test.harness
        .did_open(main_uri.clone(), initial_main_text)
        .await;
    let _ = test.harness.next_diagnostics_for(&lib_uri).await;
    let _ = test.harness.next_diagnostics_for(&main_uri).await;

    // run interleaved edit and diagnostic cycles
    for iteration in 0..4 {
        let next_lib_value = iteration + 2;
        let next_main_output = iteration + 3;
        let lib_text = format!("export const value: number = {next_lib_value};\n");
        let main_text = format!(
            "import {{ value }} from \"./lib\";\nconst output: number = value + {next_main_output};\nexport {{ output }};\n"
        );

        // apply one edit per file and verify both diagnostics stay clean
        test.harness
            .did_change(lib_uri.clone(), &lib_text, iteration + 2)
            .await;
        test.harness
            .did_change(main_uri.clone(), &main_text, iteration + 2)
            .await;
        let lib_diagnostics = test.harness.next_diagnostics_for(&lib_uri).await;
        let main_diagnostics = test.harness.next_diagnostics_for(&main_uri).await;
        assert!(lib_diagnostics.diagnostics.is_empty());
        assert!(main_diagnostics.diagnostics.is_empty());

        // request workspace diagnostics and verify both files are present
        let workspace_request = request_with_params(
            "workspace/diagnostic",
            300 + iteration as i64,
            lsp::WorkspaceDiagnosticParams {
                identifier: None,
                previous_result_ids: Vec::new(),
                work_done_progress_params: lsp::WorkDoneProgressParams {
                    work_done_token: None,
                },
                partial_result_params: lsp::PartialResultParams {
                    partial_result_token: None,
                },
            },
        );
        let workspace_response = test
            .harness
            .call(workspace_request)
            .await
            .expect("workspace diagnostic response");
        assert!(workspace_response.is_ok());
        let workspace_report: lsp::WorkspaceDiagnosticReportResult =
            decode_response_result(&workspace_response);
        let lsp::WorkspaceDiagnosticReportResult::Report(workspace_report) = workspace_report
        else {
            panic!("expected workspace diagnostic report");
        };
        let has_lib = workspace_report.items.iter().any(|item| match item {
            lsp::WorkspaceDocumentDiagnosticReport::Full(full) => full.uri == lib_uri,
            lsp::WorkspaceDocumentDiagnosticReport::Unchanged(unchanged) => {
                unchanged.uri == lib_uri
            }
        });
        let has_main = workspace_report.items.iter().any(|item| match item {
            lsp::WorkspaceDocumentDiagnosticReport::Full(full) => full.uri == main_uri,
            lsp::WorkspaceDocumentDiagnosticReport::Unchanged(unchanged) => {
                unchanged.uri == main_uri
            }
        });
        assert!(has_lib);
        assert!(has_main);

        // verify definitions still resolve to the lib symbol after each edit cycle
        let definition = test
            .goto_definition(
                &main_uri,
                lsp::Position {
                    line: 1,
                    character: 24,
                },
            )
            .await
            .expect("expected goto definition result");
        let location =
            first_definition_location(&definition).expect("expected definition location");
        assert_location_matches_path(&location, &lib_path);
        assert_eq!(location.range.start.line, 0);
        assert_eq!(location.range.start.character, 13);
    }
}

/// Document diagnostics include open file updates.
#[tokio::test]
async fn test_lsp_document_diagnostic_reflects_open_virtual_content() {
    let fs = test_fs("document_diagnostic_virtual");
    let path = fs
        .write_text("main.ds", "export const x: number = 1;\n")
        .expect("write main");

    let mut harness = harness_for_fs(&fs).await;
    let uri = uri_for_path(&path);
    harness.did_open(uri.clone(), "export const x = ;\n").await;
    let pushed = harness.next_diagnostics_for(&uri).await;
    assert!(!pushed.diagnostics.is_empty());

    let request = request_with_params(
        "textDocument/diagnostic",
        12,
        lsp::DocumentDiagnosticParams {
            text_document: lsp::TextDocumentIdentifier::new(uri),
            identifier: Some("destack".to_string()),
            previous_result_id: None,
            work_done_progress_params: lsp::WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: lsp::PartialResultParams {
                partial_result_token: None,
            },
        },
    );
    let response = harness
        .call(request)
        .await
        .expect("document diagnostic response");
    assert!(response.is_ok());

    let report: lsp::DocumentDiagnosticReportResult = decode_response_result(&response);
    let lsp::DocumentDiagnosticReportResult::Report(report) = report else {
        panic!("expected full document diagnostic report");
    };
    let lsp::DocumentDiagnosticReport::Full(full) = report else {
        panic!("expected full diagnostics");
    };
    assert!(!full.full_document_diagnostic_report.items.is_empty());
}

/// Closing a document restores filesystem backed diagnostics.
#[tokio::test]
async fn test_lsp_did_close_restores_filesystem_diagnostics() {
    let fs = test_fs("did_close_restore");
    let path = fs
        .write_text("main.ds", "export const x: number = 1;\n")
        .expect("write main");

    let mut harness = harness_for_fs(&fs).await;
    let uri = uri_for_path(&path);
    harness.did_open(uri.clone(), "export const x = ;\n").await;

    let opened = harness.next_diagnostics_for(&uri).await;
    assert!(!opened.diagnostics.is_empty());

    harness.did_close(uri.clone()).await;

    let closed = harness.next_diagnostics_for(&uri).await;
    assert!(closed.diagnostics.is_empty());
}

/// LSP watched file changes publish diagnostics.
#[tokio::test]
async fn test_lsp_watched_file_change_publishes_diagnostics() {
    let fs = test_fs("watched_change");
    let path = fs.write_text("main.ds", "export const x = ;\n").unwrap();

    let mut harness = harness_for_fs(&fs).await;

    let uri = uri_for_path(&path);
    harness
        .did_change_watched(uri.clone(), lsp::FileChangeType::CHANGED)
        .await;

    let diagnostics = harness.next_diagnostics_for(&uri).await;

    // check that the diagnostics are reported for the changed file
    assert_eq!(diagnostics.uri, uri);
    assert!(!diagnostics.diagnostics.is_empty());
}

/// LSP didCreateFiles publishes diagnostics for new files.
#[tokio::test]
async fn test_lsp_did_create_publishes_diagnostics() {
    let fs = test_fs("did_create");
    let path = fs.write_text("created.ds", "export const x = ;\n").unwrap();

    let mut harness = harness_for_fs(&fs).await;

    let uri = uri_for_path(&path);
    harness.did_create(uri.clone()).await;

    let diagnostics = harness.next_diagnostics_for(&uri).await;

    // check that the diagnostics are reported for the created file
    assert_eq!(diagnostics.uri, uri);
    assert!(!diagnostics.diagnostics.is_empty());
}

/// LSP didDeleteFiles clears diagnostics for removed files.
#[tokio::test]
async fn test_lsp_did_delete_clears_diagnostics() {
    let fs = test_fs("did_delete");
    let path = fs.write_text("deleted.ds", "export const x = ;\n").unwrap();

    let mut harness = harness_for_fs(&fs).await;

    let uri = uri_for_path(&path);
    harness.did_create(uri.clone()).await;

    let created = harness.next_diagnostics_for(&uri).await;
    assert_eq!(created.uri, uri);
    assert!(!created.diagnostics.is_empty());

    fs.remove_file(&path).unwrap();

    harness.did_delete(uri.clone()).await;

    let deleted = harness.next_diagnostics_for(&uri).await;

    // check that the diagnostics clear after delete
    assert_eq!(deleted.uri, uri);
    assert!(deleted.diagnostics.is_empty());
}

/// LSP didRenameFiles clears old diagnostics and publishes new ones.
#[tokio::test]
async fn test_lsp_did_rename_updates_diagnostics() {
    let fs = test_fs("did_rename");
    let old_path = fs.write_text("old.ds", "export const x = ;\n").unwrap();
    let new_path = fs.path_for("new.ds");

    let mut harness = harness_for_fs(&fs).await;

    let old_uri = uri_for_path(&old_path);
    let new_uri = uri_for_path(&new_path);
    harness.did_create(old_uri.clone()).await;

    let created = harness.next_diagnostics_for(&old_uri).await;
    assert_eq!(created.uri, old_uri);
    assert!(!created.diagnostics.is_empty());

    fs.rename(&old_path, &new_path).unwrap();

    harness.did_rename(old_uri.clone(), new_uri.clone()).await;

    // check that the old is cleared, new has diagnostics
    let old_diagnostics = harness.next_diagnostics_for(&old_uri).await;
    let new_diagnostics = harness.next_diagnostics_for(&new_uri).await;
    assert!(old_diagnostics.diagnostics.is_empty());
    assert!(!new_diagnostics.diagnostics.is_empty());
}

/// LSP registers file watchers during initialization.
#[tokio::test]
async fn test_lsp_registers_file_watchers() {
    let fs = test_fs("watch_register");
    let mut harness = harness_for_fs(&fs).await;

    let params = harness.next_register_capability().await;
    let registration = params
        .registrations
        .iter()
        .find(|registration| registration.method == "workspace/didChangeWatchedFiles")
        .expect("missing watched files registration");
    let options = registration
        .register_options
        .clone()
        .expect("missing watch registration options");

    let watchers = options
        .get("watchers")
        .and_then(|value| value.as_array())
        .expect("missing watchers");
    let patterns: Vec<String> = watchers
        .iter()
        .filter_map(|watcher| watcher.get("globPattern").and_then(|value| value.as_str()))
        .map(|pattern| pattern.to_string())
        .collect();

    // check that the expected watch patterns are registered
    assert!(patterns.iter().any(|pattern| pattern == "**/*.ds"));
    assert!(patterns.iter().any(|pattern| pattern == "**/destack.json"));
}

/// LSP config changes publish diagnostics for invalidated modules.
#[tokio::test]
async fn test_lsp_config_change_fanout_publishes_diagnostics() {
    // set up the workspace root with destack.json
    let fs = test_fs("config_fanout");
    let destack_config_path = fs
        .write_text("destack.json", "{ \"compiler\": {} }\n")
        .unwrap();

    // write modules with invalid syntax
    let module_a = fs.write_text("a.ds", "export const a = ;\n").unwrap();
    let module_b = fs.write_text("b.ds", "export const b = ;\n").unwrap();

    // initialize the server
    let mut harness = harness_for_fs(&fs).await;

    // open both modules to register them with the workspace
    let uri_a = uri_for_path(&module_a);
    let uri_b = uri_for_path(&module_b);
    harness
        .did_open(uri_a.clone(), "export const a = ;\n")
        .await;
    harness
        .did_open(uri_b.clone(), "export const b = ;\n")
        .await;

    // open destack.json for edits
    let destack_config_uri = uri_for_path(&destack_config_path);
    harness
        .did_open(destack_config_uri.clone(), "{ \"compiler\": {} }\n")
        .await;

    // drain initial diagnostics from didOpen
    let _ = harness.next_diagnostics_for(&uri_a).await;
    let _ = harness.next_diagnostics_for(&uri_b).await;
    let _ = harness.next_diagnostics_for(&destack_config_uri).await;

    // update destack.json to trigger module invalidation
    fs.write_text(
        &destack_config_path,
        "{ \"compiler\": { \"noThrow\": true } }\n",
    )
    .unwrap();
    harness
        .did_change(
            destack_config_uri.clone(),
            "{ \"compiler\": { \"noThrow\": true } }\n",
            2,
        )
        .await;

    // diagnostics are published for both modules
    let diagnostics_a = harness.next_diagnostics_for(&uri_a).await;
    let diagnostics_b = harness.next_diagnostics_for(&uri_b).await;
    assert_eq!(diagnostics_a.uri, uri_a);
    assert_eq!(diagnostics_b.uri, uri_b);
}

/// LSP updates stay scoped to the workspace root.
#[tokio::test]
async fn test_lsp_multi_root_scopes_diagnostics() {
    // set up two workspace roots
    let fs_a = test_fs("multi_root_a");
    let fs_b = test_fs("multi_root_b");
    let root_a = fs_a.root().to_path_buf();
    let root_b = fs_b.root().to_path_buf();
    let _ = fs_a.write_text("destack.json", "{ \"compiler\": {} }\n");
    let _ = fs_b.write_text("destack.json", "{ \"compiler\": {} }\n");

    // write invalid modules in both roots
    let module_a = fs_a.write_text("a.ds", "export const a = ;\n").unwrap();
    let module_b = fs_b.write_text("b.ds", "export const b = ;\n").unwrap();

    // initialize the server with the first root
    let mut harness = LspHarness::new(root_a.clone());
    harness.initialize().await;

    // add the second root as a workspace folder
    harness
        .did_change_workspace_folders(vec![root_b.clone()], vec![])
        .await;

    // open both modules to register them with the workspace
    let uri_a = uri_for_path(&module_a);
    let uri_b = uri_for_path(&module_b);
    harness
        .did_open(uri_a.clone(), "export const a = ;\n")
        .await;
    harness
        .did_open(uri_b.clone(), "export const b = ;\n")
        .await;

    // drain initial diagnostics
    let diagnostics_a = harness.next_diagnostics_for(&uri_a).await;
    let diagnostics_b = harness.next_diagnostics_for(&uri_b).await;
    assert!(!diagnostics_a.diagnostics.is_empty());
    assert!(!diagnostics_b.diagnostics.is_empty());

    // fix the second module
    harness
        .did_change(uri_b.clone(), "export const b: number = 1;\n", 2)
        .await;
    let diagnostics_b = harness.next_diagnostics_for(&uri_b).await;
    assert!(diagnostics_b.diagnostics.is_empty());

    // ensure no extra diagnostics are published for the first root
    let remaining = harness
        .collect_diagnostics_for_timeout(Duration::from_millis(200))
        .await;
    assert!(!remaining.iter().any(|entry| entry.uri == uri_a));
}

/// LSP initialize workspace folders register multi-root state.
#[tokio::test]
async fn test_lsp_initialize_workspace_folders_scopes_diagnostics() {
    // set up two workspace roots
    let fs_a = test_fs("multi_root_initialize_a");
    let fs_b = test_fs("multi_root_initialize_b");
    let root_a = fs_a.root().to_path_buf();
    let root_b = fs_b.root().to_path_buf();
    let _ = fs_a.write_text("destack.json", "{ \"compiler\": {} }\n");
    let _ = fs_b.write_text("destack.json", "{ \"compiler\": {} }\n");

    // write invalid modules in both roots
    let module_a = fs_a.write_text("a.ds", "export const a = ;\n").unwrap();
    let module_b = fs_b.write_text("b.ds", "export const b = ;\n").unwrap();

    // initialize the server with workspace folders at startup
    let mut harness = LspHarness::new(root_a.clone());
    #[allow(deprecated)]
    let params = lsp::InitializeParams {
        root_uri: Some(uri_for_path(&root_a)),
        workspace_folders: Some(vec![
            lsp::WorkspaceFolder {
                uri: uri_for_path(&root_a),
                name: "multi-root-a".to_string(),
            },
            lsp::WorkspaceFolder {
                uri: uri_for_path(&root_b),
                name: "multi-root-b".to_string(),
            },
        ]),
        ..Default::default()
    };
    harness.initialize_with_params(params).await;

    // open both modules to register them with the workspace
    let uri_a = uri_for_path(&module_a);
    let uri_b = uri_for_path(&module_b);
    harness
        .did_open(uri_a.clone(), "export const a = ;\n")
        .await;
    harness
        .did_open(uri_b.clone(), "export const b = ;\n")
        .await;

    // drain initial diagnostics
    let diagnostics_a = harness.next_diagnostics_for(&uri_a).await;
    let diagnostics_b = harness.next_diagnostics_for(&uri_b).await;
    assert!(!diagnostics_a.diagnostics.is_empty());
    assert!(!diagnostics_b.diagnostics.is_empty());

    // fix the second module
    harness
        .did_change(uri_b.clone(), "export const b: number = 1;\n", 2)
        .await;
    let diagnostics_b = harness.next_diagnostics_for(&uri_b).await;
    assert!(diagnostics_b.diagnostics.is_empty());

    // ensure no extra diagnostics are published for the first root
    let remaining = harness
        .collect_diagnostics_for_timeout(Duration::from_millis(200))
        .await;
    assert!(!remaining.iter().any(|entry| entry.uri == uri_a));
}

/// LSP willRenameFiles rejects multi-root rename batches for revision consistency.
#[tokio::test]
async fn test_lsp_will_rename_files_rejects_multi_root_batches() {
    // set up two workspace roots with importable modules
    let fs_a = test_fs("will_rename_multi_root_a");
    let fs_b = test_fs("will_rename_multi_root_b");
    let root_a = fs_a.root().to_path_buf();
    let root_b = fs_b.root().to_path_buf();
    let _ = fs_a.write_text("destack.json", "{ \"compiler\": {} }\n");
    let _ = fs_b.write_text("destack.json", "{ \"compiler\": {} }\n");
    let dep_a = fs_a
        .write_text("dep.ds", "export const dep = 1;\n")
        .unwrap();
    let main_a = fs_a
        .write_text(
            "main.ds",
            "import { dep } from \"./dep\";\nconst value = dep;\n",
        )
        .unwrap();
    let dep_b = fs_b
        .write_text("dep.ds", "export const dep = 2;\n")
        .unwrap();
    let main_b = fs_b
        .write_text(
            "main.ds",
            "import { dep } from \"./dep\";\nconst value = dep;\n",
        )
        .unwrap();

    // initialize the server with both roots
    let mut harness = LspHarness::new(root_a.clone());
    #[allow(deprecated)]
    let params = lsp::InitializeParams {
        root_uri: Some(uri_for_path(&root_a)),
        workspace_folders: Some(vec![
            lsp::WorkspaceFolder {
                uri: uri_for_path(&root_a),
                name: "will-rename-a".to_string(),
            },
            lsp::WorkspaceFolder {
                uri: uri_for_path(&root_b),
                name: "will-rename-b".to_string(),
            },
        ]),
        ..Default::default()
    };
    harness.initialize_with_params(params).await;

    // open both roots to warm query state for rename previews
    let dep_a_uri = uri_for_path(&dep_a);
    let dep_b_uri = uri_for_path(&dep_b);
    let main_a_uri = uri_for_path(&main_a);
    let main_b_uri = uri_for_path(&main_b);
    harness
        .did_open(dep_a_uri.clone(), "export const dep = 1;\n")
        .await;
    harness
        .did_open(dep_b_uri.clone(), "export const dep = 2;\n")
        .await;
    harness
        .did_open(
            main_a_uri.clone(),
            "import { dep } from \"./dep\";\nconst value = dep;\n",
        )
        .await;
    harness
        .did_open(
            main_b_uri.clone(),
            "import { dep } from \"./dep\";\nconst value = dep;\n",
        )
        .await;
    let _ = harness.next_diagnostics_for(&dep_a_uri).await;
    let _ = harness.next_diagnostics_for(&dep_b_uri).await;
    let _ = harness.next_diagnostics_for(&main_a_uri).await;
    let _ = harness.next_diagnostics_for(&main_b_uri).await;

    // single-root rename batches still return workspace edits
    let dep_a_new = root_a.join("dep_renamed.ds");
    let single_request = request_with_params(
        "workspace/willRenameFiles",
        41,
        lsp::RenameFilesParams {
            files: vec![lsp::FileRename {
                old_uri: uri_for_path(&dep_a).to_string(),
                new_uri: uri_for_path(&dep_a_new).to_string(),
            }],
        },
    );
    let single_response = harness
        .call(single_request)
        .await
        .expect("single-root willRenameFiles response");
    assert!(single_response.is_ok());
    let _single_edit: Option<lsp::WorkspaceEdit> = decode_response_result(&single_response);

    // multi-root rename batches are rejected to keep revision checks coherent
    let dep_b_new = root_b.join("dep_renamed.ds");
    let multi_request = request_with_params(
        "workspace/willRenameFiles",
        42,
        lsp::RenameFilesParams {
            files: vec![
                lsp::FileRename {
                    old_uri: uri_for_path(&dep_a).to_string(),
                    new_uri: uri_for_path(&dep_a_new).to_string(),
                },
                lsp::FileRename {
                    old_uri: uri_for_path(&dep_b).to_string(),
                    new_uri: uri_for_path(&dep_b_new).to_string(),
                },
            ],
        },
    );
    let multi_response = harness
        .call(multi_request)
        .await
        .expect("multi-root willRenameFiles response");
    assert!(multi_response.is_ok());
    let multi_edit: Option<lsp::WorkspaceEdit> = decode_response_result(&multi_response);
    assert!(multi_edit.is_none());
    let rejection_log = next_log_message_containing(
        &mut harness,
        "destack.will_rename_files.multi_root_rejected",
    )
    .await;
    assert_eq!(rejection_log.typ, lsp::MessageType::WARNING);
}

/// Workspace diagnostics stream partial results.
#[tokio::test]
async fn test_lsp_workspace_diagnostic_streams_partial_results() {
    let fs = test_fs("workspace_partial_results");
    let path = fs
        .write_text("main.ds", "export const x = ;\n")
        .expect("write module");

    let mut harness = harness_for_fs(&fs).await;
    let uri = uri_for_path(&path);
    harness.did_open(uri.clone(), "export const x = ;\n").await;
    let _ = harness.next_diagnostics_for(&uri).await;

    let params = lsp::WorkspaceDiagnosticParams {
        identifier: None,
        previous_result_ids: Vec::new(),
        work_done_progress_params: lsp::WorkDoneProgressParams {
            work_done_token: None,
        },
        partial_result_params: lsp::PartialResultParams {
            partial_result_token: Some(lsp::ProgressToken::Number(1)),
        },
    };
    let request = request_with_params("workspace/diagnostic", 10, params);
    let response = harness.call(request).await.expect("diagnostic response");
    assert!(response.is_ok());

    let progress = next_partial_progress(&mut harness).await;
    let report: lsp::WorkspaceDiagnosticReportPartialResult =
        serde_json::from_value(progress).expect("decode partial diagnostics");
    assert!(!report.items.is_empty());
}

/// Workspace diagnostics include open virtual file diagnostics.
#[tokio::test]
async fn test_lsp_workspace_diagnostic_reflects_open_virtual_content() {
    let fs = test_fs("workspace_diagnostic_virtual");
    let path = fs
        .write_text("main.ds", "export const x: number = 1;\n")
        .expect("write main");

    let mut harness = harness_for_fs(&fs).await;
    let uri = uri_for_path(&path);
    harness.did_open(uri.clone(), "export const x = ;\n").await;
    let _ = harness.next_diagnostics_for(&uri).await;

    let params = lsp::WorkspaceDiagnosticParams {
        identifier: None,
        previous_result_ids: Vec::new(),
        work_done_progress_params: lsp::WorkDoneProgressParams {
            work_done_token: None,
        },
        partial_result_params: lsp::PartialResultParams {
            partial_result_token: None,
        },
    };
    let request = request_with_params("workspace/diagnostic", 13, params);
    let response = harness
        .call(request)
        .await
        .expect("workspace diagnostic response");
    assert!(response.is_ok());

    let report: lsp::WorkspaceDiagnosticReportResult = decode_response_result(&response);
    let lsp::WorkspaceDiagnosticReportResult::Report(report) = report else {
        panic!("expected full workspace diagnostic report");
    };
    let target = report
        .items
        .into_iter()
        .find_map(|item| match item {
            lsp::WorkspaceDocumentDiagnosticReport::Full(full) if full.uri == uri => Some(full),
            _ => None,
        })
        .expect("missing workspace diagnostics for opened uri");
    assert!(!target.full_document_diagnostic_report.items.is_empty());
}

/// Selection ranges stream partial results.
#[tokio::test]
async fn test_lsp_selection_range_streams_partial_results() {
    let fs = test_fs("selection_range_partial");
    let main_path = fs
        .write_text("main.ds", "export const foo = 1 + 2;\n")
        .expect("write main");

    let mut harness = harness_for_fs(&fs).await;
    let uri = uri_for_path(&main_path);
    harness
        .did_open(uri.clone(), "export const foo = 1 + 2;\n")
        .await;
    let _ = harness.next_diagnostics_for(&uri).await;

    let params = lsp::SelectionRangeParams {
        text_document: lsp::TextDocumentIdentifier::new(uri.clone()),
        positions: vec![
            lsp::Position {
                line: 0,
                character: 0,
            },
            lsp::Position {
                line: 0,
                character: 10,
            },
        ],
        work_done_progress_params: lsp::WorkDoneProgressParams {
            work_done_token: None,
        },
        partial_result_params: lsp::PartialResultParams {
            partial_result_token: Some(lsp::ProgressToken::Number(7)),
        },
    };
    let request = request_with_params("textDocument/selectionRange", 11, params);
    let response = harness
        .call(request)
        .await
        .expect("selection range response");
    assert!(response.is_ok());

    let progress = next_partial_progress(&mut harness).await;
    let ranges: Vec<lsp::SelectionRange> =
        serde_json::from_value(progress).expect("decode selection ranges");
    assert!(!ranges.is_empty());
}

/// Workspace diagnostics honor cancel requests.
#[tokio::test]
async fn test_lsp_workspace_diagnostic_cancels() {
    let fs = test_fs("workspace_cancel");

    // seed many invalid files before initialize so cancellation timing
    // reflects workspace diagnostics work, not repeated didOpen recompiles
    for index in 0..256 {
        let name = format!("file_{index}.ds");
        fs.write_text(&name, "export const x = ;\n")
            .expect("write invalid workspace file");
    }
    let harness = harness_for_fs(&fs).await;

    // request workspace diagnostics with work done cancellation
    let work_done_token = lsp::ProgressToken::Number(2);
    let diag_params = lsp::WorkspaceDiagnosticParams {
        identifier: None,
        previous_result_ids: Vec::new(),
        work_done_progress_params: lsp::WorkDoneProgressParams {
            work_done_token: Some(work_done_token.clone()),
        },
        partial_result_params: lsp::PartialResultParams {
            partial_result_token: None,
        },
    };

    let result = harness
        .workspace_diagnostic_with_cancellation(
            diag_params,
            work_done_token,
            32,
            Duration::from_millis(1),
        )
        .await;

    assert_eq!(
        result.unwrap_err(),
        destack_lsp_server::jsonrpc::Error::request_cancelled()
    );
}

/// References honor work-done progress cancellation even below chunk boundaries.
#[tokio::test]
async fn test_lsp_references_progress_cancel_cancels_before_chunk_boundary() {
    // set up a large references workload in one open file
    let fs = test_fs("references_progress_cancel");

    let mut source = String::new();
    source.push_str("function target_value(): number {\n");
    source.push_str("  return 1;\n");
    source.push_str("}\n");
    for index in 0..4000 {
        source.push_str(&format!("const ref_{index} = target_value();\n"));
    }

    let path = fs
        .write_text("main.ds", &source)
        .expect("write large references source");
    let uri = uri_for_path(&path);
    let mut harness = harness_for_fs(&fs).await;
    harness.did_open(uri.clone(), &source).await;
    let _ = harness.next_diagnostics_for(&uri).await;

    // start references request with a work-done token
    let work_done_token = lsp::ProgressToken::String("refs-cancel".to_string());
    let request_id = harness
        .start_request(
            "textDocument/references",
            lsp::ReferenceParams {
                text_document_position: lsp::TextDocumentPositionParams {
                    text_document: lsp::TextDocumentIdentifier::new(uri.clone()),
                    position: lsp::Position::new(0, 10),
                },
                work_done_progress_params: lsp::WorkDoneProgressParams {
                    work_done_token: Some(work_done_token.clone()),
                },
                partial_result_params: lsp::PartialResultParams {
                    partial_result_token: None,
                },
                context: lsp::ReferenceContext {
                    include_declaration: true,
                },
            },
        )
        .await;

    // wait for begin and then cancel the work-done token
    next_work_done_progress_kind(&mut harness, &work_done_token, "begin").await;
    harness.cancel_work_done_progress(work_done_token).await;

    // verify request cancellation response
    let response = harness
        .await_request(request_id)
        .await
        .expect("expected references response");
    let error = response.error().expect("expected cancellation error");
    assert_eq!(error.code, ErrorCode::RequestCancelled);
}

/// Code action resolve hydrates workspace edits from lazy data payloads.
#[tokio::test]
async fn test_lsp_code_action_resolve_hydrates_edit() {
    // set up and initialize a workspace
    let fs = test_fs("code_action_resolve");
    let mut harness = LspHarness::new(fs.root().to_path_buf());
    #[allow(deprecated)]
    let params = lsp::InitializeParams {
        root_uri: Some(uri_for_path(fs.root())),
        capabilities: lsp::ClientCapabilities {
            text_document: Some(lsp::TextDocumentClientCapabilities {
                code_action: Some(lsp::CodeActionClientCapabilities {
                    data_support: Some(true),
                    resolve_support: Some(lsp::CodeActionCapabilityResolveSupport {
                        properties: vec!["edit".to_string()],
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    harness.initialize_with_params(params).await;

    // request resolution for a lazy action payload
    let unresolved_action = lsp::CodeAction {
        title: "organize imports".to_string(),
        kind: Some(lsp::CodeActionKind::SOURCE_ORGANIZE_IMPORTS),
        diagnostics: None,
        edit: None,
        command: None,
        is_preferred: Some(false),
        disabled: None,
        data: Some(serde_json::json!({
            "edits": {
                "files": []
            }
        })),
    };
    let resolve_request = request_with_params("codeAction/resolve", 21, unresolved_action);
    let resolve_response = harness
        .call(resolve_request)
        .await
        .expect("code action resolve response");
    assert!(resolve_response.is_ok());
    let resolved: lsp::CodeAction = decode_response_result(&resolve_response);
    assert!(resolved.edit.is_some());
}

/// Code action resolve preserves existing edits.
#[tokio::test]
async fn test_lsp_code_action_resolve_keeps_existing_edit() {
    // initialize a harness for code action resolve requests
    let fs = test_fs("code_action_resolve_existing");
    let mut harness = harness_for_fs(&fs).await;

    // resolve an action that already carries workspace edits
    let existing_edit = lsp::WorkspaceEdit::default();
    let unresolved_action = lsp::CodeAction {
        title: "organize imports".to_string(),
        kind: Some(lsp::CodeActionKind::SOURCE_ORGANIZE_IMPORTS),
        diagnostics: None,
        edit: Some(existing_edit.clone()),
        command: None,
        is_preferred: Some(false),
        disabled: None,
        data: Some(serde_json::json!({
            "edits": {
                "files": []
            }
        })),
    };
    let request = request_with_params("codeAction/resolve", 42, unresolved_action);
    let response = harness.call(request).await.expect("code action resolve");
    assert!(response.is_ok());
    let resolved: lsp::CodeAction = decode_response_result(&response);

    // verify the resolve path leaves eager edits unchanged
    assert_eq!(resolved.edit, Some(existing_edit));
    assert!(resolved.data.is_some());
}

/// Code action resolve preserves unrecognized resolve payloads.
#[tokio::test]
async fn test_lsp_code_action_resolve_keeps_unrecognized_data() {
    // initialize a harness for code action resolve requests
    let fs = test_fs("code_action_resolve_unrecognized");
    let mut harness = harness_for_fs(&fs).await;

    // resolve an action whose data does not match the resolve payload schema
    let unresolved_action = lsp::CodeAction {
        title: "organize imports".to_string(),
        kind: Some(lsp::CodeActionKind::SOURCE_ORGANIZE_IMPORTS),
        diagnostics: None,
        edit: None,
        command: None,
        is_preferred: Some(false),
        disabled: None,
        data: Some(serde_json::json!({
            "unexpected": true
        })),
    };
    let request = request_with_params("codeAction/resolve", 43, unresolved_action);
    let response = harness.call(request).await.expect("code action resolve");
    assert!(response.is_ok());
    let resolved: lsp::CodeAction = decode_response_result(&response);

    // verify unmatched payload data is retained with no synthetic edit
    assert!(resolved.edit.is_none());
    assert_eq!(
        resolved.data,
        Some(serde_json::json!({ "unexpected": true }))
    );
}

/// Semantic token full delta returns a delta payload for unchanged and changed requests.
#[tokio::test]
async fn test_lsp_semantic_tokens_full_delta_tracks_changes() {
    // set up a module with semantic token output
    let fs = test_fs("semantic_delta");
    let path = fs
        .write_text("main.ds", "export const value = 1;\n")
        .expect("write module");
    let uri = uri_for_path(&path);
    let mut harness = harness_for_fs(&fs).await;

    // open the module and drain diagnostics
    harness
        .did_open(uri.clone(), "export const value = 1;\n")
        .await;
    let _ = harness.next_diagnostics_for(&uri).await;

    // request the baseline semantic tokens
    let full_request = request_with_params(
        "textDocument/semanticTokens/full",
        30,
        lsp::SemanticTokensParams {
            text_document: lsp::TextDocumentIdentifier::new(uri.clone()),
            work_done_progress_params: lsp::WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: lsp::PartialResultParams {
                partial_result_token: None,
            },
        },
    );
    let full_response = harness
        .call(full_request)
        .await
        .expect("semantic full response");
    assert!(full_response.is_ok());
    let full_result: lsp::SemanticTokensResult = decode_response_result(&full_response);
    let lsp::SemanticTokensResult::Tokens(tokens) = full_result else {
        panic!("expected semantic token full payload");
    };
    let baseline_result_id = tokens.result_id.expect("missing baseline result id");

    // request delta for unchanged content
    let delta_request = request_with_params(
        "textDocument/semanticTokens/full/delta",
        31,
        lsp::SemanticTokensDeltaParams {
            text_document: lsp::TextDocumentIdentifier::new(uri.clone()),
            previous_result_id: baseline_result_id.clone(),
            work_done_progress_params: lsp::WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: lsp::PartialResultParams {
                partial_result_token: None,
            },
        },
    );
    let delta_response = harness
        .call(delta_request)
        .await
        .expect("semantic delta response");
    assert!(delta_response.is_ok());
    let unchanged_delta: lsp::SemanticTokensFullDeltaResult =
        decode_response_result(&delta_response);
    let lsp::SemanticTokensFullDeltaResult::TokensDelta(delta) = unchanged_delta else {
        panic!("expected semantic token delta payload");
    };
    assert!(delta.edits.is_empty());

    // apply a text change and request delta from the baseline result id
    harness
        .did_change(uri.clone(), "// moved tokens\nexport const value = 1;\n", 2)
        .await;
    let _ = harness.next_diagnostics_for(&uri).await;

    let changed_delta_request = request_with_params(
        "textDocument/semanticTokens/full/delta",
        32,
        lsp::SemanticTokensDeltaParams {
            text_document: lsp::TextDocumentIdentifier::new(uri),
            previous_result_id: baseline_result_id,
            work_done_progress_params: lsp::WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: lsp::PartialResultParams {
                partial_result_token: None,
            },
        },
    );
    let changed_delta_response = harness
        .call(changed_delta_request)
        .await
        .expect("changed semantic delta response");
    assert!(changed_delta_response.is_ok());
    let changed_delta: lsp::SemanticTokensFullDeltaResult =
        decode_response_result(&changed_delta_response);
    let lsp::SemanticTokensFullDeltaResult::TokensDelta(delta) = changed_delta else {
        panic!("expected semantic token delta payload");
    };
    assert!(delta.result_id.is_some());
}

async fn next_partial_progress(harness: &mut LspHarness) -> serde_json::Value {
    let timeout = Duration::from_secs(5);
    for _ in 0..64 {
        let request = tokio::time::timeout(timeout, harness.next_client_request())
            .await
            .expect("timeout waiting for partial progress");
        if request.method() != "$/progress" {
            continue;
        }
        let params = request.params().cloned().expect("missing progress params");
        let progress: lsp::ProgressParams =
            serde_json::from_value(params).expect("decode progress params");
        if let lsp::ProgressParamsValue::PartialResult(value) = progress.value {
            return value;
        }
    }

    panic!("missing partial progress result");
}

async fn next_work_done_progress_kind(
    harness: &mut LspHarness,
    token: &lsp::ProgressToken,
    expected_kind: &str,
) {
    let timeout = Duration::from_secs(5);
    for _ in 0..128 {
        let request = tokio::time::timeout(timeout, harness.next_client_request())
            .await
            .expect("timeout waiting for work done progress");
        if request.method() != "$/progress" {
            continue;
        }

        let params = request.params().cloned().expect("missing progress params");
        let progress: lsp::ProgressParams =
            serde_json::from_value(params).expect("decode progress params");
        if &progress.token != token {
            continue;
        }

        let lsp::ProgressParamsValue::WorkDone(work_done_progress) = progress.value else {
            continue;
        };
        let kind = match work_done_progress {
            lsp::WorkDoneProgress::Begin(_) => "begin",
            lsp::WorkDoneProgress::Report(_) => "report",
            lsp::WorkDoneProgress::End(_) => "end",
        };
        if kind == expected_kind {
            return;
        }
    }

    panic!("missing work done progress kind '{expected_kind}'");
}

async fn next_log_message_containing(
    harness: &mut LspHarness,
    needle: &str,
) -> lsp::LogMessageParams {
    let timeout = Duration::from_secs(5);
    for _ in 0..128 {
        let request = tokio::time::timeout(timeout, harness.next_client_request())
            .await
            .expect("timeout waiting for log message");
        if request.method() != "window/logMessage" {
            continue;
        }

        let params = request
            .params()
            .cloned()
            .expect("missing log message params");
        let log: lsp::LogMessageParams =
            serde_json::from_value(params).expect("decode log message params");
        if log.message.contains(needle) {
            return log;
        }
    }

    panic!("missing log message containing '{needle}'");
}

fn decode_response_result<T>(response: &Response) -> T
where
    T: serde::de::DeserializeOwned,
{
    let value = response.result().cloned().expect("response result missing");
    serde_json::from_value(value).expect("decode response result")
}

/// Return the first location encoded in a goto definition response.
fn first_definition_location(response: &lsp::GotoDefinitionResponse) -> Option<lsp::Location> {
    match response {
        lsp::GotoDefinitionResponse::Scalar(location) => Some(location.clone()),
        lsp::GotoDefinitionResponse::Array(locations) => locations.first().cloned(),
        lsp::GotoDefinitionResponse::Link(links) => links.first().map(|link| lsp::Location {
            uri: link.target_uri.clone(),
            range: link.target_selection_range,
        }),
    }
}

/// Assert that a location uri resolves to the expected filesystem path.
fn assert_location_matches_path(location: &lsp::Location, expected_path: &std::path::Path) {
    let actual_path = location
        .uri
        .to_file_path()
        .map(|path| path.into_owned())
        .expect("location uri must resolve to a file path");
    let actual_path = std::fs::canonicalize(&actual_path).unwrap_or(actual_path);
    let expected_path =
        std::fs::canonicalize(expected_path).unwrap_or_else(|_| expected_path.to_path_buf());

    assert_eq!(actual_path, expected_path);
}
