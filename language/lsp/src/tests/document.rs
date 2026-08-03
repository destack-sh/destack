use destack_lsp_types as lsp;

use super::tests::{MANIFEST, TestServer, markdown, position, range};

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
    let params = document.semantic_tokens();
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
        .assert_request::<lsp::request::SemanticTokensFullRequest>(params, Ok(expected))
        .await;
}

/// Navigate an interface method to its implementing declarations.
#[tokio::test]
async fn test_navigate_an_interface_method_to_its_implementations() {
    let source = r#"interface Greeter {
    greet(): string;
}

class Robot {
    greet(): string {
        return "beep";
    }
}

extension of Robot implements Greeter {}
"#;
    let (mut server, document) = TestServer::open_workspace(
        "member-implementations",
        &[("src/main.ds", source)],
        "src/main.ds",
    )
    .await;
    let params = document.implementations(position(1, 4));
    let expected = Some(lsp::request::GotoImplementationResponse::Link(vec![
        lsp::LocationLink {
            origin_selection_range: Some(lsp::Range::new(
                lsp::Position::new(1, 4),
                lsp::Position::new(1, 9),
            )),
            target_uri: document.uri().clone(),
            target_range: lsp::Range::new(lsp::Position::new(5, 4), lsp::Position::new(7, 5)),
            target_selection_range: lsp::Range::new(
                lsp::Position::new(5, 4),
                lsp::Position::new(5, 9),
            ),
        },
    ]));
    server
        .assert_request::<lsp::request::GotoImplementation>(params, Ok(expected))
        .await;
}

/// Complete the expected type's missing fields inside an object literal.
#[tokio::test]
async fn test_complete_expected_fields_in_an_object_literal() {
    let source = r#"struct Point {
    x: int32;
    y: int32;
}

const origin: Point = { x: 0,  };
"#;
    let mut server = TestServer::new("expected-field-completion");
    server.write("destack.json", MANIFEST);
    let document = server.write("src/main.ds", source);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;
    server.open(&document, 1, source).await;

    // accept the missing-property diagnostic of the incomplete literal
    let diagnostics = server
        .receive_notification::<lsp::notification::PublishDiagnostics>()
        .await;
    let codes: Vec<String> = diagnostics
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.code {
            Some(lsp::NumberOrString::String(code)) => Some(code.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(codes, ["missing-required-property"]);

    // offer the missing y field at the free key position
    let params = document.completion(position(5, 29));
    let response = server
        .request::<lsp::request::Completion>(params)
        .await
        .unwrap();
    let labels: Vec<String> = match response {
        Some(lsp::CompletionResponse::Array(items)) => {
            items.into_iter().map(|item| item.label).collect()
        }
        Some(lsp::CompletionResponse::List(list)) => {
            list.items.into_iter().map(|item| item.label).collect()
        }
        None => Vec::new(),
    };
    assert_eq!(labels, ["y", "origin", "Point"]);
}

/// Rename an interface method together with its implementing declaration.
#[tokio::test]
async fn test_rename_an_interface_method_with_its_implementation() {
    let source = r#"interface Greeter {
    greet(): string;
}

class Robot {
    greet(): string {
        return "beep";
    }
}

extension of Robot implements Greeter {}

declare const robot: Robot;
const sound = robot.greet();
"#;
    let (mut server, document) = TestServer::open_workspace(
        "member-union-rename",
        &[("src/main.ds", source)],
        "src/main.ds",
    )
    .await;

    // rename the interface requirement and collect every edited span
    let params = document.rename(position(1, 4), "announce");
    let response = server
        .request::<lsp::request::Rename>(params)
        .await
        .unwrap();
    let Some(edit) = response else {
        panic!("rename produced no edit");
    };
    let mut starts: Vec<(u32, u32)> = edit
        .changes
        .into_iter()
        .flat_map(|changes| changes.into_values().flatten())
        .map(|edit| (edit.range.start.line, edit.range.start.character))
        .collect();
    starts.sort_unstable();

    // expect the requirement, the implementing method, and the call to rename
    assert_eq!(starts, [(1, 4), (5, 4), (13, 20)]);
}

