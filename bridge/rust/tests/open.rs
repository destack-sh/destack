#[path = "support/session.rs"]
mod session;

use session::open_workspace;

#[test]
fn test_open_memory_source() -> destack::Result<()> {
    let fixture = open_workspace(&[
        ("destack.json", r#"{"name":"@test/app"}"#),
        ("src/index.ds", "export const value = 1;"),
    ])?;
    let workspace = fixture.workspace;

    // list seeded source files
    let paths = workspace
        .files()?
        .into_iter()
        .map(|file| file.path)
        .collect::<Vec<_>>();

    assert_eq!(paths, ["destack.json", "src/index.ds"]);

    Ok(())
}
