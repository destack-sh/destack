#[path = "support/session.rs"]
mod session;

use session::open_workspace;

#[test]
fn test_format_source() -> destack::Result<()> {
    let fixture = open_workspace(&[("destack.json", r#"{"name":"@test/app"}"#)])?;
    let workspace = fixture.workspace;
    let request = destack::FormatRequest::new(destack::Document::text(
        "src/index.ds",
        "export  const value=1;",
    ));

    // format ad hoc source text
    let output = workspace.format(workspace.revision()?, request)?;

    assert_eq!(output.text, "export const value = 1;\n");

    Ok(())
}
