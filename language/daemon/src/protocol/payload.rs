use std::collections::HashMap;

use super::{
    BinaryPayload, ClientError, CommandResponse, DaemonNotification, DaemonQueryResponse,
    DaemonResponse, PayloadBody, PayloadChunkNotification, PayloadFormat, PayloadId, ProtocolCodec,
    ProtocolCodecError, ProtocolLimits, ProtocolMessage, ProtocolNotification, Transport,
    inline_payload_max_bytes, payload_chunk_bytes,
};

/// Deferred payload chunks produced for one response.
#[derive(Debug)]
pub(crate) struct PayloadWriter {
    /// Negotiated payload limits.
    limits: ProtocolLimits,
    /// Next payload id to allocate.
    next_id: u64,
    /// Pending payloads to stream.
    pending: Vec<PendingPayload>,
}

impl PayloadWriter {
    /// Create a payload writer for one response.
    pub(crate) fn new(limits: ProtocolLimits) -> Self {
        Self {
            limits,
            next_id: 1,
            pending: Vec::new(),
        }
    }

    /// Prepare a payload for inline or deferred transfer.
    pub(crate) fn prepare(
        &mut self,
        payload: BinaryPayload,
    ) -> Result<BinaryPayload, PayloadWriteError> {
        let PayloadBody::Inline { bytes } = payload.body else {
            return Ok(payload);
        };

        // keep small payloads inline
        let inline_limit = inline_payload_max_bytes(self.limits);
        if bytes.len() <= inline_limit {
            return Ok(BinaryPayload {
                format: payload.format,
                body: PayloadBody::Inline { bytes },
            });
        }

        // require enough room for chunked transfer
        let chunk_limit = payload_chunk_bytes(self.limits);
        if chunk_limit == 0 {
            return Err(PayloadWriteError::ChunkLimitTooSmall);
        }

        // defer large payloads until after the response
        let id = self.allocate_payload_id();
        let total_bytes = bytes.len() as u64;
        self.pending.push(PendingPayload {
            id,
            format: payload.format,
            bytes,
        });

        Ok(BinaryPayload {
            format: payload.format,
            body: PayloadBody::Deferred { id, total_bytes },
        })
    }

    /// Send deferred payload chunks through a transport.
    pub(crate) fn send<T: Transport + ?Sized>(
        self,
        transport: &T,
        codec: &ProtocolCodec,
    ) -> Result<(), PayloadSendError> {
        if self.pending.is_empty() {
            return Ok(());
        }

        // derive a chunk size from negotiated limits
        let chunk_size = payload_chunk_bytes(self.limits);
        if chunk_size == 0 {
            return Err(PayloadSendError::ChunkLimitTooSmall);
        }

        // send each deferred payload
        for payload in self.pending {
            Self::send_payload_chunks(transport, codec, payload, chunk_size)?;
        }

        Ok(())
    }

    /// Allocate a payload id.
    fn allocate_payload_id(&mut self) -> PayloadId {
        let id = PayloadId::new(self.next_id);
        self.next_id += 1;

        id
    }

    /// Send one deferred payload as chunk notifications.
    fn send_payload_chunks<T: Transport + ?Sized>(
        transport: &T,
        codec: &ProtocolCodec,
        payload: PendingPayload,
        chunk_size: usize,
    ) -> Result<(), PayloadSendError> {
        // adjust chunk size to fit within payload limits
        let chunk_size = Self::fit_chunk_size(codec, &payload, chunk_size)?;

        // compute chunk counts
        let total_chunks = payload.bytes.len().div_ceil(chunk_size);
        let total =
            u32::try_from(total_chunks).map_err(|_| PayloadSendError::ChunkCountOverflow)?;

        // emit each payload chunk
        for (index, chunk) in payload.bytes.chunks(chunk_size).enumerate() {
            let index = u32::try_from(index).map_err(|_| PayloadSendError::ChunkIndexOverflow)?;
            let done = (index + 1) == total;
            let notification = PayloadChunkNotification {
                id: payload.id,
                format: payload.format,
                index,
                total,
                bytes: chunk.to_vec(),
                done,
            };
            let message = ProtocolMessage::Notification(Box::new(ProtocolNotification {
                payload: DaemonNotification::PayloadChunk(notification),
            }));
            codec
                .send_message(transport, &message)
                .map_err(PayloadSendError::Codec)?;
        }

        Ok(())
    }

