use tspp_lsp_server::jsonrpc;
use tspp_lsp_types as lsp;

use super::tests::{TestServer, markdown, position, range, replace, replace_document};

/// Open a package nested below its editor folder.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_open_nested_package() {
    let source = answer("float64", "1");
    let mut server = TestServer::new_editor_folder("nested-package");
    server.create_package("language/library");
    let document = server.write("language/library/main.tspp", &source);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // open the nested source through its declared package
    server.open(&document, 1, &source).await;
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`main.tspp:1:17`\n\n```tspp\nexport function answer(): float64\n```",
        )),
        range: Some(range(0, 16, 0, 22)),
    };
    server
        .assert_request(document.hover(position(0, 16)), Ok(Some(expected)))
        .await;
}

/// Open a standalone package nested below a configured workspace.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_open_standalone_package_below_workspace() {
    let manifest = r#"{
  "name": "lsp-workspace",
  "workspace": {
    "packages": ["member"]
  }
}
"#;
    let source = r#"import { log } from "tspp:console";

export function answer(): float64 {
  log("answer");
  return 1;
}

declare const missing: MissingType;
"#;

    // open one package excluded from its parent workspace
    let mut server = TestServer::new_editor_folder("standalone-package-below-workspace");
    server.write("destack.json", manifest);
    server.create_package("member");
    server.create_package("standalone");
    let document = server.write("standalone/main.tspp", source);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;
    server.open(&document, 1, source).await;

    // query the source through its standalone package
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`main.tspp:3:17`\n\n```tspp\nexport function answer(): float64\n```",
        )),
        range: Some(range(2, 16, 2, 22)),
    };
    server
        .assert_request(document.hover(position(2, 16)), Ok(Some(expected)))
        .await;

    // resolve imports through the same project identity
    let target = server.builtin_uri_at(
        &server.root().join("standalone"),
        "tspp://console/index.tspp",
    );
    let expected = Some(vec![lsp::DocumentLink {
        range: range(0, 20, 0, 34),
        target: Some(target),
        tooltip: None,
        data: None,
    }]);
    server.assert_request(document.links(), Ok(expected)).await;

    // publish diagnostics from the standalone revision
    let expected = vec![document.error(
        range(7, 23, 7, 34),
        "unresolved-reference",
        "cannot find 'MissingType'",
    )];
    server.assert_diagnostics(&document, 1, expected).await;
}

/// Publish and query only the latest of several rapid document revisions.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_publish_latest_document_revision() {
    let first = answer("float64", "false");
    let second = answer("float64", r#""wrong""#);
    let third = answer("float64", "true");
    let fourth = answer("float64", "1");
    let mut server = TestServer::new("latest-document-revision");
    let document = server.write("main.tspp", &first);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // supersede several invalid revisions before diagnostic work begins
    server.open(&document, 1, &first).await;
    server
        .change(&document, 2, [replace_document(second)])
        .await;
    server.change(&document, 3, [replace_document(third)]).await;
    server
        .change(&document, 4, [replace_document(fourth)])
        .await;

    // publish the current valid revision
    server.assert_diagnostics(&document, 4, Vec::new()).await;

    // query the same current revision
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`main.tspp:1:17`\n\n```tspp\nexport function answer(): float64\n```",
        )),
        range: Some(range(0, 16, 0, 22)),
    };
    server
        .assert_request(document.hover(position(0, 16)), Ok(Some(expected)))
        .await;
}

/// Serve overlapping editor queries while one declaration is typed character by character.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_query_successive_typed_revisions() {
    let initial = r#"// module
"#;
    let prefix = "declare const x";
    let suffix = ": Clone;";
    let (mut server, document) = TestServer::open_workspace(
        "successive-typed-revisions",
        &[("src/main.tspp", initial)],
        "src/main.tspp",
    )
    .await;
    let (version, cursor) = server.type_text(&document, 1, position(1, 0), prefix).await;

    // overlap queries for the incomplete binding with later revisions
    let outline = server.start_request(document.outline()).await;
    let hints = server
        .start_request(document.inlay_hints(range(1, 0, 1, cursor.character)))
        .await;
    let tokens = server.start_request(document.semantic_tokens()).await;
    let completion = server.start_request(document.completion(cursor)).await;
    let (version, cursor) = server.type_text(&document, version, cursor, suffix).await;

    // accept completed results and explicit supersession
    outline.wait_or_content_modified().await;
    hints.wait_or_content_modified().await;
    tokens.wait_or_content_modified().await;
    completion.wait_or_content_modified().await;

    // query the complete current revision
    server.request(document.outline()).await.unwrap();
    server
        .request(document.inlay_hints(range(1, 0, 1, cursor.character)))
        .await
        .unwrap();
    server.request(document.semantic_tokens()).await.unwrap();
    server.complete(document.completion(cursor)).await;

    // require successful diagnostics for the final source
    server
        .assert_diagnostics(&document, version, Vec::new())
        .await;
}

