#[path = "support/session.rs"]
mod session;

use session::open_workspace;

#[test]
fn test_parse_returns_clean_module() -> destack::Result<()> {
    let fixture = open_workspace(&[
        ("destack.json", r#"{"name":"@test/app"}"#),
        ("src/index.ds", "export const value = 1;"),
    ])?;
    let workspace = fixture.workspace;
    let revision = workspace.revision()?;
    let module = workspace.module("src/index.ds")?;

    // parse one loaded source module
    let output = workspace.parse(revision, module)?;

    assert_eq!(output.diagnostics, []);

    Ok(())
}

#[test]
fn test_parse_returns_syntax_error() -> destack::Result<()> {
    let fixture = open_workspace(&[
        ("destack.json", r#"{"name":"@test/app"}"#),
        ("src/index.ds", "export const broken ="),
    ])?;
    let workspace = fixture.workspace;
    let revision = workspace.revision()?;
    let module = workspace.module("src/index.ds")?;

    // parse and inspect the returned syntax diagnostic
    let output = workspace.parse(revision, module)?;
    let diagnostic = &output.diagnostics[0];

    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(diagnostic.code, "EP001");
    assert_eq!(diagnostic.severity, destack::DiagnosticSeverity::Error);
    assert_eq!(
        diagnostic.message,
        "parse error: unexpected End in Declarator"
    );

    Ok(())
}
