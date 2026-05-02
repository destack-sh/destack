use crate::FileChange;
use crate::tests::harness::TestLanguageService;

/// Emit diagnostics after a file update with invalid syntax.
#[test]
fn test_apply_file_emits_diagnostics() {
    let test = TestLanguageService::new("service_update");
    let valid_source = "export const x: number = 1;\n";
    let invalid_source = "export const x = ;\n";
    let path = test.write_text("main.ds", valid_source);

    let initial = test.apply_text(&path, valid_source);

    assert!(
        initial
            .updates
            .iter()
            .all(|update| update.diagnostics.is_empty()),
        "expected no diagnostics for valid content"
    );

    let updated = test.apply_text(&path, invalid_source);

    assert!(
        updated
            .updates
            .iter()
            .any(|update| !update.diagnostics.is_empty()),
        "expected diagnostics for invalid content"
    );
}

/// Restore filesystem backed diagnostics after closing an open file.
#[test]
fn test_close_file_restores_filesystem_diagnostics() {
    let test = TestLanguageService::new("service_close_file");
    let valid_source = "export const x: number = 1;\n";
    let invalid_source = "export const x = ;\n";
    let path = test.write_text("main.ds", valid_source);
    let uri = test.uri_for_path(&path);

    let opened = test
        .service
        .open_file(
            &path,
            uri.clone(),
            1,
            FileChange::Text {
                content: invalid_source.to_string(),
            },
        )
        .expect("expected open file");

    assert!(
        opened
            .updates
            .iter()
            .any(|update| !update.diagnostics.is_empty()),
        "expected diagnostics for open invalid content"
    );

    let closed = test.service.close_file(&path).expect("expected close file");

    assert!(
        closed
            .updates
            .iter()
            .any(|update| update.diagnostic_uri == uri && update.diagnostics.is_empty()),
        "expected filesystem backed diagnostics for the closed file"
    );
}

/// Apply watched file changes and surface diagnostics.
#[test]
fn test_apply_watch_event_emits_diagnostics() {
    let test = TestLanguageService::new("service_watch_diagnostics");
    let path = test.write_text("main.ds", "export const value = ;\n");

    let result = test.apply_watch_modified(&path);

    assert!(
        result
            .updates
            .iter()
            .any(|update| !update.diagnostics.is_empty()),
        "expected diagnostics for watched invalid source"
    );
}

/// Emit an explicit removed update when a tracked file is deleted.
#[test]
fn test_apply_file_emits_removed_update() {
    let test = TestLanguageService::new("service_remove_file");
    let source = "export const value = 1;\n";
    let path = test.write_text("main.ds", source);
    let uri = test.uri_for_path(&path);

    // load the file before removing it from the revision
    let _ = test.apply_text(&path, source);
    std::fs::remove_file(&path)
        .unwrap_or_else(|error| panic!("failed to remove {}: {error}", path.display()));

    let removed = test
        .service
        .apply_file(&path, FileChange::Removed)
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
    let test = TestLanguageService::new("service_config_fanout");
    let _package_path = test.write_text(
        "package.json",
        "{ \"name\": \"fanout\", \"version\": \"0.1.0\" }\n",
    );
    let config_path = test.write_text("destack.json", "{ \"compilerOptions\": {} }\n");
    let module_a = test.write_text("a.ds", "export const a = ;\n");
    let module_b = test.write_text("b.ds", "export const b = ;\n");

    // load the modules and config into the live program first
    let _ = test.apply_text(&module_a, "export const a = ;\n");
    let _ = test.apply_text(&module_b, "export const b = ;\n");
    let _ = test.apply_text(&config_path, "{ \"compilerOptions\": {} }\n");

    // change the config and expect only the config publish
    let updated = test.apply_text(
        &config_path,
        "{ \"compilerOptions\": { \"noImplicitAny\": true } }\n",
    );
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
