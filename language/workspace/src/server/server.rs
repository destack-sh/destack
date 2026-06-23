use std::sync::Arc;

use parking_lot::Mutex;

use crate::Error;

use super::connection::Connection;
use super::{ServerError, ServerOptions};
use crate::protocol::{
    HandshakeRequest, HandshakeResponse, ProtocolCodec, ProtocolError, ProtocolErrorCode,
    ProtocolLimits, ProtocolMessage, ProtocolNotification, ProtocolRequest, ProtocolResponse,
    WorkspaceRequest, WorkspaceResponse,
};
use crate::{PayloadPrepareError, PayloadSender, ServerLifecycle, Transport, Workspace};

use super::RootLease;

/// Server-side protocol handler for workspace requests.
#[derive(Debug)]
pub struct Server {
    /// Workspace backing this server.
    pub(super) workspace: Arc<dyn Workspace>,
    /// Root handle counter for this workspace.
    pub(super) root_lease: Arc<RootLease>,
    /// Server options for protocol negotiation.
    pub(super) options: ServerOptions,
    /// Mutable protocol connection state.
    pub(super) connection: Mutex<Connection>,
    /// Codec used to serialize protocol messages.
    pub(super) codec: Mutex<ProtocolCodec>,
    /// Lifecycle state for server lifetime.
    pub(super) lifecycle: ServerLifecycle,
}

impl Server {
    /// Create a new protocol server with defaults.
    pub fn new(workspace: Arc<dyn Workspace>) -> Self {
        Self::with_options(workspace, ServerOptions::default())
    }

    /// Create a new protocol server with explicit options.
    pub fn with_options(workspace: Arc<dyn Workspace>, options: ServerOptions) -> Self {
        Self::with_lifecycle(workspace, options, ServerLifecycle::default())
    }

    /// Create a new protocol server with explicit options and lifecycle state.
    pub fn with_lifecycle(
        workspace: Arc<dyn Workspace>,
        options: ServerOptions,
        lifecycle: ServerLifecycle,
    ) -> Self {
        Self::with_root_lease(
            workspace,
            Arc::new(RootLease::default()),
            options,
            lifecycle,
        )
    }

    /// Create a new protocol server with a shared root handle counter.
    pub fn with_root_lease(
        workspace: Arc<dyn Workspace>,
        root_lease: Arc<RootLease>,
        options: ServerOptions,
        lifecycle: ServerLifecycle,
    ) -> Self {
        let limits = options.limits;
        let codec = ProtocolCodec::new(limits.max_payload_bytes as usize);

        Self {
            workspace,
            root_lease,
            options,
            connection: Mutex::new(Connection::new(limits)),
            codec: Mutex::new(codec),
            lifecycle,
        }
    }

    /// Serve protocol requests over a transport until shutdown.
    pub fn serve<T: Transport + ?Sized>(&self, transport: &T) -> Result<(), ServerError> {
        // register a lease for this connection
        self.lifecycle.register_connection();

        // serve requests until shutdown
        let result = loop {
            let message = match self.recv_message(transport) {
                Ok(message) => message,
                Err(error) => break Err(error),
            };
            let response = match self.handle_message(transport, message) {
                Ok(response) => response,
                Err(error) => break Err(error),
            };
            let Some((response, payloads)) = response else {
                if self.is_shutting_down() {
                    break Ok(());
                }
                continue;
            };
            if let Err(error) = self.send_message(transport, &response) {
                break Err(error);
            }
            if let Err(error) = self.flush_payloads(transport, payloads) {
                break Err(error);
            }
            if self.is_shutting_down() {
                break Ok(());
            }
        };

        // cleanup connection resources
        self.cleanup_connection();
        self.lifecycle.unregister_connection();

        result
    }

    /// Dispatch one encoded protocol message through this server.
    pub fn dispatch(&self, payload: &[u8]) -> Result<Vec<Vec<u8>>, ServerError> {
        let transport = DispatchTransport::default();

        // decode the incoming request
        let codec = self.codec.lock();
        let message = codec.decode_message(payload).map_err(ServerError::Codec)?;
        drop(codec);

        // dispatch the request and collect progress notifications
        let response = self.handle_message(&transport, message)?;
        let Some((response, payloads)) = response else {
            return Ok(transport.finish());
        };

        // collect the final response and deferred payload chunks
        self.send_message(&transport, &response)?;
        self.flush_payloads(&transport, payloads)?;

        Ok(transport.finish())
    }

    /// Receive a protocol message from the transport.
    fn recv_message<T: Transport + ?Sized>(
        &self,
        transport: &T,
    ) -> Result<ProtocolMessage, ServerError> {
        let codec = self.codec.lock();
        let payload = transport.recv().map_err(ServerError::Transport)?;

        codec.decode_message(&payload).map_err(ServerError::Codec)
    }

    /// Send a protocol message to the transport.
    pub(super) fn send_message<T: Transport + ?Sized>(
        &self,
        transport: &T,
        message: &ProtocolMessage,
    ) -> Result<(), ServerError> {
        let codec = self.codec.lock();
        let payload = codec.encode_message(message).map_err(ServerError::Codec)?;

        transport.send(&payload).map_err(ServerError::Transport)
    }

