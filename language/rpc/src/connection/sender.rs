use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

use crate::protocol::message::Message;
use crate::protocol::payload::{PayloadChunk, PayloadId};
use crate::protocol::{CodecError, MessageCodec};
use crate::{ConnectionError, Transport};

/// Reserved bytes for the encoded chunk sequence length.
const CHUNK_LENGTH_RESERVE: usize = 16;

/// Shared encoder and transport for connection messages.
#[derive(Clone)]
pub(crate) struct MessageSender {
    /// Negotiated message codec.
    codec: MessageCodec,
    /// Underlying message transport.
    transport: Arc<dyn Transport>,
    /// Largest assembled value payload in bytes.
    max_payload_bytes: usize,
    /// Next direction-local deferred payload identifier.
    next_payload: Arc<AtomicU64>,
    /// Serializes complete messages and their contiguous chunks.
    sending: Arc<Mutex<()>>,
}

impl MessageSender {
    /// Create one message sender.
    pub(crate) fn new(
        codec: MessageCodec,
        max_payload_bytes: usize,
        transport: Arc<dyn Transport>,
    ) -> Self {
        Self {
            codec,
            transport,
            max_payload_bytes,
            next_payload: Arc::new(AtomicU64::new(1)),
            sending: Arc::new(Mutex::new(())),
        }
    }

    /// Encode and send one protocol message.
    pub(crate) fn send(&self, mut message: Message) -> Result<(), ConnectionError> {
        let _sending = self.sending.lock();

        // enforce assembled payload limits before choosing inline or deferred encoding
        if let Some(payload) = message.payload_mut()
            && let Ok(bytes) = payload.inline()
            && bytes.len() > self.max_payload_bytes
        {
            return Err(ConnectionError::Codec(CodecError::PayloadTooLarge {
                limit: self.max_payload_bytes,
                actual: bytes.len(),
            }));
        }

        // send messages that already fit without allocating payload chunks
        match self.codec.encode(&message) {
            Ok(bytes) => return self.send_bytes(&bytes),
            Err(CodecError::TooLarge { .. }) => {}
            Err(error) => return Err(ConnectionError::Codec(error)),
        }

        // defer the one value payload responsible for the oversized message
        let payload = message.payload_mut().ok_or_else(|| {
            ConnectionError::Protocol("oversized message has no deferrable payload".to_string())
        })?;
        let id = PayloadId::new(self.next_payload.fetch_add(1, Ordering::Relaxed));
        let bytes = payload
            .defer(id)
            .map_err(|_| ConnectionError::Protocol("payload is already deferred".to_string()))?;
        if bytes.len() > self.max_payload_bytes {
            return Err(ConnectionError::Codec(CodecError::PayloadTooLarge {
                limit: self.max_payload_bytes,
                actual: bytes.len(),
            }));
        }

        let message = self.codec.encode(&message)?;
        self.send_bytes(&message)?;
        self.send_chunks(id, &bytes)
    }

    /// Close the underlying transport.
    pub(crate) fn close(&self) -> Result<(), ConnectionError> {
        self.transport.close().map_err(ConnectionError::Transport)
    }

    /// Return the largest assembled value payload in bytes.
    pub(crate) const fn max_payload_bytes(&self) -> usize {
        self.max_payload_bytes
    }

    /// Encode and send every contiguous chunk for one deferred payload.
    fn send_chunks(&self, payload: PayloadId, bytes: &[u8]) -> Result<(), ConnectionError> {
        let empty = Message::Chunk(PayloadChunk {
            payload,
            bytes: Vec::new(),
        });
        let overhead = self.codec.encode(&empty)?.len();
        let chunk_bytes = self
            .codec
            .max_message_bytes()
            .checked_sub(overhead + CHUNK_LENGTH_RESERVE)
            .filter(|chunk_bytes| *chunk_bytes > 0)
            .ok_or_else(|| {
                ConnectionError::Protocol(
                    "message limit cannot hold a deferred payload chunk".to_string(),
                )
            })?;

        // chunks remain contiguous under the sender lock
        for bytes in bytes.chunks(chunk_bytes) {
            let chunk = Message::Chunk(PayloadChunk {
                payload,
                bytes: bytes.to_vec(),
            });
            let chunk = self.codec.encode(&chunk)?;
            self.send_bytes(&chunk)?;
        }

        Ok(())
    }

    /// Send one already encoded message.
    fn send_bytes(&self, bytes: &[u8]) -> Result<(), ConnectionError> {
        self.transport
            .send(bytes)
            .map_err(ConnectionError::Transport)
    }
}

impl std::fmt::Debug for MessageSender {
    /// Format this sender without exposing its transport implementation.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MessageSender")
            .finish_non_exhaustive()
    }
}
