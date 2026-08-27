use std::collections::HashMap;

use destack_lsp_types as lsp;

use super::tests::{MANIFEST, TestServer, position, range};

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
    let library = server.write("src/library.ds", library);
    let document = server.write("src/main.ds", source);
    server
        .initialize(lsp::ClientCapabilities::default(), None)
        .await
        .unwrap();
    server.initialized().await;
    server.open(&document, 1, source).await;
    server.assert_diagnostics(&document, 1, Vec::new()).await;

    // rename the declaration, import selector, and uses from one use site
    let expected = lsp::WorkspaceEdit {
        changes: Some(HashMap::from([
            (
                library.uri().clone(),
                vec![lsp::TextEdit {
                    range: range(0, 13, 0, 19),
                    new_text: "result".to_string(),
                }],
            ),
            (
                document.uri().clone(),
                vec![
                    lsp::TextEdit {
                        range: range(0, 9, 0, 15),
                        new_text: "result".to_string(),
                    },
                    lsp::TextEdit {
                        range: range(2, 16, 2, 22),
                        new_text: "result".to_string(),
                    },
                    lsp::TextEdit {
                        range: range(2, 25, 2, 31),
                        new_text: "result".to_string(),
                    },
                ],
            ),
        ])),
        ..lsp::WorkspaceEdit::default()
    };
    server
        .assert_request(
            document.rename(position(2, 16), "result"),
            Ok(Some(expected)),
        )
        .await;
}