    /// Handle a protocol message and return an optional response.
    fn handle_message<T: Transport + ?Sized>(
        &self,
        transport: &T,
        message: ProtocolMessage,
    ) -> Result<Option<(ProtocolMessage, PayloadSender)>, ServerError> {
        // mark activity for this message
        self.lifecycle.touch_activity();

        match message {
            ProtocolMessage::Request(request) => {
                let (response, payloads) = self.handle_request(transport, *request);
                Ok(Some((
                    ProtocolMessage::Response(Box::new(response)),
                    payloads,
                )))
            }
            ProtocolMessage::Notification(notification) => {
                self.handle_notification(*notification);
                Ok(None)
            }
            ProtocolMessage::Response(_) => Err(ServerError::UnexpectedResponse),
        }
    }

    /// Handle a protocol request and return the response.
    fn handle_request<T: Transport + ?Sized>(
        &self,
        transport: &T,
        request: ProtocolRequest,
    ) -> (ProtocolResponse, PayloadSender) {
        let mut payloads = PayloadSender::new(self.payload_limits());
        let payload = match request.payload {
            WorkspaceRequest::Handshake(handshake) => self.handle_handshake(handshake),
            WorkspaceRequest::Ping => Ok(WorkspaceResponse::Pong),
            WorkspaceRequest::Cancel { id } => Ok(WorkspaceResponse::Canceled { id }),
            WorkspaceRequest::Shutdown => self.handle_shutdown(),
            WorkspaceRequest::OpenRoot(request) => self.handle_open_root(request),
            WorkspaceRequest::CloseRoot(request) => self.handle_close_root(request),
            WorkspaceRequest::ReloadRoot(request) => self.handle_reload_root(request),
            WorkspaceRequest::ApplyFileOperation(request) => self.handle_file_operation(request),
            WorkspaceRequest::ApplySourceUpdate(request) => self.handle_source_update(request),
            WorkspaceRequest::StartWatch(request) => self.handle_start_watch(request),
            WorkspaceRequest::NextWatchBatch(request) => self.handle_next_watch_batch(request),
            WorkspaceRequest::StopWatch(request) => self.handle_stop_watch(request),
            WorkspaceRequest::Check { handle, input } => {
                let notify = self.progress_notification(transport, handle);

                self.handle_check(handle, input, &notify)
            }
            WorkspaceRequest::Lint { handle, input } => {
                let notify = self.progress_notification(transport, handle);

                self.handle_lint(handle, input, &notify)
            }
            WorkspaceRequest::Format { handle, input } => {
                let notify = self.progress_notification(transport, handle);

                self.handle_format(handle, input, &notify)
            }
            WorkspaceRequest::Build { handle, input } => {
                let notify = self.progress_notification(transport, handle);

                self.handle_build(handle, input, &notify)
            }
            WorkspaceRequest::Run { handle, input } => {
                let notify = self.progress_notification(transport, handle);

                self.handle_run(handle, input, &notify)
            }
            WorkspaceRequest::Test { handle, input } => {
                let notify = self.progress_notification(transport, handle);

                self.handle_test(handle, input, &notify)
            }
            WorkspaceRequest::Doc { handle, input } => {
                let notify = self.progress_notification(transport, handle);

                self.handle_doc(handle, input, &notify)
            }
            WorkspaceRequest::Bench { handle, input } => {
                let notify = self.progress_notification(transport, handle);

                self.handle_bench(handle, input, &notify)
            }
            WorkspaceRequest::Info { handle, input } => {
                let notify = self.progress_notification(transport, handle);

                self.handle_info(handle, input, &notify)
            }
            WorkspaceRequest::Targets { handle, input } => {
                let notify = self.progress_notification(transport, handle);

                self.handle_targets(handle, input, &notify)
            }
            WorkspaceRequest::Cache { handle, input } => {
                let notify = self.progress_notification(transport, handle);

                self.handle_cache(handle, input, &notify)
            }
            WorkspaceRequest::Settings { handle, input } => {
                let notify = self.progress_notification(transport, handle);

                self.handle_settings(handle, input, &notify)
            }
            WorkspaceRequest::Doctor { handle, input } => {
                let notify = self.progress_notification(transport, handle);

                self.handle_doctor(handle, input, &notify)
            }
            WorkspaceRequest::Task { handle, input } => {
                let notify = self.progress_notification(transport, handle);

                self.handle_task(handle, input, &notify)
            }
            WorkspaceRequest::Clean { handle, input } => {
                let notify = self.progress_notification(transport, handle);

                self.handle_clean(handle, input, &notify)
            }
            WorkspaceRequest::Artifact { handle, artifact } => {
                self.handle_artifact(handle, artifact)
            }
            WorkspaceRequest::Store { handle, content } => self.handle_store(handle, content),
            WorkspaceRequest::Load { handle, content } => self.handle_load(handle, content),
            WorkspaceRequest::Export { handle, request } => self.handle_export(handle, request),
            WorkspaceRequest::Query(query) => self.handle_query(query, &mut payloads),
        };

        let response = match payload {
            Ok(response) => ProtocolResponse {
                id: request.id,
                payload: response,
            },
            Err(error) => ProtocolResponse {
                id: request.id,
                payload: WorkspaceResponse::Error(error),
            },
        };

        (response, payloads)
    }

