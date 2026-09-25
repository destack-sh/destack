use tspp_core::Blob;
use tspp_repository::TraceLevel;
use tspp_source::{Edit, FileSystem};

use crate::tests::harness::TestWorkspace;
use crate::{Error, FileSelection};

/// Remove physical and repository file state in one edit.
#[test]
fn test_edit_removes_file() {
    let test = TestWorkspace::new("workspace-remove-file");
    let source = "export const value = 1;\n";
    let path = test.write_text("main.tspp", source);
    test.apply_text(&path, source);
    let base = test.workspace.revision().expect("read physical revision");

    let commit = test
        .workspace
        .edit(base, vec![Edit::Remove { path: path.clone() }])
        .expect("remove file");

    assert_eq!(
        commit.changes,
        vec![TestWorkspace::change("main.tspp", Some(source), None)]
    );
    assert!(!test.fs.exists(&path).expect("inspect removed file"));
}

/// Keep config updates scoped to their direct file change.
#[test]
fn test_edit_config_stays_direct() {
    let test = TestWorkspace::new("workspace-config-fanout");
    let config = test.write_text(
        "package.json",
        "{ \"packageManager\": \"tspp@2026.9.0\", \"name\": \"test\", \"compiler\": {} }\n",
    );
    let module_a = test.write_text("a.tspp", "export const a = ;\n");
    let module_b = test.write_text("b.tspp", "export const b = ;\n");
    test.apply_text(&module_a, "export const a = ;\n");
    test.apply_text(&module_b, "export const b = ;\n");
    test.apply_text(
        &config,
        "{ \"packageManager\": \"tspp@2026.9.0\", \"name\": \"test\", \"compiler\": {} }\n",
    );

    let source = "{ \"packageManager\": \"tspp@2026.9.0\", \"name\": \"test\", \"compiler\": { \"noThrow\": true } }\n";
    let commit = test.apply_text(&config, source);

    assert_eq!(
        commit.changes,
        vec![TestWorkspace::change(
            "package.json",
            Some(
                "{ \"packageManager\": \"tspp@2026.9.0\", \"name\": \"test\", \"compiler\": {} }\n"
            ),
            Some(source),
        )]
    );
}

/// Keep branch edits private until selected files are saved.
#[test]
fn test_edit_save_and_restore_branch() {
    let test = TestWorkspace::new("workspace-branch-save-restore");
    let first = test.write_text("first.tspp", "export const first = 1;\n");
    let second = test.write_text("second.tspp", "export const second = 1;\n");
    test.apply_text(&first, "export const first = 1;\n");
    test.apply_text(&second, "export const second = 1;\n");
    let physical = test.workspace.revision().expect("read physical revision");
    let branch = "studio".to_string();
    test.workspace
        .create_branch(branch.clone(), physical)
        .expect("create branch");
    let branches = test.workspace.branches().expect("list branches");
    assert_eq!(branches[0].name, branch);
    let trace = test.workspace.start_trace(TraceLevel::Disabled);

    // edit both files without changing physical state
    let commit = test
        .workspace
        .edit_branch(
            &branch,
            physical,
            vec![
                Edit::SetText {
                    path: first.clone(),
                    text: "export const first = 2;\n".to_string(),
                },
                Edit::SetText {
                    path: second.clone(),
                    text: "export const second = 2;\n".to_string(),
                },
            ],
            trace.as_ref(),
        )
        .expect("edit branch");
    assert_eq!(
        test.workspace.revision().expect("read physical revision"),
        physical
    );
    assert_eq!(
        test.fs.read_to_string(&first).expect("read first file"),
        "export const first = 1;\n"
    );

    // save only the first file from the branch
    let written = test
        .workspace
        .save_branch(
            &branch,
            commit.after,
            physical,
            FileSelection::Paths(vec![first.clone()]),
        )
        .expect("save first file");
    assert_eq!(
        test.fs
            .read_to_string(&first)
            .expect("read saved first file"),
        "export const first = 2;\n"
    );
    assert_eq!(
        test.fs
            .read_to_string(&second)
            .expect("read unsaved second file"),
        "export const second = 1;\n"
    );

    // restore the remaining branch change from physical state
    let restored = test
        .workspace
        .restore_branch(
            &branch,
            commit.after,
            written.after,
            FileSelection::Paths(vec![second]),
        )
        .expect("restore second file");
    assert_eq!(
        test.workspace
            .diff(written.after, restored.after)
            .expect("compare revisions"),
        Vec::new()
    );
}

