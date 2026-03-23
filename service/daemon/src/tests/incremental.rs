use std::path::Path;
use std::sync::Arc;

use destack_artifact::MemoryCacheStore;
use destack_compiler::CompilerOptions;
use destack_query as query;
use destack_source::{FileContent, FileSystem, TemporaryPhysicalFileSystem};
use destack_workspace::Session;

use crate::Daemon;
use crate::tests::TestDaemon;

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

/// Assert semantic query readiness for a virtual source file.
fn assert_virtual_navigation_ready(session: &Session, path: &Path, source: &str) {
    // resolve the file id from the tracked path
    let file_id = session
        .files
        .get_id_by_path(path)
        .expect("expected file id for virtual source");

    // require strict semantic query state for navigation
    let module = session
        .modules
        .get_by_file_id(file_id)
        .expect("expected module for virtual source");
    let module = module.as_ref();
    assert!(
        query::query_context(session, &module).is_some(),
        "expected strict semantic query state for virtual source"
    );
    drop(module);

    // verify goto definition on the call expression resolves
    let offset = source
        .find(r#"greet("World")"#)
        .expect("expected call marker") as u32
        + 1;
    let result = query::goto_definition(session, file_id, offset);
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
    let initial_update = test.update_virtual_file_for_path(&path, "export const value = 1;");
    assert!(
        initial_update.diagnostics.is_empty(),
        "expected no diagnostics for valid content"
    );

    // invalid content emits diagnostics
    let updated_update = test.update_virtual_file_for_path(&path, "export const value = ;");
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
    let session =
        Arc::new(Session::new(root.clone()).with_cache_store(Arc::new(MemoryCacheStore::new())));
    session.add_root(root.clone());

    // run compiler work in a single worker to avoid test contention
    let mut compiler_options = CompilerOptions::default();
    compiler_options.workers = 1;
    let daemon = Daemon::with_options(session.clone(), compiler_options);

    // initial diagnostics are empty
    let path = root.join("main.ds");
    let initial = daemon
        .update_virtual_file(&path, "export const value = 1;".to_string())
        .expect("virtual update failed");
    let file_id = session
        .files
        .get_id_by_path(&path)
        .expect("expected file id after virtual update");
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
        .update_virtual_file(&path, "export const value = ;".to_string())
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

/// Ensure virtual updates build strict navigation semantic query state.
#[test]
fn test_daemon_virtual_update_builds_navigation_semantic_query_state() {
    let test = TestDaemon::new();
    let path = test.root.join("main.ds");
    let source = NAVIGATION_SOURCE;

    let update = test.update_virtual_file_for_path(&path, source);
    assert!(
        update.diagnostics.is_empty(),
        "expected no diagnostics for valid content"
    );

    assert_virtual_navigation_ready(test.session.as_ref(), &path, source);
}

/// Ensure virtual updates navigate for exported function calls.
#[test]
fn test_daemon_virtual_update_navigates_exported_function_call() {
    let test = TestDaemon::new();
    let path = test.root.join("main.ds");
    let source = EXPORTED_NAVIGATION_SOURCE;

    let _ = test.update_virtual_file(&path, source);

    assert_virtual_navigation_ready(test.session.as_ref(), &path, source);
}

/// Ensure navigation still works after an initial workspace rescan.
#[test]
fn test_daemon_virtual_update_navigates_after_rescan() {
    let test = TestDaemon::new();
    let path = test.root.join("main.ds");
    let source = EXPORTED_NAVIGATION_SOURCE;

    let _ = test
        .daemon
        .rescan_roots_with_analysis(std::slice::from_ref(&test.root));
    let _ = test.update_virtual_file(&path, source);

    assert_virtual_navigation_ready(test.session.as_ref(), &path, source);
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

/// Rescans refresh file contents from disk.
#[test]
fn test_daemon_rescan_refreshes_file() {
    let test = TestDaemon::new();

    // create and register the initial file
    let path = test.write_text("main.ds", "export const value = 1;");
    let _ = test.update_file(&path, "export const value = 1;");

    // update the file outside the daemon
    test.fs
        .write(&path, b"export const value = 2;")
        .expect("write updated file");

    // rescan to refresh file contents
    let result = test.daemon.rescan_roots(std::slice::from_ref(&test.root));

    // check that the rescan reports updates without errors
    assert!(result.updated());
    assert!(result.messages.is_empty());

    let file_id = test
        .session
        .files
        .get_id_by_path(&path)
        .expect("file id should be tracked");
    let file = test.session.files.get(file_id);
    let FileContent::Text { content } = &file.content else {
        panic!("expected text content");
    };

    // check that the registry content reflects disk updates
    assert_eq!(content, "export const value = 2;");
}

/// Rescan analysis emits diagnostics for invalid content.
#[test]
fn test_daemon_rescan_with_analysis_emits_diagnostics() {
    let test = TestDaemon::new();

    // create and register the initial file
    let path = test.write_text("main.ds", "export const value = 1;");
    let _ = test.update_file(&path, "export const value = 1;");

    // update the file outside the daemon
    test.fs
        .write(&path, b"export const value = ;")
        .expect("write invalid file");

    // rescan and analyze to surface diagnostics
    let result = test
        .daemon
        .rescan_roots_with_analysis(std::slice::from_ref(&test.root));
    assert!(result.updated());

    // confirm diagnostics for the rescan update
    let file_id = test
        .session
        .files
        .get_id_by_path(&path)
        .expect("file id should be tracked");
    let update = test.update_for_file_id(&result.updates, file_id);
    assert!(
        !update.diagnostics.is_empty(),
        "expected diagnostics for invalid rescan content"
    );
}

/// Rebuilds module graphs before expanding dependents.
#[test]
fn test_daemon_update_rebuilds_module_graph() {
    let test = TestDaemon::new();

    let path_a = test.write_text("a.ds", "export const value = 1;");
    let path_b = test.write_text("b.ds", IMPORT_A_VALUE_SOURCE);

    let _ = test.update_file(&path_a, "export const value = 1;");
    let _ = test.update_file(&path_b, IMPORT_A_VALUE_SOURCE);

    let program = test.session.find_program_for_path(&path_a);
    let profile_id = program.default_profile_id_for_module(
        program
            .modules
            .get_id_by_path(&path_a)
            .expect("expected a.ds module id"),
    );
    program.drop_module_graph(profile_id);

    let updates = test.update_file(&path_a, "export const value = 2;");
    let module_b = program
        .modules
        .get_id_by_path(&path_b)
        .expect("expected b.ds module id");

    // assertion block: dependent modules are included after rebuild
    assert!(
        updates
            .iter()
            .any(|update| update.module_id == Some(module_b)),
        "expected b.ds update after graph rebuild"
    );

    let profile_id = program.default_profile_id_for_module(module_b);
    let artifacts = test
        .session
        .get_artifacts_for_program(program.as_ref())
        .expect("expected artifact store for program");
    assert!(
        artifacts.module_graph(profile_id).is_some(),
        "expected module graph to be rebuilt"
    );
}
