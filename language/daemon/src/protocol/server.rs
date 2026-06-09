use std::sync::Arc;

use destack_workspace::Error;
use parking_lot::Mutex;

use crate::{Daemon, DaemonError};

use super::connection::Connection;
use super::{
    DaemonRequest, DaemonResponse, HandshakeRequest, HandshakeResponse, PayloadSendError,
    PayloadWriteError, PayloadWriter, ProtocolCodec, ProtocolError, ProtocolErrorCode,
    ProtocolLimits, ProtocolMessage, ProtocolRange, ProtocolRequest, ProtocolResponse,
    RepositoryId, ServerControl, ServerDescriptor, Transport, TransportError,
};

/// Server side protocol handler for daemon requests.
#[derive(Debug)]
pub struct Server {
    /// The daemon backing this server.
    pub(super) daemon: Arc<Daemon>,
    /// Server options for protocol negotiation.
    pub(super) options: ServerOptions,
    /// Mutable server state.
    pub(super) state: Mutex<Connection>,
    /// Codec used to serialize protocol messages.
    pub(super) codec: Mutex<ProtocolCodec>,
    /// Control flags for server lifetime.
    pub(super) control: ServerControl,
}

/// Options for initializing a protocol server.
#[derive(Debug, Clone)]
pub struct ServerOptions {
    /// Supported protocol range.
    pub protocol: ProtocolRange,
    /// Limits advertised by the server.
    pub limits: ProtocolLimits,
    /// Server descriptor for the handshake response.
    pub server: ServerDescriptor,
}

impl Default for ServerOptions {
    /// Return default protocol server options.
    fn default() -> Self {
        Self {
            protocol: ProtocolRange::new(super::MIN_PROTOCOL_VERSION, super::PROTOCOL_VERSION),
            limits: ProtocolLimits::default(),
            server: ServerDescriptor {
                name: "destack-daemon".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                build: None,
                pid: Some(std::process::id()),
            },
        }
    }
}

impl Server {
    /// Create a new protocol server with defaults.
    pub fn new(daemon: Arc<Daemon>) -> Self {
        Self::with_options(daemon, ServerOptions::default())
    }

    /// Create a new protocol server with explicit options.
    pub fn with_options(daemon: Arc<Daemon>, options: ServerOptions) -> Self {
        Self::with_control(daemon, options, ServerControl::default())
    }

    /// Create a new protocol server with explicit options and control.
    pub fn with_control(
        daemon: Arc<Daemon>,
        options: ServerOptions,
        control: ServerControl,
    ) -> Self {
        let codec = ProtocolCodec::new(options.limits.max_payload_bytes as usize);
        Self {
            daemon,
            options,
            state: Mutex::new(Connection::new()),
            codec: Mutex::new(codec),
            control,
        }
    }

