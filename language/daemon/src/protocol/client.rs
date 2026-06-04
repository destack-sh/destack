use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

use super::{
    ClientDescriptor, CloseRootRequest, DaemonQuery, DaemonQueryResponse, DaemonRequest,
    DaemonResponse, DiagnosticBatch, DiagnosticSnapshot, FileImagesRequest, FileSnapshot,
    FileSnapshotRequest, FileUpdate, FileUpdateImage, FileUpdateRequest, FileUpdateResponse,
    HandshakeRequest, HandshakeResponse, OpenRootRequest, PayloadReceiver, ProtocolClientError,
    ProtocolCodec, ProtocolLimits, ProtocolMessage, ProtocolRange, ProtocolRequest,
    QueryRequestBody, QueryRequestPayload, QueryResponseBody, ReloadReason, ReloadRootRequest,
    RepositoryId, RequestId, RequestOptions, RootClosedResponse, RootHandleId, RootOpenOptions,
    RootOpenedResponse, RootReloadResponse, RootSnapshot, SourceUpdate, SourceUpdateRequest,
    SourceUpdateResponse, Transport, WatchBatchResponse, WatchNextRequest, WatchStartOptions,
    WatchStartRequest, WatchStartedResponse, WatchStopRequest, WatchStoppedResponse,
};
use destack_workspace::Revision;

/// Client configuration for the daemon protocol.
#[derive(Debug, Clone)]
pub struct ProtocolClientOptions {
    /// Supported protocol range for the client.
    pub protocol: ProtocolRange,
    /// Client requested protocol limits.
    pub limits: ProtocolLimits,
    /// Client descriptor for the handshake request.
    pub client: ClientDescriptor,
}

impl Default for ProtocolClientOptions {
    fn default() -> Self {
        Self {
            protocol: ProtocolRange::new(super::MIN_PROTOCOL_VERSION, super::PROTOCOL_VERSION),
            limits: ProtocolLimits::default(),
            client: ClientDescriptor {
                name: "destack-client".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                build: None,
                pid: Some(std::process::id()),
            },
        }
    }
}

/// Protocol client for sending requests to a daemon.
pub struct ProtocolClient {
    /// Transport used to send and receive messages.
    transport: Arc<dyn Transport>,
    /// Codec used for protocol payloads.
    codec: Mutex<ProtocolCodec>,
    /// Serialized request response cycles.
    requests: Mutex<()>,
    /// Negotiated session id.
    session_id: Mutex<Option<RepositoryId>>,
    /// Request id counter.
    next_request_id: AtomicU64,
}

impl std::fmt::Debug for ProtocolClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProtocolClient")
            .field("session_id", &self.session_id())
            .finish()
    }
}

impl ProtocolClient {
    /// Create a new protocol client for a transport.
    pub fn new(transport: Arc<dyn Transport>) -> Self {
        Self::with_codec(transport, ProtocolCodec::default())
    }

    /// Create a new protocol client with a custom codec.
    pub fn with_codec(transport: Arc<dyn Transport>, codec: ProtocolCodec) -> Self {
        Self {
            transport,
            codec: Mutex::new(codec),
            requests: Mutex::new(()),
            session_id: Mutex::new(None),
            next_request_id: AtomicU64::new(1),
        }
    }

    /// Return the negotiated session id, if any.
    pub fn session_id(&self) -> Option<RepositoryId> {
        *self.session_id.lock()
    }

    /// Perform a handshake with the daemon.
    pub fn handshake(
        &self,
        options: ProtocolClientOptions,
    ) -> Result<HandshakeResponse, ProtocolClientError> {
        let request = HandshakeRequest {
            protocol: options.protocol,
            client: options.client,
            limits: options.limits,
        };

        let response = self.send_request_with_options(
            DaemonRequest::Handshake(request),
            RequestOptions::default(),
        )?;
        let response = match response {
            DaemonResponse::Handshake(response) => response,
            DaemonResponse::Error(error) => {
                return Err(ProtocolClientError::Server(error));
            }
            other => {
                return Err(ProtocolClientError::UnexpectedResponse(format!(
                    "expected handshake response, got {other:?}"
                )));
            }
        };

        // update the codec limit and store the session id
        self.codec.lock().max_payload_bytes = response.limits.max_payload_bytes as usize;
        *self.session_id.lock() = Some(response.session_id);

        Ok(response)
    }

    /// Send a protocol request with default options.
    pub fn send_request(
        &self,
        payload: DaemonRequest,
    ) -> Result<DaemonResponse, ProtocolClientError> {
        self.send_request_with_options(payload, RequestOptions::default())
    }