    /// Return a chunk size that fits encoded protocol limits.
    fn fit_chunk_size(
        codec: &ProtocolCodec,
        payload: &PendingPayload,
        chunk_size: usize,
    ) -> Result<usize, PayloadSendError> {
        let mut candidate = chunk_size;
        loop {
            if candidate == 0 {
                return Err(PayloadSendError::ChunkLimitTooSmall);
            }

            // encode a representative chunk at this size
            let message = ProtocolMessage::Notification(Box::new(ProtocolNotification {
                payload: DaemonNotification::PayloadChunk(PayloadChunkNotification {
                    id: payload.id,
                    format: payload.format,
                    index: 0,
                    total: 1,
                    bytes: vec![0u8; candidate],
                    done: true,
                }),
            }));

            match codec.encode_message(&message) {
                Ok(_) => return Ok(candidate),
                Err(ProtocolCodecError::PayloadTooLarge { .. }) => {
                    candidate /= 2;
                }
                Err(error) => return Err(PayloadSendError::Codec(error)),
            }
        }
    }
}

/// Pending payload for deferred delivery.
#[derive(Debug)]
struct PendingPayload {
    /// Payload id to stream.
    id: PayloadId,
    /// Payload format.
    format: PayloadFormat,
    /// Serialized payload bytes.
    bytes: Vec<u8>,
}

/// Deferred payload chunks received while waiting for a response.
#[derive(Debug, Default)]
pub(crate) struct PayloadReceiver {
    /// Pending payload streams keyed by payload id.
    pending: HashMap<PayloadId, PayloadStream>,
    /// Completed payloads keyed by payload id.
    completed: HashMap<PayloadId, ReceivedPayload>,
}

impl PayloadReceiver {
    /// Create an empty payload receiver.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Ingest a protocol notification.
    pub(crate) fn ingest_notification(
        &mut self,
        notification: ProtocolNotification,
    ) -> Result<(), ClientError> {
        let DaemonNotification::PayloadChunk(chunk) = notification.payload else {
            return Ok(());
        };

        self.ingest_chunk(chunk)
    }

    /// Resolve payloads referenced by a response if possible.
    pub(crate) fn resolve_response(
        &mut self,
        response: &mut DaemonResponse,
    ) -> Result<bool, ClientError> {
        match response {
            DaemonResponse::QueryResult(query) => self.resolve_query_response(query),
            DaemonResponse::CommandResult(command) => self.resolve_command_response(command),
            _ => Ok(true),
        }
    }

    /// Ingest a payload chunk.
    fn ingest_chunk(&mut self, chunk: PayloadChunkNotification) -> Result<(), ClientError> {
        let total = usize::try_from(chunk.total)
            .map_err(|_| ClientError::Payload(PayloadReceiveError::ChunkTotalOverflow))?;
        if total == 0 {
            return Err(ClientError::Payload(
                PayloadReceiveError::ChunkTotalMustBeNonZero,
            ));
        }
        let index = usize::try_from(chunk.index)
            .map_err(|_| ClientError::Payload(PayloadReceiveError::ChunkIndexOverflow))?;
        if index >= total {
            return Err(ClientError::Payload(
                PayloadReceiveError::ChunkIndexOutOfRange,
            ));
        }
        if chunk.done && index + 1 != total {
            return Err(ClientError::Payload(
                PayloadReceiveError::ChunkDoneMarkerInconsistent,
            ));
        }

        // update pending chunk state
        let should_complete = {
            let state = self
                .pending
                .entry(chunk.id)
                .or_insert_with(|| PayloadStream::new(chunk.format, total));
            if state.format != chunk.format {
                return Err(ClientError::Payload(
                    PayloadReceiveError::ChunkFormatMismatch,
                ));
            }
            if state.total != total {
                return Err(ClientError::Payload(
                    PayloadReceiveError::ChunkTotalMismatch,
                ));
            }
            if state.chunks[index].is_some() {
                return Err(ClientError::Payload(
                    PayloadReceiveError::DuplicatePayloadChunk,
                ));
            }

            state.chunks[index] = Some(chunk.bytes);
            state.received += 1;
            state.received == state.total
        };

        // finalize the payload once all chunks arrive
        if should_complete {
            let state = self.pending.remove(&chunk.id).ok_or(ClientError::Payload(
                PayloadReceiveError::PayloadStateMissing,
            ))?;
            let format = state.format;
            let bytes = state.merge_chunks().map_err(ClientError::Payload)?;
            self.completed
                .insert(chunk.id, ReceivedPayload::new(format, bytes));
        }

        Ok(())
    }

