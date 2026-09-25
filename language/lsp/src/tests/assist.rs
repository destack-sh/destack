use tspp_lsp_server::jsonrpc;
use tspp_lsp_types as lsp;

use super::tests::{
    CompletionDisplay, MANIFEST, TestServer, markdown, position, range, replace_document,
};

/// Return exact signature help and inlay hints for one call.
#[tokio::test]
async fn test_return_signature_help_and_inlay_hints() {
    let source = r#"/// Send one message.
/// @param code - The destination code.
/// @param message - The message text.
function send(code: int32, message: string): boolean {
    return true;
}
const sent = send(1, "ok");
"#;
    let (mut server, document) = TestServer::open_workspace(
        "call-assists",
        &[("src/main.tspp", source)],
        "src/main.tspp",
    )
    .await;

    // select the second parameter and retain callable documentation
    let signature = Some(lsp::SignatureHelp {
        signatures: vec![lsp::SignatureInformation {
            label: "send(code: int32, message: string): boolean".to_string(),
            documentation: Some(lsp::Documentation::MarkupContent(markdown(
                "Send one message.",
            ))),
            parameters: Some(vec![
                lsp::ParameterInformation {
                    label: lsp::ParameterLabel::Simple("code: int32".to_string()),
                    documentation: Some(lsp::Documentation::MarkupContent(markdown(
                        "The destination code.",
                    ))),
                },
                lsp::ParameterInformation {
                    label: lsp::ParameterLabel::Simple("message: string".to_string()),
                    documentation: Some(lsp::Documentation::MarkupContent(markdown(
                        "The message text.",
                    ))),
                },
            ]),
            active_parameter: None,
        }],
        active_signature: Some(0),
        active_parameter: Some(1),
    });
    server
        .assert_request(document.signature_help(position(6, 21)), Ok(signature))
        .await;

    // return the inferred binding type and both parameter labels
    let binding = lsp::InlayHint {
        position: position(6, 10),
        label: lsp::InlayHintLabel::String(": boolean".to_string()),
        kind: Some(lsp::InlayHintKind::TYPE),
        text_edits: None,
        tooltip: None,
        padding_left: Some(false),
        padding_right: Some(false),
        data: None,
    };
    let parameter = lsp::InlayHint {
        position: position(6, 18),
        label: lsp::InlayHintLabel::String("code:".to_string()),
        kind: Some(lsp::InlayHintKind::PARAMETER),
        padding_right: Some(true),
        ..binding.clone()
    };
    let message = lsp::InlayHint {
        position: position(6, 21),
        label: lsp::InlayHintLabel::String("message:".to_string()),
        ..parameter.clone()
    };
    let hints = Some(vec![binding, parameter, message]);
    server
        .assert_request(document.inlay_hints(range(0, 0, 6, 27)), Ok(hints))
        .await;
}

