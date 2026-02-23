use crate::tests::harness::TestLanguageService;
use crate::{LanguageServiceError, query};

/// Resolve document symbols through workspace queries.
#[test]
fn test_workspace_service_query_document_symbols() {
    let test = TestLanguageService::new("workspace_service_query");
    let source = r#"export function add(a: number, b: number) {
    return a + b;
}
"#;
    let path = test.write_text("main.ds", source);
    let uri = test.uri_for_path(&path);

    let _ = test.update_virtual_text(&path, source);

    let response = test
        .service
        .execute_query_for_path(
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

/// Advance semantic revision after virtual file updates.
#[test]
fn test_workspace_service_revisions_advance_after_updates() {
    let test = TestLanguageService::new("workspace_service_revision_updates");
    let source_a = "export const value = 1;\n";
    let source_b = "export const value = 2;\n";
    let path = test.write_text("main.ds", source_a);

    let _ = test.update_virtual_text(&path, source_a);
    let revision_a = test
        .service
        .revision_for_path(&path)
        .expect("expected first revision");

    let _ = test.update_virtual_text(&path, source_b);
    let revision_b = test
        .service
        .revision_for_path(&path)
        .expect("expected second revision");

    assert!(revision_b > revision_a, "expected revision to increase");
}

/// Require revision preconditions for mutating queries.
#[test]
fn test_workspace_service_query_requires_revision_for_mutation() {
    let test = TestLanguageService::new("workspace_service_mutation_revision");
    let source = "export const value = 1;\n";
    let path = test.write_text("main.ds", source);

    let _ = test.update_virtual_text(&path, source);

    // reject mutating queries without an expected revision
    let missing_revision = query::QueryRequestEnvelope {
        expected_revision: None,
        request: query::QueryRequest::RenameFiles(query::RenameFilesRequest {
            renames: Vec::new(),
        }),
    };
    let missing_error = test
        .service
        .execute_query_envelope_for_path(&path, missing_revision)
        .expect_err("expected missing revision error");
    assert!(matches!(
        missing_error,
        LanguageServiceError::MissingExpectedRevision
    ));

    // reject stale revision preconditions
    let current_revision = test
        .service
        .revision_for_path(&path)
        .expect("expected current revision");
    let stale_revision = query::QueryRequestEnvelope {
        expected_revision: Some(current_revision.saturating_sub(1)),
        request: query::QueryRequest::RenameFiles(query::RenameFilesRequest {
            renames: Vec::new(),
        }),
    };
    let stale_error = test
        .service
        .execute_query_envelope_for_path(&path, stale_revision)
        .expect_err("expected stale revision error");
    assert!(matches!(
        stale_error,
        LanguageServiceError::StaleRevision { .. }
    ));

    // accept matching revision preconditions
    let matching_revision = query::QueryRequestEnvelope {
        expected_revision: Some(current_revision),
        request: query::QueryRequest::RenameFiles(query::RenameFilesRequest {
            renames: Vec::new(),
        }),
    };
    let _ = test
        .service
        .execute_query_envelope_for_path(&path, matching_revision)
        .expect("expected mutating query with matching revision");
}
