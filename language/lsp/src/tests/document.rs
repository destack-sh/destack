use destack_lsp_types as lsp;

use super::tests::{MANIFEST, TestServer, range};

/// Function index in the advertised semantic token legend.
const FUNCTION_TOKEN: u32 = 11;
/// Parameter index in the advertised semantic token legend.
const PARAMETER_TOKEN: u32 = 7;
/// Declaration bit in the advertised semantic token modifier legend.
const DECLARATION_MODIFIER: u32 = 1 << 0;
/// Deprecated bit in the advertised semantic token modifier legend.
const DEPRECATED_MODIFIER: u32 = 1 << 3;

/// Return semantic tokens that follow an imported declaration.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_return_imported_semantic_tokens() {
    let dependency = r#"@deprecated
export function oldFunction(): void {}
"#;
    let source = r#"import { oldFunction } from "./library.ds";
export function useOld(): void {
    oldFunction();
}
"#;
    let (mut server, document) = TestServer::open_workspace(
        "imported-semantic-tokens",
        &[("src/library.ds", dependency), ("src/main.ds", source)],
        "src/main.ds",
    )
    .await;
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
        .assert_request(document.semantic_tokens(), Ok(expected))
        .await;
}

/// Classify parameter uses from their parameter symbol kind.
#[tokio::test]
async fn test_classify_parameter_uses_as_parameter_tokens() {
    let source = r#"export function double(value: int32): int32 {
    return value + value;
}
"#;
    let (mut server, document) = TestServer::open_workspace(
        "parameter-semantic-tokens",
        &[("src/main.ds", source)],
        "src/main.ds",
    )
    .await;
    let expected = Some(lsp::SemanticTokensResult::Tokens(lsp::SemanticTokens {
        result_id: None,
        data: vec![
            lsp::SemanticToken {
                delta_line: 0,
                delta_start: 16,
                length: 6,
                token_type: FUNCTION_TOKEN,
                token_modifiers_bitset: DECLARATION_MODIFIER,
            },
            lsp::SemanticToken {
                delta_line: 0,
                delta_start: 7,
                length: 5,
                token_type: PARAMETER_TOKEN,
                token_modifiers_bitset: DECLARATION_MODIFIER,
            },
            lsp::SemanticToken {
                delta_line: 1,
                delta_start: 11,
                length: 5,
                token_type: PARAMETER_TOKEN,
                token_modifiers_bitset: 0,
            },
            lsp::SemanticToken {
                delta_line: 0,
                delta_start: 8,
                length: 5,
                token_type: PARAMETER_TOKEN,
                token_modifiers_bitset: 0,
            },
        ],
    }));
    server
        .assert_request(document.semantic_tokens(), Ok(expected))
        .await;
}

/// Return exact target URIs for resolved module links.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_return_resolved_document_links() {
    let source = r#"import { value } from "./library.ds";
"#;
    let mut server = TestServer::new("resolved-document-links");
    server.write("destack.json", MANIFEST);
    let library = server.write("src/library.ds", "export const value = 1;\n");
    let document = server.write("src/main.ds", source);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // map the authored specifier to its exact source identity
    server.open(&document, 1, source).await;
    server.assert_diagnostics(&document, 1, Vec::new()).await;
    let expected = Some(vec![lsp::DocumentLink {
        range: range(0, 22, 0, 36),
        target: Some(library.uri().clone()),
        tooltip: None,
        data: None,
    }]);
    server.assert_request(document.links(), Ok(expected)).await;
}

/// Open builtin module links through virtual documents.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_open_builtin_module_links() {
    let source = "import { log } from \"destack:console\";\n";
    let target: lsp::Uri = "destack://console/index.ds".parse().unwrap();
    let (mut server, document) = TestServer::open_workspace(
        "builtin-module-links",
        &[("src/main.ds", source)],
        "src/main.ds",
    )
    .await;

    // resolve the import through its builtin module identity
    let expected = Some(vec![lsp::DocumentLink {
        range: range(0, 20, 0, 37),
        target: Some(target.clone()),
        tooltip: None,
        data: None,
    }]);
    server.assert_request(document.links(), Ok(expected)).await;

    // read the exact source exposed by the resolved link
    let expected = lsp::TextDocumentContentResult {
        text: "export * from \"./console.ds\";\n".to_string(),
    };
    let actual = server.virtual_document(target).await;
    assert_eq!(actual, expected);
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
    server.assert_diagnostics(&document, 1, Vec::new()).await;

    // execute overlapping program and diagnostic roots concurrently
    let action_context = lsp::CodeActionContext {
        diagnostics: Vec::new(),
        only: Some(vec![lsp::CodeActionKind::QUICKFIX]),
        trigger_kind: None,
    };
    let actions = server
        .start_request(document.code_actions(range(0, 0, 0, 46), action_context))
        .await;
    let lenses = server.start_request(document.code_lenses()).await;

    assert_eq!(actions.wait().await, Ok(None));
    assert_eq!(lenses.wait().await, Ok(Some(Vec::new())));
}