/// Resolve declaration, documentation, and import edits for selected completion entries.
#[tokio::test]
async fn test_resolve_completion_details() {
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
    server.write("package.json", MANIFEST);
    server.write(
        "src/library.tspp",
        "export function greetFixture(): void {}\n",
    );
    let document = server.write("src/main.tspp", source);
    server
        .initialize_completion(&["detail", "documentation", "additionalTextEdits"], true)
        .await;
    server.open(&document, 1, source).await;
    server.assert_diagnostics(&document, 1, Vec::new()).await;

    // defer expanded fields until the client selects an entry
    let item = server
        .select_completion(
            document.completion(position(13, 46)),
            "HostErrorContextProcess",
        )
        .await;
    assert_eq!(
        CompletionDisplay::from(&item),
        CompletionDisplay {
            label: "HostErrorContextProcess",
            label_detail: None,
            description: None,
            detail: None,
            documentation: None,
        },
    );
    assert!(item.data.as_ref().is_some_and(|data| data.is_string()));
    let item = server.resolve_completion(item).await.unwrap();
    assert_eq!(
        CompletionDisplay::from(&item),
        CompletionDisplay {
            label: "HostErrorContextProcess",
            label_detail: None,
            description: None,
            detail: Some("struct HostErrorContextProcess"),
            documentation: Some("```tspp\nstruct HostErrorContextProcess\n```"),
        },
    );

    // show the exact declaration beside one associated type
    server
        .assert_completion(
            document.completion(position(11, 38)),
            CompletionDisplay {
                label: "Item",
                label_detail: Some(": Collection.Item"),
                description: None,
                detail: Some("Collection.Item"),
                documentation: Some("```tspp\nCollection.Item\n```"),
            },
        )
        .await;

    // show the exact value type beside one field completion
    server
        .assert_completion(
            document.completion(position(14, 26)),
            CompletionDisplay {
                label: "syscall",
                label_detail: Some(": string"),
                description: None,
                detail: Some("HostErrorContextProcess.syscall: string"),
                documentation: Some("```tspp\nHostErrorContextProcess.syscall: string\n```"),
            },
        )
        .await;

    // show parameter and return types beside one callable completion
    server
        .assert_completion(
            document.completion(position(15, 23)),
            CompletionDisplay {
                label: "send",
                label_detail: Some("(code: int32): string"),
                description: None,
                detail: Some("HostErrorContextProcess.send(code: int32): string"),
                documentation: Some(
                    r#"```tspp
HostErrorContextProcess.send(code: int32): string
```

Send one code.

## Parameters

- `code`: The code to send."#,
                ),
            },
        )
        .await;

    // reject an entry after its source revision changes
    let stale = server
        .select_completion(
            document.completion(position(13, 46)),
            "HostErrorContextProcess",
        )
        .await;
    let edited = format!(
        r#"{source}
function completeImport(): void {{
    greetFix;
    retur
}}
"#
    );
    server
        .change(&document, 2, [replace_document(edited)])
        .await;
    let error = server.resolve_completion(stale).await.unwrap_err();
    assert_eq!(error.code, jsonrpc::ErrorCode::ContentModified);

    // return an already complete keyword unchanged
    let item = server
        .select_completion(document.completion(position(19, 9)), "return")
        .await;
    assert_eq!(item.data, None);
    let resolved = server.resolve_completion(item.clone()).await.unwrap();
    assert_eq!(resolved, item);

    // return an auto import edit only after selecting its completion entry
    let item = server
        .select_completion(document.completion(position(18, 12)), "greetFixture")
        .await;
    assert_eq!(item.additional_text_edits, None);
    let item = server.resolve_completion(item).await.unwrap();
    assert_eq!(
        item.additional_text_edits,
        Some(vec![lsp::TextEdit {
            range: range(0, 0, 0, 0),
            new_text: "import { greetFixture } from \"./library\";\n".to_string(),
        }]),
    );
}

/// Return complete auto-import entries to clients without lazy resolution.
#[tokio::test]
async fn test_return_eager_completion_details() {
    let source = r#"function main(): void {
    greetFix;
}
"#;
    let mut server = TestServer::new("eager-completion-details");
    server.write("package.json", MANIFEST);
    server.write(
        "src/library.tspp",
        r#"/// Greet one fixture.
export function greetFixture(): void {}
"#,
    );
    let document = server.write("src/main.tspp", source);
    server.initialize_completion(&["detail"], false).await;
    server.open(&document, 1, source).await;

    // include expanded fields and import edits in the initial list
    let item = server
        .select_completion(document.completion(position(1, 12)), "greetFixture")
        .await;
    assert_eq!(item.data, None);
    assert_eq!(
        CompletionDisplay::from(&item),
        CompletionDisplay {
            label: "greetFixture",
            label_detail: None,
            description: None,
            detail: Some("export function greetFixture(): void — from ./library"),
            documentation: Some(concat!(
                "```tspp\n",
                "export function greetFixture(): void\n",
                "```\n\n",
                "Greet one fixture.",
            )),
        },
    );
    assert_eq!(
        item.additional_text_edits,
        Some(vec![lsp::TextEdit {
            range: range(0, 0, 0, 0),
            new_text: "import { greetFixture } from \"./library\";\n".to_string(),
        }]),
    );
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
        &[("src/main.tspp", source)],
        "src/main.tspp",
    )
    .await;

    // offer declared and inherited fields before extension methods
    let labels = server
        .completion_labels(document.completion(position(15, 18)))
        .await;
    assert_eq!(
        labels,
        ["tricks", "name", "bark", "borrow", "into", "tryInto"],
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
        &[("src/main.tspp", source)],
        "src/main.tspp",
    )
    .await;

    // expect the borrow extension but not the owned one on a borrowed receiver
    let labels = server
        .completion_labels(document.completion(position(17, 17)))
        .await;
    assert_eq!(labels, ["label", "peek", "borrow", "into", "tryInto"],);
}