    /// Send a protocol request with explicit options.
    pub fn send_request_with_options(
        &self,
        payload: DaemonRequest,
        options: RequestOptions,
    ) -> Result<DaemonResponse, ProtocolClientError> {
        // serialize the current non-multiplexed protocol client
        let _request = self.requests.lock();

        // issue the request
        let id = self.next_request_id();
        let request = ProtocolRequest {
            id,
            options,
            payload,
        };
        let message = ProtocolMessage::Request(Box::new(request));
        self.send_message(&message)?;

        // track deferred payloads while waiting for a response
        let mut payloads = PayloadReceiver::new();
        let mut pending_response: Option<DaemonResponse> = None;

        // receive messages until the response is fully resolved
        loop {
            let message = self.recv_message()?;
            match message {
                ProtocolMessage::Response(response) => {
                    let response = *response;
                    if response.id == id {
                        pending_response = Some(response.payload);
                    }
                }
                ProtocolMessage::Notification(notification) => {
                    payloads.ingest_notification(*notification)?;
                }
                ProtocolMessage::Request(_) => {
                    return Err(ProtocolClientError::UnexpectedResponse(
                        "client received request".to_string(),
                    ));
                }
            }

            // resolve deferred payloads when the response is ready
            let Some(mut response) = pending_response.take() else {
                continue;
            };
            let resolved = payloads.resolve_response(&mut response)?;
            if resolved {
                return Ok(response);
            }
            pending_response = Some(response);
        }
    }

    /// Open a daemon root handle.
    pub fn open_root(
        &self,
        workspace: PathBuf,
        root: PathBuf,
        options: RootOpenOptions,
    ) -> Result<RootOpenedResponse, ProtocolClientError> {
        // send the open root request
        let request = OpenRootRequest {
            workspace,
            root,
            options,
        };
        let response = self.send_request(DaemonRequest::OpenRoot(request))?;

        // decode the root response
        match response {
            DaemonResponse::RootOpened(response) => Ok(response),
            DaemonResponse::Error(error) => Err(ProtocolClientError::Server(error)),
            other => Err(Self::unexpected_response("root opened", other)),
        }
    }

    /// Close a daemon root handle.
    pub fn close_root(
        &self,
        handle: RootHandleId,
    ) -> Result<RootClosedResponse, ProtocolClientError> {
        // send the close root request
        let request = CloseRootRequest { handle };
        let response = self.send_request(DaemonRequest::CloseRoot(request))?;

        // decode the close response
        match response {
            DaemonResponse::RootClosed(response) => Ok(response),
            DaemonResponse::Error(error) => Err(ProtocolClientError::Server(error)),
            other => Err(Self::unexpected_response("root closed", other)),
        }
    }

    /// Reload a daemon root handle.
    pub fn reload_root(
        &self,
        handle: RootHandleId,
        reason: ReloadReason,
    ) -> Result<RootReloadResponse, ProtocolClientError> {
        // send the reload request
        let request = ReloadRootRequest { handle, reason };
        let response = self.send_request(DaemonRequest::ReloadRoot(request))?;

        // decode the reload response
        match response {
            DaemonResponse::RootReloaded(response) => Ok(response),
            DaemonResponse::Error(error) => Err(ProtocolClientError::Server(error)),
            other => Err(Self::unexpected_response("root reloaded", other)),
        }
    }

    /// Apply a file update to a daemon root handle.
    pub fn apply_file_update(
        &self,
        handle: RootHandleId,
        update: FileUpdate,
    ) -> Result<FileUpdateResponse, ProtocolClientError> {
        // send the file update request
        let request = FileUpdateRequest { handle, update };
        let response = self.send_request(DaemonRequest::ApplyFileUpdate(request))?;

        // decode the file update response
        match response {
            DaemonResponse::FileUpdated(response) => Ok(response),
            DaemonResponse::Error(error) => Err(ProtocolClientError::Server(error)),
            other => Err(Self::unexpected_response("file update applied", other)),
        }
    }

    /// Apply a source update to a daemon root handle.
    pub fn apply_source_update(
        &self,
        handle: RootHandleId,
        update: SourceUpdate,
    ) -> Result<SourceUpdateResponse, ProtocolClientError> {
        // send the source update request
        let request = SourceUpdateRequest { handle, update };
        let response = self.send_request(DaemonRequest::ApplySourceUpdate(request))?;

        // decode the source update response
        match response {
            DaemonResponse::SourceUpdated(response) => Ok(response),
            DaemonResponse::Error(error) => Err(ProtocolClientError::Server(error)),
            other => Err(Self::unexpected_response("source update applied", other)),
        }
    }

