use std::path::Path;
use std::sync::Arc;

use destack_artifact::{DiskCacheStore, MemoryCacheStore};
use destack_query as query;
use destack_source::{FileContent, FileSystem, PhysicalFileSystem, TemporaryPhysicalFileSystem};
use destack_workspace::{HostEnvironment, Repository};

use crate::Daemon;
use crate::tests::{TestDaemon, current_root_revision};

const NAVIGATION_SOURCE: &str = r#"function greet(name: string): string {
    return "Hello, " + name;
}

const msg = greet("World");
"#;

const EXPORTED_NAVIGATION_SOURCE: &str = r#"export function greet(name: string): string {
    return "Hello, " + name;
}

const msg = greet("World");
"#;

const IMPORT_A_VALUE_SOURCE: &str = r#"import { value } from "./a.ds";

value;
"#;

/// Assert query readiness for a virtual source file.
fn assert_virtual_navigation_ready(repository: &Repository, path: &Path, source: &str) {
    // resolve the file id from the tracked path
    let file_id = repository.file_id(path);

    // resolve the root revision
    let revision = current_root_revision(repository);

    assert!(
        repository
            .file(revision, file_id)
            .unwrap_or_else(|error| panic!("failed to read virtual source file: {error}"))
            .is_some(),
        "expected file id for virtual source"
    );

    // verify goto definition on the call expression resolves
    let offset = source
        .find(r#"greet("World")"#)
        .expect("expected call marker") as u32
        + 1;
    let result = query::goto_definition(repository, revision, file_id, offset);
    assert!(result.is_some(), "expected goto definition result");
}

/// Ensure daemon updates emit diagnostics for invalid syntax.
#[test]
fn test_daemon_update_emits_diagnostics() {
    let test = TestDaemon::new();

    // initial diagnostics are empty
    let initial = test.update_file_for_path("main.ds", "export const value = 1;");
    assert!(
        initial.diagnostics.is_empty(),
        "expected no diagnostics for valid content"
    );

    // invalid content emits diagnostics
    let updated = test.update_file_for_path("main.ds", "export const value = ;");
    assert!(
        !updated.diagnostics.is_empty(),
        "expected diagnostics for invalid content"
    );
}

/// Ensure virtual updates emit diagnostics for invalid content.
#[test]
fn test_daemon_virtual_update_emits_diagnostics() {
    let test = TestDaemon::new();

    // resolve the path without writing to disk
    let path = test.root.join("main.ds");

    // initial diagnostics are empty
    let initial_update = test.update_file_for_path(&path, "export const value = 1;");
    assert!(
        initial_update.diagnostics.is_empty(),
        "expected no diagnostics for valid content"
    );

    // invalid content emits diagnostics
    let updated_update = test.update_file_for_path(&path, "export const value = ;");
    assert!(
        !updated_update.diagnostics.is_empty(),
        "expected diagnostics for invalid content"
    );
}

/// Ensure virtual updates emit diagnostics on physical file systems.
#[test]
fn test_daemon_virtual_update_emits_diagnostics_physical_fs() {
    let fs = TemporaryPhysicalFileSystem::new_with_prefix("daemon_virtual_physical");
    let root = fs.root().to_path_buf();
    let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
    let repository = Arc::new(
        Repository::new(
            root.clone(),
            Arc::new(DiskCacheStore::new()),
            file_system,
            HostEnvironment::capture_process(),
        )
        .with_cache(Arc::new(MemoryCacheStore::new())),
    );

    let daemon = Daemon::new(repository.clone(), 1, None);

    // initial diagnostics are empty
    let path = root.join("main.ds");
    let initial = daemon
        .update_file(&path, "export const value = 1;".to_string())
        .expect("virtual update failed");
    let file_id = repository.file_id(&path);
    let initial_update = initial
        .updates
        .into_iter()
        .find(|update| update.file_id == file_id)
        .expect("expected update for virtual file");
    assert!(
        initial_update.diagnostics.is_empty(),
        "expected no diagnostics for valid content"
    );

    // invalid content emits diagnostics
    let updated = daemon
        .update_file(&path, "export const value = ;".to_string())
        .expect("virtual update failed");
    let updated_update = updated
        .updates
        .into_iter()
        .find(|update| update.file_id == file_id)
        .expect("expected update for virtual file");
    assert!(
        !updated_update.diagnostics.is_empty(),
        "expected diagnostics for invalid content"
    );
}

/// Ensure virtual updates build strict navigation query state.
#[test]
fn test_daemon_virtual_update_builds_navigation_semantic_query_state() {
    let test = TestDaemon::new();
    let path = test.root.join("main.ds");
    let source = NAVIGATION_SOURCE;

    let update = test.update_file_for_path(&path, source);
    assert!(
        update.diagnostics.is_empty(),
        "expected no diagnostics for valid content"
    );

    assert_virtual_navigation_ready(test.repository.as_ref(), &path, source);
}

/// Ensure virtual updates navigate for exported function calls.
#[test]
fn test_daemon_virtual_update_navigates_exported_function_call() {
    let test = TestDaemon::new();
    let path = test.root.join("main.ds");
    let source = EXPORTED_NAVIGATION_SOURCE;

    let _ = test.update_file(&path, source);

    assert_virtual_navigation_ready(test.repository.as_ref(), &path, source);
}

/// Ensure navigation still works after an initial root reload.
#[test]
fn test_daemon_virtual_update_navigates_after_reload() {
    let test = TestDaemon::new();
    let path = test.root.join("main.ds");
    let source = EXPORTED_NAVIGATION_SOURCE;

    let _ = test.daemon.reload_roots(std::slice::from_ref(&test.root));
    let _ = test.update_file(&path, source);

    assert_virtual_navigation_ready(test.repository.as_ref(), &path, source);
}

/// Ensures diagnostics clear after fixing invalid source.
#[test]
fn test_daemon_update_clears_diagnostics_after_fix() {
    let test = TestDaemon::new();

    // invalid update reports diagnostics
    let invalid = test.update_file_for_path("main.ds", "export const value = ;");
    assert!(
        !invalid.diagnostics.is_empty(),
        "expected diagnostics for invalid content"
    );

    // diagnostics clear after fix
    let fixed = test.update_file_for_path("main.ds", "export const value = 1;");
    assert!(
        fixed.diagnostics.is_empty(),
        "expected diagnostics to clear after fix"
    );
}

/// Filesystem reloads refresh file contents from disk.
#[test]
fn test_daemon_reload_refreshes_file() {
    let test = TestDaemon::new();

    // create and register the initial file
    let path = test.write_text("main.ds", "export const value = 1;");
    let _ = test.update_file(&path, "export const value = 1;");

    // update the file outside the daemon
    test.fs
        .write(&path, b"export const value = 2;")
        .expect("write updated file");

    // reload to refresh file contents
    let result = test.daemon.reload_roots(std::slice::from_ref(&test.root));

    // check that the reload reports updates without errors
    assert!(result.updated());
    assert!(result.messages.is_empty());

    let file = test.file_for_path(&path);
    let FileContent::Text { content } = file.content.payload() else {
        panic!("expected text content");
    };

    // check that the registry content reflects disk updates
    assert_eq!(content, "export const value = 2;");
}

/// Filesystem reload only refreshes source state.
#[test]
fn test_daemon_reload_does_not_realize_diagnostics() {
    let test = TestDaemon::new();

    // create and register the initial file
    let path = test.write_text("main.ds", "export const value = 1;");
    let _ = test.update_file(&path, "export const value = 1;");

    // update the file outside the daemon
    test.fs
        .write(&path, b"export const value = ;")
        .expect("write invalid file");

    // reload only refreshes the tracked file state
    let result = test.daemon.reload_roots(std::slice::from_ref(&test.root));
    assert!(result.updated());

    // confirm the reload update stays source only
    let file_id = test.file_id_for_path(&path);
    let update = test.update_for_file_id(&result.updates, file_id);
    assert!(
        update.diagnostics.is_empty(),
        "expected reload to avoid implicit diagnostic realization"
    );
}

/// Updates only realize directly changed modules.
#[test]
fn test_daemon_update_does_not_fan_out_to_dependents() {
    let test = TestDaemon::new();

    let path_a = test.write_text("a.ds", "export const value = 1;");
    let path_b = test.write_text("b.ds", IMPORT_A_VALUE_SOURCE);

    let _ = test.update_file(&path_a, "export const value = 1;");
    let _ = test.update_file(&path_b, IMPORT_A_VALUE_SOURCE);

    let updates = test.update_file(&path_a, "export const value = 2;");
    let module_b = test.module_id_for_path(&path_b);

    // assertion block: dependent modules are not included implicitly
    assert!(
        updates
            .iter()
            .all(|update| update.module_id != Some(module_b)),
        "expected dependent modules to require explicit realization"
    );
}
