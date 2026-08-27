use destack_lsp_types as lsp;

use super::tests::{CompletionDisplay, MANIFEST, TestServer, markdown, position, range};

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
