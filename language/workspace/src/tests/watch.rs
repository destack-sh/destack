use futures::executor::block_on;

use destack_repository::Change;
use destack_source::{ContentId, Edit, FileId, FileSystem};

use crate::tests::harness::TestWorkspace;
use crate::{Error, FileOperation, WatchEvent};

/// Begin at the current revision and emit every later commit in order.
#[test]
fn test_watch_emits_ready_and_commits() {
    let test = TestWorkspace::new("workspace_watch_commits");
    let path = test.path_for("src/main.ds");
    let revision = test.workspace.revision().expect("read revision");
    let mut watch = test.workspace.watch().expect("open watch");

    // observe the atomic subscription revision
    let ready = block_on(watch.next()).expect("receive ready event");
    assert_eq!(ready, WatchEvent::Ready { revision });

    // publish and observe the first exact semantic commit
    let first = test
        .workspace
        .apply_source_edits(vec![Edit::SetText {
            path: path.clone(),
            text: "export const value = 1;\n".to_string(),
        }])
        .expect("apply first edit");
    assert_eq!(
        block_on(watch.next()).expect("receive first commit"),
        WatchEvent::Commit(first)
    );

    // publish and observe the next exact semantic commit
    let second = test
        .workspace
        .apply_source_edits(vec![Edit::SetText {
            path,
            text: "export const value = 2;\n".to_string(),
        }])
        .expect("apply second edit");
    assert_eq!(
        block_on(watch.next()).expect("receive second commit"),
        WatchEvent::Commit(second)
    );
}

/// Retain both revisions of the latest delivered commit across repository pruning.
#[test]
fn test_watch_retains_commit_revisions() {
    let test = TestWorkspace::new("workspace_watch_retains_revisions");
    let path = test.path_for("src/main.ds");
    let mut watch = test.workspace.watch().expect("open watch");
    let _ready = block_on(watch.next()).expect("receive ready event");

    // deliver one commit, then advance the root again
    let commit = test
        .workspace
        .apply_source_edits(vec![Edit::SetText {
            path: path.clone(),
            text: "export const value = 1;\n".to_string(),
        }])
        .expect("apply delivered edit");
    assert_eq!(
        block_on(watch.next()).expect("receive delivered commit"),
        WatchEvent::Commit(commit.clone())
    );
    test.workspace
        .apply_source_edits(vec![Edit::SetText {
            path,
            text: "export const value = 2;\n".to_string(),
        }])
        .expect("apply pending edit");

    // preserve both sides of the delivered transition after explicit pruning
    let repository = &test.workspace.repository;
    repository
        .prune_unreachable()
        .expect("prune unreachable revisions");
    let before = repository.pin(commit.before).expect("pin commit input");
    let after = repository.pin(commit.after).expect("pin commit result");

    assert_eq!(before.revision(), commit.before);
    assert_eq!(after.revision(), commit.after);
}

/// Resolve root relative file operations and publish their exact commit.
#[test]
fn test_watch_emits_file_operation_commit() {
    let test = TestWorkspace::new("workspace_watch_file_operation");
    let path = test.path_for("main.ds");
    let uri = test.uri_for_path(&path);
    let mut watch = test.workspace.watch().expect("open watch");
    let _ready = block_on(watch.next()).expect("receive ready event");

    // apply one root relative editor operation
    let commit = test
        .workspace
        .apply_file_operation(FileOperation::OpenText {
            path: "main.ds".into(),
            uri,
            version: 1,
            content: "export const value = 1;\n".to_string(),
        })
        .expect("apply file operation")
        .expect("file operation should commit");

    assert_eq!(
        block_on(watch.next()).expect("receive file commit"),
        WatchEvent::Commit(commit)
    );
    assert!(
        test.workspace
            .is_file_open("main.ds".as_ref())
            .expect("inspect relative file")
    );
}

/// Move disk and semantic source together and publish one exact commit.
#[test]
fn test_watch_emits_move_commit() {
    let test = TestWorkspace::new("workspace_watch_move");
    let from = test.write_text("from.ds", "export const value = 1;\n");
    let to = test.path_for("nested/to.ds");
    let _initial = test.apply_text(&from, "export const value = 1;\n");
    let mut watch = test.workspace.watch().expect("open watch");
    let _ready = block_on(watch.next()).expect("receive ready event");

    // move one root relative file through disk and repository state
    let commit = test
        .workspace
        .apply_file_operation(FileOperation::Move {
            from: "from.ds".into(),
            to: "nested/to.ds".into(),
        })
        .expect("move file")
        .expect("move should commit");
    assert_eq!(
        commit.changes,
        vec![
            TestWorkspace::change("from.ds", Some("export const value = 1;\n"), None,),
            TestWorkspace::change("nested/to.ds", None, Some("export const value = 1;\n"),),
        ]
    );
    assert_eq!(
        test.fs.read_to_string(&to).expect("read moved file"),
        "export const value = 1;\n"
    );
    assert!(!test.fs.exists(&from).expect("inspect source path"));
    assert_eq!(
        block_on(watch.next()).expect("receive move commit"),
        WatchEvent::Commit(commit)
    );
}