    /// Serve protocol requests over a transport until shutdown.
    pub fn serve<T: Transport + ?Sized>(&self, transport: &T) -> Result<(), ServerError> {
        // register a lease for this connection
        self.control.register_connection();

        // serve requests until shutdown
        let result = loop {
            let message = match self.recv_message(transport) {
                Ok(message) => message,
                Err(error) => break Err(error),
            };
            let response = match self.handle_message(message) {
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
        self.control.unregister_connection();

        result
    }

    /// Receive a protocol message from the transport.
    fn recv_message<T: Transport + ?Sized>(
        &self,
        transport: &T,
    ) -> Result<ProtocolMessage, ServerError> {
        let codec = self.codec.lock();
        codec.recv_message(transport).map_err(ServerError::Codec)
    }

    /// Send a protocol message to the transport.
    fn send_message<T: Transport + ?Sized>(
        &self,
        transport: &T,
        message: &ProtocolMessage,
    ) -> Result<(), ServerError> {
        let codec = self.codec.lock();
        codec
            .send_message(transport, message)
            .map_err(ServerError::Codec)
    }

    /// Handle a protocol message and return an optional response.
    fn handle_message(
        &self,
        message: ProtocolMessage,
    ) -> Result<Option<(ProtocolMessage, PayloadWriter)>, ServerError> {
        // mark activity for this message
        self.control.touch_activity();

        match message {
            ProtocolMessage::Request(request) => {
                let (response, payloads) = self.handle_request(*request);
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
    fn handle_request(&self, request: ProtocolRequest) -> (ProtocolResponse, PayloadWriter) {
        let mut payloads = PayloadWriter::new(self.payload_limits());
        let payload = match request.payload {
            DaemonRequest::Handshake(handshake) => self.handle_handshake(handshake),
            DaemonRequest::Ping => Ok(DaemonResponse::Pong),
            DaemonRequest::Cancel { id } => Ok(DaemonResponse::Canceled { id }),
            DaemonRequest::Shutdown => self.handle_shutdown(),
            DaemonRequest::OpenRoot(request) => self.handle_open_root(request),
            DaemonRequest::CloseRoot(request) => self.handle_close_root(request),
            DaemonRequest::ReloadRoot(request) => self.handle_reload_root(request),
            DaemonRequest::ApplyFileOperation(request) => self.handle_file_operation(request),
            DaemonRequest::ApplySourceUpdate(request) => self.handle_source_update(request),
            DaemonRequest::StartWatch(request) => self.handle_start_watch(request),
            DaemonRequest::NextWatchBatch(request) => self.handle_next_watch_batch(request),
            DaemonRequest::StopWatch(request) => self.handle_stop_watch(request),
            DaemonRequest::Command(request) => self.handle_command(*request, &mut payloads),
            DaemonRequest::Query(query) => self.handle_query(query, &mut payloads),
        };

        let response = match payload {
            Ok(response) => ProtocolResponse {
                id: request.id,
                payload: response,
            },
            Err(error) => ProtocolResponse {
                id: request.id,
                payload: DaemonResponse::Error(error),
            },
        };

        (response, payloads)
    }

    /// Handle notifications sent from the client.
    fn handle_notification(&self, _notification: super::ProtocolNotification) {
        // ignore client notifications
    }

    /// Check if the server is shutting down.
    fn is_shutting_down(&self) -> bool {
        self.state.lock().shutting_down || self.control.is_shutting_down()
    }

    /// Handle protocol handshake negotiation.
    fn handle_handshake(&self, request: HandshakeRequest) -> Result<DaemonResponse, ProtocolError> {
        let mut state = self.state.lock();
        if state.session_id.is_some() {
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
        state.negotiated_limits = Some(limits);
        let session_id = RepositoryId::new(1);
        state.session_id = Some(session_id);

        // update payload limits on the codec
        self.codec.lock().max_payload_bytes = limits.max_payload_bytes as usize;

        let response = HandshakeResponse {
            protocol: negotiated,
            server: self.options.server.clone(),
            limits,
            session_id,
        };

        Ok(DaemonResponse::Handshake(response))
    }

    /// Handle a shutdown request.
    fn handle_shutdown(&self) -> Result<DaemonResponse, ProtocolError> {
        self.state.lock().shutting_down = true;
        self.control.request_shutdown();
        Ok(DaemonResponse::ShutdownAck)
    }

    /// Ensure a session is established.
    pub(super) fn require_session(&self) -> Result<(), ProtocolError> {
        let state = self.state.lock();
        if state.session_id.is_some() {
            Ok(())
        } else {
            Err(self.protocol_error(ProtocolErrorCode::NotReady, "handshake required"))
        }
    }

    /// Convert a daemon error to a protocol error.
    pub(super) fn daemon_error(&self, error: DaemonError) -> ProtocolError {
        self.protocol_error(ProtocolErrorCode::Internal, &error.to_string())
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
        ProtocolError {
            code,
            message: message.to_string(),
            detail: None,
            retryable: false,
            retry_after_ms: None,
        }
    }

    /// Convert a payload write error into a protocol error.
    pub(super) fn payload_error(&self, error: PayloadWriteError) -> ProtocolError {
        match error {
            PayloadWriteError::ChunkLimitTooSmall => self.protocol_error(
                ProtocolErrorCode::TooLarge,
                "payload limit too small for streaming",
            ),
        }
    }

    /// Determine the negotiated protocol limits.
    pub(super) fn payload_limits(&self) -> ProtocolLimits {
        // prefer negotiated limits when available
        let state = self.state.lock();
        state.negotiated_limits.unwrap_or(self.options.limits)
    }

    /// Stream pending payloads as chunk notifications.
    fn flush_payloads<T: Transport + ?Sized>(
        &self,
        transport: &T,
        payloads: PayloadWriter,
    ) -> Result<(), ServerError> {
        let codec = self.codec.lock();
        payloads
            .send(transport, &codec)
            .map_err(ServerError::Payload)
    }
}
/// Errors returned by protocol server loops.
#[derive(Debug)]
pub enum ServerError {
    /// Protocol codec error.
    Codec(super::ProtocolCodecError),
    /// Payload transfer error.
    Payload(PayloadSendError),
    /// Transport error.
    Transport(TransportError),
    /// Unexpected response message.
    UnexpectedResponse,
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerError::Codec(error) => write!(f, "protocol codec error: {error}"),
            ServerError::Payload(error) => write!(f, "payload error: {error}"),
            ServerError::Transport(error) => write!(f, "transport error: {error}"),
            ServerError::UnexpectedResponse => {
                write!(f, "unexpected protocol response received by server")
            }
        }
    }
}

impl std::error::Error for ServerError {}

impl From<TransportError> for ServerError {
    fn from(error: TransportError) -> Self {
        ServerError::Transport(error)
    }
}
