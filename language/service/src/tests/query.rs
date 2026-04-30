use crate::LanguageServiceError;
use crate::tests::harness::TestLanguageService;
use destack_query as query;
use destack_workspace::Revision;

/// Resolve document symbols through workspace queries.
#[test]
fn test_read_query_returns_document_symbols() {
    let test = TestLanguageService::new("service_query_symbols");
    let source = r#"export function add(a: number, b: number) {
    return a + b;
}
"#;
    let path = test.write_text("main.ds", source);
    let uri = test.uri_for_path(&path);

    let _ = test.update_virtual_text(&path, source);

    let response = test
        .service
        .read_query(
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

/// Preserve inferred inlay type hints after virtual edits remove explicit annotations.
#[test]
fn test_read_query_uses_current_overlay_text() {
    let test = TestLanguageService::new("service_query_overlay");
    let source_a = r#"function greet(name: string, greeting: string): string {
    return greeting + ", " + name;
}
const msg: string = greet("World", "Hello");
"#;
    let source_b = r#"function greet(name: string, greeting: string): string {
    return greeting + ", " + name;
}
const msg = greet("World", "Hello");
"#;
    let path = test.write_text("main.ds", source_a);
    let uri = test.uri_for_path(&path);

    let _ = test.update_virtual_text(&path, source_a);
    let _ = test.update_virtual_text(&path, source_b);

    // query the full file so the binding and call-site hints are both in range
    let response = test
        .service
        .read_query(
            &path,
            query::QueryRequest::InlayHints(query::InlayHintsRequest {
                uri,
                start: 0,
                end: source_b.len() as u32,
            }),
        )
        .expect("expected inlay hints query response");
    let query::QueryResponse::InlayHints(query::InlayHintsResponse { hints }) = response.response
    else {
        panic!("expected inlay hints query response payload");
    };

    // keep the inferred binding type after the annotation disappears
    let type_hint = hints
        .iter()
        .any(|hint| hint.kind == query::InlayHintKind::Type);
    assert!(
        type_hint,
        "expected one inferred type hint after the virtual update"
    );
}

/// Advance semantic revision after virtual file updates.
#[test]
fn test_apply_file_advances_revision() {
    let test = TestLanguageService::new("service_revision_updates");
    let source_a = "export const value = 1;\n";
    let source_b = "export const value = 2;\n";
    let path = test.write_text("main.ds", source_a);

    let _ = test.update_virtual_text(&path, source_a);
    let revision_a = test
        .service
        .revision_at(&path)
        .expect("expected first revision");

    let _ = test.update_virtual_text(&path, source_b);
    let revision_b = test
        .service
        .revision_at(&path)
        .expect("expected second revision");

    assert_ne!(revision_b, revision_a, "expected revision to change");
}

/// Require revision preconditions for mutating queries.
#[test]
fn test_write_query_requires_current_revision() {
    let test = TestLanguageService::new("service_mutation_revision");
    let source = "export const value = 1;\n";
    let path = test.write_text("main.ds", source);

    let _ = test.update_virtual_text(&path, source);
    let current_revision = test
        .service
        .revision_at(&path)
        .expect("expected current revision");

    // reject mutating queries without an expected revision
    let missing_revision = query::QueryRequestEnvelope {
        expected_revision: None,
        request: query::QueryRequest::RenameFiles(query::RenameFilesRequest {
            renames: Vec::new(),
        }),
    };
    let missing_error = test
        .service
        .write_query_at(&path, missing_revision)
        .expect_err("expected missing revision error");
    assert!(matches!(
        missing_error,
        LanguageServiceError::MissingExpectedRevision
    ));

    // reject stale revision preconditions
    let stale_revision = query::QueryRequestEnvelope {
        expected_revision: Some(Revision::NULL),
        request: query::QueryRequest::RenameFiles(query::RenameFilesRequest {
            renames: Vec::new(),
        }),
    };
    let stale_error = test
        .service
        .write_query_at(&path, stale_revision)
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
        .write_query_at(&path, matching_revision)
        .expect("expected mutating query with matching revision");
}

