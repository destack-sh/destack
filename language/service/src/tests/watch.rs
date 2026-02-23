use crate::tests::harness::TestLanguageService;

/// Apply modified watch events and surface diagnostics.
#[test]
fn test_workspace_service_watch_events_update_diagnostics() {
    let test = TestLanguageService::new("workspace_service_watch_diagnostics");
    let source = "export const value = ;\n";
    let path = test.write_text("main.ds", source);

    let result = test.apply_watch_modified(&path);

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
    let source = r#"{
    "extends": "./missing.dsconfig.json"
}
"#;
    let path = test.write_text("dsconfig.json", source);

    let result = test.apply_watch_modified(&path);

    assert!(
        result.updates.iter().any(|update| update
            .file
            .path
            .as_ref()
            .is_some_and(|update_path| update_path == &path)),
        "expected config file update to be tracked"
    );
}

/// Apply watch events for binary assets without utf8 decode errors.
#[test]
fn test_workspace_service_watch_events_update_binary_assets() {
    let test = TestLanguageService::new("workspace_service_watch_binary");
    let source_path = test.write_text(
        "main.ds",
        r#"
import icon from "./icon.png";

icon;
"#,
    );
    let asset_path = test.path_for("icon.png");
    let _ = test
        .fs
        .write_bytes(&asset_path, &[0x89, b'P', b'N', b'G', 0x00])
        .expect("expected asset write");

    let _ = test.update_virtual_text(
        &source_path,
        r#"
import icon from "./icon.png";

icon;
"#,
    );
    let _ = test
        .fs
        .write_bytes(&asset_path, &[0x89, b'P', b'N', b'G', 0x01])
        .expect("expected updated asset write");

    let result = test.apply_watch_modified(&asset_path);

    assert!(
        result
            .messages
            .iter()
            .all(|message| message.code != "watch_read_failed"),
        "expected binary watch updates to avoid utf8 read failures"
    );
    assert!(
        result.updates.iter().any(|update| update
            .file
            .path
            .as_ref()
            .is_some_and(|update_path| update_path == &asset_path)),
        "expected binary file update to be tracked"
    );
}
