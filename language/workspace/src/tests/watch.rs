use futures::executor::block_on;

use tspp_repository::TraceLevel;
use tspp_source::{Edit, FileSystem};

use crate::tests::harness::TestWorkspace;
use crate::{Error, WatchEvent};

/// Begin at physical state's current revision and emit each later commit in order.
#[test]
fn test_watch_emits_ready_and_commits() {
    let test = TestWorkspace::new("workspace-watch-commits");
    let path = test.path_for("src/main.tspp");
    let revision = test.workspace.revision().expect("read revision");
    let mut watch = test.workspace.watch().expect("open watch");

    assert_eq!(
        block_on(watch.next()).expect("receive ready event"),
        WatchEvent::Ready { revision }
    );

    // publish and observe both exact transitions
    let first = test
        .write(vec![Edit::SetText {
            path: path.clone(),
            text: "export const value = 1;\n".to_string(),
        }])
        .expect("write first edit");
    assert_eq!(
        block_on(watch.next()).expect("receive first commit"),
        WatchEvent::Commit(first)
    );

    let second = test
        .write(vec![Edit::SetText {
            path,
            text: "export const value = 2;\n".to_string(),
        }])
        .expect("write second edit");
    assert_eq!(
        block_on(watch.next()).expect("receive second commit"),
        WatchEvent::Commit(second)
    );
}

/// Route commits only to watches of the changed branch and terminate removed branches.
#[test]
fn test_watch_selects_and_removes_branch() {
    let test = TestWorkspace::new("workspace-watch-branch");
    let physical = test.workspace.revision().expect("read physical revision");
    let branch = "watch".to_string();
    test.workspace
        .create_branch(branch.clone(), physical)
        .expect("create branch");
    let mut watch = test.workspace.watch_branch(&branch).expect("watch branch");
    let _ready = block_on(watch.next()).expect("receive ready event");
    let trace = test.workspace.start_trace(TraceLevel::Disabled);

    let commit = test
        .workspace
        .edit_branch(
            &branch,
            physical,
            vec![Edit::SetText {
                path: test.path_for("main.tspp"),
                text: "export const value = 1;\n".to_string(),
            }],
            trace.as_ref(),
        )
        .expect("edit branch");

    assert_eq!(
        block_on(watch.next()).expect("receive branch commit"),
        WatchEvent::Commit(commit)
    );
    assert_eq!(
        test.workspace.revision().expect("read physical revision"),
        physical
    );

    test.workspace
        .remove_branch(&branch)
        .expect("remove watched branch");
    let error = block_on(watch.next()).expect_err("removed branch should terminate watch");
    assert!(matches!(error, Error::WatchRemoved { .. }));
}