/// Reject write queries on the read query API.
#[test]
fn test_read_query_rejects_write_request() {
    let test = TestLanguageService::new("service_read_mode_mismatch");
    let source = "export const value = 1;\n";
    let path = test.write_text("main.ds", source);

    let _ = test.update_virtual_text(&path, source);

    let envelope = query::QueryRequestEnvelope {
        expected_revision: None,
        request: query::QueryRequest::RenameFiles(query::RenameFilesRequest {
            renames: Vec::new(),
        }),
    };
    let error = test
        .service
        .read_query_at(&path, envelope)
        .expect_err("expected read mode mismatch");
    assert!(matches!(
        error,
        LanguageServiceError::QueryModeMismatch {
            expected: query::QueryExecutionMode::Read,
            actual: query::QueryExecutionMode::Write,
            ..
        }
    ));
}

/// Reject expected revisions on read query APIs.
#[test]
fn test_read_query_rejects_expected_revision() {
    let test = TestLanguageService::new("service_read_expected_revision");
    let source = "export const value = 1;\n";
    let path = test.write_text("main.ds", source);
    let uri = test.uri_for_path(&path);

    let _ = test.update_virtual_text(&path, source);

    let envelope = query::QueryRequestEnvelope {
        expected_revision: Some(Revision::NULL),
        request: query::QueryRequest::DocumentSymbols(query::DocumentSymbolsRequest { uri }),
    };
    let error = test
        .service
        .read_query_at(&path, envelope)
        .expect_err("expected unexpected revision error");
    assert!(matches!(
        error,
        LanguageServiceError::UnexpectedExpectedRevision {
            expected_revision: Revision::NULL
        }
    ));
}

/// Resolve cross-module references for exported symbols.
#[test]
fn test_read_query_finds_cross_module_references() {
    let test = TestLanguageService::new("service_find_references_cross_module");
    let lib_source = "export function ping(): void {}\n";
    let main_source = "import { ping } from \"./lib.ds\";\nping();\n";
    let lib_path = test.write_text("lib.ds", lib_source);
    let main_path = test.write_text("main.ds", main_source);
    let lib_uri = test.uri_for_path(&lib_path);

    // apply virtual updates for both files
    let _ = test.update_virtual_text(&lib_path, lib_source);
    let _ = test.update_virtual_text(&main_path, main_source);

    // query references from the exported symbol definition
    let ping_offset = lib_source
        .find("ping")
        .unwrap_or_else(|| panic!("expected 'ping' in lib source")) as u32;
    let response = test
        .service
        .read_query(
            &lib_path,
            query::QueryRequest::FindReferences(query::FindReferencesRequest {
                uri: lib_uri,
                offset: ping_offset,
                include_declaration: true,
            }),
        )
        .expect("expected references query response");
    let query::QueryResponse::FindReferences(query::FindReferencesResponse { result }) =
        response.response
    else {
        panic!("expected references query response payload");
    };
    let result = result.expect("expected references query result");

    // assert all three references: definition, import specifier, and call site
    assert_eq!(result.references.len(), 3);
}

/// Rename exported functions across module boundaries.
#[test]
fn test_write_query_renames_cross_module_symbol() {
    let test = TestLanguageService::new("service_rename_cross_module");
    let lib_source = "export function greet(name: string): string {\n    return name;\n}\n";
    let main_source = "import { greet } from \"./lib.ds\";\nconst output = greet(\"Ada\");\n";
    let lib_path = test.write_text("lib.ds", lib_source);
    let main_path = test.write_text("main.ds", main_source);
    let lib_uri = test.uri_for_path(&lib_path);

    // apply virtual updates for both files
    let _ = test.update_virtual_text(&lib_path, lib_source);
    let _ = test.update_virtual_text(&main_path, main_source);

    // query rename from the exported definition
    let greet_offset = lib_source
        .find("greet")
        .unwrap_or_else(|| panic!("expected 'greet' in lib source")) as u32;
    let response = test
        .service
        .write_query_at(
            &lib_path,
            query::QueryRequestEnvelope {
                expected_revision: Some(
                    test.service
                        .revision_at(&lib_path)
                        .expect("expected revision for rename"),
                ),
                request: query::QueryRequest::Rename(query::RenameRequest {
                    uri: lib_uri,
                    offset: greet_offset,
                    new_name: "salute".to_string(),
                }),
            },
        )
        .expect("expected rename query response");
    let query::QueryResponse::Rename(query::RenameResponse { result }) = response.response else {
        panic!("expected rename query response payload");
    };
    let result = result.expect("expected rename query result");

    // assert scope: two files touched, three symbol edits total
    assert_eq!(result.edits.file_count(), 2);
    assert_eq!(result.edits.total_edits(), 3);
}
