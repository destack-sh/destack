use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

use crate::protocol::ProtocolCodecError;

use super::{
    BinaryPayload, ClientInfo, CommandResponse, DaemonNotification, DaemonQueryResponse,
    DaemonRequest, DaemonResponse, HandshakeRequest, HandshakeResponse, PayloadBody,
    PayloadChunkNotification, PayloadFormat, PayloadId, ProtocolCodec, ProtocolError,
    ProtocolLimits, ProtocolMessage, ProtocolNotification, ProtocolRange, ProtocolRequest,
    RepositoryId, RequestId, RequestOptions, Transport, TransportError,
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
        let mut inbox = PayloadInbox::new();
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
                    inbox.ingest_notification(*notification)?;
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
            let resolved = inbox.resolve_response(&mut response)?;
            if resolved {
                return Ok(response);
            }
            pending_response = Some(response);
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

#[derive(Debug, Default)]
struct PayloadInbox {
    /// Pending payload streams keyed by payload id.
    pending: HashMap<PayloadId, PendingPayloadState>,
    /// Completed payloads keyed by payload id.
    completed: HashMap<PayloadId, CompletedPayload>,
}

impl PayloadInbox {
    /// Create an empty payload inbox.
    fn new() -> Self {
        Self::default()
    }

    /// Ingest a protocol notification.
    fn ingest_notification(
        &mut self,
        notification: ProtocolNotification,
    ) -> Result<(), ProtocolClientError> {
        // track payload chunk notifications
        let DaemonNotification::PayloadChunk(chunk) = notification.payload else {
            return Ok(());
        };

        self.ingest_chunk(chunk)
    }

    /// Ingest a payload chunk.
    fn ingest_chunk(&mut self, chunk: PayloadChunkNotification) -> Result<(), ProtocolClientError> {
        // validate chunk metadata
        let total = usize::try_from(chunk.total)
            .map_err(|_| ProtocolClientError::Payload(PayloadStreamError::ChunkTotalOverflow))?;
        if total == 0 {
            return Err(ProtocolClientError::Payload(
                PayloadStreamError::ChunkTotalMustBeNonZero,
            ));
        }
        let index = usize::try_from(chunk.index)
            .map_err(|_| ProtocolClientError::Payload(PayloadStreamError::ChunkIndexOverflow))?;
        if index >= total {
            return Err(ProtocolClientError::Payload(
                PayloadStreamError::ChunkIndexOutOfRange,
            ));
        }
        if chunk.done && index + 1 != total {
            return Err(ProtocolClientError::Payload(
                PayloadStreamError::ChunkDoneMarkerInconsistent,
            ));
        }

        // update pending chunk state
        let should_complete = {
            let state = self
                .pending
                .entry(chunk.id)
                .or_insert_with(|| PendingPayloadState::new(chunk.format, total));
            if state.format != chunk.format {
                return Err(ProtocolClientError::Payload(
                    PayloadStreamError::ChunkFormatMismatch,
                ));
            }
            if state.total != total {
                return Err(ProtocolClientError::Payload(
                    PayloadStreamError::ChunkTotalMismatch,
                ));
            }
            if state.chunks[index].is_some() {
                return Err(ProtocolClientError::Payload(
                    PayloadStreamError::DuplicatePayloadChunk,
                ));
            }

            state.chunks[index] = Some(chunk.bytes);
            state.received += 1;
            state.received == state.total
        };

        // finalize the payload once all chunks arrive
        if should_complete {
            let state = self
                .pending
                .remove(&chunk.id)
                .ok_or(ProtocolClientError::Payload(
                    PayloadStreamError::PayloadStateMissing,
                ))?;
            let format = state.format;
            let bytes = state.merge_chunks().map_err(ProtocolClientError::Payload)?;
            self.completed
                .insert(chunk.id, CompletedPayload::new(format, bytes));
        }

        Ok(())
    }

    /// Resolve payloads referenced by a response if possible.
    fn resolve_response(
        &mut self,
        response: &mut DaemonResponse,
    ) -> Result<bool, ProtocolClientError> {
        // update the response with any completed payloads
        let resolved = match response {
            DaemonResponse::QueryResult(query) => self.resolve_query_response(query)?,
            DaemonResponse::CommandResult(command) => self.resolve_command_response(command)?,
            _ => true,
        };

        Ok(resolved)
    }

    fn resolve_query_response(
        &mut self,
        response: &mut DaemonQueryResponse,
    ) -> Result<bool, ProtocolClientError> {
        // resolve binary payloads in query responses
        match response {
            DaemonQueryResponse::Query(payload) => self.resolve_payload(&mut payload.payload),
            DaemonQueryResponse::QueryBatch(payloads) => {
                // resolve each payload and return early when any stream is incomplete
                for payload in payloads {
                    let resolved = self.resolve_payload(&mut payload.payload)?;
                    if !resolved {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
            _ => Ok(true),
        }
    }

    fn resolve_command_response(
        &mut self,
        response: &mut CommandResponse,
    ) -> Result<bool, ProtocolClientError> {
        // resolve command payloads when present
        let Some(payload) = response.data.as_mut() else {
            return Ok(true);
        };

        self.resolve_payload(payload)
    }

    fn resolve_payload(
        &mut self,
        payload: &mut BinaryPayload,
    ) -> Result<bool, ProtocolClientError> {
        // inline payloads are already resolved
        let PayloadBody::Deferred { id, total_bytes } = payload.body else {
            return Ok(true);
        };

        // wait until the payload has been fully received
        let completed = match self.completed.remove(&id) {
            Some(completed) => completed,
            None => return Ok(false),
        };

        // validate payload metadata
        if completed.format != payload.format {
            return Err(ProtocolClientError::Payload(
                PayloadStreamError::PayloadFormatMismatch,
            ));
        }
        if completed.bytes.len() != total_bytes as usize {
            return Err(ProtocolClientError::Payload(
                PayloadStreamError::PayloadSizeMismatch,
            ));
        }

        // replace the payload with inline bytes
        payload.body = PayloadBody::Inline {
            bytes: completed.bytes,
        };

        Ok(true)
    }
}

#[derive(Debug)]
struct PendingPayloadState {
    /// Payload format for the stream.
    format: PayloadFormat,
    /// Total chunks expected.
    total: usize,
    /// Number of chunks received.
    received: usize,
    /// Chunk storage by index.
    chunks: Vec<Option<Vec<u8>>>,
}

impl PendingPayloadState {
    /// Create a new pending payload state.
    fn new(format: PayloadFormat, total: usize) -> Self {
        Self {
            format,
            total,
            received: 0,
            chunks: vec![None; total],
        }
    }

    /// Merge stored chunks into a contiguous byte buffer.
    fn merge_chunks(self) -> Result<Vec<u8>, PayloadStreamError> {
        // compute total payload size
        let capacity = self
            .chunks
            .iter()
            .map(|chunk| chunk.as_ref().map_or(0, |bytes| bytes.len()))
            .sum();

        // assemble the final payload bytes
        let mut bytes = Vec::with_capacity(capacity);
        for chunk in self.chunks {
            let chunk = chunk.ok_or(PayloadStreamError::PayloadChunksMissingAtFinalize)?;
            bytes.extend(chunk);
        }

        Ok(bytes)
    }
}

/// Errors returned while receiving chunked payload streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadStreamError {
    /// Chunk total does not fit in memory index space.
    ChunkTotalOverflow,
    /// Chunk total is zero.
    ChunkTotalMustBeNonZero,
    /// Chunk index does not fit in memory index space.
    ChunkIndexOverflow,
    /// Chunk index is out of range for total.
    ChunkIndexOutOfRange,
    /// Chunk done marker does not match the final index.
    ChunkDoneMarkerInconsistent,
    /// Chunk format differs from the existing stream.
    ChunkFormatMismatch,
    /// Chunk total differs from the existing stream.
    ChunkTotalMismatch,
    /// Chunk was already received.
    DuplicatePayloadChunk,
    /// Payload state is missing at finalize.
    PayloadStateMissing,
    /// Payload chunks are missing at finalize.
    PayloadChunksMissingAtFinalize,
    /// Completed payload format does not match deferred payload format.
    PayloadFormatMismatch,
    /// Completed payload size does not match deferred payload size.
    PayloadSizeMismatch,
}

impl std::fmt::Display for PayloadStreamError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChunkTotalOverflow => write!(formatter, "chunk total overflow"),
            Self::ChunkTotalMustBeNonZero => write!(formatter, "chunk total must be non zero"),
            Self::ChunkIndexOverflow => write!(formatter, "chunk index overflow"),
            Self::ChunkIndexOutOfRange => write!(formatter, "chunk index out of range"),
            Self::ChunkDoneMarkerInconsistent => {
                write!(formatter, "chunk done marker is inconsistent")
            }
            Self::ChunkFormatMismatch => write!(formatter, "chunk format mismatch"),
            Self::ChunkTotalMismatch => write!(formatter, "chunk total mismatch"),
            Self::DuplicatePayloadChunk => write!(formatter, "duplicate payload chunk"),
            Self::PayloadStateMissing => write!(formatter, "payload state missing"),
            Self::PayloadChunksMissingAtFinalize => {
                write!(formatter, "payload chunks missing at finalize")
            }
            Self::PayloadFormatMismatch => write!(formatter, "payload format mismatch"),
            Self::PayloadSizeMismatch => write!(formatter, "payload size mismatch"),
        }
    }
}

