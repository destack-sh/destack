use destack_source::{FileContent, FileSystem};
use destack_workspace::ModuleGraphKey;

use crate::tests::TestDaemon;

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
    let result = test.daemon.rescan_roots(&[test.root.clone()]);

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

/// Rebuilds module graphs before expanding dependents.
#[test]
fn test_daemon_update_rebuilds_module_graph() {
    let test = TestDaemon::new();

    let path_a = test.write_text("a.ds", "export const value = 1;");
    let path_b = test.write_text(
        "b.ds",
        r#"
import { value } from "./a.ds";

value;
"#,
    );

    let _ = test.update_file(&path_a, "export const value = 1;");
    let _ = test.update_file(
        &path_b,
        r#"
import { value } from "./a.ds";

value;
"#,
    );

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
    let graph_key = ModuleGraphKey::new(profile_id);
    assert!(
        program.index.module_graphs.contains_key(&graph_key),
        "expected module graph to be rebuilt"
    );
}
