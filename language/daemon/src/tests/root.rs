use crate::tests::{TestDaemon, TestProtocolHarness};
use destack_session as session;
use destack_workspace::protocol::{
    ProtocolErrorCode, SourceUpdateRequest, WorkspaceRequest, WorkspaceResponse,
};

/// Keeps updates isolated to the root that changed.
#[test]
fn test_daemon_updates_do_not_cross_roots() {
    let root_a = std::path::PathBuf::from("/root/a");
    let root_b = std::path::PathBuf::from("/root/b");
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
    let response = harness.send_request(WorkspaceRequest::ApplySourceUpdate(SourceUpdateRequest {
        handle,
        update: session::Update {
            base: None,
            edits: vec![session::Edit::SetText {
                path: path.clone(),
                text: "export const value = 1;\n".to_string(),
            }],
        },
    }));

    // assert revision and file update payloads
    match response {
        WorkspaceResponse::SourceUpdated(response) => {
            assert_ne!(response.commit.before, response.commit.after);
            assert!(response.commit.updates.iter().any(|update| {
                update.file.as_ref().and_then(|file| file.path.as_deref()) == Some(path.as_path())
            }));
        }
        other => panic!("unexpected response: {other:?}"),
    }

    harness.shutdown();
}

/// Rejects source updates that escape an opened protocol root.
#[test]
fn test_protocol_source_update_rejects_escaped_path() {
    let harness = TestProtocolHarness::default();
    harness.handshake();
    let handle = harness.open_root();
    let path = harness.test.root.join("../outside.ds");

    // apply one escaped source update through protocol
    let response = harness.send_request(WorkspaceRequest::ApplySourceUpdate(SourceUpdateRequest {
        handle,
        update: session::Update {
            base: None,
            edits: vec![session::Edit::SetText {
                path,
                text: "export const value = 1;\n".to_string(),
            }],
        },
    }));

    // assert the protocol boundary rejects the escaped path
    match response {
        WorkspaceResponse::Error(error) => {
            assert_eq!(error.code, ProtocolErrorCode::Forbidden);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    harness.shutdown();
}
