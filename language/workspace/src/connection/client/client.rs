use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

use super::ClientError;
use crate::protocol::{
    ClientDescriptor, HandshakeRequest, HandshakeResponse, MIN_PROTOCOL_VERSION, PROTOCOL_VERSION,
    ProtocolCodec, ProtocolLimits, ProtocolMessage, ProtocolNotification, ProtocolRange,
    ProtocolRequest, RequestId, RequestOptions, WorkspaceNotification, WorkspaceRequest,
    WorkspaceResponse,
};
use crate::{PayloadBuffer, ProgressEvent, Transport};

/// Client configuration for the workspace protocol.
#[derive(Debug, Clone)]
pub struct ClientOptions {
    /// Supported protocol range for the client.
    pub protocol: ProtocolRange,
    /// Client requested protocol limits.
    pub limits: ProtocolLimits,
    /// Client descriptor for the handshake request.
    pub client: ClientDescriptor,
}

impl Default for ClientOptions {
    /// Return default client options.
    fn default() -> Self {
        Self {
            protocol: ProtocolRange::new(MIN_PROTOCOL_VERSION, PROTOCOL_VERSION),
            limits: ProtocolLimits::default(),
            client: ClientDescriptor {
                name: "destack-client".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                build: None,
            },
        }
    }
}

/// Client for sending workspace protocol requests.
pub struct Client {
    /// Transport used to send and receive messages.
    transport: Arc<dyn Transport>,
    /// Codec used for protocol payloads.
    codec: Mutex<ProtocolCodec>,
    /// Serialized request response cycles.
    requests: Mutex<()>,
    /// Request id counter.
    next_request_id: AtomicU64,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Client")
            .field("transport", &"<transport>")
            .finish()
    }
}

impl Client {
    /// Create a new client for a transport.
    pub fn new(transport: Arc<dyn Transport>) -> Self {
        Self::with_codec(transport, ProtocolCodec::default())
    }

    /// Create a new client with a custom codec.
    pub fn with_codec(transport: Arc<dyn Transport>, codec: ProtocolCodec) -> Self {
        Self {
            transport,
            codec: Mutex::new(codec),
            requests: Mutex::new(()),
            next_request_id: AtomicU64::new(1),
        }
    }

    /// Perform a handshake with the workspace server.
    pub fn handshake(&self, options: ClientOptions) -> Result<HandshakeResponse, ClientError> {
        let request = HandshakeRequest {
            protocol: options.protocol,
            client: options.client,
            limits: options.limits,
        };

        let response = self.send_request_with_options(
            WorkspaceRequest::Handshake(request),
            RequestOptions::default(),
        )?;
        let response = match response {
            WorkspaceResponse::Handshake(response) => response,
            WorkspaceResponse::Error(error) => {
                return Err(ClientError::Server(error));
            }
            other => {
                return Err(ClientError::UnexpectedResponse(format!(
                    "expected handshake response, got {other:?}"
                )));
            }
        };

        // update the codec limit selected by the server
        self.codec.lock().max_payload_bytes = response.limits.max_payload_bytes as usize;

        Ok(response)
    }

    /// Send a protocol request with default options.
    pub fn send_request(
        &self,
        payload: WorkspaceRequest,
    ) -> Result<WorkspaceResponse, ClientError> {
        self.send_request_with_options(payload, RequestOptions::default())
    }

    /// Send a protocol request with explicit options.
    pub fn send_request_with_options(
        &self,
        payload: WorkspaceRequest,
        options: RequestOptions,
    ) -> Result<WorkspaceResponse, ClientError> {
        self.send_request_with_progress(payload, options, &mut |_| {})
    }

    /// Send a protocol request, forwarding interim progress events.
    pub fn send_request_with_progress(
        &self,
        payload: WorkspaceRequest,
        options: RequestOptions,
        on_progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<WorkspaceResponse, ClientError> {
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
        let mut payloads = PayloadBuffer::new();
        let mut pending_response: Option<WorkspaceResponse> = None;

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
                    let notification = *notification;
                    match notification.payload {
                        WorkspaceNotification::Progress(progress) => on_progress(progress.event),
                        payload => {
                            payloads.ingest_notification(ProtocolNotification { payload })?
                        }
                    }
                }
                ProtocolMessage::Request(_) => {
                    return Err(ClientError::UnexpectedResponse(
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

    /// Send a raw protocol message.
    pub fn send_message(&self, message: &ProtocolMessage) -> Result<(), ClientError> {
        let codec = self.codec.lock();
        let payload = codec.encode_message(message).map_err(ClientError::Codec)?;

        self.transport
            .send(&payload)
            .map_err(ClientError::Transport)
    }

    /// Receive a raw protocol message.
    pub fn recv_message(&self) -> Result<ProtocolMessage, ClientError> {
        let codec = self.codec.lock();
        let payload = self.transport.recv().map_err(ClientError::Transport)?;

        codec.decode_message(&payload).map_err(ClientError::Codec)
    }

    /// Allocate the next request id.
    fn next_request_id(&self) -> RequestId {
        let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        RequestId::new(id)
    }

    /// Build an unexpected response error.
    pub(super) fn unexpected_response(expected: &str, response: WorkspaceResponse) -> ClientError {
        ClientError::UnexpectedResponse(format!("expected {expected} response, got {response:?}"))
    }
}