/// Rename an export through its import selector chain.
#[tokio::test]
async fn test_rename_an_export_through_its_import_selector() {
    let library = r#"export const answer: int32 = 42;
"#;
    let source = r#"import { answer } from "./library.ds";

const doubled = answer + answer;
"#;
    let mut server = TestServer::new("selector-chain-rename");
    server.write("destack.json", MANIFEST);
    server.write("src/library.ds", library);
    let document = server.write("src/main.ds", source);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;
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

    // rename the declaration from a use site across the selector chain
    let params = document.rename(position(2, 16), "result");
    let response = server
        .request::<lsp::request::Rename>(params)
        .await
        .unwrap();
    let Some(edit) = response else {
        panic!("rename produced no edit");
    };
    let mut edits: Vec<(String, u32, u32)> = edit
        .changes
        .into_iter()
        .flat_map(|changes| changes.into_iter())
        .flat_map(|(uri, edits)| {
            let path = uri.path().as_str().to_string();
            edits.into_iter().map(move |edit| {
                (
                    path.clone(),
                    edit.range.start.line,
                    edit.range.start.character,
                )
            })
        })
        .collect();
    edits.sort();

    // expect the declaration, the import selector, and both uses to rename
    let files: Vec<&str> = edits
        .iter()
        .map(|(path, _, _)| path.rsplit('/').next().unwrap())
        .collect();
    assert_eq!(files, ["library.ds", "main.ds", "main.ds", "main.ds"]);
}

/// Complete a receiver's own, extension, and inherited members after a dot.
#[tokio::test]
async fn test_complete_apparent_members_after_a_dot() {
    let source = r#"class Animal {
    name: string = "";
}

class Dog extends Animal {
    tricks: int32 = 0;
}

extension of Dog {
    bark(): string {
        return "woof";
    }
}

declare const dog: Dog;
const sound = dog.name;
"#;
    let (mut server, document) = TestServer::open_workspace(
        "apparent-member-completion",
        &[("src/main.ds", source)],
        "src/main.ds",
    )
    .await;

    // offer own members before extension members before inherited members
    let params = document.completion(position(15, 18));
    let response = server
        .request::<lsp::request::Completion>(params)
        .await
        .unwrap();
    let labels: Vec<String> = match response {
        Some(lsp::CompletionResponse::Array(items)) => {
            items.into_iter().map(|item| item.label).collect()
        }
        Some(lsp::CompletionResponse::List(list)) => {
            list.items.into_iter().map(|item| item.label).collect()
        }
        None => Vec::new(),
    };
    assert_eq!(labels, ["tricks", "name", "bark", "borrow"]);
}

/// Withhold consuming extension members from borrowed receivers.
#[tokio::test]
async fn test_withhold_owned_extension_members_from_borrowed_receivers() {
    let source = r#"class Crate {
    label: string = "";
}

extension of ^Crate {
    consume(): string {
        return "gone";
    }
}

extension of &readonly Crate {
    peek(): string {
        return "seen";
    }
}

function inspect(crate: &readonly Crate): string {
    return crate.peek();
}
"#;
    let (mut server, document) = TestServer::open_workspace(
        "ownership-member-completion",
        &[("src/main.ds", source)],
        "src/main.ds",
    )
    .await;

    // expect the borrow extension but not the owned one on a borrowed receiver
    let params = document.completion(position(17, 17));
    let response = server
        .request::<lsp::request::Completion>(params)
        .await
        .unwrap();
    let labels: Vec<String> = match response {
        Some(lsp::CompletionResponse::Array(items)) => {
            items.into_iter().map(|item| item.label).collect()
        }
        Some(lsp::CompletionResponse::List(list)) => {
            list.items.into_iter().map(|item| item.label).collect()
        }
        None => Vec::new(),
    };
    assert_eq!(labels, ["label", "peek", "borrow"]);
}

