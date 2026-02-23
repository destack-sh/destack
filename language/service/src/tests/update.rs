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
