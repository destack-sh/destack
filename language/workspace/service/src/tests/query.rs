use destack_workspace::query;

use crate::tests::harness::TestWorkspaceService;

/// Resolve document symbols through workspace queries.
#[test]
fn test_workspace_service_query_document_symbols() {
    let test = TestWorkspaceService::new("workspace_service_query");
    let path = test.path_for("main.ds");
    let uri = test.uri_for_path(&path);
    let source = r#"export function add(a: number, b: number) {
    return a + b;
}
"#;

    let _ = test
        .fs
        .write_text("main.ds", source)
        .expect("expected source write");

    let _ = test
        .service
        .update_virtual_file(&path, source.to_string())
        .expect("expected virtual update");

    let response = test
        .service
        .query_for_path(
            &path,
            query::QueryRequest::DocumentSymbols(query::DocumentSymbolsRequest { uri }),
        )
        .expect("expected query response");

    let query::QueryResponse::DocumentSymbols(document_symbols) = response.response else {
        panic!("expected document symbols response")
    };

    assert!(
        !document_symbols.symbols.is_empty(),
        "expected at least one document symbol"
    );
}
