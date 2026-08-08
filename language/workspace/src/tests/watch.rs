use futures::executor::block_on;

use destack_source::{Edit, FileSystem};

use crate::tests::harness::TestWorkspace;
use crate::{Error, FileOperation, WatchEvent, Workspace};

/// Begin at the current revision and emit every later commit in order.
#[test]
fn test_watch_emits_ready_and_commits() {
    let test = TestWorkspace::new("workspace_watch_commits");
    let root = &test.roots[0];
    let path = test.path_for("src/main.ds");
    let revision = test.workspace.revision(root).expect("read revision");
    let mut watch = test.workspace.watch(root).expect("open watch");

    // observe the atomic subscription revision
    let ready = block_on(watch.next()).expect("receive ready event");
    assert_eq!(ready, WatchEvent::Ready { revision });

    // publish and observe two exact semantic commits
    let first = test
        .workspace
        .apply_source_edits(
            root,
            vec![Edit::SetText {
                path: path.clone(),
                text: "export const value = 1;\n".to_string(),
            }],
        )
        .expect("apply first edit");
    let second = test
        .workspace
        .apply_source_edits(
            root,
            vec![Edit::SetText {
                path,
                text: "export const value = 2;\n".to_string(),
            }],
        )
        .expect("apply second edit");

    assert_eq!(
        block_on(watch.next()).expect("receive first commit"),
        WatchEvent::Commit(first)
    );
    assert_eq!(
        block_on(watch.next()).expect("receive second commit"),
        WatchEvent::Commit(second)
    );
}

/// Resolve root relative file operations and publish their exact commit.
#[test]
fn test_watch_emits_file_operation_commit() {
    let test = TestWorkspace::new("workspace_watch_file_operation");
    let root = &test.roots[0];
    let path = test.path_for("main.ds");
    let uri = test.uri_for_path(&path);
    let mut watch = test.workspace.watch(root).expect("open watch");
    let _ready = block_on(watch.next()).expect("receive ready event");

    // apply one root relative editor operation
    let commit = test
        .workspace
        .file(
            root,
            FileOperation::OpenText {
                path: "main.ds".into(),
                uri,
                version: 1,
                content: "export const value = 1;\n".to_string(),
            },
        )
        .expect("apply file operation")
        .expect("file operation should commit");

    assert_eq!(
        block_on(watch.next()).expect("receive file commit"),
        WatchEvent::Commit(commit)
    );
    assert!(
        test.workspace
            .is_file_open(root, "main.ds".as_ref())
            .expect("inspect relative file")
    );
}

/// Move disk and semantic source together and publish one exact commit.
#[test]
fn test_watch_emits_move_commit() {
    let test = TestWorkspace::new("workspace_watch_move");
    let root = &test.roots[0];
    let from = test.write_text("from.ds", "export const value = 1;\n");
    let to = test.path_for("nested/to.ds");
    let _initial = test.apply_text(&from, "export const value = 1;\n");
    let mut watch = test.workspace.watch(root).expect("open watch");
    let _ready = block_on(watch.next()).expect("receive ready event");

    // move one root relative file through disk and repository state
    let commit = test
        .workspace
        .file(
            root,
            FileOperation::Move {
                from: "from.ds".into(),
                to: "nested/to.ds".into(),
            },
        )
        .expect("move file")
        .expect("move should commit");
    let update_paths = commit
        .updates
        .iter()
        .map(|update| update.diagnostic_uri.to_path_buf())
        .collect::<Vec<_>>();
    let removed = commit
        .updates
        .iter()
        .map(|update| update.is_removed)
        .collect::<Vec<_>>();

    assert_eq!(update_paths, vec![Some(from.clone()), Some(to.clone())]);
    assert_eq!(removed, vec![true, false]);
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
    let root = &test.roots[0];
    let path = test.path_for("src/main.ds");
    let mut watch = test.workspace.watch(root).expect("open watch");
    let _ready = block_on(watch.next()).expect("receive ready event");

    // change physical source outside the workspace
    test.fs
        .write_text(&path, "export const value = 1;\n")
        .expect("write physical source");
    let commit = test
        .workspace
        .reload(root)
        .expect("reload root")
        .expect("reload should commit");

    assert_eq!(commit.updates.len(), 1);
    assert_eq!(commit.updates[0].diagnostic_uri.to_path_buf(), Some(path));
    assert_eq!(
        block_on(watch.next()).expect("receive reload commit"),
        WatchEvent::Commit(commit)
    );
    assert_eq!(
        test.workspace.reload(root).expect("reload unchanged root"),
        None
    );
}

