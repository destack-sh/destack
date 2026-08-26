use destack_lsp_types as lsp;

use super::tests::{CompletionDisplay, MANIFEST, TestServer, markdown, position, range};

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
    let diagnostics = server.receive_diagnostics(&document, 1).await;
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
    assert_eq!(labels, ["y"]);
}

/// Return declaration, member, and callable details in completion lists.
#[tokio::test]
async fn test_return_completion_details() {
    let source = r#"struct HostErrorContextProcess {
    kind: "process";
    syscall?: string;
    /// Send one code.
    /// @param code - The code to send.
    send(code: int32): string { return ""; }
}

interface Collection {
    type Item;
}
function item<T: Collection>(): T.Item { return todo("item"); }

declare const context: HostErrorContextProcess;
const syscall = context.syscall;
const sent = context.send(1);
"#;
    let mut server = TestServer::new("completion-details");
    server.write("destack.json", MANIFEST);
    let document = server.write("src/main.ds", source);
    let capabilities = lsp::ClientCapabilities {
        text_document: Some(lsp::TextDocumentClientCapabilities {
            completion: Some(lsp::CompletionClientCapabilities {
                completion_item: Some(lsp::CompletionItemCapability {
                    label_details_support: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    server.initialize(capabilities, None).await.unwrap();
    server.initialized().await;
    server.open(&document, 1, source).await;
    server.assert_diagnostics(&document, 1, Vec::new()).await;

    // show the exact declaration beside one type completion
    server
        .assert_completion(
            &document,
            position(13, 46),
            CompletionDisplay {
                label: "HostErrorContextProcess",
                label_detail: None,
                description: None,
                detail: Some("struct HostErrorContextProcess"),
                documentation: Some("```ds\nstruct HostErrorContextProcess\n```"),
            },
        )
        .await;

    // show the exact declaration beside one associated type
    server
        .assert_completion(
            &document,
            position(11, 38),
            CompletionDisplay {
                label: "Item",
                label_detail: Some(": Collection.Item"),
                description: None,
                detail: Some("Collection.Item"),
                documentation: Some("```ds\nCollection.Item\n```"),
            },
        )
        .await;

    // show the exact value type beside one field completion
    server
        .assert_completion(
            &document,
            position(14, 26),
            CompletionDisplay {
                label: "syscall",
                label_detail: Some(": string"),
                description: None,
                detail: Some("HostErrorContextProcess.syscall: string"),
                documentation: Some("```ds\nHostErrorContextProcess.syscall: string\n```"),
            },
        )
        .await;

    // show parameter and return types beside one callable completion
    server
        .assert_completion(
            &document,
            position(15, 23),
            CompletionDisplay {
                label: "send",
                label_detail: Some("(code: int32): string"),
                description: None,
                detail: Some("HostErrorContextProcess.send(code: int32): string"),
                documentation: Some(concat!(
                    "```ds\n",
                    "HostErrorContextProcess.send(code: int32): string\n",
                    "```\n\n",
                    "Send one code.\n\n",
                    "## Parameters\n\n",
                    "- `code`: The code to send.",
                )),
            },
        )
        .await;
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
    server.assert_diagnostics(&document, 1, Vec::new()).await;

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
    assert_eq!(
        labels,
        [
            "tricks", "name", "bark", "toString", "borrow", "into", "tryInto",
        ],
    );
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
    assert_eq!(
        labels,
        ["label", "peek", "toString", "borrow", "into", "tryInto"],
    );
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
    server.assert_diagnostics(&document, 1, Vec::new()).await;
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
    server
        .assert_request::<lsp::request::DocumentLinkRequest>(document.links(), Ok(expected))
        .await;

    // read the exact source exposed by the resolved link
    let params = lsp::TextDocumentContentParams { uri: target };
    let expected = lsp::TextDocumentContentResult {
        text: "export * from \"./console.ds\";\n".to_string(),
    };
    server
        .assert_request::<lsp::request::TextDocumentContentRequest>(params, Ok(expected))
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
            "`destack://decorator/intrinsic.ds:7:16`\n\n\
             ```ds\n@languageItem(\"decorator.languageItem\")\n\
             export newtype languageItem = (string,) | ()\n```\n\n\
             Compiler language item marker.",
        )),
        range: Some(range(0, 1, 0, 13)),
    };
    server
        .assert_request::<lsp::request::HoverRequest>(params, Ok(Some(expected)))
        .await;

    let params = document.hover(position(3, 1));
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`destack://decorator/intrinsic.ds:3:16`\n\n\
             ```ds\n@languageItem(\"decorator.intrinsic\")\n\
             export newtype intrinsic = (string,) | ()\n```\n\n\
             Compiler intrinsic marker.",
        )),
        range: Some(range(3, 1, 3, 10)),
    };
    server
        .assert_request::<lsp::request::HoverRequest>(params, Ok(Some(expected)))
        .await;

    // retain the ordinary declaration identity of an intrinsic function
    let params = document.hover(position(4, 25));
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`main.ds:5:25`\n\n\
             ```ds\n@intrinsic(\"test.value\")\n\
             export declare function value(): int32\n```",
        )),
        range: Some(range(4, 24, 4, 29)),
    };
    server
        .assert_request::<lsp::request::HoverRequest>(params, Ok(Some(expected)))
        .await;
}

/// Render declaration decorators and workspace-relative hover locations.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_render_hover_declaration() {
    let source = r#"newtype marker = (string,);

/// Provide one service.
@marker("service")
export interface Service {}

declare const service: Service;
"#;
    let (mut server, document) = TestServer::open_workspace(
        "render-hover-declaration",
        &[("src/main.ds", source)],
        "src/main.ds",
    )
    .await;

    // render the complete declaration and its workspace location
    let params = document.hover(position(6, 24));
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`src/main.ds:5:18`\n\n\
             ```ds\n@marker(\"service\")\nexport interface Service\n```\n\n\
             Provide one service.",
        )),
        range: Some(range(6, 23, 6, 30)),
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
    server.assert_diagnostics(&document, 1, Vec::new()).await;
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
    server.assert_diagnostics(&document, 1, Vec::new()).await;

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
