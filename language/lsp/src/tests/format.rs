use tspp_lsp_types as lsp;

use super::tests::{TestServer, position, range};

/// Return exact edits for whole document, range, and on type formatting.
#[tokio::test]
async fn test_format_document_source() {
    let source = r#"function greet():string{return "hi";}
"#;
    let (mut server, document) = TestServer::open_workspace(
        "document-formatting",
        &[("src/main.tspp", source)],
        "src/main.tspp",
    )
    .await;

    // describe the canonical replacement
    let edits = Some(vec![lsp::TextEdit {
        range: range(0, 0, 1, 0),
        new_text: "function greet(): string {\n    return \"hi\";\n}\n".to_string(),
    }]);

    // format the complete document
    server
        .assert_request(document.format(), Ok(edits.clone()))
        .await;

    // format the declaration selected by a source range
    server
        .assert_request(document.format_range(range(0, 0, 0, 37)), Ok(edits.clone()))
        .await;

    // format the declaration containing the closing brace trigger
    server
        .assert_request(document.format_on_type(position(0, 37), "}"), Ok(edits))
        .await;
}