    /// Start watching a daemon root handle.
    pub fn start_watch(
        &self,
        handle: RootHandleId,
        roots: Vec<PathBuf>,
        options: WatchStartOptions,
    ) -> Result<WatchStartedResponse, ProtocolClientError> {
        // send the watch start request
        let request = WatchStartRequest {
            handle,
            roots,
            options,
        };
        let response = self.send_request(DaemonRequest::StartWatch(request))?;

        // decode the watch start response
        match response {
            DaemonResponse::WatchStarted(response) => Ok(response),
            DaemonResponse::Error(error) => Err(ProtocolClientError::Server(error)),
            other => Err(Self::unexpected_response("watch started", other)),
        }
    }

    /// Receive and apply the next watch batch for a daemon root handle.
    pub fn next_watch_batch(
        &self,
        handle: RootHandleId,
    ) -> Result<WatchBatchResponse, ProtocolClientError> {
        // send the watch next request
        let request = WatchNextRequest { handle };
        let response = self.send_request(DaemonRequest::NextWatchBatch(request))?;

        // decode the watch batch response
        match response {
            DaemonResponse::WatchBatchReady(response) => Ok(response),
            DaemonResponse::Error(error) => Err(ProtocolClientError::Server(error)),
            other => Err(Self::unexpected_response("watch batch ready", other)),
        }
    }

    /// Stop watching a daemon root handle.
    pub fn stop_watch(
        &self,
        handle: RootHandleId,
    ) -> Result<WatchStoppedResponse, ProtocolClientError> {
        // send the watch stop request
        let request = WatchStopRequest { handle };
        let response = self.send_request(DaemonRequest::StopWatch(request))?;

        // decode the watch stop response
        match response {
            DaemonResponse::WatchStopped(response) => Ok(response),
            DaemonResponse::Error(error) => Err(ProtocolClientError::Server(error)),
            other => Err(Self::unexpected_response("watch stopped", other)),
        }
    }

    /// Request diagnostics for a daemon root handle.
    pub fn diagnostics(
        &self,
        handle: RootHandleId,
    ) -> Result<Vec<DiagnosticBatch>, ProtocolClientError> {
        // send the diagnostics query
        let response =
            self.send_request(DaemonRequest::Query(DaemonQuery::Diagnostics { handle }))?;

        // decode the diagnostics response
        match response {
            DaemonResponse::QueryResult(DaemonQueryResponse::Diagnostics(response)) => Ok(response),
            DaemonResponse::Error(error) => Err(ProtocolClientError::Server(error)),
            other => Err(Self::unexpected_response("diagnostics query", other)),
        }
    }

    /// Request rich diagnostics for a daemon root handle.
    pub fn diagnostic_snapshots(
        &self,
        handle: RootHandleId,
    ) -> Result<Vec<DiagnosticSnapshot>, ProtocolClientError> {
        // send the diagnostics query
        let response =
            self.send_request(DaemonRequest::Query(DaemonQuery::DiagnosticSnapshots {
                handle,
            }))?;

        // decode the diagnostics response
        match response {
            DaemonResponse::QueryResult(DaemonQueryResponse::DiagnosticSnapshots(response)) => {
                Ok(response)
            }
            DaemonResponse::Error(error) => Err(ProtocolClientError::Server(error)),
            other => Err(Self::unexpected_response(
                "diagnostic snapshots query",
                other,
            )),
        }
    }

    /// Request diagnostics for one file path.
    pub fn file_diagnostics(
        &self,
        handle: RootHandleId,
        path: PathBuf,
    ) -> Result<Option<DiagnosticSnapshot>, ProtocolClientError> {
        // send the file diagnostics query
        let response = self.send_request(DaemonRequest::Query(DaemonQuery::FileDiagnostics {
            handle,
            path,
        }))?;

        // decode the file diagnostics response
        match response {
            DaemonResponse::QueryResult(DaemonQueryResponse::FileDiagnostics(response)) => {
                Ok(response)
            }
            DaemonResponse::Error(error) => Err(ProtocolClientError::Server(error)),
            other => Err(Self::unexpected_response("file diagnostics query", other)),
        }
    }

    /// Request the current semantic revision for a daemon root handle.
    pub fn current_revision(&self, handle: RootHandleId) -> Result<Revision, ProtocolClientError> {
        // send the revision query
        let response = self.send_request(DaemonRequest::Query(DaemonQuery::CurrentRevision {
            handle,
        }))?;

        // decode the revision response
        match response {
            DaemonResponse::QueryResult(DaemonQueryResponse::CurrentRevision(response)) => {
                Ok(response)
            }
            DaemonResponse::Error(error) => Err(ProtocolClientError::Server(error)),
            other => Err(Self::unexpected_response("revision query", other)),
        }
    }

