use crate::tests::TestDaemon;

/// Ensure daemon updates emit diagnostics for invalid syntax.
#[test]
fn test_daemon_update_emits_diagnostics() {
    let test = TestDaemon::new();

    // initial diagnostics are empty
    let initial = test.update_file("main.ds", "export const value = 1;");
    assert!(
        initial.diagnostics.is_empty(),
        "expected no diagnostics for valid content"
    );

    // invalid content emits diagnostics
    let updated = test.update_file("main.ds", "export const value = ;");
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
    let invalid = test.update_file("main.ds", "export const value = ;");
    assert!(
        !invalid.diagnostics.is_empty(),
        "expected diagnostics for invalid content"
    );

    // diagnostics clear after fix
    let fixed = test.update_file("main.ds", "export const value = 1;");
    assert!(
        fixed.diagnostics.is_empty(),
        "expected diagnostics to clear after fix"
    );
}
