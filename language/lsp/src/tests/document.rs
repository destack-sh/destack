use destack_lsp_types as lsp;

use super::tests::TestServer;

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
