use destack_source::{FileWatchEvent, FileWatchEventKind};

use crate::tests::harness::TestLanguageService;

/// Apply modified watch events and surface diagnostics.
#[test]
fn test_workspace_service_watch_events_update_diagnostics() {
    let test = TestLanguageService::new("workspace_service_watch_diagnostics");
    let path = test.path_for("main.ds");
    let source = "export const value = ;\n";
    let _ = test
        .fs
        .write_text(&path, source)
        .expect("expected source write");

    let event = FileWatchEvent {
        path: path.clone(),
        previous_path: None,
        kind: FileWatchEventKind::Modified,
    };
    let result = test
        .service
        .apply_watch_events(vec![event])
        .expect("expected watch apply result");

    assert!(
        result
            .updates
            .iter()
            .any(|update| !update.diagnostics.is_empty()),
        "expected diagnostics for watched invalid source"
    );
}

/// Apply config watch events and include config file updates.
#[test]
fn test_workspace_service_watch_events_include_config_updates() {
    let test = TestLanguageService::new("workspace_service_watch_config");
    let path = test.path_for("dsconfig.json");
    let source = r#"{
    "extends": "./missing.dsconfig.json"
}
"#;
    let _ = test
        .fs
        .write_text(&path, source)
        .expect("expected config write");

    let event = FileWatchEvent {
        path: path.clone(),
        previous_path: None,
        kind: FileWatchEventKind::Modified,
    };
    let result = test
        .service
        .apply_watch_events(vec![event])
        .expect("expected watch apply result");

    assert!(
        result.updates.iter().any(|update| update
            .file
            .path
            .as_ref()
            .is_some_and(|update_path| update_path == &path)),
        "expected config file update to be tracked"
    );
}
