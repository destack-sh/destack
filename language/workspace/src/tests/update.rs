use crate::tests::harness::TestWorkspace;
use crate::{Edit, Error};

/// Emit an explicit removed update when a tracked file is deleted.
#[test]
fn test_apply_file_emits_removed_update() {
    let test = TestWorkspace::new("workspace_remove_file");
    let source = "export const value = 1;\n";
    let path = test.write_text("main.ds", source);
    let uri = test.uri_for_path(&path);

    // load the file before removing it from the revision
    let _ = test.apply_text(&path, source);
    std::fs::remove_file(&path)
        .unwrap_or_else(|error| panic!("failed to remove {}: {error}", path.display()));

    let removed = test
        .workspace
        .apply_file(Edit::Remove { path: path.clone() })
        .expect("expected removed file update");

    assert!(
        removed.updates.iter().any(|update| {
            update.diagnostic_uri == uri && update.is_removed && update.file.is_none()
        }),
        "expected an explicit removed update"
    );
}

/// Keep config updates scoped to direct file publishes.
#[test]
fn test_apply_config_update_stays_direct() {
    let test = TestWorkspace::new("workspace_config_fanout");
    let config_path = test.write_text("destack.json", "{ \"compiler\": {} }\n");
    let module_a = test.write_text("a.ds", "export const a = ;\n");
    let module_b = test.write_text("b.ds", "export const b = ;\n");

    // load the modules and config into the live program first
    let _ = test.apply_text(&module_a, "export const a = ;\n");
    let _ = test.apply_text(&module_b, "export const b = ;\n");
    let _ = test.apply_text(&config_path, "{ \"compiler\": {} }\n");

    // change the config and expect only the config publish
    let updated = test.apply_text(&config_path, "{ \"compiler\": { \"noThrow\": true } }\n");
    let updated_paths: Vec<_> = updated
        .updates
        .iter()
        .filter_map(|update| update.file.as_ref().and_then(|file| file.path.as_ref()))
        .cloned()
        .collect();

    assert!(
        updated_paths.iter().all(|path| path != &module_a),
        "expected config updates to avoid implicit module fanout"
    );
    assert!(
        updated_paths.iter().all(|path| path != &module_b),
        "expected config updates to avoid implicit module fanout"
    );
    assert!(
        updated_paths.iter().any(|path| path == &config_path),
        "expected direct config publish"
    );
}

/// Preserve open file protocol state through open, change, and close.
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

    assert!(
        test.workspace.has_open_file(&path),
        "expected open file state"
    );

    let changed = "export const value = 2;\n";
    let changed = test
        .workspace
        .change_file(
            uri.clone(),
            2,
            Edit::SetText {
                path: path.clone(),
                text: changed.to_string(),
            },
        )
        .expect("expected changed file");

    assert!(
        changed
            .updates
            .iter()
            .any(|update| update.diagnostic_uri == uri && update.diagnostic_version == Some(2)),
        "expected changed update to use client uri and version"
    );

    let _ = test
        .workspace
        .close_file(&path)
        .expect("expected closed file");

    assert!(
        !test.workspace.has_open_file(&path),
        "expected closed file state"
    );
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
