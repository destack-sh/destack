use super::TestSession;

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
