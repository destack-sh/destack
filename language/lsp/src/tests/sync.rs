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
        contents: lsp::HoverContents::Markup(markdown(format!(
            "**Signature**\n\n```ds\nexport function answer(): float64\n```\n\n\
             **Location**\n\n`{}:1:17`",
            document.uri().as_str()
        ))),
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

    // publish only the current valid revision
    server
        .assert_notification::<lsp::notification::PublishDiagnostics>(
            lsp::PublishDiagnosticsParams {
                uri: document.uri().clone(),
                diagnostics: Vec::new(),
                version: Some(4),
            },
        )
        .await;

    // query the same current revision
    let params = document.hover(position(0, 16));
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(format!(
            "**Signature**\n\n```ds\nexport function answer(): float64\n```\n\n\
             **Location**\n\n`{}:1:17`",
            document.uri().as_str()
        ))),
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

/// Query successive editor revisions of the authored Builtin Package.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_query_authored_builtin_while_typing() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../library");
    let root = fs::canonicalize(root).unwrap();
    let path = root.join("src/fs/fs.ds");
    let source = fs::read_to_string(&path).unwrap();
    let document = TestDocument::from(path.as_path());
    let mut server = TestServer::new("authored-builtin-typing");
    server
        .initialize_workspace(&root, lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;
    server.open(&document, 1, &source).await;
    let (version, cursor) = server
        .type_text(&document, 1, position(0, 0), "declare const x")
        .await;

    // overlap authored Builtin Package queries with the completing revisions
    let outline = server
        .start_request::<lsp::request::DocumentSymbolRequest>(document.outline())
        .await;
    let hints = server
        .start_request::<lsp::request::InlayHintRequest>(document.inlay_hints(range(
            0,
            0,
            0,
            cursor.character,
        )))
        .await;
    let tokens = server
        .start_request::<lsp::request::SemanticTokensFullRequest>(document.semantic_tokens())
        .await;
    let (version, cursor) = server
        .type_text(&document, version, cursor, ": Clone;")
        .await;

    // accept completed results and explicit supersession
    outline.wait_or_content_modified().await;
    hints.wait_or_content_modified().await;
    tokens.wait_or_content_modified().await;

    // query the final authored Builtin Package revision
    server
        .request::<lsp::request::DocumentSymbolRequest>(document.outline())
        .await
        .unwrap();
    server
        .request::<lsp::request::InlayHintRequest>(document.inlay_hints(range(
            0,
            0,
            0,
            cursor.character,
        )))
        .await
        .unwrap();
    server
        .request::<lsp::request::SemanticTokensFullRequest>(document.semantic_tokens())
        .await
        .unwrap();

    // require successful diagnostics for the final source
    server
        .assert_diagnostics(&document, version, Vec::new())
        .await;
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
    server
        .assert_notification::<lsp::notification::PublishDiagnostics>(
            lsp::PublishDiagnosticsParams {
                uri: document.uri().clone(),
                diagnostics: Vec::new(),
                version: Some(2),
            },
        )
        .await;

    // observe the renamed parameter through a semantic query
    let params = document.hover(position(0, 16));
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(format!(
            "**Signature**\n\n```ds\nexport function choose(input: string): string\n```\n\n\
             **Location**\n\n`{}:1:17`",
            document.uri().as_str()
        ))),
        range: Some(range(0, 16, 0, 22)),
    };
    server
        .assert_request::<lsp::request::HoverRequest>(params, Ok(Some(expected)))
        .await;
}

/// Publish parser diagnostics when checking fails.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_publish_parser_diagnostics_before_check_failure() {
    let valid = "struct Report {}\nclass Foo {}\n";
    let invalid = "struct Report {}\nclass Foo {\n    @;\n    like\n}\n";
    let mut server = TestServer::new("parser-diagnostics-before-check-failure");
    let document = server.write("main.ds", valid);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;

    // begin from one valid checked revision
    server.open(&document, 1, valid).await;
    server.assert_diagnostics(&document, 1, Vec::new()).await;

    // retain parser diagnostics when the edited field also fails checking
    server
        .change(&document, 2, [replace_document(invalid)])
        .await;
    server
        .assert_diagnostics(
            &document,
            2,
            vec![
                lsp::Diagnostic {
                    range: range(3, 4, 3, 8),
                    severity: Some(lsp::DiagnosticSeverity::ERROR),
                    code: Some(lsp::NumberOrString::String(
                        "missing-type-annotation".to_string(),
                    )),
                    source: Some("destack".to_string()),
                    message: "missing type annotation".to_string(),
                    ..lsp::Diagnostic::default()
                },
                lsp::Diagnostic {
                    range: range(2, 5, 2, 6),
                    severity: Some(lsp::DiagnosticSeverity::ERROR),
                    code: Some(lsp::NumberOrString::String("unexpected-token".to_string())),
                    source: Some("destack".to_string()),
                    message: "unexpected ;".to_string(),
                    ..lsp::Diagnostic::default()
                },
                lsp::Diagnostic {
                    range: range(2, 5, 2, 6),
                    severity: Some(lsp::DiagnosticSeverity::ERROR),
                    code: Some(lsp::NumberOrString::String("expected-member".to_string())),
                    source: Some("destack".to_string()),
                    message: "expected member".to_string(),
                    ..lsp::Diagnostic::default()
                },
            ],
        )
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
        contents: lsp::HoverContents::Markup(markdown(format!(
            "**Signature**\n\n```ds\nexport function answer(): boolean\n```\n\n\
             **Location**\n\n`{}:1:17`",
            document.uri().as_str()
        ))),
        range: Some(range(0, 16, 0, 22)),
    };
    server
        .assert_request::<lsp::request::HoverRequest>(params, Ok(Some(expected)))
        .await;

    // return ownership to the unchanged filesystem source
    server.close(&document).await;
    let params = document.hover(position(0, 16));
    let expected = lsp::Hover {
        contents: lsp::HoverContents::Markup(markdown(format!(
            "**Signature**\n\n```ds\nexport function answer(): float64\n```\n\n\
             **Location**\n\n`{}:1:17`",
            document.uri().as_str()
        ))),
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
        contents: lsp::HoverContents::Markup(markdown(format!(
            "**Signature**\n\n```ds\nexport function answer(): string\n```\n\n\
             **Location**\n\n`{}:1:17`",
            document.uri().as_str()
        ))),
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
        contents: lsp::HoverContents::Markup(markdown(format!(
            "**Signature**\n\n```ds\nexport function answer(): float64\n```\n\n\
             **Location**\n\n`{}:1:17`",
            document.uri().as_str()
        ))),
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
