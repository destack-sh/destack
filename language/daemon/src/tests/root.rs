use std::path::PathBuf;

use crate::protocol::{
    DaemonRequest, DaemonResponse, SourceEdit, SourceUpdate, SourceUpdateRequest,
};
use crate::tests::{TestDaemon, TestProtocolHarness};

/// Tracks roots independently.
#[test]
fn test_daemon_tracks_root_by_handle() {
    let root_a = PathBuf::from("/root/a");
    let root_b = PathBuf::from("/root/b");
    let test = TestDaemon::new_with_roots(vec![root_a.clone(), root_b.clone()]);
    let harness = TestProtocolHarness::from_test(test);
    harness.handshake();

    // check the workspace root is opened before protocol handles
    assert_eq!(harness.test.daemon.root_count(), 1);

    // open two independent protocol roots
    let handle_a = harness.open_root_path(root_a);
    let handle_b = harness.open_root_path(root_b);

    // check that each protocol root has its own live state
    assert_ne!(handle_a, handle_b);
    assert_eq!(harness.test.daemon.root_count(), 3);

    harness.shutdown();
}

/// Keeps updates isolated to the root that changed.
#[test]
fn test_daemon_updates_do_not_cross_roots() {
    let root_a = PathBuf::from("/root/a");
    let root_b = PathBuf::from("/root/b");
    let test = TestDaemon::new_with_roots(vec![root_a.clone(), root_b.clone()]);

    let file_a = root_a.join("main.ds");
    let file_b = root_b.join("main.ds");
    test.update_file(&file_b, "export const b = 2;");

    // update root a and collect the daemon updates
    let updates = test.update_file(&file_a, "export const a = 1;");

    // assertion block
    assert!(updates.iter().all(|update| {
        update.file.as_ref().and_then(|file| file.path.as_deref()) != Some(file_b.as_path())
    }));
}

/// Advances the root revision for protocol source updates.
#[test]
fn test_protocol_source_update_advances_root_revision() {
    let harness = TestProtocolHarness::default();
    harness.handshake();
    let handle = harness.open_root();
    let path = harness.test.root.join("main.ds");

    // apply one atomic source update through protocol
    let response = harness.send_request(DaemonRequest::ApplySourceUpdate(SourceUpdateRequest {
        handle,
        update: SourceUpdate {
            base: None,
            edits: vec![SourceEdit::SetText {
                path: path.clone(),
                text: "export const value = 1;\n".to_string(),
            }],
        },
    }));

    // assert revision and file update payloads
    match response {
        DaemonResponse::SourceUpdated(response) => {
            assert_ne!(response.before, response.after);
            assert!(response.updates.iter().any(|update| {
                update.file.as_ref().and_then(|file| file.path.as_deref()) == Some(path.as_path())
            }));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    harness.shutdown();
}