/// Publish one disk reload and suppress later no-op reloads.
#[test]
fn test_reload_emits_one_commit() {
    let test = TestWorkspace::new("workspace_watch_reload");
    let path = test.path_for("src/main.ds");
    let mut watch = test.workspace.watch().expect("open watch");
    let _ready = block_on(watch.next()).expect("receive ready event");

    // change physical source outside the workspace
    test.fs
        .write_text(&path, "export const value = 1;\n")
        .expect("write physical source");
    let commit = test
        .workspace
        .reload()
        .expect("reload root")
        .expect("reload should commit");

    assert_eq!(
        commit.changes,
        vec![TestWorkspace::change(
            "src/main.ds",
            None,
            Some("export const value = 1;\n"),
        )]
    );
    assert_eq!(
        block_on(watch.next()).expect("receive reload commit"),
        WatchEvent::Commit(commit)
    );
    assert_eq!(
        test.workspace.reload().expect("reload unchanged root"),
        None
    );
}

/// Commit only host paths selected for reconciliation.
#[test]
fn test_reconcile_changed_paths() {
    let test = TestWorkspace::new("workspace_watch_reconcile");
    let first = test.write_text("src/first.ds", "export const first = 1;\n");
    let second = test.write_text("src/second.ds", "export const second = 1;\n");
    let _initial = test
        .workspace
        .reload()
        .expect("reload initial files")
        .expect("initial files should commit");

    // leave paths excluded by the package manifest outside repository state
    let ignored = test.write_text(".git/index", "ignored\n");
    assert_eq!(
        test.workspace
            .reconcile(vec![ignored])
            .expect("reconcile excluded path"),
        None
    );

    // change two tracked files but reconcile only the selected host path
    test.fs
        .write_text(&first, "export const first = 2;\n")
        .expect("write first physical source");
    test.fs
        .write_text(&second, "export const second = 2;\n")
        .expect("write second physical source");
    let commit = test
        .workspace
        .reconcile(vec![first.clone(), first.clone()])
        .expect("reconcile first path")
        .expect("first path should commit");

    assert_eq!(
        commit.changes,
        vec![TestWorkspace::change(
            "src/first.ds",
            Some("export const first = 1;\n"),
            Some("export const first = 2;\n"),
        )]
    );
    assert_eq!(
        test.workspace
            .reconcile(vec![first])
            .expect("reconcile unchanged first path"),
        None
    );

    // leave the other physical change for the full reload fallback
    let commit = test
        .workspace
        .reload()
        .expect("reload remaining path")
        .expect("remaining path should commit");

    assert_eq!(
        commit.changes,
        vec![TestWorkspace::change(
            "src/second.ds",
            Some("export const second = 1;\n"),
            Some("export const second = 2;\n"),
        )]
    );
}

/// Remove every tracked file beneath one deleted host directory.
#[test]
fn test_reconcile_deleted_directory() {
    let test = TestWorkspace::new("workspace_watch_deleted_directory");
    let directory = test.path_for("src/deleted");
    test.write_text("src/deleted/first.ds", "export const first = 1;\n");
    test.write_text("src/deleted/second.ds", "export const second = 1;\n");
    let _initial = test
        .workspace
        .reload()
        .expect("reload initial directory")
        .expect("initial directory should commit");

    // reconcile one directory deletion without requiring child events
    std::fs::remove_dir_all(&directory).expect("remove physical source directory");
    let commit = test
        .workspace
        .reconcile(vec![directory])
        .expect("reconcile deleted directory")
        .expect("deleted directory should commit");
    assert_eq!(
        commit.changes,
        vec![
            TestWorkspace::change(
                "src/deleted/first.ds",
                Some("export const first = 1;\n"),
                None,
            ),
            TestWorkspace::change(
                "src/deleted/second.ds",
                Some("export const second = 1;\n"),
                None,
            ),
        ]
    );
}

/// Discover every included source file beneath one changed host directory.
#[test]
fn test_reconcile_changed_directory() {
    let test = TestWorkspace::new("workspace_watch_changed_directory");
    let directory = test.path_for("src/generated");
    test.write_text("src/generated/first.ds", "export const first = 1;\n");
    test.write_text("src/generated/second.ds", "export const second = 1;\n");

    // reconcile one directory hint without requiring child events
    let commit = test
        .workspace
        .reconcile(vec![directory])
        .expect("reconcile changed directory")
        .expect("changed directory should commit");
    assert_eq!(
        commit.changes,
        vec![
            TestWorkspace::change(
                "src/generated/first.ds",
                None,
                Some("export const first = 1;\n"),
            ),
            TestWorkspace::change(
                "src/generated/second.ds",
                None,
                Some("export const second = 1;\n"),
            ),
        ]
    );
}

