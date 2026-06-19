#[path = "support/session.rs"]
mod session;

use session::open_workspace;

#[test]
fn test_lint_accepts_module() -> destack::Result<()> {
    let fixture = open_workspace(&[
        ("destack.json", r#"{"name":"@test/app"}"#),
        ("src/index.ds", "export const value = 1;"),
    ])?;
    let workspace = fixture.workspace;
    let revision = workspace.revision()?;
    let module = workspace.module("src/index.ds")?;
    let profile = workspace.profile(revision, module, "js")?;

    // lint the loaded module profile
    let output = workspace.lint(
        revision,
        destack::LintRequest::new(destack::Scope::module(module, profile)),
    )?;

    assert_eq!(output.diagnostics, []);

    Ok(())
}
