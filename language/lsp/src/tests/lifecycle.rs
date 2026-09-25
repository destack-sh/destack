use std::fs;

use serde_json::{Value, json, to_value};
use tspp_lsp_types as lsp;

use super::tests::{TestServer, markdown, position, range};

/// Complete initialization without requesting unsupported client capabilities.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_initialize_minimal_client() {
    let mut server = TestServer::new("initialize-minimal-client");
    server.write("main.tspp", "export const value: float64 = 1;\n");

    let initialized = server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // advertise the canonical virtual source scheme
    let content = initialized
        .capabilities
        .workspace
        .and_then(|workspace| workspace.text_document_content);
    assert_eq!(
        content,
        Some(lsp::TextDocumentContentOptions {
            schemes: vec!["tspp".to_string()],
        })
    );

    server.assert_no_message();
}

/// Register watched files and read settings from a capable client.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_initialize_workspace_client() {
    let mut server = TestServer::new("initialize-workspace-client");
    server.write("main.tspp", "export const value: float64 = 1;\n");
    let capabilities = lsp::ClientCapabilities {
        workspace: Some(lsp::WorkspaceClientCapabilities {
            configuration: Some(true),
            did_change_watched_files: Some(lsp::DidChangeWatchedFilesClientCapabilities {
                dynamic_registration: Some(true),
                relative_pattern_support: Some(true),
            }),
            ..lsp::WorkspaceClientCapabilities::default()
        }),
        ..lsp::ClientCapabilities::default()
    };
    server.initialize(capabilities, None).await.unwrap();

    // begin initialized processing while serving client requests
    let initialized = server
        .start_notification::<lsp::notification::Initialized>(lsp::InitializedParams {})
        .await;

    // register the complete watched file set
    let (id, registration) = server
        .receive_request::<lsp::request::RegisterCapability>()
        .await;
    assert_eq!(
        registration,
        lsp::RegistrationParams {
            registrations: vec![lsp::Registration {
                id: "tspp.watch".to_string(),
                method: "workspace/didChangeWatchedFiles".to_string(),
                register_options: Some(
                    to_value(lsp::DidChangeWatchedFilesRegistrationOptions {
                        watchers: ["**/*"]
                            .into_iter()
                            .map(|pattern| lsp::FileSystemWatcher {
                                glob_pattern: pattern.to_string().into(),
                                kind: None,
                            })
                            .collect(),
                    })
                    .unwrap(),
                ),
            }],
        }
    );
    server
        .respond::<lsp::request::RegisterCapability>(id, Ok(()))
        .await;

    // answer the exact requested configuration sections
    let (id, configuration) = server
        .receive_request::<lsp::request::WorkspaceConfiguration>()
        .await;
    assert_eq!(
        configuration,
        lsp::ConfigurationParams {
            items: vec![
                lsp::ConfigurationItem {
                    scope_uri: None,
                    section: Some("tspp.completion".to_string()),
                },
                lsp::ConfigurationItem {
                    scope_uri: None,
                    section: Some("tspp.inlayHints".to_string()),
                },
            ],
        }
    );
    let settings: Vec<Value> = vec![
        json!({ "autoImports": false }),
        json!({ "parameterHints": false, "typeHints": false }),
    ];
    server
        .respond::<lsp::request::WorkspaceConfiguration>(id, Ok(settings))
        .await;

    initialized.wait().await;
    server.assert_no_message();
}

/// Reconcile one removed path after its transient parent directories disappear.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_reconcile_removed_nested_path() {
    let mut server = TestServer::new("removed-nested-path");
    server.write("main.tspp", "export const value: float64 = 1;\n");
    let removed = server.write("target/debug/incremental/working/output", "transient");
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // remove the complete transient directory before delivering its event
    fs::remove_dir_all(server.root().join("target")).unwrap();
    server
        .notify::<lsp::notification::DidChangeWatchedFiles>(lsp::DidChangeWatchedFilesParams {
            changes: vec![lsp::FileEvent {
                uri: removed.uri().clone(),
                typ: lsp::FileChangeType::DELETED,
            }],
        })
        .await;

    server.assert_no_message();
}

/// Reload changed filesystem source through the advertised command.
#[tokio::test]
async fn test_reload_workspace_source() {
    let initial = r#"export function answer(): float64 { return 1; }
"#;
    let changed = r#"export function answer(): string { return "changed"; }
"#;
    let mut server = TestServer::new("reload-workspace-source");
    let document = server.write("main.tspp", initial);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // replace the physical source before reloading the workspace
    server.write("main.tspp", changed);
    let request = server.reload();
    server.assert_request(request, Ok(None)).await;

    // observe the reloaded declaration through a semantic request
    let hover = Some(lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`main.tspp:1:17`\n\n```tspp\nexport function answer(): string\n```",
        )),
        range: Some(range(0, 16, 0, 22)),
    });
    server
        .assert_request(document.hover(position(0, 16)), Ok(hover))
        .await;
}

/// Clear diagnostics after a file rename notification.
#[tokio::test]
async fn test_clear_renamed_file_diagnostics() {
    let source = r#"const value = missing;
"#;
    let mut server = TestServer::new("renamed-file-diagnostics");
    let renamed = server.write("renamed.tspp", source);
    let renamed_target = server.document("renamed-again.tspp");
    let renamed_error = renamed.error(
        range(0, 14, 0, 21),
        "unresolved-reference",
        "cannot find 'missing'",
    );
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // retain diagnostics before closing the editor document
    server.open(&renamed, 1, source).await;
    server
        .assert_diagnostics(&renamed, 1, vec![renamed_error.clone()])
        .await;
    server.close(&renamed).await;
    server
        .assert_diagnostics(&renamed, None, vec![renamed_error])
        .await;

    // clear the old identity after a rename notification
    server
        .notify::<lsp::notification::DidRenameFiles>(lsp::RenameFilesParams {
            files: vec![lsp::FileRename {
                old_uri: renamed.uri().to_string(),
                new_uri: renamed_target.uri().to_string(),
            }],
        })
        .await;
    server.assert_diagnostics(&renamed, None, Vec::new()).await;
}

/// Clear diagnostics after a file delete notification.
#[tokio::test]
async fn test_clear_deleted_file_diagnostics() {
    let source = r#"const value = missing;
"#;
    let mut server = TestServer::new("deleted-file-diagnostics");
    let deleted = server.write("deleted.tspp", source);
    let deleted_error = deleted.error(
        range(0, 14, 0, 21),
        "unresolved-reference",
        "cannot find 'missing'",
    );
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // retain diagnostics before closing the editor document
    server.open(&deleted, 1, source).await;
    server
        .assert_diagnostics(&deleted, 1, vec![deleted_error.clone()])
        .await;
    server.close(&deleted).await;
    server
        .assert_diagnostics(&deleted, None, vec![deleted_error])
        .await;

    // clear the removed identity after a delete notification
    server
        .notify::<lsp::notification::DidDeleteFiles>(lsp::DeleteFilesParams {
            files: vec![lsp::FileDelete {
                uri: deleted.uri().to_string(),
            }],
        })
        .await;
    server.assert_diagnostics(&deleted, None, Vec::new()).await;
}