/// Retain both revisions of the latest delivered commit across repository pruning.
#[test]
fn test_watch_retains_commit_revisions() {
    let test = TestWorkspace::new("workspace-watch-retains-revisions");
    let path = test.path_for("src/main.tspp");
    let mut watch = test.workspace.watch().expect("open watch");
    let _ready = block_on(watch.next()).expect("receive ready event");

    let commit = test
        .write(vec![Edit::SetText {
            path: path.clone(),
            text: "export const value = 1;\n".to_string(),
        }])
        .expect("write delivered edit");
    assert_eq!(
        block_on(watch.next()).expect("receive delivered commit"),
        WatchEvent::Commit(commit.clone())
    );
    test.write(vec![Edit::SetText {
        path,
        text: "export const value = 2;\n".to_string(),
    }])
    .expect("write pending edit");

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

/// Move physical and repository source together and publish one exact commit.
#[test]
fn test_watch_emits_move_commit() {
    let test = TestWorkspace::new("workspace-watch-move");
    let from = test.write_text("from.tspp", "export const value = 1;\n");
    let to = test.path_for("nested/to.tspp");
    test.apply_text(&from, "export const value = 1;\n");
    let mut watch = test.workspace.watch().expect("open watch");
    let _ready = block_on(watch.next()).expect("receive ready event");

    let commit = test
        .write(vec![Edit::Move {
            from: from.clone(),
            to: to.clone(),
        }])
        .expect("move file");

    assert_eq!(
        commit.changes,
        vec![
            TestWorkspace::change("from.tspp", Some("export const value = 1;\n"), None),
            TestWorkspace::change("nested/to.tspp", None, Some("export const value = 1;\n")),
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

/// Publish one physical reload and suppress later no-op reloads.
#[test]
fn test_reload_emits_one_commit() {
    let test = TestWorkspace::new("workspace-watch-reload");
    let path = test.path_for("src/main.tspp");
    let mut watch = test.workspace.watch().expect("open watch");
    let _ready = block_on(watch.next()).expect("receive ready event");

    test.fs
        .write_text(&path, "export const value = 1;\n")
        .expect("write physical source");
    let commit = test
        .workspace
        .reload()
        .expect("reload workspace")
        .expect("reload should commit");

    assert_eq!(
        commit.changes,
        vec![TestWorkspace::change(
            "src/main.tspp",
            None,
            Some("export const value = 1;\n"),
        )]
    );
    assert_eq!(
        block_on(watch.next()).expect("receive reload commit"),
        WatchEvent::Commit(commit)
    );
    assert_eq!(
        test.workspace.reload().expect("reload unchanged workspace"),
        None
    );
}

/// Commit only physical paths selected for reconciliation.
#[test]
fn test_reconcile_selects_paths() {
    let test = TestWorkspace::new("workspace-watch-reconcile");
    let first = test.write_text("src/first.tspp", "export const first = 1;\n");
    let second = test.write_text("src/second.tspp", "export const second = 1;\n");
    test.workspace
        .reload()
        .expect("reload initial files")
        .expect("initial files should commit");

    let ignored = test.write_text(".git/index", "ignored\n");
    assert_eq!(
        test.workspace
            .reconcile(vec![ignored])
            .expect("reconcile excluded path"),
        None
    );

    test.fs
        .write_text(&first, "export const first = 2;\n")
        .expect("write first source");
    test.fs
        .write_text(&second, "export const second = 2;\n")
        .expect("write second source");
    let commit = test
        .workspace
        .reconcile(vec![first.clone(), first.clone()])
        .expect("reconcile first path")
        .expect("first path should commit");

    assert_eq!(
        commit.changes,
        vec![TestWorkspace::change(
            "src/first.tspp",
            Some("export const first = 1;\n"),
            Some("export const first = 2;\n"),
        )]
    );
    assert_eq!(
        test.workspace
            .reconcile(vec![first])
            .expect("reconcile unchanged path"),
        None
    );

    let commit = test
        .workspace
        .reload()
        .expect("reload remaining path")
        .expect("remaining path should commit");
    assert_eq!(
        commit.changes,
        vec![TestWorkspace::change(
            "src/second.tspp",
            Some("export const second = 1;\n"),
            Some("export const second = 2;\n"),
        )]
    );
}

/// Reconcile every tracked file beneath one deleted physical directory.
#[test]
fn test_reconcile_deleted_directory() {
    let test = TestWorkspace::new("workspace-watch-deleted-directory");
    let directory = test.path_for("src/deleted");
    test.write_text("src/deleted/first.tspp", "export const first = 1;\n");
    test.write_text("src/deleted/second.tspp", "export const second = 1;\n");
    test.workspace
        .reload()
        .expect("reload initial directory")
        .expect("initial directory should commit");

    std::fs::remove_dir_all(&directory).expect("remove physical directory");
    let commit = test
        .workspace
        .reconcile(vec![directory])
        .expect("reconcile deleted directory")
        .expect("deleted directory should commit");

    assert_eq!(
        commit.changes,
        vec![
            TestWorkspace::change(
                "src/deleted/first.tspp",
                Some("export const first = 1;\n"),
                None,
            ),
            TestWorkspace::change(
                "src/deleted/second.tspp",
                Some("export const second = 1;\n"),
                None,
            ),
        ]
    );
}

/// Discover every included source file beneath one changed physical directory.
#[test]
fn test_reconcile_changed_directory() {
    let test = TestWorkspace::new("workspace-watch-changed-directory");
    let directory = test.path_for("src/generated");
    test.write_text("src/generated/first.tspp", "export const first = 1;\n");
    test.write_text("src/generated/second.tspp", "export const second = 1;\n");

    let commit = test
        .workspace
        .reconcile(vec![directory])
        .expect("reconcile changed directory")
        .expect("changed directory should commit");

    assert_eq!(
        commit.changes,
        vec![
            TestWorkspace::change(
                "src/generated/first.tspp",
                None,
                Some("export const first = 1;\n"),
            ),
            TestWorkspace::change(
                "src/generated/second.tspp",
                None,
                Some("export const second = 1;\n"),
            ),
        ]
    );
}

/// Fail explicitly when a consumer exceeds the bounded commit backlog.
#[test]
fn test_watch_reports_lag() {
    let test = TestWorkspace::new("workspace-watch-lag");
    let root = &test.root;
    let path = test.path_for("main.tspp");
    let mut watch = test.workspace.watch().expect("open watch");
    let _ready = block_on(watch.next()).expect("receive ready event");

    for value in 1..=2 {
        test.write(vec![Edit::SetText {
            path: path.clone(),
            text: format!("export const value = {value};\n"),
        }])
        .expect("write edit");
    }

    let error = block_on(watch.next()).expect_err("report watch lag");
    assert!(matches!(error, Error::WatchLagged { root: lagged, branch: None } if lagged == *root));
}

/// Fail an active watch when its workspace closes.
#[test]
fn test_watch_reports_closed_workspace() {
    let test = TestWorkspace::new("workspace-watch-close");
    let root = &test.root;
    let mut watch = test.workspace.watch().expect("open watch");
    let _ready = block_on(watch.next()).expect("receive ready event");

    test.workspace.close();

    let error = block_on(watch.next()).expect_err("report closed workspace");
    assert!(matches!(error, Error::WatchClosed { root: closed, branch: None } if closed == *root));
}

/// Terminate affected watches and recover after physical observation resumes.
#[test]
fn test_watch_reports_host_failure_and_recovers() {
    let test = TestWorkspace::new("workspace-watch-host-failure");
    let root = &test.root;
    let mut failed = test.workspace.watch().expect("open failed watch");
    let _ready = block_on(failed.next()).expect("receive ready event");

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
        .expect_err("block watches during host failure");
    assert!(
        matches!(error, Error::WatchFailed { root: failed, detail } if failed == *root && detail == "host watcher stopped")
    );

    assert_eq!(test.workspace.reload().expect("reload workspace"), None);
    test.workspace
        .resume_watch()
        .expect("resume host observation");
    let revision = test.workspace.revision().expect("read revision");
    let mut recovered = test.workspace.watch().expect("open recovered watch");

    assert_eq!(
        block_on(recovered.next()).expect("receive recovered ready event"),
        WatchEvent::Ready { revision }
    );
}
