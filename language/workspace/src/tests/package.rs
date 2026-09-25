use futures::executor::block_on;

use crate::command::{CheckInput, CommandOptions, CommandRevision};
use crate::tests::harness::TestWorkspace;

/// Import only packages declared by the workspace.
#[test]
fn test_workspace_imports_declared_packages() {
    let test = TestWorkspace::new("workspace-declared-packages");
    test.write_text(
        "package.json",
        r#"{
  "packageManager": "tspp@2026.9.0",
  "name": "workspace",
  "workspaces": ["packages/*"]
}
"#,
    );
    test.write_text(
        "packages/app/package.json",
        "{ \"packageManager\": \"tspp@2026.9.0\", \"name\": \"app\" }\n",
    );
    test.write_text("packages/app/src/main.tspp", "export const main = 1;\n");
    test.write_text(
        "unrelated/package/package.json",
        "{ \"packageManager\": \"tspp@2026.9.0\", \"name\": \"other\" }\n",
    );
    test.write_text(
        "unrelated/package/src/foreign.tspp",
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
            "package.json".to_string(),
            "packages/app/package.json".to_string(),
            "packages/app/src/main.tspp".to_string(),
        ]
    );
}

/// Select command inputs from declared workspace packages.
#[test]
fn test_check_selects_declared_workspace_packages() {
    let test = TestWorkspace::new("check-declared-workspace-packages");
    test.write_text(
        "package.json",
        r#"{
  "packageManager": "tspp@2026.9.0",
  "name": "workspace",
  "workspaces": ["packages/*"]
}
"#,
    );
    test.write_text(
        "packages/app/package.json",
        "{ \"packageManager\": \"tspp@2026.9.0\", \"name\": \"app\" }\n",
    );
    test.write_text(
        "packages/app/src/main.tspp",
        "export const answer: int32 = 42;\n",
    );
    test.write_text(
        "packages/lib/package.json",
        "{ \"packageManager\": \"tspp@2026.9.0\", \"name\": \"lib\" }\n",
    );
    test.write_text(
        "packages/lib/src/index.tspp",
        "export const name = \"lib\";\n",
    );
    test.write_text(
        "unrelated/package/package.json",
        "{ \"packageManager\": \"tspp@2026.9.0\", \"name\": \"other\" }\n",
    );
    test.write_text(
        "unrelated/package/src/invalid.tspp",
        "export const invalid: string = 42;\n",
    );

    // reopen from the complete physical source tree
    let test = test.restart();
    let mut input = CheckInput::from((CommandRevision::Current, CommandOptions::default()));
    input.lint = false;
    let output = block_on(test.workspace.check(input, None)).expect("check workspace packages");

    assert!(output.success);
    assert_eq!(output.module_count, 2);
    assert!(output.diagnostics.is_empty());
}
