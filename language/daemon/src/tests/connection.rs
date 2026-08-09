use destack_repository::Change;
use destack_rpc::{CallError, Code};
use destack_source::{ContentId, Edit, FileId};
use destack_workspace::{
    ApplySourceUpdateRequest, ReadRevisionRequest, SourceUpdate, WatchEvent, WatchRequest,
};

use super::harness::TestDaemon;
use crate::OpenWorkspaceRequest;

/// Round trip workspace operations and daemon shutdown over IPC RPC.
#[test]
fn test_serve_workspace_connection() {
    let daemon = TestDaemon::start("daemon_rpc_connection");
    let connection = daemon.connect();
    let root = daemon.root().to_path_buf();
    let opened = connection
        .daemon()
        .open_workspace(OpenWorkspaceRequest { root: root.clone() })
        .expect("root should open")
        .value;
    assert_eq!(opened.root, root);

    let path = root.join("main.ds");
    let commit = connection
        .workspace()
        .apply_source_update(ApplySourceUpdateRequest {
            root: root.clone(),
            update: SourceUpdate {
                base: Some(opened.revision),
                edits: vec![Edit::SetText {
                    path,
                    text: "export const answer = 42;\n".to_string(),
                }],
            },
        })
        .expect("source update should apply")
        .value;
    let revision = connection
        .workspace()
        .read_revision(ReadRevisionRequest { root })
        .expect("revision should read")
        .value;

    assert_eq!(commit.after, revision);
    assert_ne!(commit.before, commit.after);

    daemon.shutdown(connection);
}

/// Keep one workspace root alive when another client disconnects.
#[test]
fn test_share_workspace_root_across_connections() {
    let daemon = TestDaemon::start("daemon_shared_workspace_root");
    let first = daemon.connect();
    let second = daemon.connect();
    let root = daemon.root().to_path_buf();
    let first_opened = first
        .daemon()
        .open_workspace(OpenWorkspaceRequest { root: root.clone() })
        .expect("first root should open")
        .value;
    let second_opened = second
        .daemon()
        .open_workspace(OpenWorkspaceRequest { root: root.clone() })
        .expect("second root should open")
        .value;

    assert_eq!(first_opened, second_opened);

    first.close().expect("first connection should close");
    let revision = second
        .workspace()
        .read_revision(ReadRevisionRequest { root })
        .expect("shared root should remain open")
        .value;

    assert_eq!(revision, second_opened.revision);

    daemon.shutdown(second);
}

/// Publish one physical change as the same semantic commit to every client.
#[test]
fn test_share_physical_workspace_commit_across_connections() {
    let daemon = TestDaemon::start("daemon_shared_physical_commit");
    let first = daemon.connect();
    let second = daemon.connect();
    let root = daemon.root().to_path_buf();
    let mut first_watch = first
        .workspace()
        .watch(WatchRequest { root: root.clone() })
        .expect("first watch should start");
    let mut second_watch = second
        .workspace()
        .watch(WatchRequest { root: root.clone() })
        .expect("second watch should start");

    // establish both subscriptions at one exact revision
    let first_ready = first_watch
        .receive()
        .expect("first ready should receive")
        .expect("first ready should exist");
    let second_ready = second_watch
        .receive()
        .expect("second ready should receive")
        .expect("second ready should exist");
    assert_eq!(first_ready, second_ready);
    assert!(matches!(first_ready, WatchEvent::Ready { .. }));

    // write disk truth through the daemon owned host watch
    daemon.write_text("src/main.ds", "export const answer = 42;\n");
    let first_commit = first_watch
        .receive()
        .expect("first commit should receive")
        .expect("first commit should exist");
    let second_commit = second_watch
        .receive()
        .expect("second commit should receive")
        .expect("second commit should exist");

    assert_eq!(first_commit, second_commit);
    let WatchEvent::Commit(commit) = first_commit else {
        panic!("physical change should produce one semantic commit");
    };
    assert_eq!(
        commit.changes,
        vec![Change {
            file: FileId::from_logical_str("src/main.ds"),
            path: "src/main.ds".to_string(),
            before: None,
            after: Some(ContentId::for_text("export const answer = 42;\n")),
        }]
    );
    let revision = second
        .workspace()
        .read_revision(ReadRevisionRequest { root })
        .expect("shared revision should read")
        .value;
    assert_eq!(commit.after, revision);

    first_watch.cancel().expect("first watch should cancel");
    second_watch.cancel().expect("second watch should cancel");
    assert_eq!(
        first_watch.receive().expect("first watch should finish"),
        None
    );
    assert_eq!(
        second_watch.receive().expect("second watch should finish"),
        None
    );
    let first_error = first_watch
        .response()
        .expect_err("first cancellation should fail the call");
    let second_error = second_watch
        .response()
        .expect_err("second cancellation should fail the call");
    let CallError::Status(first_status) = first_error else {
        panic!("expected first cancellation status, got {first_error}");
    };
    let CallError::Status(second_status) = second_error else {
        panic!("expected second cancellation status, got {second_error}");
    };

    assert_eq!(first_status.code, Code::Canceled);
    assert_eq!(first_status.message, "call did not complete");
    assert_eq!(second_status, first_status);

    first.close().expect("first connection should close");
    daemon.shutdown(second);
}