/// Reject stale writes before changing physical or repository state.
#[test]
fn test_edit_rejects_stale_revision() {
    let test = TestWorkspace::new("workspace-stale-write");
    let path = test.write_text("main.tspp", "export const value = 1;\n");
    test.apply_text(&path, "export const value = 1;\n");
    let stale = test.workspace.revision().expect("read stale revision");
    test.apply_text(&path, "export const value = 2;\n");
    let current = test.workspace.revision().expect("read current revision");

    let error = test
        .workspace
        .edit(
            stale,
            vec![Edit::SetText {
                path: path.clone(),
                text: "export const value = 3;\n".to_string(),
            }],
        )
        .expect_err("reject stale write");

    assert!(matches!(error, Error::StaleRevision { .. }));
    assert_eq!(
        test.fs.read_to_string(&path).expect("read retained file"),
        "export const value = 2;\n"
    );
    assert_eq!(
        test.workspace.revision().expect("read retained revision"),
        current
    );
}

/// Reject a write when physical content changed outside the workspace revision.
#[test]
fn test_edit_rejects_changed_physical_file() {
    let test = TestWorkspace::new("workspace-changed-physical-file");
    let before = "export const value = 1;\n";
    let changed = "export const value = 2;\n";
    let path = test.write_text("main.tspp", before);
    test.apply_text(&path, before);
    let physical = test.workspace.revision().expect("read physical revision");
    test.fs
        .write(&path, changed.as_bytes())
        .expect("change physical file");

    let error = test
        .workspace
        .edit(
            physical,
            vec![Edit::SetText {
                path: path.clone(),
                text: "export const value = 3;\n".to_string(),
            }],
        )
        .expect_err("reject changed physical file");

    assert!(matches!(
        error,
        Error::FileChanged {
            path: error_path,
            expected: Some(expected),
            actual: Some(actual),
        } if error_path.as_ref() == path
            && expected == Blob::for_bytes(before.as_bytes())
            && actual == Blob::for_bytes(changed.as_bytes())
    ));
    assert_eq!(
        test.fs.read_to_string(&path).expect("read changed file"),
        changed
    );
    assert_eq!(
        test.workspace.revision().expect("read retained revision"),
        physical
    );
}

/// Reject source edits that escape the workspace root.
#[test]
fn test_edit_rejects_escaping_path() {
    let test = TestWorkspace::new("workspace-escape-source-edit");
    let physical = test.workspace.revision().expect("read physical revision");
    let branch = "escape".to_string();
    test.workspace
        .create_branch(branch.clone(), physical)
        .expect("create branch");
    let trace = test.workspace.start_trace(TraceLevel::Disabled);

    let error = test
        .workspace
        .edit_branch(
            &branch,
            physical,
            vec![Edit::SetText {
                path: "../outside.tspp".into(),
                text: "export const escaped = true;\n".to_string(),
            }],
            trace.as_ref(),
        )
        .expect_err("reject escaping source path");

    assert!(matches!(error, Error::PathNotInRoot { .. }));
}

/// Restore every physical file when one write fails.
#[test]
fn test_edit_restores_failed_batch() {
    let failed_source = "export const value = 4;\n";
    let test = TestWorkspace::new_with_write_failure(
        "workspace-write-failure",
        "second.tspp",
        failed_source,
    );
    let first = test.write_text("first.tspp", "export const value = 1;\n");
    let second = test.write_text("second.tspp", "export const value = 2;\n");
    test.apply_text(&first, "export const value = 1;\n");
    test.apply_text(&second, "export const value = 2;\n");
    let before = test.workspace.revision().expect("read physical revision");

    let error = test
        .workspace
        .edit(
            before,
            vec![
                Edit::SetText {
                    path: first.clone(),
                    text: "export const value = 3;\n".to_string(),
                },
                Edit::SetText {
                    path: second.clone(),
                    text: failed_source.to_string(),
                },
            ],
        )
        .expect_err("fail write");

    assert_eq!(
        error.to_string(),
        format!(
            "filesystem error at {}: injected write failure",
            second.display()
        )
    );
    assert_eq!(
        test.fs
            .read_to_string(&first)
            .expect("read restored first file"),
        "export const value = 1;\n"
    );
    assert_eq!(
        test.fs
            .read_to_string(&second)
            .expect("read retained second file"),
        "export const value = 2;\n"
    );
    assert_eq!(
        test.workspace.revision().expect("read retained revision"),
        before
    );
}
