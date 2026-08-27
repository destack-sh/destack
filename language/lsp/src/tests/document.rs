use destack_lsp_types as lsp;

use serde_json::json;

use super::tests::{MANIFEST, TestDocument, TestServer, position, range};

/// Function index in the advertised semantic token legend.
const FUNCTION_TOKEN: u32 = 11;
/// Interface index in the advertised semantic token legend.
const INTERFACE_TOKEN: u32 = 4;
/// Parameter index in the advertised semantic token legend.
const PARAMETER_TOKEN: u32 = 7;
/// Declaration bit in the advertised semantic token modifier legend.
const DECLARATION_MODIFIER: u32 = 1 << 0;
/// Deprecated bit in the advertised semantic token modifier legend.
const DEPRECATED_MODIFIER: u32 = 1 << 3;
/// Default library bit in the advertised semantic token modifier legend.
const DEFAULT_LIBRARY_MODIFIER: u32 = 1 << 8;

/// Return exact document symbols, tokens, folds, and selections.
#[tokio::test]
async fn test_return_document_structure() {
    let source = r#"export function choose(value: string): string {
    if (value == "") {
        return "empty";
    }
    return value;
}
"#;
    let (mut server, document) = TestServer::open_workspace(
        "document-structure",
        &[("src/main.ds", source)],
        "src/main.ds",
    )
    .await;

    // return the exact authored declaration hierarchy
    let outline = Some(lsp::DocumentSymbolResponse::Nested(vec![
        lsp::DocumentSymbol {
            name: "choose".to_string(),
            detail: Some("(value: string): string".to_string()),
            kind: lsp::SymbolKind::FUNCTION,
            tags: None,
            #[allow(deprecated)]
            deprecated: None,
            range: range(0, 0, 5, 1),
            selection_range: range(0, 16, 0, 22),
            children: None,
        },
    ]));
    server.assert_request(document.outline(), Ok(outline)).await;

    // return only semantic tokens inside the requested source range
    let tokens = Some(lsp::SemanticTokensRangeResult::Tokens(
        lsp::SemanticTokens {
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
            ],
        },
    ));
    server
        .assert_request(
            document.semantic_tokens_range(range(0, 0, 0, 47)),
            Ok(tokens),
        )
        .await;

    // return the function and nested conditional folds
    let folds = Some(vec![
        lsp::FoldingRange {
            start_line: 0,
            start_character: None,
            end_line: 5,
            end_character: None,
            kind: None,
            collapsed_text: None,
        },
        lsp::FoldingRange {
            start_line: 1,
            start_character: None,
            end_line: 3,
            end_character: None,
            kind: None,
            collapsed_text: None,
        },
    ]);
    server
        .assert_request(document.folding_ranges(), Ok(folds))
        .await;

    // return every enclosing range from the reference to its declaration
    let selections = Some(vec![lsp::SelectionRange {
        range: range(4, 11, 4, 16),
        parent: Some(Box::new(lsp::SelectionRange {
            range: range(4, 4, 4, 16),
            parent: Some(Box::new(lsp::SelectionRange {
                range: range(0, 46, 5, 1),
                parent: Some(Box::new(lsp::SelectionRange {
                    range: range(0, 0, 5, 1),
                    parent: None,
                })),
            })),
        })),
    }]);
    server
        .assert_request(document.selection_ranges([position(4, 11)]), Ok(selections))
        .await;
}

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

/// Open builtin documents with links, navigation, and semantic tokens.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_open_builtin_documents() {
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
    let actual = server.virtual_document(target.clone()).await;
    assert_eq!(actual, expected);

    // query the opened builtin index through its module
    let index = TestDocument::from(target);
    server.open(&index, 1, &expected.text).await;
    let console: lsp::Uri = "destack://console/console.ds".parse().unwrap();
    let expected = Some(vec![lsp::DocumentLink {
        range: range(0, 14, 0, 28),
        target: Some(console.clone()),
        tooltip: None,
        data: None,
    }]);
    server.assert_request(index.links(), Ok(expected)).await;

    // return semantic tokens from the builtin implementation source
    let source = server.virtual_document(console.clone()).await.text;
    let console = TestDocument::from(console);
    server.open(&console, 1, &source).await;
    let expected = Some(lsp::SemanticTokensRangeResult::Tokens(
        lsp::SemanticTokens {
            result_id: None,
            data: vec![lsp::SemanticToken {
                delta_line: 5,
                delta_start: 17,
                length: 14,
                token_type: INTERFACE_TOKEN,
                token_modifiers_bitset: DECLARATION_MODIFIER | DEFAULT_LIBRARY_MODIFIER,
            }],
        },
    ));
    server
        .assert_request(
            console.semantic_tokens_range(range(5, 0, 5, 33)),
            Ok(expected),
        )
        .await;

    // navigate one builtin type reference
    let expected = Some(lsp::GotoDefinitionResponse::Link(vec![lsp::LocationLink {
        origin_selection_range: Some(range(20, 26, 20, 40)),
        target_uri: console.uri().clone(),
        target_range: range(5, 0, 14, 1),
        target_selection_range: range(5, 17, 5, 31),
    }]));
    server
        .assert_request(console.definition(position(20, 27)), Ok(expected))
        .await;
}

/// Return exact workspace symbols and executable declaration lenses.
#[tokio::test]
async fn test_return_workspace_symbols_and_code_lenses() {
    let source = r#"export function quartz(): void {}

quartz();
"#;
    let mut server = TestServer::new("workspace-symbols-and-lenses");
    server.write("destack.json", MANIFEST);
    let document = server.write("src/main.ds", source);
    let options = Some(json!({ "codeLensCommands": ["references"] }));
    server
        .initialize(lsp::ClientCapabilities::default(), options)
        .await
        .unwrap();
    server.initialized().await;
    server.open(&document, 1, source).await;
    server.assert_diagnostics(&document, 1, Vec::new()).await;

    // search the complete workspace and project the authored location
    #[allow(deprecated)]
    let symbols = Some(lsp::WorkspaceSymbolResponse::Flat(vec![
        lsp::SymbolInformation {
            name: "quartz".to_string(),
            kind: lsp::SymbolKind::FUNCTION,
            tags: None,
            deprecated: None,
            location: lsp::Location {
                uri: document.uri().clone(),
                range: range(0, 0, 0, 33),
            },
            container_name: None,
        },
    ]));
    let request = server.workspace_symbols("quartz");
    server.assert_request(request, Ok(symbols)).await;

    // retain only actions implemented by the connected client bridge
    let lenses = Some(vec![lsp::CodeLens {
        range: range(0, 16, 0, 22),
        command: Some(lsp::Command {
            title: "1 reference".to_string(),
            command: "destack.showReferences".to_string(),
            arguments: Some(vec![
                serde_json::to_value(document.uri()).unwrap(),
                serde_json::to_value(position(0, 16)).unwrap(),
            ]),
        }),
        data: None,
    }]);
    server
        .assert_request(document.code_lenses(), Ok(lenses))
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
