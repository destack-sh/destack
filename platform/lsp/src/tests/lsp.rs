use std::sync::Arc;
use std::time::Duration;

use destack_compiler::CompilerOptions;
use destack_lsp_server::jsonrpc::Response;
use destack_lsp_server::{LanguageServer, LspService};
use destack_lsp_types as lsp;
use destack_resolver::{ResolveOptions, Resolver};
use destack_service::LanguageService as LspLanguageService;
use destack_source::{
    FileSystem, OverlayFileSystem, PhysicalFileSystem, TemporaryPhysicalFileSystem,
};
use destack_workspace::{MemoryCacheStore, Session, Workspace};
use futures::{SinkExt, StreamExt};
use tower::Service;

use super::fixture::TestLsp;
use super::harness::{
    LspHarness, harness_for_fs, notification_with_params, request_with_params, test_fs,
    uri_for_path,
};
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
    assert!(diagnostics.diagnostics.is_empty());
}

/// LSP workspace service emits diagnostics for virtual updates.
#[test]
fn test_lsp_workspace_service_virtual_update_emits_diagnostics() {
    let fs = TemporaryPhysicalFileSystem::new_with_prefix("lsp_workspace_client");
    let overlay = Arc::new(OverlayFileSystem::with_inner(Arc::new(
        PhysicalFileSystem::new(),
    )));
    let root = fs.root().to_path_buf();
    let session = Session::new(root.clone())
        .with_fs(overlay)
        .with_cache_store(Arc::new(MemoryCacheStore::new()));
    let resolver = Resolver::from_session(&session, ResolveOptions::default());
    let workspace = resolver
        .discover_workspace(&root)
        .unwrap_or_else(|_| Workspace::single_package(root.clone()));
    let session = Arc::new(session.with_workspace(workspace));
    session.add_root(root.clone());

    // keep lsp workspace-service test deterministic: use a single compiler worker
    let compiler_options = CompilerOptions {
        workers: 1,
        ..CompilerOptions::default()
    };
    let workspace_service =
        LspLanguageService::with_options(session.clone(), vec![root.clone()], compiler_options)
            .expect("expected workspace service");

    let path = root.join("main.ds");
    let _ = fs.write_text("main.ds", "export const x: number = 1;\n");
    let initial = workspace_service
        .update_virtual_file(&path, "export const x: number = 1;\n".to_string())
        .expect("expected initial update");
    assert!(
        initial
            .updates
            .iter()
            .all(|update| update.diagnostics.is_empty()),
        "expected no diagnostics for valid content"
    );

    let updated = workspace_service
        .update_virtual_file(&path, "export const x = ;\n".to_string())
        .expect("expected updated diagnostics");
    assert!(
        updated
            .updates
            .iter()
            .any(|update| !update.diagnostics.is_empty()),
        "expected diagnostics for invalid content"
    );

    workspace_service.shutdown();
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
    assert_eq!(updated.uri, uri);
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
    assert_eq!(updated.uri, uri);
    assert!(!updated.diagnostics.is_empty());
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
    assert!(definition.is_some());

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

/// Document diagnostics include open virtual file updates.
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

    let mut diagnostics = Vec::new();
    diagnostics.push(harness.next_diagnostics().await);
    diagnostics.push(harness.next_diagnostics().await);

    // check that the old is cleared, new has diagnostics
    assert_eq!(diagnostics.len(), 2);
    let old_diagnostics = diagnostics.iter().find(|diag| diag.uri == old_uri);
    let new_diagnostics = diagnostics.iter().find(|diag| diag.uri == new_uri);
    let old_diagnostics = old_diagnostics.expect("missing old diagnostics");
    let new_diagnostics = new_diagnostics.expect("missing new diagnostics");
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
    assert!(patterns.iter().any(|pattern| pattern == "**/dsconfig.json"));
    assert!(
        patterns
            .iter()
            .any(|pattern| pattern == "**/tsconfig*.json")
    );
}