    /// Resolve binary payloads in query responses.
    fn resolve_query_response(
        &mut self,
        response: &mut DaemonQueryResponse,
    ) -> Result<bool, ClientError> {
        match response {
            DaemonQueryResponse::Query(payload) => self.resolve_payload(&mut payload.payload),
            DaemonQueryResponse::QueryBatch(payloads) => {
                // return early when any stream is incomplete
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

    /// Resolve command payloads when present.
    fn resolve_command_response(
        &mut self,
        response: &mut CommandResponse,
    ) -> Result<bool, ClientError> {
        let Some(payload) = response.data.as_mut() else {
            return Ok(true);
        };

        self.resolve_payload(payload)
    }

    /// Resolve one deferred payload.
    fn resolve_payload(&mut self, payload: &mut BinaryPayload) -> Result<bool, ClientError> {
        let PayloadBody::Deferred { id, total_bytes } = payload.body else {
            return Ok(true);
        };

        let completed = match self.completed.remove(&id) {
            Some(completed) => completed,
            None => return Ok(false),
        };

        // validate payload metadata
        if completed.format != payload.format {
            return Err(ClientError::Payload(
                PayloadReceiveError::PayloadFormatMismatch,
            ));
        }
        if completed.bytes.len() != total_bytes as usize {
            return Err(ClientError::Payload(
                PayloadReceiveError::PayloadSizeMismatch,
            ));
        }

        payload.body = PayloadBody::Inline {
            bytes: completed.bytes,
        };

        Ok(true)
    }
}

/// Pending payload stream.
#[derive(Debug)]
struct PayloadStream {
    /// Payload format for the stream.
    format: PayloadFormat,
    /// Total chunks expected.
    total: usize,
    /// Number of chunks received.
    received: usize,
    /// Chunk storage by index.
    chunks: Vec<Option<Vec<u8>>>,
}

impl PayloadStream {
    /// Create a new pending payload stream.
    fn new(format: PayloadFormat, total: usize) -> Self {
        Self {
            format,
            total,
            received: 0,
            chunks: vec![None; total],
        }
    }

    /// Merge stored chunks into a contiguous byte buffer.
    fn merge_chunks(self) -> Result<Vec<u8>, PayloadReceiveError> {
        let capacity = self
            .chunks
            .iter()
            .map(|chunk| chunk.as_ref().map_or(0, |bytes| bytes.len()))
            .sum();

        let mut bytes = Vec::with_capacity(capacity);
        for chunk in self.chunks {
            let chunk = chunk.ok_or(PayloadReceiveError::PayloadChunksMissingAtFinalize)?;
            bytes.extend(chunk);
        }

        Ok(bytes)
    }
}

/// Errors returned while receiving chunked payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadReceiveError {
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

impl std::fmt::Display for PayloadReceiveError {
    /// Format the payload receive error.
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

impl std::error::Error for PayloadReceiveError {}

/// Errors returned while sending chunked payloads.
#[derive(Debug)]
pub enum PayloadSendError {
    /// Protocol codec error.
    Codec(ProtocolCodecError),
    /// Chunked transfer cannot fit inside negotiated limits.
    ChunkLimitTooSmall,
    /// Payload chunk count does not fit in protocol index space.
    ChunkCountOverflow,
    /// Payload chunk index does not fit in protocol index space.
    ChunkIndexOverflow,
}

impl std::fmt::Display for PayloadSendError {
    /// Format the payload send error.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Codec(error) => write!(formatter, "protocol codec error: {error}"),
            Self::ChunkLimitTooSmall => write!(formatter, "payload chunk limit too small"),
            Self::ChunkCountOverflow => write!(formatter, "payload chunk count overflow"),
            Self::ChunkIndexOverflow => write!(formatter, "payload chunk index overflow"),
        }
    }
}

impl std::error::Error for PayloadSendError {
    /// Return the underlying error source when present.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Codec(error) => Some(error),
            _ => None,
        }
    }
}

/// Errors returned while preparing chunked payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadWriteError {
    /// Chunked transfer cannot fit inside negotiated limits.
    ChunkLimitTooSmall,
}

impl std::fmt::Display for PayloadWriteError {
    /// Format the payload write error.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChunkLimitTooSmall => write!(formatter, "payload limit too small for streaming"),
        }
    }
}

impl std::error::Error for PayloadWriteError {}

/// Received payload bytes.
#[derive(Debug)]
struct ReceivedPayload {
    /// Payload format for the bytes.
    format: PayloadFormat,
    /// Payload bytes.
    bytes: Vec<u8>,
}

impl ReceivedPayload {
    /// Create a received payload entry.
    fn new(format: PayloadFormat, bytes: Vec<u8>) -> Self {
        Self { format, bytes }
    }
}
