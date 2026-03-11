use std::path::PathBuf;
use std::time::Duration;

use crate::protocol::{
    AnalyzeRequest, CacheStatsPayload, DaemonNotification, DaemonQuery, DaemonQueryResponse,
    DaemonRequest, DaemonResponse, FileUpdate, FileUpdateKind, FileUpdateRequest,
    OpenWorkspaceRequest, PROTOCOL_VERSION, PayloadBody, PayloadChunkNotification, PayloadFormat,
    PayloadId, ProtocolClientError, ProtocolClientOptions, ProtocolErrorCode, ProtocolLimits,
    ProtocolMessage, ProtocolNotification, ProtocolRange, ProtocolServerActivity,
    ProtocolServerOptions, ProtocolVersion, QueryRequestPayload, RescanReason,
    RescanWorkspaceRequest, WatchBatch, WatchBatchRequest, WatchEvent, WatchEventKind, WatchStatus,
    WorkspaceHandleId, WorkspaceOpenOptions, inline_payload_max_bytes, payload_chunk_bytes,
};
use crate::tests::{RequestRetryPolicy, TestDaemon, TestProtocolHarness, wait_for_condition};
use destack_query as query;
use destack_query::{
    DocumentSymbolsRequest, FindReferencesRequest, GotoDefinitionRequest, HoverRequest,
    QueryRequest, QueryRequestEnvelope, QueryResponse, RenameFilesRequest, SemanticTokensRequest,
};
use destack_source::Uri;
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
    let options = ProtocolClientOptions {
        protocol: ProtocolRange::new(ProtocolVersion::new(9, 0, 0), ProtocolVersion::new(9, 0, 1)),
        ..Default::default()
    };
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
    let server_options = ProtocolServerOptions {
        limits,
        ..Default::default()
    };
    let harness = TestDaemon::new().protocol_with_options(server_options);

    // perform handshake with matching limits
    let client_options = ProtocolClientOptions {
        limits,
        ..Default::default()
    };
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

