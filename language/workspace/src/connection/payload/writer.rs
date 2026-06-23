use super::{PayloadSendError, PayloadWriteError};
use crate::Transport;
use crate::protocol::{
    BinaryPayload, PayloadBody, PayloadChunkNotification, PayloadId, ProtocolCodec,
    ProtocolCodecError, ProtocolLimits, ProtocolMessage, ProtocolNotification,
    WorkspaceNotification, inline_payload_max_bytes, payload_chunk_bytes,
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
                body: PayloadBody::Inline { bytes },
            });
        }

        // require enough room for chunked transfer
        let chunk_limit = payload_chunk_bytes(self.limits).map_err(PayloadWriteError::Codec)?;
        if chunk_limit == 0 {
            return Err(PayloadWriteError::ChunkLimitTooSmall);
        }

        // defer large payloads until after the response
        let id = self.allocate_payload_id();
        let total_bytes = bytes.len() as u64;
        self.pending.push(PendingPayload { id, bytes });

        Ok(BinaryPayload {
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
        let chunk_size = payload_chunk_bytes(self.limits).map_err(PayloadSendError::Codec)?;
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
                index,
                total,
                bytes: chunk.to_vec(),
                done,
            };
            let message = ProtocolMessage::Notification(Box::new(ProtocolNotification {
                payload: WorkspaceNotification::PayloadChunk(notification),
            }));
            let payload = codec
                .encode_message(&message)
                .map_err(PayloadSendError::Codec)?;
            transport
                .send(&payload)
                .map_err(PayloadSendError::Transport)?;
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
                payload: WorkspaceNotification::PayloadChunk(PayloadChunkNotification {
                    id: payload.id,
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
    /// Serialized payload bytes.
    bytes: Vec<u8>,
}
