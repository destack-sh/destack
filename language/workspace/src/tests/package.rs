use crate::tests::harness::TestWorkspace;

/// Import only packages declared by the workspace.
#[test]
fn test_workspace_imports_declared_packages() {
    let test = TestWorkspace::new("workspace-declared-packages");
    test.write_text(
        "destack.json",
        r#"{
  "name": "workspace",
  "workspace": {
    "packages": ["packages/*"]
  }
}
"#,
    );
    test.write_text("packages/app/destack.json", "{ \"name\": \"app\" }\n");
    test.write_text("packages/app/src/main.ds", "export const main = 1;\n");
    test.write_text(
        "unrelated/package/destack.json",
        "{ \"name\": \"other\" }\n",
    );
    test.write_text(
        "unrelated/package/src/foreign.ds",
        "export const foreign = 1;\n",
    );

    // reopen from the complete physical source tree
    let test = test.restart();
    let revision = test.workspace.revision().expect("read workspace revision");
    let mut paths = test
        .workspace
        .repository
        .files(revision)
        .expect("read workspace files")
        .into_iter()
        .map(|file| file.path)
        .collect::<Vec<_>>();
    paths.sort();

    assert_eq!(
        paths,
        vec![
            "destack.json".to_string(),
            "packages/app/destack.json".to_string(),
            "packages/app/src/main.ds".to_string(),
        ]
    );
}
