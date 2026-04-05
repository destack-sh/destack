use crate::tests::harness::TestLanguageService;

/// Emit diagnostics after a virtual update with invalid syntax.
#[test]
fn test_workspace_service_virtual_update_emits_diagnostics() {
    let test = TestLanguageService::new("workspace_service_update");
    let valid_source = "export const x: number = 1;\n";
    let invalid_source = "export const x = ;\n";
    let path = test.write_text("main.ds", valid_source);

    let initial = test.update_virtual_text(&path, valid_source);

    assert!(
        initial
            .updates
            .iter()
            .all(|update| update.diagnostics.is_empty()),
        "expected no diagnostics for valid content"
    );

    let updated = test.update_virtual_text(&path, invalid_source);

    assert!(
        updated
            .updates
            .iter()
            .any(|update| !update.diagnostics.is_empty()),
        "expected diagnostics for invalid content"
    );
}

/// Fan out config impact updates to affected workspace modules.
#[test]
fn test_workspace_service_config_update_fanout_emits_module_updates() {
    let test = TestLanguageService::new("workspace_service_config_fanout");
    let _package_path = test.write_text(
        "package.json",
        "{ \"name\": \"fanout\", \"version\": \"0.1.0\" }\n",
    );
    let config_path = test.write_text("destack.json", "{ \"compilerOptions\": {} }\n");
    let module_a = test.write_text("a.ds", "export const a = ;\n");
    let module_b = test.write_text("b.ds", "export const b = ;\n");

    // admit the modules and config into the live program first
    let _ = test.update_virtual_text(&module_a, "export const a = ;\n");
    let _ = test.update_virtual_text(&module_b, "export const b = ;\n");
    let _ = test.update_virtual_text(&config_path, "{ \"compilerOptions\": {} }\n");

    // change the config and expect fanout updates for both modules
    let updated = test.update_virtual_text(
        &config_path,
        "{ \"compilerOptions\": { \"noImplicitAny\": true } }\n",
    );
    let updated_paths: Vec<_> = updated
        .updates
        .iter()
        .filter_map(|update| update.file.path.as_ref())
        .cloned()
        .collect();

    assert!(
        updated_paths.iter().any(|path| path == &module_a),
        "expected config update fanout for a.ds"
    );
    assert!(
        updated_paths.iter().any(|path| path == &module_b),
        "expected config update fanout for b.ds"
    );
}