/// Preserve open editor truth when the underlying disk file changes.
#[test]
fn test_reload_preserves_open_file() {
    let test = TestWorkspace::new("workspace_reload_open_file");
    let root = &test.roots[0];
    let path = test.write_text("src/main.ds", "export const value = 1;\n");
    let uri = test.uri_for_path(&path);
    let _open = test
        .workspace
        .file(
            root,
            FileOperation::OpenText {
                path: "src/main.ds".into(),
                uri,
                version: 1,
                content: "export const value = 2;\n".to_string(),
            },
        )
        .expect("open file")
        .expect("open should commit");
    let revision = test.workspace.revision(root).expect("read open revision");

    // change only the physical file hidden beneath the editor overlay
    test.fs
        .write_text(&path, "export const value = 3;\n")
        .expect("write physical source");

    assert_eq!(test.workspace.reload(root).expect("reload root"), None);
    assert_eq!(
        test.workspace
            .revision(root)
            .expect("read retained revision"),
        revision
    );
}

/// Fail explicitly when a consumer exceeds the bounded commit backlog.
#[test]
fn test_watch_reports_lag() {
    let test = TestWorkspace::new("workspace_watch_lag");
    let root = &test.roots[0];
    let path = test.path_for("main.ds");
    let mut watch = test.workspace.watch(root).expect("open watch");
    let _ready = block_on(watch.next()).expect("receive ready event");

    // exceed the bounded subscription without consuming commits
    for value in 0..128 {
        test.workspace
            .apply_source_edits(
                root,
                vec![Edit::SetText {
                    path: path.clone(),
                    text: format!("export const value = {value};\n"),
                }],
            )
            .expect("apply edit");
    }

    let error = block_on(watch.next()).expect_err("report watch lag");
    assert!(matches!(error, Error::WatchLagged { root: lagged } if lagged == *root));
}

/// Fail an active watch when its exact root closes.
#[test]
fn test_watch_reports_closed_root() {
    let test = TestWorkspace::new("workspace_watch_close");
    let root = &test.roots[0];
    let mut watch = test.workspace.watch(root).expect("open watch");
    let _ready = block_on(watch.next()).expect("receive ready event");

    test.workspace.close_root(root).expect("close root");

    let error = block_on(watch.next()).expect_err("report closed root");
    assert!(matches!(error, Error::WatchClosed { root: closed } if closed == *root));
}

/// Terminate affected subscriptions and recover only after a successful reload.
#[test]
fn test_watch_reports_host_failure_and_recovers() {
    let test = TestWorkspace::new("workspace_watch_host_failure");
    let root = &test.roots[0];
    let mut failed = test.workspace.watch(root).expect("open failed watch");
    let _ready = block_on(failed.next()).expect("receive ready event");

    // fail both the active watch and later subscriptions
    test.workspace
        .fail_watch(root, "host watcher stopped".to_string())
        .expect("fail host watch");
    let error = block_on(failed.next()).expect_err("report active host failure");
    assert!(
        matches!(error, Error::WatchFailed { root: failed, detail } if failed == *root && detail == "host watcher stopped")
    );

    let error = test
        .workspace
        .watch(root)
        .expect_err("host failure should block new watches");
    assert!(
        matches!(error, Error::WatchFailed { root: failed, detail } if failed == *root && detail == "host watcher stopped")
    );

    // restore new subscriptions only after the root reloads successfully
    assert_eq!(test.workspace.reload(root).expect("reload root"), None);
    let revision = test.workspace.revision(root).expect("read revision");
    let mut recovered = test.workspace.watch(root).expect("open recovered watch");
    assert_eq!(
        block_on(recovered.next()).expect("receive recovered ready event"),
        WatchEvent::Ready { revision }
    );
}
