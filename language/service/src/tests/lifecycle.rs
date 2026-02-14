use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

use crate::query;
use crate::tests::harness::TestLanguageService;

/// Route file queries across multiple workspace roots.
#[test]
fn test_workspace_service_routes_queries_across_roots() {
    let test = TestLanguageService::new_with_roots("workspace_service_multi_root", 2);
    let path_a = test.path_for_root(0, "main.ds");
    let path_b = test.path_for_root(1, "main.ds");
    let uri_a = test.uri_for_path(&path_a);
    let uri_b = test.uri_for_path(&path_b);
    let source = r#"export function id(value: number) {
    return value;
}
"#;

    let _ = test
        .fs
        .write_text(&path_a, source)
        .expect("expected root a write");
    let _ = test
        .fs
        .write_text(&path_b, source)
        .expect("expected root b write");

    let _ = test
        .service
        .update_virtual_file(&path_a, source.to_string())
        .expect("expected root a update");
    let _ = test
        .service
        .update_virtual_file(&path_b, source.to_string())
        .expect("expected root b update");

    let response_a = test
        .service
        .query_for_path(
            &path_a,
            query::QueryRequest::DocumentSymbols(query::DocumentSymbolsRequest { uri: uri_a }),
        )
        .expect("expected root a query response");
    let response_b = test
        .service
        .query_for_path(
            &path_b,
            query::QueryRequest::DocumentSymbols(query::DocumentSymbolsRequest { uri: uri_b }),
        )
        .expect("expected root b query response");

    assert_ne!(
        response_a.snapshot_id, response_b.snapshot_id,
        "expected distinct workspace handles for each root"
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
            service.with_program_for_path(&path, |_program, _compiler| {
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
            });
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
