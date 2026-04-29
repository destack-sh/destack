use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

use crate::tests::harness::TestLanguageService;
use destack_query as query;
use destack_workspace::Revision;

/// Route file queries across multiple workspace roots.
#[test]
fn test_workspace_service_routes_queries_across_roots() {
    let test = TestLanguageService::new_with_roots("workspace_service_multi_root", 2);
    let source = r#"export function id(value: number) {
    return value;
}
"#;
    let path_a = test.write_text_for_root(0, "main.ds", source);
    let path_b = test.write_text_for_root(1, "main.ds", source);
    let uri_a = test.uri_for_path(&path_a);
    let uri_b = test.uri_for_path(&path_b);

    let _ = test.update_virtual_text(&path_a, source);
    let _ = test.update_virtual_text(&path_b, source);

    let response_a = test
        .service
        .execute_read_query_for_path(
            &path_a,
            query::QueryRequest::DocumentSymbols(query::DocumentSymbolsRequest { uri: uri_a }),
        )
        .expect("expected root a query response");
    let response_b = test
        .service
        .execute_read_query_for_path(
            &path_b,
            query::QueryRequest::DocumentSymbols(query::DocumentSymbolsRequest { uri: uri_b }),
        )
        .expect("expected root b query response");

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

/// Serialize compiler callbacks for concurrent root handle initialization.
#[test]
fn test_workspace_service_serializes_concurrent_program_callbacks() {
    let test = TestLanguageService::new("workspace_service_concurrency");
    let path = test.path_for("main.ds");
    let service = Arc::new(test.service);
    let thread_count = 8usize;
    let barrier = Arc::new(Barrier::new(thread_count + 1));
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();

    for _ in 0..thread_count {
        let service = service.clone();
        let path = path.clone();
        let barrier = barrier.clone();
        let active = active.clone();
        let max_active = max_active.clone();
        handles.push(thread::spawn(move || {
            barrier.wait();
            let root = service
                .workspace_root_for_path(&path)
                .expect("expected workspace root");
            service
                .with_workspace_for_root(&root, |_program, _compiler| {
                    let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                    loop {
                        let observed = max_active.load(Ordering::SeqCst);
                        if current <= observed {
                            break;
                        }
                        if max_active
                            .compare_exchange(observed, current, Ordering::SeqCst, Ordering::SeqCst)
                            .is_ok()
                        {
                            break;
                        }
                    }

                    thread::sleep(Duration::from_millis(20));
                    let _ = active.fetch_sub(1, Ordering::SeqCst);
                })
                .expect("expected workspace path routing");
        }));
    }

    barrier.wait();
    for handle in handles {
        handle.join().expect("expected worker join");
    }

    assert_eq!(
        max_active.load(Ordering::SeqCst),
        1,
        "expected a single compile callback at a time"
    );
}

/// Remove closed-root symbols from workspace indexes.
#[test]
fn test_workspace_service_close_root_removes_workspace_symbol_entries() {
    let test = TestLanguageService::new_with_roots("workspace_service_close_root_indexes", 2);
    let source_a = "export function alphaRootOnly() {}\n";
    let source_b = "export function betaRootOnly() {}\n";
    let path_a = test.write_text_for_root(0, "main.ds", source_a);
    let path_b = test.write_text_for_root(1, "main.ds", source_b);

    let _ = test.update_virtual_text(&path_a, source_a);
    let _ = test.update_virtual_text(&path_b, source_b);

    let response_before = test
        .service
        .execute_read_query_for_path(
            &path_b,
            query::QueryRequest::WorkspaceSymbols(query::WorkspaceSymbolsRequest {
                query: "alphaRootOnly".to_string(),
                max_results: 20,
            }),
        )
        .expect("expected workspace symbol response before close");
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
        .close_workspace_root(&test.roots[0])
        .expect("expected root close");

    let response_after = test
        .service
        .execute_read_query_for_path(
            &path_b,
            query::QueryRequest::WorkspaceSymbols(query::WorkspaceSymbolsRequest {
                query: "alphaRootOnly".to_string(),
                max_results: 20,
            }),
        )
        .expect("expected workspace symbol response after close");
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
fn test_workspace_service_close_root_removes_auto_import_entries() {
    let test = TestLanguageService::new_with_roots("workspace_service_close_root_imports", 2);
    let source_a = "export function alphaRootOnly() {}\n";
    let source_b = "function main() {\n    alp\n}\n";
    let path_a = test.write_text_for_root(0, "lib.ds", source_a);
    let path_b = test.write_text_for_root(1, "main.ds", source_b);
    let uri_b = test.uri_for_path(&path_b);
    let offset_b = source_b
        .find("alp")
        .unwrap_or_else(|| panic!("expected completion prefix")) as u32
        + 3;

    let _ = test.update_virtual_text(&path_a, source_a);
    let _ = test.update_virtual_text(&path_b, source_b);

    let response_before = test
        .service
        .execute_read_query_for_path(
            &path_b,
            query::QueryRequest::Completion(query::CompletionRequest {
                uri: uri_b.clone(),
                offset: offset_b,
                trigger: query::CompletionTrigger::Invoked,
                include_imports: true,
            }),
        )
        .expect("expected completion response before close");
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
        .close_workspace_root(&test.roots[0])
        .expect("expected root close");

    let response_after = test
        .service
        .execute_read_query_for_path(
            &path_b,
            query::QueryRequest::Completion(query::CompletionRequest {
                uri: uri_b,
                offset: offset_b,
                trigger: query::CompletionTrigger::Invoked,
                include_imports: true,
            }),
        )
        .expect("expected completion response after close");
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