/// Complete blanket extension members on primitive receivers.
#[tokio::test]
async fn test_complete_blanket_extension_members_on_floats() {
    let source = r#"declare const value: float64;
const scaled: float64 = value.sqrt();
"#;
    let (mut server, document) = TestServer::open_workspace(
        "blanket-member-completion",
        &[("src/main.tspp", source)],
        "src/main.tspp",
    )
    .await;

    // render selected Float blanket members on a float receiver
    server
        .assert_completion(
            document.completion(position(1, 30)),
            CompletionDisplay {
                label: "isNaN",
                label_detail: None,
                description: None,
                detail: Some("isNaN(): boolean"),
                documentation: Some(
                    "```tspp\nisNaN(): boolean\n```\n\nReturn whether this value is NaN.",
                ),
            },
        )
        .await;
    server
        .assert_completion(
            document.completion(position(1, 30)),
            CompletionDisplay {
                label: "sqrt",
                label_detail: None,
                description: None,
                detail: Some("sqrt(): T"),
                documentation: Some("```tspp\nsqrt(): T\n```\n\nReturn the square root."),
            },
        )
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
    let document = server.write("main.tspp", source);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // follow both intrinsic decorator references to their library declarations
    server.open(&document, 1, source).await;
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`tspp://decorator/intrinsic.tspp:7:16`\n\n\
             ```tspp\n@languageItem(\"decorator.languageItem\")\n\
             export newtype languageItem = (string,) | ()\n```\n\n\
             Compiler language item marker.",
        )),
        range: Some(range(0, 1, 0, 13)),
    };
    server
        .assert_request(document.hover(position(0, 1)), Ok(Some(expected)))
        .await;

    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`tspp://decorator/intrinsic.tspp:3:16`\n\n\
             ```tspp\n@languageItem(\"decorator.intrinsic\")\n\
             export newtype intrinsic = (string,) | ()\n```\n\n\
             Compiler intrinsic marker.",
        )),
        range: Some(range(3, 1, 3, 10)),
    };
    server
        .assert_request(document.hover(position(3, 1)), Ok(Some(expected)))
        .await;

    // retain the ordinary declaration identity of an intrinsic function
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`main.tspp:5:25`\n\n\
             ```tspp\n@intrinsic(\"test.value\")\n\
             export declare function value(): int32\n```",
        )),
        range: Some(range(4, 24, 4, 29)),
    };
    server
        .assert_request(document.hover(position(4, 25)), Ok(Some(expected)))
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
        &[("src/main.tspp", source)],
        "src/main.tspp",
    )
    .await;

    // render the complete declaration and its workspace location
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`src/main.tspp:5:18`\n\n\
             ```tspp\n@marker(\"service\")\nexport interface Service\n```\n\n\
             Provide one service.",
        )),
        range: Some(range(6, 23, 6, 30)),
    };
    server
        .assert_request(document.hover(position(6, 24)), Ok(Some(expected)))
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
    let document = server.write("main.tspp", source);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // return the expression documentation without a symbol declaration
    server.open(&document, 1, source).await;
    server.assert_diagnostics(&document, 1, Vec::new()).await;
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown("Computed value.")),
        range: Some(range(2, 4, 2, 6)),
    };
    server
        .assert_request(document.hover(position(2, 4)), Ok(Some(expected)))
        .await;
}
