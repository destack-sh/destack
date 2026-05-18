use std::path::Path;

use crate::tests::harness::TestLanguageService;
use destack_query as query;
use destack_source::{ModuleId, ProfileId, Span, TargetId};
use destack_workspace::{Repository, RepositoryError, Revision};

/// Resolve document symbols through workspace queries.
#[test]
fn test_read_query_returns_document_symbols() {
    let test = TestLanguageService::new("service_query_symbols");
    let source = r#"export function add(a: number, b: number) {
    return a + b;
}
"#;
    let path = test.write_text("main.ds", source);

    let _ = test.apply_text(&path, source);
    let profile_id = query_profile(&test, &path);

    let response = test
        .service
        .read_file_query(&path, profile_id, |module, _, _| {
            Ok(Some(query::QueryRequest::DocumentSymbols(
                query::DocumentSymbolsRequest { module },
            )))
        })
        .expect("expected query response");
    let Some((_, _, response)) = response else {
        panic!("expected document symbols query response");
    };

    let query::QueryResponse::DocumentSymbols(document_symbols) = response.response else {
        panic!("expected document symbols response")
    };

    assert!(
        !document_symbols.symbols.is_empty(),
        "expected at least one document symbol"
    );
}

/// Preserve inferred inlay type hints after file edits remove explicit annotations.
#[test]
fn test_read_query_uses_latest_file_text() {
    let test = TestLanguageService::new("service_query_file_text");
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

    let _ = test.apply_text(&path, source_a);
    let _ = test.apply_text(&path, source_b);
    let profile_id = query_profile(&test, &path);

    // query the full file so the binding and call-site hints are both in range
    let response = test
        .service
        .read_file_query(&path, profile_id, |module, file_id, _| {
            Ok(Some(query::QueryRequest::InlayHints(
                query::InlayHintsRequest {
                    range: query::QueryRange {
                        module,
                        span: Span::new(file_id, 0, source_b.len() as u32),
                    },
                },
            )))
        })
        .expect("expected inlay hints query response");
    let Some((_, _, response)) = response else {
        panic!("expected inlay hints query response");
    };
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
        "expected one inferred type hint after the file update"
    );
}

/// Advance the repository revision after file updates.
#[test]
fn test_apply_file_advances_revision() {
    let test = TestLanguageService::new("service_revision_updates");
    let source_a = "export const value = 1;\n";
    let source_b = "export const value = 2;\n";
    let path = test.write_text("main.ds", source_a);

    let _ = test.apply_text(&path, source_a);
    let revision_a = test
        .service
        .revision_at(&path)
        .expect("expected first revision");

    let _ = test.apply_text(&path, source_b);
    let revision_b = test
        .service
        .revision_at(&path)
        .expect("expected second revision");

    assert_ne!(revision_b, revision_a, "expected revision to change");
}

/// Require matching revision preconditions for mutating queries.
#[test]
fn test_write_query_requires_matching_revision() {
    let test = TestLanguageService::new("service_mutation_revision");
    let source = "export const value = 1;\n";
    let path = test.write_text("main.ds", source);

    let _ = test.apply_text(&path, source);
    let current_revision = test
        .service
        .revision_at(&path)
        .expect("expected current revision");

    // reject stale revision preconditions
    let stale_error = test
        .service
        .write_query(
            &path,
            Revision::NULL,
            query::QueryRequest::RenameFiles(query::RenameFilesRequest {
                profile_ids: Vec::new(),
                renames: Vec::new(),
            }),
        )
        .expect_err("expected stale revision error");
    assert!(matches!(
        stale_error,
        LanguageServiceError::StaleRevision { .. }
    ));

    // accept matching revision preconditions
    let _ = test
        .service
        .write_query(
            &path,
            current_revision,
            query::QueryRequest::RenameFiles(query::RenameFilesRequest {
                profile_ids: Vec::new(),
                renames: Vec::new(),
            }),
        )
        .expect("expected mutating query with matching revision");
}

