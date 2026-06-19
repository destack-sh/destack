#[path = "support/session.rs"]
mod session;

use session::open_workspace;

#[test]
fn test_check_accepts_valid_module() -> destack::Result<()> {
    let fixture = open_workspace(&[
        ("destack.json", r#"{"name":"@test/app"}"#),
        ("src/index.ds", "export const value = 1;"),
    ])?;
    let workspace = fixture.workspace;
    let revision = workspace.revision()?;
    let module = workspace.module("src/index.ds")?;
    let profile = workspace.profile(revision, module, "js")?;

    // check the module under the selected target profile
    let output = workspace.check(revision, module, profile)?;

    assert_eq!(output.diagnostics, []);

    Ok(())
}

#[test]
fn test_check_reports_missing_type_annotation() -> destack::Result<()> {
    let fixture = open_workspace(&[
        ("destack.json", r#"{"name":"@test/app"}"#),
        ("src/index.ds", "const values = [];"),
    ])?;
    let workspace = fixture.workspace;
    let revision = workspace.revision()?;
    let module = workspace.module("src/index.ds")?;
    let profile = workspace.profile(revision, module, "js")?;

    // check the module and read its type diagnostics
    let output = workspace.check(revision, module, profile)?;

    assert_eq!(output.diagnostics.len(), 2);
    assert_eq!(output.diagnostics[0].code, "EC101");
    assert_eq!(
        output.diagnostics[0].severity,
        destack::DiagnosticSeverity::Error
    );
    assert_eq!(output.diagnostics[0].message, "missing type annotation");
    assert_eq!(output.diagnostics[1].code, "EC101");
    assert_eq!(
        output.diagnostics[1].severity,
        destack::DiagnosticSeverity::Error
    );
    assert_eq!(output.diagnostics[1].message, "missing type annotation");

    Ok(())
}
