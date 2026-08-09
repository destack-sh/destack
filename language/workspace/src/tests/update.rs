use destack_source::{Edit, FileSystem};

use crate::Error;
use crate::tests::harness::TestWorkspace;

/// Emit one explicit removal when a tracked file is deleted.
#[test]
fn test_apply_file_emits_removal() {
    let test = TestWorkspace::new("workspace_remove_file");
    let source = "export const value = 1;\n";
    let path = test.write_text("main.ds", source);

    // load the file before removing it from the revision
    let _ = test.apply_text(&path, source);
    std::fs::remove_file(&path)
        .unwrap_or_else(|error| panic!("failed to remove {}: {error}", path.display()));

    let removed = test
        .workspace
        .apply_file(Edit::Remove { path: path.clone() })
        .expect("expected removed file update");

    assert_eq!(
        removed.changes,
        vec![TestWorkspace::change("main.ds", Some(source), None)]
    );
}

/// Keep config updates scoped to direct file publishes.
#[test]
fn test_apply_config_update_stays_direct() {
    let test = TestWorkspace::new("workspace_config_fanout");
    let config_path = test.write_text("destack.json", "{ \"name\": \"test\", \"compiler\": {} }\n");
    let module_a = test.write_text("a.ds", "export const a = ;\n");
    let module_b = test.write_text("b.ds", "export const b = ;\n");

    // load the modules and config into the live program first
    let _ = test.apply_text(&module_a, "export const a = ;\n");
    let _ = test.apply_text(&module_b, "export const b = ;\n");
    let _ = test.apply_text(&config_path, "{ \"name\": \"test\", \"compiler\": {} }\n");

    // change the config and expect only the config publish
    let updated = test.apply_text(
        &config_path,
        "{ \"name\": \"test\", \"compiler\": { \"noThrow\": true } }\n",
    );
    assert_eq!(
        updated.changes,
        vec![TestWorkspace::change(
            "destack.json",
            Some("{ \"name\": \"test\", \"compiler\": {} }\n"),
            Some("{ \"name\": \"test\", \"compiler\": { \"noThrow\": true } }\n"),
        )]
    );
}

/// Preserve the client uri and version through open, change, and close.
#[test]
fn test_open_file_tracks_client_state() {
    let test = TestWorkspace::new("workspace_open_file");
    let source = "export const value = 1;\n";
    let path = test.write_text("main.ds", source);
    let uri = test.uri_for_path(&path);

    let _ = test
        .workspace
        .open_file(
            uri.clone(),
            1,
            Edit::SetText {
                path: path.clone(),
                text: source.to_string(),
            },
        )
        .expect("expected open file");

    assert!(test.workspace.has_open_file(&path), "expected open file");

    let changed_source = "export const value = 2;\n";
    let changed = test
        .workspace
        .change_file(
            uri.clone(),
            2,
            Edit::SetText {
                path: path.clone(),
                text: changed_source.to_string(),
            },
        )
        .expect("expected changed file");

    assert_eq!(
        changed.changes,
        vec![TestWorkspace::change(
            "main.ds",
            Some(source),
            Some(changed_source),
        )]
    );
    let open = test
        .workspace
        .find_open_file(&path)
        .expect("expected open file");
    assert_eq!(open.uri, uri);
    assert_eq!(open.version, 2);

    let _ = test
        .workspace
        .close_file(&path)
        .expect("expected closed file");

    assert!(!test.workspace.has_open_file(&path), "expected closed file");
}

/// Remove open files when closing a workspace.
#[test]
fn test_close_workspace_clears_open_files() {
    let test = TestWorkspace::new("workspace_close");
    let source = "export const value = 1;\n";
    let path = test.write_text("main.ds", source);
    let uri = test.uri_for_path(&path);

    let _ = test
        .workspace
        .open_file(
            uri,
            1,
            Edit::SetText {
                path: path.clone(),
                text: source.to_string(),
            },
        )
        .expect("expected open file");

    // close the workspace and check that live editor state disappears
    test.workspace.close();

    assert!(
        !test.workspace.has_open_file(&path),
        "expected root close to remove open file"
    );
}

