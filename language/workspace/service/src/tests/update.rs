use crate::tests::harness::TestWorkspaceService;

/// Emit diagnostics after a virtual update with invalid syntax.
#[test]
fn test_workspace_service_virtual_update_emits_diagnostics() {
    let test = TestWorkspaceService::new("workspace_service_update");
    let path = test.path_for("main.ds");
    let valid_source = "export const x: number = 1;\n";
    let invalid_source = "export const x = ;\n";

    let _ = test
        .fs
        .write_text("main.ds", valid_source)
        .expect("expected initial write");

    let initial = test
        .service
        .update_virtual_file(&path, valid_source.to_string())
        .expect("expected initial update");

    assert!(
        initial
            .updates
            .iter()
            .all(|update| update.diagnostics.is_empty()),
        "expected no diagnostics for valid content"
    );

    let updated = test
        .service
        .update_virtual_file(&path, invalid_source.to_string())
        .expect("expected updated diagnostics");

    assert!(
        updated
            .updates
            .iter()
            .any(|update| !update.diagnostics.is_empty()),
        "expected diagnostics for invalid content"
    );
}