/// Preserve open editor truth when the underlying disk file changes.
#[test]
fn test_reload_preserves_open_file() {
    let test = TestWorkspace::new("workspace_reload_open_file");
    let path = test.write_text("src/main.ds", "export const value = 1;\n");
    let uri = test.uri_for_path(&path);
    let _open = test
        .workspace
        .apply_file_operation(FileOperation::OpenText {
            path: "src/main.ds".into(),
            uri,
            version: 1,
            content: "export const value = 2;\n".to_string(),
        })
        .expect("open file")
        .expect("open should commit");
    let revision = test.workspace.revision().expect("read open revision");

    // change only the physical file hidden beneath the editor overlay
    test.fs
        .write_text(&path, "export const value = 3;\n")
        .expect("write physical source");

    assert_eq!(test.workspace.reload().expect("reload root"), None);
    assert_eq!(
        test.workspace.revision().expect("read retained revision"),
        revision
    );
}

/// Preserve open binary content until the editor closes the file.
#[test]
fn test_reload_preserves_open_binary_file() {
    let test = TestWorkspace::new("workspace_reload_open_binary_file");
    let path = test
        .fs
        .write_bytes("src/image.png", &[1])
        .expect("write initial image");
    test.workspace.reload().expect("reload initial image");
    let uri = test.uri_for_path(&path);

    // open editor bytes and then change the hidden physical file
    test.workspace
        .apply_file_operation(FileOperation::OpenBytes {
            path: "src/image.png".into(),
            uri,
            version: 1,
            content: vec![2],
        })
        .expect("open image")
        .expect("open should commit");
    let revision = test.workspace.revision().expect("read open revision");
    test.fs
        .write_bytes(&path, &[3])
        .expect("write physical image");

    // retain editor truth across physical reconciliation
    assert_eq!(test.workspace.reload().expect("reload root"), None);
    assert_eq!(
        test.workspace
            .repository
            .file_system()
            .read(&path)
            .expect("read overlay"),
        [2]
    );
    assert_eq!(
        test.workspace.revision().expect("read retained revision"),
        revision
    );

    // restore and commit physical truth when the editor closes
    let commit = test
        .workspace
        .apply_file_operation(FileOperation::Close {
            path: "src/image.png".into(),
        })
        .expect("close image")
        .expect("close should commit");
    assert_eq!(
        commit.changes,
        [Change {
            file: FileId::from_logical_str("src/image.png"),
            path: "src/image.png".to_string(),
            before: Some(ContentId::for_binary(&[2])),
            after: Some(ContentId::for_binary(&[3])),
        }]
    );
}

/// Fail explicitly when a consumer exceeds the bounded commit backlog.
#[test]
fn test_watch_reports_lag() {
    let test = TestWorkspace::new("workspace_watch_lag");
    let root = &test.root;
    let path = test.path_for("main.ds");
    let mut watch = test.workspace.watch().expect("open watch");
    let _ready = block_on(watch.next()).expect("receive ready event");

    // publish two commits without consuming the single pending transition
    for value in 1..=2 {
        test.workspace
            .apply_source_edits(vec![Edit::SetText {
                path: path.clone(),
                text: format!("export const value = {value};\n"),
            }])
            .expect("apply edit");
    }

    let error = block_on(watch.next()).expect_err("report watch lag");
    assert!(matches!(error, Error::WatchLagged { root: lagged } if lagged == *root));
}

/// Fail an active watch when its exact root closes.
#[test]
fn test_watch_reports_closed_root() {
    let test = TestWorkspace::new("workspace_watch_close");
    let root = &test.root;
    let mut watch = test.workspace.watch().expect("open watch");
    let _ready = block_on(watch.next()).expect("receive ready event");

    test.workspace.close();

    let error = block_on(watch.next()).expect_err("report closed root");
    assert!(matches!(error, Error::WatchClosed { root: closed } if closed == *root));
}

/// Terminate affected subscriptions and recover after host observation resumes.
#[test]
fn test_watch_reports_host_failure_and_recovers() {
    let test = TestWorkspace::new("workspace_watch_host_failure");
    let root = &test.root;
    let mut failed = test.workspace.watch().expect("open failed watch");
    let _ready = block_on(failed.next()).expect("receive ready event");

    // fail both the active watch and later subscriptions
    test.workspace
        .fail_watch("host watcher stopped".to_string())
        .expect("fail host watch");
    let error = block_on(failed.next()).expect_err("report active host failure");
    assert!(
        matches!(error, Error::WatchFailed { root: failed, detail } if failed == *root && detail == "host watcher stopped")
    );

    let error = test
        .workspace
        .watch()
        .expect_err("host failure should block new watches");
    assert!(
        matches!(error, Error::WatchFailed { root: failed, detail } if failed == *root && detail == "host watcher stopped")
    );

    // retain failure through a plain reload
    assert_eq!(test.workspace.reload().expect("reload root"), None);
    let error = test
        .workspace
        .watch()
        .expect_err("reload should not resume host observation");
    assert!(
        matches!(error, Error::WatchFailed { root: failed, detail } if failed == *root && detail == "host watcher stopped")
    );

    // restore subscriptions only after host observation resumes
    test.workspace
        .resume_watch()
        .expect("host observation should resume");
    let revision = test.workspace.revision().expect("read revision");
    let mut recovered = test.workspace.watch().expect("open recovered watch");
    assert_eq!(
        block_on(recovered.next()).expect("receive recovered ready event"),
        WatchEvent::Ready { revision }
    );
}
