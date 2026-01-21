use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

use crate::protocol::ProtocolCodecError;

use super::{
    ClientInfo, DaemonRequest, DaemonResponse, HandshakeRequest, HandshakeResponse, ProtocolCodec,
    ProtocolError, ProtocolLimits, ProtocolMessage, ProtocolRange, ProtocolRequest, RequestId,
    RequestOptions, SessionId, Transport, TransportError,
};

/// Client configuration for the daemon protocol.
#[derive(Debug, Clone)]
pub struct ProtocolClientOptions {
    /// Supported protocol range for the client.
    pub protocol: ProtocolRange,
    /// Client requested protocol limits.
    pub limits: ProtocolLimits,
    /// Metadata for the client handshake request.
    pub client_info: ClientInfo,
}

impl Default for ProtocolClientOptions {
    fn default() -> Self {
        Self {
            protocol: ProtocolRange::new(super::MIN_PROTOCOL_VERSION, super::PROTOCOL_VERSION),
            limits: ProtocolLimits::default(),
            client_info: ClientInfo {
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
    /// Negotiated session id.
    session_id: Mutex<Option<SessionId>>,
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
            session_id: Mutex::new(None),
            next_request_id: AtomicU64::new(1),
        }
    }

    /// Return the negotiated session id, if any.
    pub fn session_id(&self) -> Option<SessionId> {
        *self.session_id.lock()
    }

    /// Perform a handshake with the daemon.
    pub fn handshake(
        &self,
        options: ProtocolClientOptions,
    ) -> Result<HandshakeResponse, ProtocolClientError> {
        let request = HandshakeRequest {
            protocol: options.protocol,
            client: options.client_info,
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
        let id = self.next_request_id();
        let request = ProtocolRequest {
            id,
            options,
            payload,
        };
        let message = ProtocolMessage::Request(request);
        self.send_message(&message)?;

        loop {
            match self.recv_message()? {
                ProtocolMessage::Response(response) => {
                    if response.id == id {
                        return Ok(response.payload);
                    }
                }
                ProtocolMessage::Notification(_) => {
                    continue;
                }
                ProtocolMessage::Request(_) => {
                    return Err(ProtocolClientError::UnexpectedResponse(
                        "client received request".to_string(),
                    ));
                }
            }
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
}

/// Errors returned by protocol clients.
#[derive(Debug)]
pub enum ProtocolClientError {
    /// Protocol codec error.
    Codec(ProtocolCodecError),
    /// Transport error.
    Transport(TransportError),
    /// Server returned an error response.
    Server(ProtocolError),
    /// The response payload was unexpected.
    UnexpectedResponse(String),
}

impl std::fmt::Display for ProtocolClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolClientError::Codec(error) => write!(f, "protocol codec error: {error}"),
            ProtocolClientError::Transport(error) => write!(f, "transport error: {error}"),
            ProtocolClientError::Server(error) => write!(f, "server error: {error}"),
            ProtocolClientError::UnexpectedResponse(message) => {
                write!(f, "unexpected response: {message}")
            }
        }
    }
}

impl std::error::Error for ProtocolClientError {}

impl From<TransportError> for ProtocolClientError {
    fn from(error: TransportError) -> Self {
        ProtocolClientError::Transport(error)
    }
}