/// Resolve cross-module references for exported symbols.
#[test]
fn test_read_query_finds_cross_module_references() {
    let test = TestLanguageService::new("service_find_references_cross_module");
    let lib_source = "export function ping(): void {}\n";
    let main_source = "import { ping } from \"./lib.ds\";\nping();\n";
    let lib_path = test.write_text("lib.ds", lib_source);
    let main_path = test.write_text("main.ds", main_source);

    // apply file updates for both files
    let _ = test.apply_text(&lib_path, lib_source);
    let _ = test.apply_text(&main_path, main_source);
    let profile_id = query_profile(&test, &lib_path);

    // query references from the exported symbol definition
    let ping_offset = lib_source
        .find("ping")
        .unwrap_or_else(|| panic!("expected 'ping' in lib source")) as u32;
    let response = test
        .service
        .read_file_query(&lib_path, profile_id, |module, file_id, _| {
            Ok(Some(query::QueryRequest::FindReferences(
                query::FindReferencesRequest {
                    position: query::QueryPosition {
                        module,
                        file_id,
                        offset: ping_offset,
                    },
                    include_declaration: true,
                },
            )))
        })
        .expect("expected references query response");
    let Some((_, _, response)) = response else {
        panic!("expected references query response");
    };
    let query::QueryResponse::FindReferences(query::FindReferencesResponse { references }) =
        response.response
    else {
        panic!("expected references query response payload");
    };

    // assert all three references: definition, import specifier, and call site
    assert_eq!(references.len(), 3);
}

/// Rename exported functions across module boundaries.
#[test]
fn test_write_query_renames_cross_module_symbol() {
    let test = TestLanguageService::new("service_rename_cross_module");
    let lib_source = "export function greet(name: string): string {\n    return name;\n}\n";
    let main_source = "import { greet } from \"./lib.ds\";\nconst output = greet(\"Ada\");\n";
    let lib_path = test.write_text("lib.ds", lib_source);
    let main_path = test.write_text("main.ds", main_source);

    // apply file updates for both files
    let _ = test.apply_text(&lib_path, lib_source);
    let _ = test.apply_text(&main_path, main_source);

    // query rename from the exported definition
    let greet_offset = lib_source
        .find("greet")
        .unwrap_or_else(|| panic!("expected 'greet' in lib source")) as u32;
    let revision = test
        .service
        .revision_at(&lib_path)
        .expect("expected revision for rename");
    let profile_id = query_profile(&test, &lib_path);
    let response = test
        .service
        .read_file(&lib_path, |repository, file_id, _file, revision| {
            let Some(module_id) = repository.module_id_for_file(revision, file_id)? else {
                panic!("expected module for lib file");
            };

            Ok(query::QueryPosition {
                module: query::QueryModule {
                    module_id,
                    profile_id,
                },
                file_id,
                offset: greet_offset,
            })
        })
        .and_then(|position| {
            test.service.write_query(
                &lib_path,
                revision,
                query::QueryRequest::Rename(query::RenameRequest {
                    position,
                    new_name: "salute".to_string(),
                }),
            )
        })
        .expect("expected rename query response");
    let query::QueryResponse::Rename(query::RenameResponse { edit }) = response.response else {
        panic!("expected rename query response payload");
    };
    let edit = edit.expect("expected rename query edit");

    // assert scope: two files touched, three symbol edits total
    assert_eq!(edit.file_count(), 2);
    assert_eq!(edit.total_edits(), 3);
}

/// Return the explicit query profile used by service query tests.
fn query_profile(test: &TestLanguageService, path: &Path) -> ProfileId {
    test.service
        .read_file(path, |repository, file_id, _file, revision| {
            let Some(module_id) = repository.module_id_for_file(revision, file_id)? else {
                panic!("expected module for query file");
            };

            query_profile_for_module(repository, revision, module_id)
        })
        .expect("expected query profile")
}

/// Return the explicit built-in target profile for a module.
fn query_profile_for_module(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
) -> Result<ProfileId, RepositoryError> {
    let module = repository
        .module(revision, module_id)?
        .unwrap_or_else(|| panic!("expected module for query profile"));
    let target_id = TargetId::new(module.package_id, "default");
    let profile = repository
        .target_profile(revision, target_id)?
        .unwrap_or_else(|| panic!("expected built-in default target profile"));

    Ok(profile.id())
}
