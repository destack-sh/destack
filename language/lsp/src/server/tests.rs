use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use destack_lsp_server::jsonrpc::{Id, Request, Response};
use destack_lsp_server::{ClientSocket, LspService};
use destack_lsp_types as lsp;
use futures::StreamExt;
use serde_json::to_value;
use tower::{Service, ServiceExt};

use super::DestackLanguageServer;

static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(1);

/// One isolated LSP workspace.
struct LspFixture {
    /// Temporary workspace root.
    root: PathBuf,
    /// Main source path.
    main: PathBuf,
}

impl LspFixture {
    /// Create one configured workspace with an initial source file.
    fn new(name: &str, source: &str) -> Self {
        let id = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("destack-lsp-{}-{name}-{id}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("destack.json"), query_config()).unwrap();
        let main = root.join("main.ds");
        fs::write(&main, source).unwrap();

        Self { root, main }
    }

    /// Return one LSP URI for a fixture path.
    fn uri(&self, path: &Path) -> lsp::Uri {
        crate::uri::path(path).unwrap()
    }
}

impl Drop for LspFixture {
    /// Remove the isolated workspace.
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

/// Applies successive document versions and queries only the latest source.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn test_apply_successive_document_versions() {
    let first_source = r#"export function answer(): float64 {
  const value: float64 = false;
  return value;
}
"#;
    let second_source = r#"export function answer(): float64 {
  const value: float64 = 1;
  return value;
}
"#;
    let fixture = LspFixture::new("successive-document-versions", first_source);
    let root_uri = fixture.uri(&fixture.root);
    let main_uri = fixture.uri(&fixture.main);
    let (mut service, mut socket) = LspService::new(DestackLanguageServer::new);

    // initialize one real workspace root
    #[allow(deprecated)]
    let initialize = lsp::InitializeParams {
        root_uri: Some(root_uri),
        ..lsp::InitializeParams::default()
    };
    let request = Request::build("initialize")
        .params(to_value(initialize).unwrap())
        .id(Id::Number(1))
        .finish();
    let response = service.ready().await.unwrap().call(request).await.unwrap();
    assert!(response.is_some_and(|response| response.is_ok()));

    // replace version one before its diagnostic debounce expires
    let open = lsp::DidOpenTextDocumentParams {
        text_document: lsp::TextDocumentItem {
            uri: main_uri.clone(),
            language_id: "destack".to_string(),
            version: 1,
            text: first_source.to_string(),
        },
    };
    let request = Request::build("textDocument/didOpen")
        .params(to_value(open).unwrap())
        .finish();
    assert_eq!(
        service.ready().await.unwrap().call(request).await.unwrap(),
        None
    );

    let change = lsp::DidChangeTextDocumentParams {
        text_document: lsp::VersionedTextDocumentIdentifier {
            uri: main_uri.clone(),
            version: 2,
        },
        content_changes: vec![lsp::TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: second_source.to_string(),
        }],
    };
    let request = Request::build("textDocument/didChange")
        .params(to_value(change).unwrap())
        .finish();
    assert_eq!(
        service.ready().await.unwrap().call(request).await.unwrap(),
        None
    );

    // require one exact empty diagnostic publication for version two
    let notification = next_client_message(&mut socket).await;
    assert_eq!(notification.method(), "textDocument/publishDiagnostics");
    let diagnostics = serde_json::from_value::<lsp::PublishDiagnosticsParams>(
        notification.params().cloned().unwrap(),
    )
    .unwrap();
    assert_eq!(
        diagnostics,
        lsp::PublishDiagnosticsParams {
            uri: main_uri.clone(),
            diagnostics: Vec::new(),
            version: Some(2),
        }
    );

    // query the same latest open document through the full LSP flow
    let hover = lsp::HoverParams {
        text_document_position_params: lsp::TextDocumentPositionParams {
            text_document: lsp::TextDocumentIdentifier {
                uri: main_uri.clone(),
            },
            position: lsp::Position {
                line: 0,
                character: 16,
            },
        },
        work_done_progress_params: lsp::WorkDoneProgressParams::default(),
    };
    let request = Request::build("textDocument/hover")
        .params(to_value(hover).unwrap())
        .id(Id::Number(2))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(request)
        .await
        .unwrap()
        .unwrap();
    let main = fs::canonicalize(&fixture.main).unwrap();
    let expected_hover = lsp::Hover {
        contents: lsp::HoverContents::Markup(lsp::MarkupContent {
            kind: lsp::MarkupKind::Markdown,
            value: format!(
                "**Signature**\n\n```ds\nexport function answer(): float64\n```\n\n\
                 **Location**\n\n`{}:1:17`",
                main.display()
            ),
        }),
        range: Some(lsp::Range {
            start: lsp::Position {
                line: 0,
                character: 16,
            },
            end: lsp::Position {
                line: 0,
                character: 22,
            },
        }),
    };
    assert_eq!(
        response,
        Response::from_ok(Id::Number(2), to_value(Some(expected_hover)).unwrap())
    );
}

/// Wait for one server to client message with a bounded integration timeout.
async fn next_client_message(socket: &mut ClientSocket) -> Request {
    tokio::time::timeout(Duration::from_secs(15), socket.next())
        .await
        .unwrap()
        .unwrap()
}

/// Return one query configuration with an explicit target.
fn query_config() -> &'static str {
    r#"{
  "targets": {
    "default": {
      "entry": ["main.ds"]
    }
  },
  "defaultTarget": "default"
}
"#
}
