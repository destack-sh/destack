use std::path::Path;

use crate::tests::harness::TestLanguageService;
use destack_query as query;
use destack_source::{ModuleId, ProfileId, TargetId};
use destack_workspace::{Repository, RepositoryError, Revision};

/// Route file queries across multiple workspace roots.
#[test]
fn test_route_queries_across_roots() {
    let test = TestLanguageService::new_with_roots("service_multi_root", 2);
    let source = r#"export function id(value: number) {
    return value;
}
"#;
    let path_a = test.write_text_for_root(0, "main.ds", source);
    let path_b = test.write_text_for_root(1, "main.ds", source);

    let _ = test.apply_text(&path_a, source);
    let _ = test.apply_text(&path_b, source);
    let profile_a = query_profile(&test, &path_a);
    let profile_b = query_profile(&test, &path_b);

    let response_a = test
        .service
        .read_file_query(&path_a, profile_a, |module, _, _| {
            Ok(Some(query::QueryRequest::DocumentSymbols(
                query::DocumentSymbolsRequest { module },
            )))
        })
        .expect("expected root a query response")
        .expect("expected root a query response")
        .2;
    let response_b = test
        .service
        .read_file_query(&path_b, profile_b, |module, _, _| {
            Ok(Some(query::QueryRequest::DocumentSymbols(
                query::DocumentSymbolsRequest { module },
            )))
        })
        .expect("expected root b query response")
        .expect("expected root b query response")
        .2;

    assert_ne!(
        response_a.revision,
        Revision::NULL,
        "expected a valid root a revision"
    );
    assert_ne!(
        response_b.revision,
        Revision::NULL,
        "expected a valid root b revision"
    );
}

/// Remove closed-root symbols from service lookup state.
#[test]
fn test_close_root_removes_workspace_symbol_entries() {
    let test = TestLanguageService::new_with_roots("service_close_root_symbols", 2);
    let source_a = "export function alphaRootOnly() {}\n";
    let source_b = "export function betaRootOnly() {}\n";
    let path_a = test.write_text_for_root(0, "main.ds", source_a);
    let path_b = test.write_text_for_root(1, "main.ds", source_b);

    let _ = test.apply_text(&path_a, source_a);
    let _ = test.apply_text(&path_b, source_b);
    let profile_b = query_profile(&test, &path_b);

    let response_before = test
        .service
        .read_file_query(&path_b, profile_b, |module, _, _| {
            Ok(Some(query::QueryRequest::WorkspaceSymbols(
                query::WorkspaceSymbolsRequest {
                    query: "alphaRootOnly".to_string(),
                    profile_ids: vec![module.profile_id],
                    max_results: 20,
                },
            )))
        })
        .expect("expected workspace symbol response before close")
        .expect("expected workspace symbol response before close")
        .2;
    let query::QueryResponse::WorkspaceSymbols(before_symbols) = response_before.response else {
        panic!("expected workspace symbol response before close")
    };

    assert!(
        before_symbols
            .symbols
            .iter()
            .any(|symbol| symbol.name == "alphaRootOnly"),
        "expected root 0 symbols before closing the root"
    );

    test.service
        .close_root(&test.roots[0])
        .expect("expected root close");

    let response_after = test
        .service
        .read_file_query(&path_b, profile_b, |module, _, _| {
            Ok(Some(query::QueryRequest::WorkspaceSymbols(
                query::WorkspaceSymbolsRequest {
                    query: "alphaRootOnly".to_string(),
                    profile_ids: vec![module.profile_id],
                    max_results: 20,
                },
            )))
        })
        .expect("expected workspace symbol response after close")
        .expect("expected workspace symbol response after close")
        .2;
    let query::QueryResponse::WorkspaceSymbols(after_symbols) = response_after.response else {
        panic!("expected workspace symbol response after close")
    };

    assert!(
        after_symbols
            .symbols
            .iter()
            .all(|symbol| symbol.name != "alphaRootOnly"),
        "expected closed-root symbols to be removed from the shared index state"
    );
}

/// Remove closed-root auto imports from completion indexes.
#[test]
fn test_close_root_removes_auto_import_entries() {
    let test = TestLanguageService::new_with_roots("service_close_root_imports", 2);
    let source_a = "export function alphaRootOnly() {}\n";
    let source_b = "function main() {\n    alp\n}\n";
    let path_a = test.write_text_for_root(0, "lib.ds", source_a);
    let path_b = test.write_text_for_root(1, "main.ds", source_b);
    let offset_b = source_b
        .find("alp")
        .unwrap_or_else(|| panic!("expected completion prefix")) as u32
        + 3;

    let _ = test.apply_text(&path_a, source_a);
    let _ = test.apply_text(&path_b, source_b);
    let profile_b = query_profile(&test, &path_b);

    let response_before = test
        .service
        .read_file_query(&path_b, profile_b, |module, file_id, _| {
            Ok(Some(query::QueryRequest::Completion(
                query::CompletionRequest {
                    position: query::QueryPosition {
                        module,
                        file_id,
                        offset: offset_b,
                    },
                    trigger: query::CompletionTrigger::Invoked,
                    include_imports: true,
                },
            )))
        })
        .expect("expected completion response before close")
        .expect("expected completion response before close")
        .2;
    let query::QueryResponse::Completion(before_completion) = response_before.response else {
        panic!("expected completion response before close")
    };

    assert!(
        before_completion
            .items
            .iter()
            .any(|item| item.label == "alphaRootOnly"),
        "expected root 0 auto import before closing the root"
    );

    test.service
        .close_root(&test.roots[0])
        .expect("expected root close");

    let response_after = test
        .service
        .read_file_query(&path_b, profile_b, |module, file_id, _| {
            Ok(Some(query::QueryRequest::Completion(
                query::CompletionRequest {
                    position: query::QueryPosition {
                        module,
                        file_id,
                        offset: offset_b,
                    },
                    trigger: query::CompletionTrigger::Invoked,
                    include_imports: true,
                },
            )))
        })
        .expect("expected completion response after close")
        .expect("expected completion response after close")
        .2;
    let query::QueryResponse::Completion(after_completion) = response_after.response else {
        panic!("expected completion response after close")
    };

    assert!(
        after_completion
            .items
            .iter()
            .all(|item| item.label != "alphaRootOnly"),
        "expected closed-root auto imports to be removed from the shared index state"
    );
}

/// Return the explicit query profile used by service session tests.
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
