use std::path::PathBuf;

use destack_repository::{DestackLayoutOverride, Environment, Ref, Settings};
use destack_source::FileId;

use super::TestSession;
use crate::{Edit, TextEdit, TextRange, open_repository_from_memory};

#[test]
fn test_open_imports_root_package_sources() {
    let test = TestSession::open(&[
        (
            "destack.json",
            r#"{
  "name": "@test/app"
}
"#,
        ),
        (
            "src/index.ds",
            r#"
export const value = 1;
"#,
        ),
        (
            "src/nested/user.ds",
            r#"
export const user = 2;
"#,
        ),
        (
            "README.md",
            r#"
Not hot dog.
"#,
        ),
    ])
    .unwrap();

    test.assert_files(&["destack.json", "src/index.ds", "src/nested/user.ds"]);
}

#[test]
fn test_open_applies_default_source_excludes() {
    let test = TestSession::open(&[
        (
            "destack.json",
            r#"{
  "name": "@test/app",
  "include": ["**"]
}
"#,
        ),
        (
            "src/index.ds",
            r#"
export const value = 1;
"#,
        ),
        (
            "node_modules/pkg/src/hidden.ds",
            r#"
export const hidden = 2;
"#,
        ),
        (
            "target/generated.ds",
            r#"
export const generated = 3;
"#,
        ),
        (
            "vendor/pkg/src/hidden.ds",
            r#"
export const vendored = 4;
"#,
        ),
    ])
    .unwrap();

    test.assert_files(&["destack.json", "src/index.ds"]);
}

#[test]
fn test_open_memory_applies_ordered_edits() {
    let source_text = "export const value = 1;\n";
    let start = source_text.find('1').unwrap() as u32;
    let end = start + 1;
    let repository = open_repository_from_memory(
        PathBuf::from("/workspace"),
        vec![
            Edit::SetText {
                path: PathBuf::from("destack.json"),
                text: r#"{
  "name": "@test/app"
}
"#
                .to_string(),
            },
            Edit::SetText {
                path: PathBuf::from("src/index.ds"),
                text: source_text.to_string(),
            },
            Edit::EditText {
                path: PathBuf::from("src/index.ds"),
                edits: vec![TextEdit {
                    range: TextRange { start, end },
                    text: "2".to_string(),
                }],
            },
        ],
        Environment::default(),
        Settings::default(),
        DestackLayoutOverride::default(),
    )
    .unwrap();
    let revision = repository
        .current(&Ref::for_root(repository.path()))
        .unwrap();
    let file_id = FileId::from_logical_str("src/index.ds");
    let file = repository.file(revision, file_id).unwrap().unwrap();

    assert_eq!(file.text(), "export const value = 2;\n");
}

#[test]
fn test_open_discovers_workspace_root_from_nested_path() {
    let test = TestSession::open_at(
        "/workspace/packages/app/src/index.ds",
        &[
            (
                "destack.json",
                r#"{
  "name": "@test/workspace"
}
"#,
            ),
            (
                "packages/app/destack.json",
                r#"{
  "name": "@test/app"
}
"#,
            ),
            (
                "packages/app/src/index.ds",
                r#"
export const value = 1;
"#,
            ),
        ],
    )
    .unwrap();

    test.assert_root("packages/app");
    test.assert_files(&["destack.json", "src/index.ds"]);
}

#[test]
fn test_reload_updates_changed_source_file() {
    let test = TestSession::open(&[
        (
            "destack.json",
            r#"{
  "name": "@test/app"
}
"#,
        ),
        (
            "src/index.ds",
            r#"
export const value = 1;
"#,
        ),
    ])
    .unwrap();
    let before = test.revision();

    test.write(
        "src/index.ds",
        r#"export const value = 2;
"#,
    );
    let updates = test.reload();

    test.assert_revision_changed(before);
    test.assert_update_paths(&updates, &["src/index.ds"]);
    test.assert_files(&["destack.json", "src/index.ds"]);
}

#[test]
fn test_reload_removes_deleted_source_file() {
    let test = TestSession::open(&[
        (
            "destack.json",
            r#"{
  "name": "@test/app"
}
"#,
        ),
        (
            "src/index.ds",
            r#"
export const value = 1;
"#,
        ),
        (
            "src/user.ds",
            r#"
export const user = 2;
"#,
        ),
    ])
    .unwrap();

    test.remove("src/user.ds");
    let updates = test.reload();

    test.assert_update_paths(&updates, &["src/user.ds"]);
    test.assert_removed_updates(&updates);
    test.assert_files(&["destack.json", "src/index.ds"]);
}

#[test]
fn test_reload_ignores_untracked_file() {
    let test = TestSession::open(&[
        (
            "destack.json",
            r#"{
  "name": "@test/app"
}
"#,
        ),
        (
            "src/index.ds",
            r#"
export const value = 1;
"#,
        ),
    ])
    .unwrap();

    test.write(
        "README.md",
        r#"not source
"#,
    );
    let updates = test.reload();

    test.assert_no_updates(&updates);
    test.assert_files(&["destack.json", "src/index.ds"]);
}