/// Complete constructor receiver members immediately after progressive typing.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_complete_constructor_receiver_after_typing() {
    let source = r#"class User {
    name: string;
    age: uint;

    constructor(name: string, age: uint) {
        this.name = name;
        this.age = age;
        this."#;

    // type the class through successive document revisions
    let mut server = TestServer::new("constructor-receiver-typing");
    let document = server.write("main.tspp", "");
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;
    server.open(&document, 1, "").await;
    server.assert_diagnostics(&document, 1, Vec::new()).await;
    let (_version, cursor) = server.type_text(&document, 1, position(0, 0), source).await;

    // complete the current receiver while background work observes earlier revisions
    let labels = server.completion_labels(document.completion(cursor)).await;
    assert_eq!(labels, ["name", "age", "borrow", "into", "tryInto"],);
}

/// Apply one ordered batch of ranged edits using UTF-16 source positions.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_apply_incremental_document_changes() {
    let source = r#"export function choose(value: string): string {
  const emoji: string = "👋"; const current: string = value;
  return current;
}
"#;
    let mut server = TestServer::new("incremental-document-changes");
    let document = server.write("main.tspp", source);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // rename declarations and references from the end of the document
    server.open(&document, 1, source).await;
    server
        .change(
            &document,
            2,
            [
                replace(range(2, 9, 2, 16), "selected"),
                replace(range(1, 54, 1, 59), "input"),
                replace(range(1, 36, 1, 43), "selected"),
                replace(range(0, 23, 0, 28), "input"),
            ],
        )
        .await;

    // require the edited document to remain valid
    server.assert_diagnostics(&document, 2, Vec::new()).await;

    // observe the renamed parameter through a semantic query
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`main.tspp:1:17`\n\n```tspp\nexport function choose(input: string): string\n```",
        )),
        range: Some(range(0, 16, 0, 22)),
    };
    server
        .assert_request(document.hover(position(0, 16)), Ok(Some(expected)))
        .await;
}

/// Restore filesystem contents when an editor document closes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_restore_file_after_closing_document() {
    let stored = answer("float64", "1");
    let opened = answer("boolean", "true");
    let mut server = TestServer::new("close-document");
    let document = server.write("main.tspp", &stored);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // query the editor owned source
    server.open(&document, 1, &opened).await;
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`main.tspp:1:17`\n\n```tspp\nexport function answer(): boolean\n```",
        )),
        range: Some(range(0, 16, 0, 22)),
    };
    server
        .assert_request(document.hover(position(0, 16)), Ok(Some(expected)))
        .await;

    // return ownership to the unchanged filesystem source
    server.close(&document).await;
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`main.tspp:1:17`\n\n```tspp\nexport function answer(): float64\n```",
        )),
        range: Some(range(0, 16, 0, 22)),
    };
    server
        .assert_request(document.hover(position(0, 16)), Ok(Some(expected)))
        .await;
}

/// Keep saved editor contents after returning ownership to the filesystem.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_save_document_contents() {
    let stored = answer("float64", "1");
    let opened = answer("boolean", "true");
    let saved = answer("string", r#""saved""#);
    let mut server = TestServer::new("save-document");
    let document = server.write("main.tspp", &stored);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // update both the editor document and its physical file
    server.open(&document, 1, &opened).await;
    server.write("main.tspp", &saved);
    server.save(&document, Some(&saved)).await;

    // return ownership to the saved physical source
    server.close(&document).await;
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`main.tspp:1:17`\n\n```tspp\nexport function answer(): string\n```",
        )),
        range: Some(range(0, 16, 0, 22)),
    };
    server
        .assert_request(document.hover(position(0, 16)), Ok(Some(expected)))
        .await;
}

/// Retain a project until its final editor document closes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_retain_project_for_open_document() {
    let source = answer("float64", "1");
    let mut server = TestServer::new("open-document-project");
    let document = server.write("main.tspp", &source);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;
    server.open(&document, 1, &source).await;

    // remove the editor folder while its document remains open
    let folder = lsp::WorkspaceFolder {
        uri: server.root_uri(),
        name: "open-document-project".to_string(),
    };
    server
        .notify::<lsp::notification::DidChangeWorkspaceFolders>(
            lsp::DidChangeWorkspaceFoldersParams {
                event: lsp::WorkspaceFoldersChangeEvent {
                    added: Vec::new(),
                    removed: vec![folder],
                },
            },
        )
        .await;

    // continue serving the open document from its retained project
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`main.tspp:1:17`\n\n```tspp\nexport function answer(): float64\n```",
        )),
        range: Some(range(0, 16, 0, 22)),
    };
    server
        .assert_request(document.hover(position(0, 16)), Ok(Some(expected)))
        .await;

    // release the project after its final document closes
    server.close(&document).await;
    let expected = Err(jsonrpc::Error::invalid_params(format!(
        "no Destack project owns {}",
        document.uri().path().as_str()
    )));
    server
        .assert_request(document.hover(position(0, 16)), expected)
        .await;
}

/// Build one answer function.
fn answer(return_type: &str, value: &str) -> String {
    format!(
        r#"export function answer(): {return_type} {{
  const value: {return_type} = {value};
  return value;
}}
"#
    )
}