/// Reject mutations after the workspace closes.
#[test]
fn test_close_workspace_rejects_mutation() {
    let test = TestWorkspace::new("workspace_close_mutation");
    test.workspace.close();
    let error = test.workspace.write().expect_err("reject closed mutation");

    assert!(matches!(error, Error::WorkspaceClosed));
}

/// Reject stale open file changes before mutating source state.
#[test]
fn test_change_file_rejects_stale_version() {
    let test = TestWorkspace::new("workspace_stale_file");
    let source = "export const value = 1;\n";
    let path = test.write_text("main.ds", source);
    let uri = test.uri_for_path(&path);

    let _ = test
        .workspace
        .open_file(
            uri.clone(),
            2,
            Edit::SetText {
                path: path.clone(),
                text: source.to_string(),
            },
        )
        .expect("expected open file");

    let error = test
        .workspace
        .change_file(
            uri,
            2,
            Edit::SetText {
                path,
                text: source.to_string(),
            },
        )
        .expect_err("expected stale file version");

    assert!(matches!(error, Error::StaleOpenFile { .. }));
}

/// Reject stale disk writes before changing filesystem or Workspace source state.
#[test]
fn test_write_source_edits_rejects_stale_revision() {
    let test = TestWorkspace::new("workspace_stale_write");
    let disk_source = "export const value = 1;\n";
    let live_source = "export const value = 2;\n";
    let attempted_source = "export const value = 3;\n";
    let path = test.write_text("main.ds", disk_source);
    test.apply_text(&path, disk_source);
    let stale = test.workspace.revision().expect("read stale base");
    test.apply_text(&path, live_source);
    let live = test.workspace.revision().expect("read live revision");

    let error = test
        .workspace
        .write_source_edits_if_current(
            stale,
            vec![Edit::SetText {
                path: path.clone(),
                text: attempted_source.to_string(),
            }],
        )
        .expect_err("reject stale source write");

    assert!(matches!(error, Error::StaleRevision { .. }));
    assert_eq!(
        test.fs.read_to_string(&path).expect("read disk source"),
        disk_source
    );
    assert_eq!(
        test.workspace.revision().expect("read retained revision"),
        live
    );
}

/// Reject source updates that escape their exact workspace root.
#[test]
fn test_apply_source_edits_rejects_escaping_path() {
    let test = TestWorkspace::new("workspace_escape_source_edit");
    let error = test
        .workspace
        .apply_source_edits(vec![Edit::SetText {
            path: "../outside.ds".into(),
            text: "export const escaped = true;\n".to_string(),
        }])
        .expect_err("reject escaping source path");

    assert!(matches!(error, Error::PathNotInRoot { .. }));
}

/// Restore every changed file when one source write fails.
#[test]
fn test_write_source_edits_restores_failed_batch() {
    let test = TestWorkspace::new_with_write_failure("source-write-failure", "second.ds");
    let first_source = "export const value = 1;\n";
    let second_source = "export const value = 2;\n";
    let first = test.write_text("first.ds", first_source);
    let second = test.write_text("second.ds", second_source);
    test.apply_text(&first, first_source);
    test.apply_text(&second, second_source);
    let before = test
        .workspace
        .revision()
        .expect("read revision before failed write");
    let edits = vec![
        Edit::SetText {
            path: first.clone(),
            text: "export const value = 3;\n".to_string(),
        },
        Edit::SetText {
            path: second.clone(),
            text: "export const value = 4;\n".to_string(),
        },
    ];

    let error = test
        .workspace
        .write_source_edits_if_current(before, edits)
        .expect_err("fail source write batch");

    assert_eq!(
        error.to_string(),
        format!(
            "filesystem error at {}: injected write failure",
            second.display()
        )
    );
    assert_eq!(
        test.fs.read_to_string(&first).expect("read first source"),
        first_source
    );
    assert_eq!(
        test.fs.read_to_string(&second).expect("read second source"),
        second_source
    );
    assert_eq!(
        test.workspace.revision().expect("read unchanged revision"),
        before
    );
}
