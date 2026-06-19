#[path = "support/session.rs"]
mod session;

use session::open_workspace;

#[test]
fn test_build_links_js_bundle() -> destack::Result<()> {
    let fixture = open_workspace(&[
        ("destack.json", r#"{"name":"@test/app"}"#),
        ("src/index.ds", "export const value = 1;"),
    ])?;
    let workspace = fixture.workspace;
    let root = workspace.root();
    let revision = workspace.revision()?;
    let module = workspace.module("src/index.ds")?;
    let target = workspace.target(revision, module.id.package_id, "js")?;

    // build the package target through the public bridge
    let output = workspace.build(revision, destack::BuildRequest::target(target))?;

    let destack::BuildOutput::Bundle { bundle, .. } = output else {
        panic!("expected bundle output");
    };
    let file = &bundle.files[0];

    assert_eq!(bundle.emit, destack::EmitFormat::Js);
    assert_eq!(bundle.mode, destack::BundleMode::SingleFile);
    assert_eq!(bundle.files.len(), 1);
    assert_eq!(file.section, destack::BundleSection::Module);
    assert_eq!(relative_uri(&root, &file.uri), "dist/js.js");
    assert_eq!(file.file_type, destack::FileType::JavaScript);
    assert_eq!(file.source, None);

    Ok(())
}

fn relative_uri(root: &str, uri: &str) -> String {
    uri.strip_prefix(root)
        .unwrap_or(uri)
        .trim_start_matches('/')
        .to_string()
}