    /// Handle notifications sent from the client.
    fn handle_notification(&self, _notification: ProtocolNotification) {
        // ignore client notifications
    }

    /// Check if the server is shutting down.
    fn is_shutting_down(&self) -> bool {
        self.connection.lock().shutting_down || self.lifecycle.is_shutting_down()
    }

    /// Handle protocol handshake negotiation.
    fn handle_handshake(
        &self,
        request: HandshakeRequest,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        let mut state = self.connection.lock();
        if state.is_ready {
            return Err(
                self.protocol_error(ProtocolErrorCode::Conflict, "handshake already completed")
            );
        }

        let negotiated = self
            .options
            .protocol
            .negotiate(&request.protocol)
            .ok_or_else(|| {
                self.protocol_error(
                    ProtocolErrorCode::UnsupportedVersion,
                    "protocol versions are incompatible",
                )
            })?;

        let limits = self.options.limits.negotiate(&request.limits);
        state.limits = limits;
        state.is_ready = true;

        // update payload limits on the codec
        self.codec.lock().max_payload_bytes = limits.max_payload_bytes as usize;

        let response = HandshakeResponse {
            protocol: negotiated,
            server: self.options.server.clone(),
            limits,
        };

        Ok(WorkspaceResponse::Handshake(response))
    }

    /// Handle a shutdown request.
    fn handle_shutdown(&self) -> Result<WorkspaceResponse, ProtocolError> {
        self.connection.lock().shutting_down = true;
        self.lifecycle.request_shutdown();
        Ok(WorkspaceResponse::ShutdownAck)
    }

    /// Ensure a session is established.
    pub(super) fn require_session(&self) -> Result<(), ProtocolError> {
        let state = self.connection.lock();
        if state.is_ready {
            Ok(())
        } else {
            Err(self.protocol_error(ProtocolErrorCode::NotReady, "handshake required"))
        }
    }

    /// Convert a workspace error to a protocol error.
    pub(super) fn workspace_error(&self, context: &str, error: Error) -> ProtocolError {
        // map workspace errors into protocol domain errors
        let code = match error {
            Error::FileMissing { .. } | Error::PathNotInRoot { .. } => ProtocolErrorCode::NotFound,
            Error::StaleOpenFile { .. } => ProtocolErrorCode::Conflict,
            Error::InvalidEdit { .. } | Error::InvalidTextChange { .. } => {
                ProtocolErrorCode::InvalidRequest
            }
            Error::StaleRevision { .. } => ProtocolErrorCode::Conflict,
            Error::Repository(_)
            | Error::Session(_)
            | Error::Io { .. }
            | Error::Internal { .. } => ProtocolErrorCode::Internal,
        };

        self.protocol_error(code, &format!("{context} failed: {error}"))
    }

    /// Create a protocol error with standard fields.
    pub(super) fn protocol_error(&self, code: ProtocolErrorCode, message: &str) -> ProtocolError {
        ProtocolError::new(code, message)
    }

    /// Convert a payload prepare error into a protocol error.
    pub(super) fn payload_error(&self, error: PayloadPrepareError) -> ProtocolError {
        match error {
            PayloadPrepareError::Codec(error) => self.protocol_error(
                ProtocolErrorCode::Internal,
                &format!("payload serialization failed: {error}"),
            ),
            PayloadPrepareError::ChunkLimitTooSmall => self.protocol_error(
                ProtocolErrorCode::TooLarge,
                "payload limit too small for streaming",
            ),
        }
    }

    /// Determine the negotiated protocol limits.
    pub(super) fn payload_limits(&self) -> ProtocolLimits {
        self.connection.lock().limits
    }

    /// Stream pending payloads as chunk notifications.
    fn flush_payloads<T: Transport + ?Sized>(
        &self,
        transport: &T,
        payloads: PayloadSender,
    ) -> Result<(), ServerError> {
        let codec = self.codec.lock();
        payloads
            .send(transport, &codec)
            .map_err(ServerError::Payload)
    }
}

/// Transport collecting frames produced by one direct server dispatch.
#[derive(Debug, Default)]
struct DispatchTransport {
    /// Frames emitted during the dispatch.
    frames: Mutex<Vec<Vec<u8>>>,
}

impl DispatchTransport {
    /// Return collected response frames.
    fn finish(self) -> Vec<Vec<u8>> {
        self.frames.into_inner()
    }
}

impl Transport for DispatchTransport {
    fn send(&self, payload: &[u8]) -> Result<(), crate::TransportError> {
        self.frames.lock().push(payload.to_vec());

        Ok(())
    }

    fn recv(&self) -> Result<Vec<u8>, crate::TransportError> {
        Err(crate::TransportError::Closed)
    }

    fn close(&self) {}
}