/// LSP config changes publish diagnostics for invalidated modules.
#[tokio::test]
async fn test_lsp_config_change_fanout_publishes_diagnostics() {
    // set up the workspace root with a package and dsconfig
    let fs = test_fs("config_fanout");
    let _package_path = fs
        .write_text(
            "package.json",
            "{ \"name\": \"fanout\", \"version\": \"0.1.0\" }\n",
        )
        .unwrap();
    let dsconfig_path = fs
        .write_text("dsconfig.json", "{ \"compilerOptions\": {} }\n")
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

    // open the dsconfig for edits
    let dsconfig_uri = uri_for_path(&dsconfig_path);
    harness
        .did_open(dsconfig_uri.clone(), "{ \"compilerOptions\": {} }\n")
        .await;

    // drain initial diagnostics from didOpen
    let _ = harness.next_diagnostics_for(&uri_a).await;
    let _ = harness.next_diagnostics_for(&uri_b).await;
    let _ = harness.next_diagnostics_for(&dsconfig_uri).await;

    // update dsconfig to trigger module invalidation
    fs.write_text(
        &dsconfig_path,
        "{ \"compilerOptions\": { \"noImplicitAny\": true } }\n",
    )
    .unwrap();
    harness
        .did_change(
            dsconfig_uri.clone(),
            "{ \"compilerOptions\": { \"noImplicitAny\": true } }\n",
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
    let _ = fs_a.write_text("dsconfig.json", "{ \"compilerOptions\": {} }\n");
    let _ = fs_b.write_text("dsconfig.json", "{ \"compilerOptions\": {} }\n");

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
    let mut file_paths = Vec::new();
    for index in 0..256 {
        let name = format!("file_{index}.ds");
        let path = fs
            .write_text(&name, "export const x = ;\n")
            .expect("write module");
        file_paths.push(path);
    }

    let (mut service, client) = LspService::new(crate::DestackLanguageServer::new);
    let (mut requests, mut responses) = client.split();
    let client_task = tokio::spawn(async move {
        while let Some(request) = requests.next().await {
            if let Some(id) = request.id().cloned() {
                let response = Response::from_ok(id, serde_json::Value::Null);
                let _ = responses.send(response).await;
            }
        }
    });

    #[allow(deprecated)]
    let params = lsp::InitializeParams {
        root_uri: Some(uri_for_path(fs.root())),
        ..Default::default()
    };
    let request = request_with_params("initialize", 1, params);
    let response = service.call(request).await.expect("initialize response");
    let response = response.expect("initialize response missing");
    assert!(response.is_ok());
    let initialized = notification_with_params("initialized", lsp::InitializedParams {});
    let _ = service.call(initialized).await;

    let server = service.inner();

    for path in file_paths.iter() {
        let uri = uri_for_path(path);
        server
            .did_open(lsp::DidOpenTextDocumentParams {
                text_document: lsp::TextDocumentItem::new(
                    uri,
                    "destack".to_string(),
                    1,
                    "export const x = ;\n".to_string(),
                ),
            })
            .await;
    }

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

    let diagnostic_future = server.workspace_diagnostic(diag_params);
    let cancel_future = async {
        for _ in 0..32 {
            server
                .work_done_progress_cancel(lsp::WorkDoneProgressCancelParams {
                    token: work_done_token.clone(),
                })
                .await;
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    };

    let (result, _) = tokio::join!(diagnostic_future, cancel_future);
    client_task.abort();
    assert_eq!(
        result.unwrap_err(),
        destack_lsp_server::jsonrpc::Error::request_cancelled()
    );
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

fn decode_response_result<T>(response: &Response) -> T
where
    T: serde::de::DeserializeOwned,
{
    let value = response.result().cloned().expect("response result missing");
    serde_json::from_value(value).expect("decode response result")
}
