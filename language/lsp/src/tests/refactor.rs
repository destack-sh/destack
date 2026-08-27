use destack_lsp_types as lsp;

use super::tests::{MANIFEST, TestServer, position};

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
