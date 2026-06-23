use std::collections::HashMap;

use super::PayloadReceiveError;
use crate::ClientError;
use crate::protocol::{
    BinaryPayload, PayloadBody, PayloadChunkNotification, PayloadId, ProtocolNotification,
    WorkspaceNotification, WorkspaceQueryResponse, WorkspaceResponse,
};

/// Deferred payload chunks received while waiting for a response.
#[derive(Debug, Default)]
pub(crate) struct PayloadBuffer {
    /// Pending payload streams keyed by payload id.
    pending: HashMap<PayloadId, PayloadStream>,
    /// Completed payloads keyed by payload id.
    completed: HashMap<PayloadId, ReceivedPayload>,
}

impl PayloadBuffer {
    /// Create an empty payload buffer.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Ingest a protocol notification.
    pub(crate) fn ingest_notification(
        &mut self,
        notification: ProtocolNotification,
    ) -> Result<(), ClientError> {
        let WorkspaceNotification::PayloadChunk(chunk) = notification.payload else {
            return Ok(());
        };

        self.ingest_chunk(chunk)
    }

    /// Resolve payloads referenced by a response if possible.
    pub(crate) fn resolve_response(
        &mut self,
        response: &mut WorkspaceResponse,
    ) -> Result<bool, ClientError> {
        match response {
            WorkspaceResponse::QueryResult(query) => self.resolve_query_response(query),
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
                .or_insert_with(|| PayloadStream::new(total));
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
            let bytes = state.merge_chunks().map_err(ClientError::Payload)?;
            self.completed.insert(chunk.id, ReceivedPayload::new(bytes));
        }

        Ok(())
    }

    /// Resolve binary payloads in query responses.
    fn resolve_query_response(
        &mut self,
        response: &mut WorkspaceQueryResponse,
    ) -> Result<bool, ClientError> {
        match response {
            WorkspaceQueryResponse::Query(payload) => self.resolve_payload(&mut payload.payload),
            WorkspaceQueryResponse::QueryBatch(payloads) => {
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

    /// Resolve one deferred payload.
    fn resolve_payload(&mut self, payload: &mut BinaryPayload) -> Result<bool, ClientError> {
        let PayloadBody::Deferred { id, total_bytes } = payload.body else {
            return Ok(true);
        };

        let completed = match self.completed.remove(&id) {
            Some(completed) => completed,
            None => return Ok(false),
        };

        // validate payload size
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
    /// Total chunks expected.
    total: usize,
    /// Number of chunks received.
    received: usize,
    /// Chunk storage by index.
    chunks: Vec<Option<Vec<u8>>>,
}

impl PayloadStream {
    /// Create a new pending payload stream.
    fn new(total: usize) -> Self {
        Self {
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

/// Received payload bytes.
#[derive(Debug)]
struct ReceivedPayload {
    /// Payload bytes.
    bytes: Vec<u8>,
}

impl ReceivedPayload {
    /// Create a received payload entry.
    fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}
