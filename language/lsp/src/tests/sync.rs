use std::fs;
use std::path::Path;

use destack_lsp_server::jsonrpc;
use destack_lsp_types as lsp;

use super::tests::{
    TestDocument, TestServer, markdown, position, range, replace, replace_document,
};

/// Open a package nested below its editor folder.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_open_nested_package() {
    let source = answer("float64", "1");
    let mut server = TestServer::new_editor_folder("nested-package");
    server.create_package("language/library");
    let document = server.write("language/library/main.ds", &source);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // open the nested source through its declared package
    server.open(&document, 1, &source).await;
    let params = document.hover(position(0, 16));
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`main.ds:1:17`\n\n```ds\nexport function answer(): float64\n```",
        )),
        range: Some(range(0, 16, 0, 22)),
    };
    server
        .assert_request::<lsp::request::HoverRequest>(params, Ok(Some(expected)))
        .await;
}

/// Publish and query only the latest of several rapid document revisions.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_publish_latest_document_revision() {
    let first = answer("float64", "false");
    let second = answer("float64", r#""wrong""#);
    let third = answer("float64", "true");
    let fourth = answer("float64", "1");
    let mut server = TestServer::new("latest-document-revision");
    let document = server.write("main.ds", &first);
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
    let params = document.hover(position(0, 16));
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`main.ds:1:17`\n\n```ds\nexport function answer(): float64\n```",
        )),
        range: Some(range(0, 16, 0, 22)),
    };
    server
        .assert_request::<lsp::request::HoverRequest>(params, Ok(Some(expected)))
        .await;
}

/// Serve overlapping editor queries while one declaration is typed character by character.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_query_successive_typed_revisions() {
    let initial = "// module\n";
    let prefix = "declare const x";
    let suffix = ": Clone;";
    let (mut server, document) = TestServer::open_workspace(
        "successive-typed-revisions",
        &[("src/main.ds", initial)],
        "src/main.ds",
    )
    .await;
    let (version, cursor) = server.type_text(&document, 1, position(1, 0), prefix).await;

    // overlap queries for the incomplete binding with later revisions
    let outline = server
        .start_request::<lsp::request::DocumentSymbolRequest>(document.outline())
        .await;
    let hints = server
        .start_request::<lsp::request::InlayHintRequest>(document.inlay_hints(range(
            1,
            0,
            1,
            cursor.character,
        )))
        .await;
    let tokens = server
        .start_request::<lsp::request::SemanticTokensFullRequest>(document.semantic_tokens())
        .await;
    let completion = server
        .start_request::<lsp::request::Completion>(document.completion(cursor))
        .await;
    let (version, cursor) = server.type_text(&document, version, cursor, suffix).await;

    // accept completed results and explicit supersession
    outline.wait_or_content_modified().await;
    hints.wait_or_content_modified().await;
    tokens.wait_or_content_modified().await;
    completion.wait_or_content_modified().await;

    // query the complete current revision
    server
        .request::<lsp::request::DocumentSymbolRequest>(document.outline())
        .await
        .unwrap();
    server
        .request::<lsp::request::InlayHintRequest>(document.inlay_hints(range(
            1,
            0,
            1,
            cursor.character,
        )))
        .await
        .unwrap();
    server
        .request::<lsp::request::SemanticTokensFullRequest>(document.semantic_tokens())
        .await
        .unwrap();
    server
        .request::<lsp::request::Completion>(document.completion(cursor))
        .await
        .unwrap();

    // require successful diagnostics for the final source
    server
        .assert_diagnostics(&document, version, Vec::new())
        .await;
}

