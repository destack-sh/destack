use destack_lsp_types as lsp;

use super::tests::{TestServer, range};

/// Function index in the advertised semantic token legend.
const FUNCTION_TOKEN: u32 = 11;
/// Declaration bit in the advertised semantic token modifier legend.
const DECLARATION_MODIFIER: u32 = 1 << 0;
/// Deprecated bit in the advertised semantic token modifier legend.
const DEPRECATED_MODIFIER: u32 = 1 << 3;

/// Return semantic tokens that follow an imported declaration.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_return_imported_semantic_tokens() {
    let manifest = r#"{
  "name": "lsp-fixture",
  "targets": {
    "default": {
      "include": ["src/**/*.ds"]
    }
  },
  "defaultTarget": "default"
}
"#;
    let dependency = r#"@deprecated
export function oldFunction(): void {}
"#;
    let source = r#"import { oldFunction } from "./library.ds";
export function useOld(): void {
    oldFunction();
}
"#;
    let mut server = TestServer::new("imported-semantic-tokens");
    server.write("destack.json", manifest);
    server.write("src/library.ds", dependency);
    let document = server.write("src/main.ds", source);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // classify the import and call from their exported declaration
    server.open(&document, 1, source).await;
    server
        .assert_notification::<lsp::notification::PublishDiagnostics>(
            lsp::PublishDiagnosticsParams {
                uri: document.uri().clone(),
                diagnostics: Vec::new(),
                version: Some(1),
            },
        )
        .await;
    let params = document.semantic_tokens();
    let expected = Some(lsp::SemanticTokensResult::Tokens(lsp::SemanticTokens {
        result_id: None,
        data: vec![
            lsp::SemanticToken {
                delta_line: 0,
                delta_start: 9,
                length: 11,
                token_type: FUNCTION_TOKEN,
                token_modifiers_bitset: DECLARATION_MODIFIER | DEPRECATED_MODIFIER,
            },
            lsp::SemanticToken {
                delta_line: 1,
                delta_start: 16,
                length: 6,
                token_type: FUNCTION_TOKEN,
                token_modifiers_bitset: DECLARATION_MODIFIER,
            },
            lsp::SemanticToken {
                delta_line: 1,
                delta_start: 4,
                length: 11,
                token_type: FUNCTION_TOKEN,
                token_modifiers_bitset: DEPRECATED_MODIFIER,
            },
        ],
    }));
    server
        .assert_request::<lsp::request::SemanticTokensFullRequest>(params, Ok(expected))
        .await;
}

/// Return exact target URIs for resolved module links.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_return_resolved_document_links() {
    let manifest = r#"{
  "name": "lsp-fixture",
  "targets": {
    "default": {
      "include": ["src/**/*.ds"]
    }
  },
  "defaultTarget": "default"
}
"#;
    let source = r#"import { value } from "./library.ds";
"#;
    let mut server = TestServer::new("resolved-document-links");
    server.write("destack.json", manifest);
    let library = server.write("src/library.ds", "export const value = 1;\n");
    let document = server.write("src/main.ds", source);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // map the authored specifier to its exact source identity
    server.open(&document, 1, source).await;
    server
        .assert_notification::<lsp::notification::PublishDiagnostics>(
            lsp::PublishDiagnosticsParams {
                uri: document.uri().clone(),
                diagnostics: Vec::new(),
                version: Some(1),
            },
        )
        .await;
    let expected = Some(vec![lsp::DocumentLink {
        range: range(0, 22, 0, 36),
        target: Some(library.uri().clone()),
        tooltip: None,
        data: None,
    }]);
    server
        .assert_request::<lsp::request::DocumentLinkRequest>(document.links(), Ok(expected))
        .await;
}

/// Complete concurrent semantic document requests over shared artifacts.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_run_concurrent_document_queries() {
    let source = "export function answer(): number { return 1; }\n";
    let mut server = TestServer::new("concurrent-document-queries");
    let document = server.write("main.ds", source);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // establish one valid semantic revision
    server.open(&document, 1, source).await;
    server
        .assert_notification::<lsp::notification::PublishDiagnostics>(
            lsp::PublishDiagnosticsParams {
                uri: document.uri().clone(),
                diagnostics: Vec::new(),
                version: Some(1),
            },
        )
        .await;

    // execute overlapping program and diagnostic roots concurrently
    let action_context = lsp::CodeActionContext {
        diagnostics: Vec::new(),
        only: Some(vec![lsp::CodeActionKind::QUICKFIX]),
        trigger_kind: None,
    };
    let actions = server
        .start_request::<lsp::request::CodeActionRequest>(
            document.code_actions(range(0, 0, 0, 46), action_context),
        )
        .await;
    let lenses = server
        .start_request::<lsp::request::CodeLensRequest>(document.code_lenses())
        .await;

    assert_eq!(actions.wait().await, Ok(None));
    assert_eq!(lenses.wait().await, Ok(Some(Vec::new())));
}
