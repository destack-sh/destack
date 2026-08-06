use destack_source::Edit;
use destack_workspace::{
    ApplySourceUpdateRequest, OpenRootRequest, ReadRevisionRequest, SourceUpdate,
};

use super::harness::TestDaemon;

/// Round trip workspace operations and daemon shutdown over IPC RPC.
#[test]
fn test_serve_workspace_connection() {
    let daemon = TestDaemon::start("daemon_rpc_connection");
    let connection = daemon.connect();
    let root = daemon.root().to_path_buf();
    let opened = connection
        .workspace()
        .open_root(OpenRootRequest { root: root.clone() })
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
        .workspace()
        .open_root(OpenRootRequest { root: root.clone() })
        .expect("first root should open")
        .value;
    let second_opened = second
        .workspace()
        .open_root(OpenRootRequest { root: root.clone() })
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