impl std::error::Error for PayloadStreamError {}

#[derive(Debug)]
struct CompletedPayload {
    /// Payload format for the bytes.
    format: PayloadFormat,
    /// Payload bytes.
    bytes: Vec<u8>,
}

impl CompletedPayload {
    /// Create a completed payload entry.
    fn new(format: PayloadFormat, bytes: Vec<u8>) -> Self {
        Self { format, bytes }
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
    /// Payload streaming error.
    Payload(PayloadStreamError),
    /// The response payload was unexpected.
    UnexpectedResponse(String),
}

impl std::fmt::Display for ProtocolClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolClientError::Codec(error) => write!(f, "protocol codec error: {error}"),
            ProtocolClientError::Transport(error) => write!(f, "transport error: {error}"),
            ProtocolClientError::Server(error) => write!(f, "server error: {error}"),
            ProtocolClientError::Payload(error) => write!(f, "payload error: {error}"),
            ProtocolClientError::UnexpectedResponse(message) => {
                write!(f, "unexpected response: {message}")
            }
        }
    }
}

impl std::error::Error for ProtocolClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Codec(error) => Some(error),
            Self::Transport(error) => Some(error),
            Self::Server(error) => Some(error),
            Self::Payload(error) => Some(error),
            Self::UnexpectedResponse(_) => None,
        }
    }
}

impl From<TransportError> for ProtocolClientError {
    fn from(error: TransportError) -> Self {
        ProtocolClientError::Transport(error)
    }
}
