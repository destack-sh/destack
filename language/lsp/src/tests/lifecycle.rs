use destack_lsp_types as lsp;
use serde_json::{Value, json, to_value};

use super::tests::TestServer;

/// Complete initialization without requesting unsupported client capabilities.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_initialize_minimal_client() {
    let mut server = TestServer::new("initialize-minimal-client");
    server.write("main.ds", "export const value: float64 = 1;\n");

    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    server.assert_no_message();
}

/// Register watched files and read settings from a capable client.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_initialize_workspace_client() {
    let mut server = TestServer::new("initialize-workspace-client");
    server.write("main.ds", "export const value: float64 = 1;\n");
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
                id: "destack.watch".to_string(),
                method: "workspace/didChangeWatchedFiles".to_string(),
                register_options: Some(
                    to_value(lsp::DidChangeWatchedFilesRegistrationOptions {
                        watchers: [
                            "**/*.ds",
                            "**/*.d.ds",
                            "**/*.txt",
                            "**/*.toml",
                            "**/*.yaml",
                            "**/*.yml",
                            "**/*.json",
                            "**/*.env",
                            "**/*.md",
                            "**/*.html",
                            "**/*.htm",
                            "**/*.css",
                            "**/*.svg",
                            "**/*.map",
                            "**/destack.json",
                        ]
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
                    section: Some("destack.completion".to_string()),
                },
                lsp::ConfigurationItem {
                    scope_uri: None,
                    section: Some("destack.inlayHints".to_string()),
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
