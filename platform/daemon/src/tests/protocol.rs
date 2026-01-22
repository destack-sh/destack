use std::path::PathBuf;

use crate::protocol::{
    CacheStatsPayload, DaemonNotification, DaemonQuery, DaemonQueryResponse, DaemonRequest,
    DaemonResponse, FileUpdate, FileUpdateKind, FileUpdateRequest, OpenWorkspaceRequest,
    PROTOCOL_VERSION, PayloadBody, PayloadChunkNotification, PayloadFormat, PayloadId,
    ProtocolClientError, ProtocolClientOptions, ProtocolErrorCode, ProtocolLimits, ProtocolMessage,
    ProtocolNotification, ProtocolRange, ProtocolServerOptions, ProtocolVersion, RescanReason,
    RescanWorkspaceRequest, WatchBatch, WatchBatchRequest, WatchEvent, WatchEventKind, WatchStatus,
    WorkspaceHandleId, WorkspaceOpenOptions, inline_payload_max_bytes, payload_chunk_bytes,
};
use crate::tests::{TestDaemon, TestProtocolHarness};
use destack_workspace::{CacheValidate, WorkspaceIndexHeader, WorkspaceIndexSnapshot};

/// Performs a handshake and ping roundtrip.
#[test]
fn test_protocol_handshake_ping() {
    // build the protocol harness
    let harness = TestProtocolHarness::new();

    // handshake and ping
    let response = harness.handshake();
    assert_eq!(response.protocol, PROTOCOL_VERSION);

    let response = harness.send_request(DaemonRequest::Ping);
    assert!(matches!(response, DaemonResponse::Pong));

    // request shutdown and join the server
    harness.shutdown();
}

/// Applies a file update over the protocol.
#[test]
fn test_protocol_apply_file_update() {
    // build the protocol harness
    let harness = TestProtocolHarness::new();

    // seed a file on disk
    let file_path = harness.test.write_text("app.ds", "export const value = 1");

    // handshake and open the workspace
    let _ = harness.handshake();
    let handle = harness.open_workspace();

    // apply a file update
    let update = FileUpdate {
        path: file_path.clone(),
        update: FileUpdateKind::Text {
            content: "export const value = 2".to_string(),
        },
        write_to_disk: true,
    };
    let response = harness.send_request(DaemonRequest::ApplyFileUpdate(FileUpdateRequest {
        handle,
        update,
    }));
    let updates = match response {
        DaemonResponse::FileUpdated(response) => response.updates,
        other => panic!("unexpected response: {other:?}"),
    };
    assert!(!updates.is_empty());

    // request shutdown and join the server
    harness.shutdown();
}

/// Rejects incompatible protocol versions.
#[test]
fn test_protocol_handshake_rejects_version() {
    // build the protocol harness
    let harness = TestProtocolHarness::new();

    // request an incompatible protocol range
    let mut options = ProtocolClientOptions::default();
    options.protocol =
        ProtocolRange::new(ProtocolVersion::new(9, 0, 0), ProtocolVersion::new(9, 0, 1));
    let error = harness
        .client
        .handshake(options)
        .expect_err("expected handshake error");

    // assert error code
    match error {
        ProtocolClientError::Server(error) => {
            assert_eq!(error.code, ProtocolErrorCode::UnsupportedVersion);
        }
        other => panic!("unexpected error: {other:?}"),
    }

    harness.shutdown();
}

/// Defers large payloads and reassembles them on the client.
#[test]
fn test_protocol_deferred_payload_roundtrip() {
    // configure small payload limits to force deferral
    let limits = ProtocolLimits::new(4096, 4096, 8, 16);
    let mut server_options = ProtocolServerOptions::default();
    server_options.limits = limits;
    let harness = TestDaemon::new().protocol_with_options(server_options);

    // perform handshake with matching limits
    let mut client_options = ProtocolClientOptions::default();
    client_options.limits = limits;
    let _ = harness.handshake_with(client_options);

    // sanity check that payload chunk sizes fit the protocol limit
    let chunk_size = payload_chunk_bytes(limits);
    let chunk_message = ProtocolMessage::Notification(Box::new(ProtocolNotification {
        payload: DaemonNotification::PayloadChunk(PayloadChunkNotification {
            id: PayloadId::new(1),
            format: PayloadFormat::Postcard,
            index: 0,
            total: 1,
            bytes: vec![0u8; chunk_size],
            done: true,
        }),
    }));
    let encoded = postcard::to_allocvec(&chunk_message).expect("chunk encode");
    assert!(
        encoded.len() <= limits.max_payload_bytes as usize,
        "chunk message too large: {size} bytes",
        size = encoded.len()
    );

    // open the workspace before seeding modules
    let handle = harness.open_workspace();

    // seed the workspace with many modules
    for index in 0..128 {
        let path = harness.test.root.join(format!(
            "src/very_long_file_name_{index}_to_expand_workspace_index.ds"
        ));
        let contents = format!("export const value = {index}");
        let _ = harness
            .test
            .daemon
            .update_file(&path, contents)
            .expect("update file");
    }

    // build and apply a workspace index snapshot
    let program = harness
        .test
        .session
        .get_program(&harness.test.root)
        .expect("program");
    let header = WorkspaceIndexHeader::new(
        "test".to_string(),
        harness.test.root.clone(),
        None,
        0,
        0,
        CacheValidate::Strict,
    );
    let snapshot = WorkspaceIndexSnapshot::from_program(program.as_ref(), header)
        .expect("workspace index snapshot");
    let inline_limit = inline_payload_max_bytes(limits);
    let snapshot_bytes = postcard::to_allocvec(&snapshot).expect("snapshot bytes");
    assert!(
        snapshot_bytes.len() > inline_limit,
        "expected workspace index to exceed inline limit, got {len} bytes",
        len = snapshot_bytes.len()
    );
    program.apply_workspace_index(snapshot);

    // request the workspace index
    let response =
        harness.send_request(DaemonRequest::Query(DaemonQuery::WorkspaceIndex { handle }));
    let payload = match response {
        DaemonResponse::QueryResult(DaemonQueryResponse::WorkspaceIndex(payload)) => payload,
        other => panic!("unexpected response: {other:?}"),
    };

    // assert the payload was reconstructed from deferred chunks
    let PayloadBody::Inline { bytes } = payload.body else {
        panic!("expected inline payload after streaming");
    };
    assert!(
        bytes.len() > inline_limit,
        "expected deferred payload, got {len} bytes",
        len = bytes.len()
    );

    harness.shutdown();
}

