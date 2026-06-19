#[path = "support/session.rs"]
mod session;

use session::open_workspace;

#[test]
fn test_edit_source() -> destack::Result<()> {
    let fixture = open_workspace(&[
        ("destack.json", r#"{"name":"@test/app"}"#),
        ("src/index.ds", "export const value = 1;"),
    ])?;
    let workspace = fixture.workspace;

    // apply one edit
    let commit = workspace.edit(vec![destack::Edit::set_text(
        "src/next.ds",
        "export const next = 2;",
    )])?;

    // read files after the edit
    let paths = workspace
        .files()?
        .into_iter()
        .map(|file| file.path)
        .collect::<Vec<_>>();

    assert_ne!(commit.before.id, commit.after.id);
    assert_eq!(paths, ["destack.json", "src/index.ds", "src/next.ds"]);

    Ok(())
}
