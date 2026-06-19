#[path = "support/session.rs"]
mod session;

use session::open_workspace;

#[test]
fn test_content_reads_source_dependency() -> destack::Result<()> {
    let fixture = open_workspace(&[
        ("destack.json", r#"{"name":"@test/app"}"#),
        ("src/index.ds", "export const value = 1;"),
    ])?;
    let workspace = fixture.workspace;
    let revision = workspace.revision()?;
    let module = workspace.module("src/index.ds")?;
    let key = destack::ArtifactKey::DirParsed { module: module.id };

    // require the parsed artifact that records source content dependencies
    let record = workspace.artifact_record(revision, key)?;

    // read the source content dependencies through the public content API
    let mut file_contents = Vec::new();
    for dependency in record.dependencies {
        if let destack::ArtifactDependency::Source {
            dependency: destack::ArtifactSourceDependency::FileContent { content, .. },
        } = dependency
        {
            let content = content.into_source().expect("source content id");

            file_contents.push(workspace.text(content)?);
        }
    }

    assert_eq!(file_contents, ["export const value = 1;"]);

    Ok(())
}