    /// Request query context for one root handle.
    pub fn root_snapshot(
        &self,
        handle: RootHandleId,
        target: Option<String>,
    ) -> Result<RootSnapshot, ProtocolClientError> {
        // send the root snapshot query
        let response = self.send_request(DaemonRequest::Query(DaemonQuery::RootSnapshot {
            handle,
            target,
        }))?;

        // decode the root snapshot response
        match response {
            DaemonResponse::QueryResult(DaemonQueryResponse::RootSnapshot(response)) => {
                Ok(response)
            }
            DaemonResponse::Error(error) => Err(ProtocolClientError::Server(error)),
            other => Err(Self::unexpected_response("root snapshot query", other)),
        }
    }

    /// Request a source file snapshot.
    pub fn file_snapshot(
        &self,
        handle: RootHandleId,
        request: FileSnapshotRequest,
    ) -> Result<Option<FileSnapshot>, ProtocolClientError> {
        // send the file snapshot query
        let response = self.send_request(DaemonRequest::Query(DaemonQuery::FileSnapshot {
            handle,
            request,
        }))?;

        // decode the file snapshot response
        match response {
            DaemonResponse::QueryResult(DaemonQueryResponse::FileSnapshot(response)) => {
                Ok(response)
            }
            DaemonResponse::Error(error) => Err(ProtocolClientError::Server(error)),
            other => Err(Self::unexpected_response("file snapshot query", other)),
        }
    }

    /// Request source file images for one revision.
    pub fn file_images(
        &self,
        handle: RootHandleId,
        request: FileImagesRequest,
    ) -> Result<Vec<FileUpdateImage>, ProtocolClientError> {
        // send the file images query
        let response = self.send_request(DaemonRequest::Query(DaemonQuery::FileImages {
            handle,
            request,
        }))?;

        // decode the file images response
        match response {
            DaemonResponse::QueryResult(DaemonQueryResponse::FileImages(response)) => Ok(response),
            DaemonResponse::Error(error) => Err(ProtocolClientError::Server(error)),
            other => Err(Self::unexpected_response("file images query", other)),
        }
    }

    /// Execute one semantic query for a daemon root handle.
    pub fn execute_query(
        &self,
        handle: RootHandleId,
        request: QueryRequestBody,
    ) -> Result<QueryResponseBody, ProtocolClientError> {
        // encode the query request
        let request =
            QueryRequestPayload::from_body(request).map_err(ProtocolClientError::QueryPayload)?;

        // send the semantic query
        let response = self.send_request(DaemonRequest::Query(DaemonQuery::Execute {
            handle,
            request,
        }))?;

        // decode the query response
        match response {
            DaemonResponse::QueryResult(DaemonQueryResponse::Query(response)) => response
                .decode_response()
                .map_err(ProtocolClientError::QueryPayload),
            DaemonResponse::Error(error) => Err(ProtocolClientError::Server(error)),
            other => Err(Self::unexpected_response("query", other)),
        }
    }

    /// Execute semantic queries for a daemon root handle.
    pub fn execute_query_batch(
        &self,
        handle: RootHandleId,
        requests: Vec<QueryRequestBody>,
    ) -> Result<Vec<QueryResponseBody>, ProtocolClientError> {
        // encode query requests
        let requests = requests
            .into_iter()
            .map(QueryRequestPayload::from_body)
            .collect::<Result<Vec<_>, _>>()
            .map_err(ProtocolClientError::QueryPayload)?;

        // send the semantic query batch
        let response = self.send_request(DaemonRequest::Query(DaemonQuery::ExecuteBatch {
            handle,
            requests,
        }))?;

        // decode the query response batch
        match response {
            DaemonResponse::QueryResult(DaemonQueryResponse::QueryBatch(responses)) => responses
                .into_iter()
                .map(|response| response.decode_response())
                .collect::<Result<Vec<_>, _>>()
                .map_err(ProtocolClientError::QueryPayload),
            DaemonResponse::Error(error) => Err(ProtocolClientError::Server(error)),
            other => Err(Self::unexpected_response("query batch", other)),
        }
    }

    /// Send a raw protocol message.
    pub fn send_message(&self, message: &ProtocolMessage) -> Result<(), ProtocolClientError> {
        let codec = self.codec.lock();
        codec
            .send_message(self.transport.as_ref(), message)
            .map_err(ProtocolClientError::Codec)
    }

    /// Receive a raw protocol message.
    pub fn recv_message(&self) -> Result<ProtocolMessage, ProtocolClientError> {
        let codec = self.codec.lock();
        codec
            .recv_message(self.transport.as_ref())
            .map_err(ProtocolClientError::Codec)
    }

    /// Allocate the next request id.
    fn next_request_id(&self) -> RequestId {
        let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        RequestId::new(id)
    }

    /// Build an unexpected response error.
    fn unexpected_response(expected: &str, response: DaemonResponse) -> ProtocolClientError {
        ProtocolClientError::UnexpectedResponse(format!(
            "expected {expected} response, got {response:?}"
        ))
    }
}