/// Complete blanket extension members on primitive receivers.
#[tokio::test]
async fn test_complete_blanket_extension_members_on_floats() {
    let source = r#"declare const value: float64;
const scaled: float64 = value.sqrt();
"#;
    let (mut server, document) = TestServer::open_workspace(
        "blanket-member-completion",
        &[("src/main.ds", source)],
        "src/main.ds",
    )
    .await;

    // expect the Float blanket members on a float literal
    let params = document.completion(position(1, 30));
    let response = server
        .request::<lsp::request::Completion>(params)
        .await
        .unwrap();
    let labels: Vec<String> = match response {
        Some(lsp::CompletionResponse::Array(items)) => {
            items.into_iter().map(|item| item.label).collect()
        }
        Some(lsp::CompletionResponse::List(list)) => {
            list.items.into_iter().map(|item| item.label).collect()
        }
        None => Vec::new(),
    };
    assert!(labels.contains(&"sqrt".to_string()), "labels: {labels:?}");
    assert!(labels.contains(&"isNaN".to_string()), "labels: {labels:?}");
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

/// Resolve intrinsic declarations exported through the builtin global provider.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_hover_builtin_intrinsics() {
    let source = r#"@languageItem("test.Value")
export newtype Value = string;

@intrinsic("test.value")
export declare function value(): int32;
"#;
    let mut server = TestServer::new("builtin-intrinsic-hover");
    let document = server.write("main.ds", source);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // follow both intrinsic decorator references to their library declarations
    server.open(&document, 1, source).await;
    let params = document.hover(position(0, 1));
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "**Signature**\n\n```ds\nexport newtype languageItem = (string,) | ()\n```\n\n\
             **Documentation**\n\nCompiler language item marker.\n\n\
             ```\n@languageItem(\"memory.Unique\")\nexport newtype Unique<T> = intrinsic;\n```\n\n\
             **Location**\n\n`destack://decorator/intrinsic.ds:17:16`",
        )),
        range: Some(range(0, 1, 0, 13)),
    };
    server
        .assert_request::<lsp::request::HoverRequest>(params, Ok(Some(expected)))
        .await;

    let params = document.hover(position(3, 1));
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "**Signature**\n\n```ds\nexport newtype intrinsic = (string,) | ()\n```\n\n\
             **Documentation**\n\nCompiler intrinsic marker.\n\n\
             ```\n@intrinsic\ndeclare function typeOf<T>(value: T): Type<T>;\n```\n\n\
             **Location**\n\n`destack://decorator/intrinsic.ds:8:16`",
        )),
        range: Some(range(3, 1, 3, 10)),
    };
    server
        .assert_request::<lsp::request::HoverRequest>(params, Ok(Some(expected)))
        .await;

    // retain the ordinary declaration identity of an intrinsic function
    let params = document.hover(position(4, 25));
    let location = format!("{}:5:25", document.uri().as_str());
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(format!(
            "**Signature**\n\n```ds\nexport declare function value(): int32\n```\n\n\
             **Location**\n\n`{location}`"
        ))),
        range: Some(range(4, 24, 4, 29)),
    };
    server
        .assert_request::<lsp::request::HoverRequest>(params, Ok(Some(expected)))
        .await;
}

/// Return documentation attached directly to an authored expression.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_hover_expression_documentation() {
    let source = r#"const result =
    /// Computed value.
    42;
"#;
    let mut server = TestServer::new("expression-documentation-hover");
    let document = server.write("main.ds", source);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // return the expression documentation without a symbol declaration
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
    let params = document.hover(position(2, 4));
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown("Computed value.")),
        range: Some(range(2, 4, 2, 6)),
    };
    server
        .assert_request::<lsp::request::HoverRequest>(params, Ok(Some(expected)))
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