/// Query incomplete binding revisions before a language item.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_query_incomplete_binding_before_language_item() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../library");
    let root = fs::canonicalize(root).unwrap();
    let path = root.join("src/memory/capability.ds");
    let library = fs::read_to_string(&path).unwrap();

    // insert an editable line before the Copy language item
    let language_item = "@languageItem(\"memory.Copy\")";
    let language_item_offset = library.find(language_item).unwrap();
    let insertion_line = library[..language_item_offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count() as u32;
    let source = library.replacen(language_item, &format!("\n{language_item}"), 1);

    // open the authored Builtin Package
    let document = TestDocument::from(path.as_path());
    let mut server = TestServer::new("incomplete-binding-before-language-item");
    server
        .initialize_workspace(&root, lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;
    server.open(&document, 1, &source).await;
    let (version, cursor) = server
        .type_text(&document, 1, position(insertion_line, 0), "declare const")
        .await;

    // query the incomplete declaration revision
    server
        .request::<lsp::request::SemanticTokensFullRequest>(document.semantic_tokens())
        .await
        .unwrap();
    server
        .assert_diagnostics(
            &document,
            version,
            vec![document.error(
                range(insertion_line + 1, 0, insertion_line + 1, 1),
                "expected-declarator",
                "expected declarator",
            )],
        )
        .await;

    // complete the declaration
    let (version, cursor) = server
        .type_text(&document, version, cursor, " x: Clone;")
        .await;

    // query the complete revision
    server
        .request::<lsp::request::DocumentSymbolRequest>(document.outline())
        .await
        .unwrap();
    server
        .request::<lsp::request::InlayHintRequest>(document.inlay_hints(range(
            insertion_line,
            0,
            insertion_line,
            cursor.character,
        )))
        .await
        .unwrap();
    server
        .request::<lsp::request::SemanticTokensFullRequest>(document.semantic_tokens())
        .await
        .unwrap();

    // clear diagnostics after completing the declaration
    server
        .assert_diagnostics(&document, version, Vec::new())
        .await;

    // request member completion on the declared value
    let (_version, cursor) = server.type_text(&document, version, cursor, "\nx.").await;
    server
        .request::<lsp::request::Completion>(document.completion(cursor))
        .await
        .unwrap();
    server
        .request::<lsp::request::DocumentSymbolRequest>(document.outline())
        .await
        .unwrap();
    server
        .request::<lsp::request::SemanticTokensFullRequest>(document.semantic_tokens())
        .await
        .unwrap();
    server
        .request::<lsp::request::InlayHintRequest>(document.inlay_hints(range(
            0,
            0,
            cursor.line,
            cursor.character,
        )))
        .await
        .unwrap();
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
    let document = server.write("main.ds", source);
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
    let params = document.hover(position(0, 16));
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`main.ds:1:17`\n\n```ds\nexport function choose(input: string): string\n```",
        )),
        range: Some(range(0, 16, 0, 22)),
    };
    server
        .assert_request::<lsp::request::HoverRequest>(params, Ok(Some(expected)))
        .await;
}

/// Restore filesystem contents when an editor document closes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_restore_file_after_closing_document() {
    let stored = answer("float64", "1");
    let opened = answer("boolean", "true");
    let mut server = TestServer::new("close-document");
    let document = server.write("main.ds", &stored);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // query the editor owned source
    server.open(&document, 1, &opened).await;
    let params = document.hover(position(0, 16));
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`main.ds:1:17`\n\n```ds\nexport function answer(): boolean\n```",
        )),
        range: Some(range(0, 16, 0, 22)),
    };
    server
        .assert_request::<lsp::request::HoverRequest>(params, Ok(Some(expected)))
        .await;

    // return ownership to the unchanged filesystem source
    server.close(&document).await;
    let params = document.hover(position(0, 16));
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`main.ds:1:17`\n\n```ds\nexport function answer(): float64\n```",
        )),
        range: Some(range(0, 16, 0, 22)),
    };
    server
        .assert_request::<lsp::request::HoverRequest>(params, Ok(Some(expected)))
        .await;
}

/// Keep saved editor contents after returning ownership to the filesystem.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_save_document_contents() {
    let stored = answer("float64", "1");
    let opened = answer("boolean", "true");
    let saved = answer("string", r#""saved""#);
    let mut server = TestServer::new("save-document");
    let document = server.write("main.ds", &stored);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // update both the editor document and its physical file
    server.open(&document, 1, &opened).await;
    server.write("main.ds", &saved);
    server.save(&document, Some(&saved)).await;

    // return ownership to the saved physical source
    server.close(&document).await;
    let params = document.hover(position(0, 16));
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`main.ds:1:17`\n\n```ds\nexport function answer(): string\n```",
        )),
        range: Some(range(0, 16, 0, 22)),
    };
    server
        .assert_request::<lsp::request::HoverRequest>(params, Ok(Some(expected)))
        .await;
}

/// Retain a project until its final editor document closes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_retain_project_for_open_document() {
    let source = answer("float64", "1");
    let mut server = TestServer::new("open-document-project");
    let document = server.write("main.ds", &source);
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
    let params = document.hover(position(0, 16));
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(
            "`main.ds:1:17`\n\n```ds\nexport function answer(): float64\n```",
        )),
        range: Some(range(0, 16, 0, 22)),
    };
    server
        .assert_request::<lsp::request::HoverRequest>(params, Ok(Some(expected)))
        .await;

    // release the project after its final document closes
    server.close(&document).await;
    let params = document.hover(position(0, 16));
    let expected = Err(jsonrpc::Error::invalid_params(format!(
        "no Destack project owns {}",
        document.uri().path().as_str()
    )));
    server
        .assert_request::<lsp::request::HoverRequest>(params, expected)
        .await;
}

/// Build one answer function.
fn answer(return_type: &str, value: &str) -> String {
    format!(
        "export function answer(): {return_type} {{\n  const value: {return_type} = {value};\n  \
         return value;\n}}\n"
    )
}
