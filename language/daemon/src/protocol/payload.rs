use std::collections::HashMap;

use super::{
    BinaryPayload, CommandResponse, DaemonNotification, DaemonQueryResponse, DaemonResponse,
    PayloadBody, PayloadChunkNotification, PayloadFormat, PayloadId, ProtocolClientError,
    ProtocolNotification,
};

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
    ) -> Result<(), ProtocolClientError> {
        let DaemonNotification::PayloadChunk(chunk) = notification.payload else {
            return Ok(());
        };

        self.ingest_chunk(chunk)
    }

    /// Resolve payloads referenced by a response if possible.
    pub(crate) fn resolve_response(
        &mut self,
        response: &mut DaemonResponse,
    ) -> Result<bool, ProtocolClientError> {
        match response {
            DaemonResponse::QueryResult(query) => self.resolve_query_response(query),
            DaemonResponse::CommandResult(command) => self.resolve_command_response(command),
            _ => Ok(true),
        }
    }

    /// Ingest a payload chunk.
    fn ingest_chunk(&mut self, chunk: PayloadChunkNotification) -> Result<(), ProtocolClientError> {
        let total = usize::try_from(chunk.total)
            .map_err(|_| ProtocolClientError::Payload(PayloadReceiveError::ChunkTotalOverflow))?;
        if total == 0 {
            return Err(ProtocolClientError::Payload(
                PayloadReceiveError::ChunkTotalMustBeNonZero,
            ));
        }
        let index = usize::try_from(chunk.index)
            .map_err(|_| ProtocolClientError::Payload(PayloadReceiveError::ChunkIndexOverflow))?;
        if index >= total {
            return Err(ProtocolClientError::Payload(
                PayloadReceiveError::ChunkIndexOutOfRange,
            ));
        }
        if chunk.done && index + 1 != total {
            return Err(ProtocolClientError::Payload(
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
                return Err(ProtocolClientError::Payload(
                    PayloadReceiveError::ChunkFormatMismatch,
                ));
            }
            if state.total != total {
                return Err(ProtocolClientError::Payload(
                    PayloadReceiveError::ChunkTotalMismatch,
                ));
            }
            if state.chunks[index].is_some() {
                return Err(ProtocolClientError::Payload(
                    PayloadReceiveError::DuplicatePayloadChunk,
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
                    PayloadReceiveError::PayloadStateMissing,
                ))?;
            let format = state.format;
            let bytes = state.merge_chunks().map_err(ProtocolClientError::Payload)?;
            self.completed
                .insert(chunk.id, ReceivedPayload::new(format, bytes));
        }

        Ok(())
    }

    /// Resolve binary payloads in query responses.
    fn resolve_query_response(
        &mut self,
        response: &mut DaemonQueryResponse,
    ) -> Result<bool, ProtocolClientError> {
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
    ) -> Result<bool, ProtocolClientError> {
        let Some(payload) = response.data.as_mut() else {
            return Ok(true);
        };

        self.resolve_payload(payload)
    }

    /// Resolve one deferred payload.
    fn resolve_payload(
        &mut self,
        payload: &mut BinaryPayload,
    ) -> Result<bool, ProtocolClientError> {
        let PayloadBody::Deferred { id, total_bytes } = payload.body else {
            return Ok(true);
        };

        let completed = match self.completed.remove(&id) {
            Some(completed) => completed,
            None => return Ok(false),
        };

        // validate payload metadata
        if completed.format != payload.format {
            return Err(ProtocolClientError::Payload(
                PayloadReceiveError::PayloadFormatMismatch,
            ));
        }
        if completed.bytes.len() != total_bytes as usize {
            return Err(ProtocolClientError::Payload(
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