#[test]
fn test_reload_removes_files_after_source_pattern_changes() {
    let test = TestSession::open(&[
        (
            "destack.json",
            r#"{
  "name": "@test/app",
  "include": ["src/**"]
}
"#,
        ),
        (
            "src/index.ds",
            r#"
export const value = 1;
"#,
        ),
        (
            "lib/index.ds",
            r#"
export const value = 2;
"#,
        ),
    ])
    .unwrap();

    test.write(
        "destack.json",
        r#"{
  "name": "@test/app",
  "include": ["lib/**"]
}
"#,
    );
    let updates = test.reload();

    test.assert_update_paths(&updates, &["destack.json", "lib/index.ds", "src/index.ds"]);
    test.assert_files(&["destack.json", "lib/index.ds"]);
}

#[test]
fn test_open_imports_workspace_packages() {
    let test = TestSession::open(&[
        (
            "destack.json",
            r#"{
  "workspace": {
    "packages": ["packages/*"]
  }
}
"#,
        ),
        (
            "packages/a/destack.json",
            r#"{
  "name": "@test/a"
}
"#,
        ),
        (
            "packages/a/src/index.ds",
            r#"
export const a = 1;
"#,
        ),
        (
            "packages/b/destack.json",
            r#"{
  "name": "@test/b"
}
"#,
        ),
        (
            "packages/b/src/index.ds",
            r#"
export const b = 2;
"#,
        ),
    ])
    .unwrap();

    test.assert_files(&[
        "destack.json",
        "packages/a/destack.json",
        "packages/a/src/index.ds",
        "packages/b/destack.json",
        "packages/b/src/index.ds",
    ]);
}

#[test]
fn test_open_imports_path_dependencies() {
    let test = TestSession::open(&[
        (
            "destack.json",
            r#"{
  "workspace": {
    "packages": ["app"]
  }
}
"#,
        ),
        (
            "app/destack.json",
            r#"{
  "name": "@test/app",
  "dependencies": {
    "@test/shared": {
      "source": "path",
      "path": "../shared"
    }
  }
}
"#,
        ),
        (
            "app/src/index.ds",
            r#"
export const app = 1;
"#,
        ),
        (
            "shared/destack.json",
            r#"{
  "name": "@test/shared"
}
"#,
        ),
        (
            "shared/src/index.ds",
            r#"
export const shared = 2;
"#,
        ),
    ])
    .unwrap();

    test.assert_files(&[
        "destack.json",
        "app/destack.json",
        "app/src/index.ds",
        "shared/destack.json",
        "shared/src/index.ds",
    ]);
}

#[test]
fn test_load_module_from_fs_adds_requested_module() {
    let test = TestSession::open(&[(
        "destack.json",
        r#"{
  "name": "@test/app"
}
"#,
    )])
    .unwrap();

    test.write(
        "tools/task.ds",
        r#"export const task = 1;
"#,
    );
    test.load_module("tools/task.ds");

    test.assert_files(&["destack.json", "tools/task.ds"]);
}

#[test]
fn test_edit_commits_multiple_edits_atomically() {
    let test = TestSession::open(&[(
        "destack.json",
        r#"{
  "name": "@test/app"
}
"#,
    )])
    .unwrap();

    let commit = test.edit(vec![
        Edit::SetText {
            path: "src/index.ds".into(),
            text: "export const value = 1;\n".to_string(),
        },
        Edit::SetText {
            path: "src/user.ds".into(),
            text: "export const user = 2;\n".to_string(),
        },
    ]);

    test.assert_commit_paths(&commit, &["src/index.ds", "src/user.ds"]);
    test.assert_files(&["destack.json", "src/index.ds", "src/user.ds"]);
}

#[test]
fn test_edit_materializes_text_edits() {
    let test = TestSession::open(&[
        (
            "destack.json",
            r#"{
  "name": "@test/app"
}
"#,
        ),
        ("src/index.ds", "export const value = 1;\n"),
    ])
    .unwrap();

    let commit = test.edit(vec![Edit::EditText {
        path: "src/index.ds".into(),
        edits: vec![TextEdit {
            range: TextRange { start: 21, end: 22 },
            text: "2".to_string(),
        }],
    }]);

    test.assert_commit_paths(&commit, &["src/index.ds"]);
    assert_eq!(test.text("src/index.ds"), "export const value = 2;\n");
}

#[test]
fn test_edit_reports_source_and_destination_for_move() {
    let test = TestSession::open(&[
        (
            "destack.json",
            r#"{
  "name": "@test/app"
}
"#,
        ),
        ("src/index.ds", "export const value = 1;\n"),
    ])
    .unwrap();

    let commit = test.edit(vec![Edit::Move {
        from: "src/index.ds".into(),
        to: "src/main.ds".into(),
    }]);

    test.assert_commit_paths(&commit, &["src/index.ds", "src/main.ds"]);
    test.assert_files(&["destack.json", "src/main.ds"]);
}
