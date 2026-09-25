use tspp_lsp_types as lsp;

use serde_json::json;

use super::tests::{TestDocument, TestServer, position, range};

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
        &[("src/main.tspp", source)],
        "src/main.tspp",
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
    let source = r#"import { oldFunction } from "./library.tspp";
export function useOld(): void {
    oldFunction();
}
"#;
    let (mut server, document) = TestServer::open_workspace(
        "imported-semantic-tokens",
        &[("src/library.tspp", dependency), ("src/main.tspp", source)],
        "src/main.tspp",
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
        &[("src/main.tspp", source)],
        "src/main.tspp",
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
    let source = r#"import { value } from "./library.tspp";
"#;
    let library = r#"export const value = 1;
"#;
    let (mut server, document) = TestServer::open_workspace(
        "resolved-document-links",
        &[("src/library.tspp", library), ("src/main.tspp", source)],
        "src/main.tspp",
    )
    .await;
    let library = server.document("src/library.tspp");

    // link the authored specifier to its source file
    let expected = Some(vec![lsp::DocumentLink {
        range: range(0, 22, 0, 38),
        target: Some(library.uri().clone()),
        tooltip: None,
        data: None,
    }]);
    server.assert_request(document.links(), Ok(expected)).await;
}

/// Query builtin documents across physical and virtual sources.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_query_builtin_documents() {
    let source = r#"import { log } from "tspp:console";

log("ready");
"#;
    let manifest = r#"{
  "packageManager": "tspp@2026.9.0",
  "name": "builtin-module-links",
  "workspaces": ["app"]
}
"#;

    // open a source from one configured workspace package
    let mut server = TestServer::new("builtin-module-links");
    server.write("package.json", manifest);
    server.create_package("app");
    let document = server.write("app/main.tspp", source);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;
    server.open(&document, 1, source).await;
    server.assert_diagnostics(&document, 1, Vec::new()).await;
    let target = server.builtin_uri("tspp://console/index.tspp");

    // resolve the import through its builtin module identity
    let expected = Some(vec![lsp::DocumentLink {
        range: range(0, 20, 0, 34),
        target: Some(target.clone()),
        tooltip: None,
        data: None,
    }]);
    server.assert_request(document.links(), Ok(expected)).await;

    // navigate from physical source to the builtin declaration
    let console = server.builtin_uri("tspp://console/console.tspp");
    let expected = Some(lsp::GotoDefinitionResponse::Link(vec![lsp::LocationLink {
        origin_selection_range: Some(range(2, 0, 2, 3)),
        target_uri: console.clone(),
        target_range: range(104, 0, 106, 1),
        target_selection_range: range(104, 16, 104, 19),
    }]));
    server
        .assert_request(document.definition(position(2, 1)), Ok(expected))
        .await;

    // read the exact source exposed by the resolved link
    let expected = lsp::TextDocumentContentResult {
        text: "export * from \"./console.tspp\";\n".to_string(),
    };
    let actual = server.virtual_document(target.clone()).await;
    assert_eq!(actual, expected);

    // query the opened builtin index through its module
    let index = TestDocument::from(target);
    server.open(&index, 1, &expected.text).await;
    server.assert_document_diagnostics(&index, Vec::new()).await;
    let expected = Some(vec![lsp::DocumentLink {
        range: range(0, 14, 0, 30),
        target: Some(console.clone()),
        tooltip: None,
        data: None,
    }]);
    server.assert_request(index.links(), Ok(expected)).await;

    // return semantic tokens from the builtin implementation source
    let source = server.virtual_document(console.clone()).await.text;
    let console = TestDocument::from(console);
    server.open(&console, 1, &source).await;
    server
        .assert_document_diagnostics(&console, Vec::new())
        .await;
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
    let definition = Some(lsp::GotoDefinitionResponse::Link(vec![console.link(
        range(20, 26, 20, 40),
        range(5, 0, 14, 1),
        range(5, 17, 5, 31),
    )]));
    server
        .assert_request(console.definition(position(20, 27)), Ok(definition.clone()))
        .await;

    // reopen immutable source through the same qualified identity
    server.close(&console).await;
    server.open(&console, 2, &source).await;
    server
        .assert_request(console.definition(position(20, 27)), Ok(definition))
        .await;
}

/// Keep builtin document identities distinct across editor workspace folders.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_distinguish_builtin_documents_by_project() {
    let source = r#"import { log } from "tspp:console";
"#;
    let mut server = TestServer::new_editor_folder("builtin-project-identity");
    server.create_package("first");
    server.create_package("second");
    let first = server.write("first/main.tspp", source);
    let second = server.write("second/main.tspp", source);
    let first_root = server.root().join("first");
    let second_root = server.root().join("second");
    server
        .initialize_folders(&[&first_root, &second_root])
        .await
        .unwrap();
    server.initialized().await;

    let first_target = server.builtin_uri_at(&first_root, "tspp://console/index.tspp");
    let second_target = server.builtin_uri_at(&second_root, "tspp://console/index.tspp");
    let link_range = range(0, 20, 0, 34);
    let first_links = Some(vec![lsp::DocumentLink {
        range: link_range,
        target: Some(first_target.clone()),
        tooltip: None,
        data: None,
    }]);
    let second_links = Some(vec![lsp::DocumentLink {
        range: link_range,
        target: Some(second_target.clone()),
        tooltip: None,
        data: None,
    }]);

    server.assert_request(first.links(), Ok(first_links)).await;
    server
        .assert_request(second.links(), Ok(second_links))
        .await;
    assert_ne!(first_target, second_target);

    // resolve each qualified URI through its exact project
    let expected = lsp::TextDocumentContentResult {
        text: "export * from \"./console.tspp\";\n".to_string(),
    };
    let first_content = server.virtual_document(first_target).await;
    let second_content = server.virtual_document(second_target).await;
    assert_eq!(first_content, expected);
    assert_eq!(second_content, expected);
    server.assert_no_message();
}

/// Return exact workspace symbols and executable declaration lenses.
#[tokio::test]
async fn test_return_workspace_symbols_and_code_lenses() {
    let source = r#"export function quartz(): void {}

quartz();
"#;
    let mut server = TestServer::new_editor_folder("workspace-symbols-and-lenses");
    server.create_package("package");
    let document = server.write("package/main.tspp", source);
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
            location: document.location(range(0, 0, 0, 33)),
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
            command: "tspp.showReferences".to_string(),
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
    let source = r#"export function answer(): number { return 1; }
"#;
    let mut server = TestServer::new("concurrent-document-queries");
    let document = server.write("main.tspp", source);
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