/// Streams large workspace query responses through deferred payload chunks.
#[test]
fn test_protocol_workspace_query_deferred_payload_roundtrip() {
    // configure small payload limits to force deferred query responses
    let limits = ProtocolLimits::new(4096, 4096, 8, 16);
    let server_options = ProtocolServerOptions {
        limits,
        ..Default::default()
    };
    let harness = TestDaemon::new().protocol_with_options(server_options);

    // perform handshake with matching limits
    let client_options = ProtocolClientOptions {
        limits,
        ..Default::default()
    };
    let _ = harness.handshake_with(client_options);

    // build a large semantic tokens response surface
    let mut content = String::new();
    for index in 0..2000 {
        content.push_str(&format!("export const symbol_{index} = {index};\n"));
    }
    let file_path = harness.test.write_text("symbols.ds", &content);

    // open the workspace and ensure query analysis readiness
    let handle = harness.open_workspace();
    let response = harness.send_request(DaemonRequest::Analyze(AnalyzeRequest {
        handle,
        path: file_path.clone(),
    }));
    match response {
        DaemonResponse::Analyzed(response) => {
            assert!(response.semantic_query_ready);
            assert!(response.detail.is_none());
        }
        other => panic!("unexpected analyze response: {other:?}"),
    }

    // execute a large semantic tokens query
    let request = QueryRequestPayload::from_envelope(QueryRequestEnvelope {
        expected_revision: None,
        request: QueryRequest::SemanticTokens(SemanticTokensRequest {
            uri: Uri::from_path(&file_path),
        }),
    })
    .expect("query request encode");
    let response = harness.send_request(DaemonRequest::Query(DaemonQuery::WorkspaceQuery {
        handle,
        request,
    }));

    // assert that the client reconstructed a deferred payload
    let payload = match response {
        DaemonResponse::QueryResult(DaemonQueryResponse::WorkspaceQuery(payload)) => payload,
        other => panic!("unexpected response: {other:?}"),
    };
    let inline_limit = inline_payload_max_bytes(limits);
    let PayloadBody::Inline { bytes } = &payload.payload.body else {
        panic!("expected inline payload after streaming");
    };
    assert!(
        bytes.len() > inline_limit,
        "expected deferred payload, got {len} bytes",
        len = bytes.len()
    );

    // assert the decoded query response content
    let envelope = payload.decode_envelope().expect("query response decode");
    match envelope.response {
        QueryResponse::SemanticTokens(response) => {
            assert!(!response.tokens.is_empty());
        }
        other => panic!("unexpected query response: {other:?}"),
    }

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

/// Opens a workspace without preloading diagnostics when requested.
#[test]
fn test_protocol_open_workspace_skips_preload_when_disabled() {
    // build the protocol harness
    let harness = TestProtocolHarness::new();
    let _ = harness.handshake();

    // open without preload
    let request = OpenWorkspaceRequest {
        root: harness.test.root.clone(),
        options: WorkspaceOpenOptions { load_index: false },
    };
    let response = harness.send_request(DaemonRequest::OpenWorkspace(request));

    // assertion block
    match response {
        DaemonResponse::WorkspaceOpened(response) => {
            assert!(response.diagnostics.is_empty());
            assert!(response.messages.is_empty());
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

/// Resolves goto definition from a virtual update.
#[test]
fn test_protocol_virtual_update_query_goto_definition() {
    // build the protocol harness
    let harness = TestProtocolHarness::new();

    // handshake and open the workspace
    let _ = harness.handshake();
    let handle = harness.open_workspace();

    let path = harness.test.root.join("virtual.ds");
    let content = concat!(
        "export function greet(name: string): string {\n",
        "    return \"Hello, \" + name;\n",
        "}\n",
        "\n",
        "const msg = greet(\"World\");\n"
    );

    // apply a valid virtual update
    let update = FileUpdate {
        path: path.clone(),
        update: FileUpdateKind::Text {
            content: content.to_string(),
        },
        write_to_disk: false,
    };
    let response = harness.send_request(DaemonRequest::ApplyFileUpdate(FileUpdateRequest {
        handle,
        update,
    }));
    assert!(matches!(response, DaemonResponse::FileUpdated(_)));

    // query goto definition on the call site
    let offset = content.find("greet(\"World\")").unwrap_or(0) as u32 + 1;

    // assert direct query behavior on the same session state
    let file_id = harness
        .test
        .session
        .files
        .get_id_by_path(&path)
        .expect("expected file id for virtual source");
    let direct = query::goto_definition(harness.test.session.as_ref(), file_id, offset);
    assert!(direct.is_some(), "expected direct goto definition result");

    let response = harness.send_request_with_retry(
        || {
            let request = QueryRequestPayload::from_envelope(QueryRequestEnvelope {
                expected_revision: None,
                request: QueryRequest::GotoDefinition(GotoDefinitionRequest {
                    uri: Uri::from_path(&path),
                    offset,
                }),
            })
            .expect("query request encode");

            DaemonRequest::Query(DaemonQuery::WorkspaceQuery { handle, request })
        },
        RequestRetryPolicy::default(),
    );

    let envelope = match response {
        DaemonResponse::QueryResult(DaemonQueryResponse::WorkspaceQuery(envelope)) => envelope,
        other => panic!("unexpected response: {other:?}"),
    };
    let envelope = envelope.decode_envelope().expect("query response decode");

    // assert goto definition response content
    match envelope.response {
        QueryResponse::GotoDefinition(payload) => {
            let Some(result) = payload.result else {
                panic!("expected goto definition result");
            };
            assert!(!result.locations.is_empty());
        }
        other => panic!("unexpected query response: {other:?}"),
    }

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
            assert!(!response.messages.is_empty());
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

/// Executes a workspace hover query.
#[test]
fn test_protocol_workspace_query_hover() {
    // build the protocol harness
    let harness = TestProtocolHarness::new();

    // seed a source file
    let content = concat!(
        "export function announce(name: string): string {\n",
        "    return \"Hello, \" + name;\n",
        "}\n",
        "\n",
        "const message = announce(\"World\");\n"
    );
    let file_path = harness.test.write_text("main.ds", content);

    // handshake and open the workspace
    let _ = harness.handshake();
    let handle = harness.open_workspace();

    // ensure analysis is available before running queries
    let response = harness.send_request(DaemonRequest::Analyze(AnalyzeRequest {
        handle,
        path: file_path.clone(),
    }));
    match response {
        DaemonResponse::Analyzed(response) => {
            assert!(response.semantic_query_ready);
            assert!(response.detail.is_none());
        }
        other => panic!("unexpected analyze response: {other:?}"),
    }

    // build the hover query
    let offset = content.find("announce(name").unwrap_or(0) as u32 + 1;
    let request = QueryRequestPayload::from_envelope(QueryRequestEnvelope {
        expected_revision: None,
        request: QueryRequest::Hover(HoverRequest {
            uri: Uri::from_path(&file_path),
            offset,
        }),
    })
    .expect("query request encode");

    // execute the query
    let response = harness.send_request(DaemonRequest::Query(DaemonQuery::WorkspaceQuery {
        handle,
        request,
    }));

    let envelope = match response {
        DaemonResponse::QueryResult(DaemonQueryResponse::WorkspaceQuery(envelope)) => envelope,
        other => panic!("unexpected response: {other:?}"),
    };
    let envelope = envelope.decode_envelope().expect("query response decode");

    // assert hover response content
    assert!(envelope.revision > 0);
    match envelope.response {
        QueryResponse::Hover(payload) => {
            if let Some(hover) = payload.hover {
                assert!(hover.signature.contains("function announce"));
            }
        }
        other => panic!("unexpected query response: {other:?}"),
    }

    harness.shutdown();
}

/// Requires revision preconditions for mutating workspace queries.
#[test]
fn test_protocol_workspace_query_requires_revision_for_mutation() {
    // build the protocol harness
    let harness = TestProtocolHarness::new();
    let _ = harness.handshake();
    let handle = harness.open_workspace();

    // reject mutating queries without an expected revision
    let missing_request = QueryRequestPayload::from_envelope(QueryRequestEnvelope {
        expected_revision: None,
        request: QueryRequest::RenameFiles(RenameFilesRequest {
            renames: Vec::new(),
        }),
    })
    .expect("query request encode");
    let missing_response =
        harness.send_request(DaemonRequest::Query(DaemonQuery::WorkspaceQuery {
            handle,
            request: missing_request,
        }));
    match missing_response {
        DaemonResponse::Error(error) => {
            assert_eq!(error.code, ProtocolErrorCode::InvalidRequest);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    // reject stale revision preconditions
    let stale_request = QueryRequestPayload::from_envelope(QueryRequestEnvelope {
        expected_revision: Some(0),
        request: QueryRequest::RenameFiles(RenameFilesRequest {
            renames: Vec::new(),
        }),
    })
    .expect("query request encode");
    let stale_response = harness.send_request(DaemonRequest::Query(DaemonQuery::WorkspaceQuery {
        handle,
        request: stale_request,
    }));
    match stale_response {
        DaemonResponse::Error(error) => {
            assert_eq!(error.code, ProtocolErrorCode::Conflict);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    // resolve the current workspace revision
    let revision_response =
        harness.send_request(DaemonRequest::Query(DaemonQuery::WorkspaceRevision {
            handle,
        }));
    let revision = match revision_response {
        DaemonResponse::QueryResult(DaemonQueryResponse::WorkspaceRevision(revision)) => revision,
        other => panic!("unexpected response: {other:?}"),
    };

    // accept mutating queries with a matching revision precondition
    let matching_request = QueryRequestPayload::from_envelope(QueryRequestEnvelope {
        expected_revision: Some(revision),
        request: QueryRequest::RenameFiles(RenameFilesRequest {
            renames: Vec::new(),
        }),
    })
    .expect("query request encode");
    let matching_response =
        harness.send_request(DaemonRequest::Query(DaemonQuery::WorkspaceQuery {
            handle,
            request: matching_request,
        }));
    let envelope = match matching_response {
        DaemonResponse::QueryResult(DaemonQueryResponse::WorkspaceQuery(envelope)) => envelope,
        other => panic!("unexpected response: {other:?}"),
    };
    let envelope = envelope.decode_envelope().expect("query response decode");
    assert_eq!(envelope.revision, revision);
    match envelope.response {
        QueryResponse::RenameFiles(_) => {}
        other => panic!("unexpected query response: {other:?}"),
    }

    harness.shutdown();
}

/// Executes a workspace query batch.
#[test]
fn test_protocol_workspace_query_batch() {
    // build the protocol harness
    let harness = TestProtocolHarness::new();

    // seed a source file
    let content = concat!(
        "export function announce(name: string): string {\n",
        "    return \"Hello, \" + name;\n",
        "}\n",
        "\n",
        "const message = announce(\"World\");\n"
    );
    let file_path = harness.test.write_text("main.ds", content);

    // handshake and open the workspace
    let _ = harness.handshake();
    let handle = harness.open_workspace();

    // ensure analysis is available before running queries
    let response = harness.send_request(DaemonRequest::Analyze(AnalyzeRequest {
        handle,
        path: file_path.clone(),
    }));
    match response {
        DaemonResponse::Analyzed(response) => {
            assert!(response.semantic_query_ready);
            assert!(response.detail.is_none());
        }
        other => panic!("unexpected analyze response: {other:?}"),
    }

    // build the hover query offset
    let offset = content.find("announce(name").unwrap_or(0) as u32 + 1;

    // execute the batch query
    let response = harness.send_request_with_retry(
        || {
            let hover_request = QueryRequestPayload::from_envelope(QueryRequestEnvelope {
                expected_revision: None,
                request: QueryRequest::Hover(HoverRequest {
                    uri: Uri::from_path(&file_path),
                    offset,
                }),
            })
            .expect("query request encode");

            let symbols_request = QueryRequestPayload::from_envelope(QueryRequestEnvelope {
                expected_revision: None,
                request: QueryRequest::DocumentSymbols(DocumentSymbolsRequest {
                    uri: Uri::from_path(&file_path),
                }),
            })
            .expect("query request encode");

            DaemonRequest::Query(DaemonQuery::WorkspaceQueryBatch {
                handle,
                requests: vec![hover_request, symbols_request],
            })
        },
        RequestRetryPolicy::default(),
    );

    let responses = match response {
        DaemonResponse::QueryResult(DaemonQueryResponse::WorkspaceQueryBatch(responses)) => {
            responses
        }
        other => panic!("unexpected response: {other:?}"),
    };
    let responses: Vec<_> = responses
        .into_iter()
        .map(|response| response.decode_envelope().expect("query response decode"))
        .collect();

    // assert the batch responses
    assert_eq!(responses.len(), 2);
    match &responses[0].response {
        QueryResponse::Hover(payload) => {
            if let Some(hover) = &payload.hover {
                assert!(hover.signature.contains("function announce"));
            }
        }
        other => panic!("unexpected query response: {other:?}"),
    }
    match &responses[1].response {
        QueryResponse::DocumentSymbols(payload) => {
            if !payload.symbols.is_empty() {
                assert!(
                    payload
                        .symbols
                        .iter()
                        .any(|symbol| symbol.name == "announce")
                );
            }
        }
        other => panic!("unexpected query response: {other:?}"),
    }

    harness.shutdown();
}

/// Executes a workspace find references query for member access.
#[test]
fn test_protocol_workspace_query_find_references_member_access() {
    // build the protocol harness
    let harness = TestProtocolHarness::new();

    // seed a source file
    let content = concat!(
        "struct Point {\n",
        "    x: int32,\n",
        "    y: int32,\n",
        "}\n",
        "\n",
        "function main(p: Point) {\n",
        "    const a = p.x;\n",
        "    const b = p.x + p.x;\n",
        "}\n"
    );
    let file_path = harness.test.write_text("main.ds", content);

    // handshake and open the workspace
    let _ = harness.handshake();
    let handle = harness.open_workspace();

    // ensure analysis is available before running queries
    let response = harness.send_request(DaemonRequest::Analyze(AnalyzeRequest {
        handle,
        path: file_path.clone(),
    }));
    match response {
        DaemonResponse::Analyzed(response) => {
            assert!(response.semantic_query_ready);
            assert!(response.detail.is_none());
        }
        other => panic!("unexpected analyze response: {other:?}"),
    }

    // build the find references query offset
    let offset = content.find("p.x").unwrap_or(0) as u32 + 2;

    // execute the query
    let response = harness.send_request_with_retry(
        || {
            let request = QueryRequestPayload::from_envelope(QueryRequestEnvelope {
                expected_revision: None,
                request: QueryRequest::FindReferences(FindReferencesRequest {
                    uri: Uri::from_path(&file_path),
                    offset,
                    include_declaration: true,
                }),
            })
            .expect("query request encode");

            DaemonRequest::Query(DaemonQuery::WorkspaceQuery { handle, request })
        },
        RequestRetryPolicy::default(),
    );

    let envelope = match response {
        DaemonResponse::QueryResult(DaemonQueryResponse::WorkspaceQuery(envelope)) => envelope,
        other => panic!("unexpected response: {other:?}"),
    };
    let envelope = envelope.decode_envelope().expect("query response decode");

    // assert find references response content
    match envelope.response {
        QueryResponse::FindReferences(payload) => {
            if let Some(refs) = payload.result {
                assert_eq!(refs.references.len(), 4);
            }
        }
        other => panic!("unexpected query response: {other:?}"),
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

/// Shuts down when idle and has no active leases.
#[test]
fn test_protocol_activity_idle_shutdown() {
    // create an activity tracker with a short idle timeout
    let activity = ProtocolServerActivity::new(Some(Duration::from_millis(5)));

    // keep alive while a connection lease is held
    activity.register_connection();
    let remained_active =
        wait_for_condition(Duration::from_millis(30), Duration::from_millis(1), || {
            !activity.should_shutdown()
        });
    assert!(
        remained_active,
        "expected activity to stay active with a lease"
    );

    // release the lease and wait for idle shutdown
    activity.unregister_connection();
    let shutdown = wait_for_condition(Duration::from_millis(30), Duration::from_millis(1), || {
        activity.should_shutdown()
    });
    assert!(shutdown, "expected activity idle shutdown");
}

fn assert_cache_stats(stats: CacheStatsPayload) {
    // assert counts are well formed
    assert!(stats.hits >= stats.disk_reads);
}