/// Rejects requests before handshake completes.
#[test]
fn test_protocol_requires_handshake() {
    // build the protocol harness
    let harness = TestProtocolHarness::new();

    let request = OpenWorkspaceRequest {
        root: harness.test.root.clone(),
        options: WorkspaceOpenOptions::default(),
    };
    let response = harness.send_request(DaemonRequest::OpenWorkspace(request));
    match response {
        DaemonResponse::Error(error) => {
            assert_eq!(error.code, ProtocolErrorCode::NotReady);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    harness.shutdown();
}

/// Handles missing workspace handles gracefully.
#[test]
fn test_protocol_unknown_workspace_handle() {
    // build the protocol harness
    let harness = TestProtocolHarness::new();
    let _ = harness.handshake();

    let response = harness.send_request(DaemonRequest::RescanWorkspace(RescanWorkspaceRequest {
        handle: WorkspaceHandleId::new(999),
        reason: RescanReason::Manual,
    }));
    match response {
        DaemonResponse::Error(error) => {
            assert_eq!(error.code, ProtocolErrorCode::NotFound);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    harness.shutdown();
}

/// Rejects updates outside the workspace root.
#[test]
fn test_protocol_rejects_update_outside_root() {
    // build the protocol harness
    let harness = TestProtocolHarness::new();
    let _ = harness.handshake();
    let handle = harness.open_workspace();

    let update = FileUpdate {
        path: PathBuf::from("/other/file.ds"),
        update: FileUpdateKind::Text {
            content: "export const value = 1".to_string(),
        },
        write_to_disk: false,
    };
    let response = harness.send_request(DaemonRequest::ApplyFileUpdate(FileUpdateRequest {
        handle,
        update,
    }));
    match response {
        DaemonResponse::Error(error) => {
            assert_eq!(error.code, ProtocolErrorCode::Forbidden);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    harness.shutdown();
}

/// Applies binary, touch, and remove updates.
#[test]
fn test_protocol_file_update_variants() {
    // build the protocol harness
    let harness = TestProtocolHarness::new();

    // seed a binary file
    let binary_path = harness.test.write_text("data.dsb", "payload");

    // handshake and open the workspace
    let _ = harness.handshake();
    let handle = harness.open_workspace();

    // apply a binary update
    let update = FileUpdate {
        path: binary_path.clone(),
        update: FileUpdateKind::Bytes {
            content: vec![1, 2, 3],
        },
        write_to_disk: true,
    };
    let response = harness.send_request(DaemonRequest::ApplyFileUpdate(FileUpdateRequest {
        handle,
        update,
    }));
    assert!(matches!(response, DaemonResponse::FileUpdated(_)));

    // apply a touch update
    let touch = FileUpdate {
        path: binary_path.clone(),
        update: FileUpdateKind::Touch,
        write_to_disk: false,
    };
    let response = harness.send_request(DaemonRequest::ApplyFileUpdate(FileUpdateRequest {
        handle,
        update: touch,
    }));
    assert!(matches!(response, DaemonResponse::FileUpdated(_)));

    // remove the file on disk
    let remove = FileUpdate {
        path: binary_path,
        update: FileUpdateKind::Removed,
        write_to_disk: true,
    };
    let response = harness.send_request(DaemonRequest::ApplyFileUpdate(FileUpdateRequest {
        handle,
        update: remove,
    }));
    assert!(matches!(response, DaemonResponse::FileUpdated(_)));

    harness.shutdown();
}

/// Applies virtual file updates over the protocol.
#[test]
fn test_protocol_virtual_update_emits_diagnostics() {
    // build the protocol harness
    let harness = TestProtocolHarness::new();

    // handshake and open the workspace
    let _ = harness.handshake();
    let handle = harness.open_workspace();

    let path = harness.test.root.join("virtual.ds");

    // apply a valid virtual update
    let valid = FileUpdate {
        path: path.clone(),
        update: FileUpdateKind::Text {
            content: "export const value = 1".to_string(),
        },
        write_to_disk: false,
    };
    let response = harness.send_request(DaemonRequest::ApplyFileUpdate(FileUpdateRequest {
        handle,
        update: valid,
    }));
    let updates = match response {
        DaemonResponse::FileUpdated(response) => response.updates,
        other => panic!("unexpected response: {other:?}"),
    };
    assert!(
        updates.iter().all(|update| update.diagnostics.is_empty()),
        "expected no diagnostics for valid virtual content"
    );

    // apply an invalid virtual update
    let invalid = FileUpdate {
        path,
        update: FileUpdateKind::Text {
            content: "export const value = ;".to_string(),
        },
        write_to_disk: false,
    };
    let response = harness.send_request(DaemonRequest::ApplyFileUpdate(FileUpdateRequest {
        handle,
        update: invalid,
    }));
    let updates = match response {
        DaemonResponse::FileUpdated(response) => response.updates,
        other => panic!("unexpected response: {other:?}"),
    };
    assert!(
        updates.iter().any(|update| !update.diagnostics.is_empty()),
        "expected diagnostics for invalid virtual content"
    );

    harness.shutdown();
}

/// Applies a watch batch and respects overflow.
#[test]
fn test_protocol_watch_batch_roundtrip() {
    // build the protocol harness
    let harness = TestProtocolHarness::new();
    let _ = harness.handshake();
    let handle = harness.open_workspace();

    let batch = WatchBatch {
        events: vec![WatchEvent {
            path: harness.test.root.join("app.ds"),
            previous_path: None,
            kind: WatchEventKind::Modified,
        }],
        status: vec![WatchStatus::RescanRequested {
            roots: vec![harness.test.root.clone()],
            reason: RescanReason::Manual,
        }],
        overflowed: true,
        started_at_ns: 10,
        ended_at_ns: 20,
    };
    let response = harness.send_request(DaemonRequest::ApplyWatchBatch(WatchBatchRequest {
        handle,
        batch,
    }));
    match response {
        DaemonResponse::WatchBatchApplied(response) => {
            assert!(response.rescan);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    harness.shutdown();
}

/// Queries diagnostics after an invalid update.
#[test]
fn test_protocol_query_diagnostics() {
    // build the protocol harness
    let harness = TestProtocolHarness::new();

    // seed an invalid source file
    let file_path = harness.test.write_text("bad.ds", "function {");

    // handshake and open the workspace
    let _ = harness.handshake();
    let handle = harness.open_workspace();

    // apply an invalid update
    let update = FileUpdate {
        path: file_path,
        update: FileUpdateKind::Text {
            content: "function {".to_string(),
        },
        write_to_disk: true,
    };
    let _ = harness.send_request(DaemonRequest::ApplyFileUpdate(FileUpdateRequest {
        handle,
        update,
    }));

    let response = harness.send_request(DaemonRequest::Query(DaemonQuery::Diagnostics { handle }));
    match response {
        DaemonResponse::QueryResult(DaemonQueryResponse::Diagnostics(batches)) => {
            assert!(!batches.is_empty());
        }
        other => panic!("unexpected response: {other:?}"),
    }

    harness.shutdown();
}

/// Queries cache stats after handshake.
#[test]
fn test_protocol_query_cache_stats() {
    // build the protocol harness
    let harness = TestProtocolHarness::new();
    let _ = harness.handshake();
    let handle = harness.open_workspace();

    let response = harness.send_request(DaemonRequest::Query(DaemonQuery::CacheStats { handle }));
    match response {
        DaemonResponse::QueryResult(DaemonQueryResponse::CacheStats(stats)) => {
            assert_cache_stats(stats);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    harness.shutdown();
}

/// Ensures repeated shutdowns do not panic.
#[test]
fn test_protocol_shutdown_idempotent() {
    // build the protocol harness
    let harness = TestProtocolHarness::new();
    let _ = harness.handshake();

    let response = harness.send_request(DaemonRequest::Shutdown);
    assert!(matches!(response, DaemonResponse::ShutdownAck));

    let response = harness.client.send_request(DaemonRequest::Shutdown);
    match response {
        Ok(response) => {
            assert!(matches!(
                response,
                DaemonResponse::ShutdownAck | DaemonResponse::Error(_)
            ));
        }
        Err(ProtocolClientError::Codec(_) | ProtocolClientError::Transport(_)) => {}
        Err(other) => panic!("unexpected error: {other:?}"),
    }

    harness.join();
}

fn assert_cache_stats(stats: CacheStatsPayload) {
    // assert counts are well formed
    assert!(stats.hits >= stats.disk_reads);
}
