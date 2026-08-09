use std::sync::Arc;

use crate::protocol::message::Message;
use crate::protocol::payload::{Payload, PayloadId};
use crate::protocol::{CodecError, MessageCodec};
use crate::{ConnectionError, Transport};

/// Decoder and deferred-payload assembler for one message direction.
pub(crate) struct MessageReceiver {
    /// Negotiated message codec.
    codec: MessageCodec,
    /// Underlying message transport.
    transport: Arc<dyn Transport>,
    /// Largest assembled value payload in bytes.
    max_payload_bytes: usize,
    /// Message awaiting its contiguous payload chunks.
    pending: Option<PendingPayload>,
}

impl MessageReceiver {
    /// Create one message receiver.
    pub(crate) fn new(
        codec: MessageCodec,
        max_payload_bytes: usize,
        transport: Arc<dyn Transport>,
    ) -> Self {
        Self {
            codec,
            transport,
            max_payload_bytes,
            pending: None,
        }
    }

    /// Receive one complete message with its payload assembled.
    pub(crate) fn receive(&mut self) -> Result<Message, ConnectionError> {
        loop {
            let bytes = self.transport.receive()?;
            if let Some(message) = self.push(&bytes)? {
                return Ok(message);
            }
        }
    }

    /// Decode one transport message and return a fully assembled RPC message.
    pub(crate) fn push(&mut self, bytes: &[u8]) -> Result<Option<Message>, ConnectionError> {
        let mut message = self.codec.decode(bytes)?;

        // assemble the next required contiguous payload chunk
        if self.pending.is_some() {
            let Message::Chunk(chunk) = message else {
                return Err(ConnectionError::Protocol(
                    "deferred payload chunks are not contiguous".to_string(),
                ));
            };

            return self.push_chunk(chunk.payload, &chunk.bytes);
        }

        // reject chunks without a preceding deferred payload
        if matches!(message, Message::Chunk(_)) {
            return Err(ConnectionError::Protocol(
                "payload chunk has no deferred message".to_string(),
            ));
        }

        // return inline messages or begin bounded payload assembly
        let Some(payload) = message.payload_mut() else {
            return Ok(Some(message));
        };
        match payload {
            Payload::Inline(bytes) => {
                self.validate_size(bytes.len())?;

                Ok(Some(message))
            }
            Payload::Deferred { .. } => {
                let Some((id, byte_len)) = payload.deferred() else {
                    return Err(ConnectionError::Protocol(
                        "deferred payload has no identity".to_string(),
                    ));
                };
                let byte_len = usize::try_from(byte_len).map_err(|_| {
                    ConnectionError::Protocol(
                        "payload length does not fit this platform".to_string(),
                    )
                })?;
                self.validate_size(byte_len)?;
                if byte_len == 0 {
                    payload.resolve(Vec::new());

                    return Ok(Some(message));
                }
                self.pending = Some(PendingPayload {
                    message,
                    id,
                    byte_len,
                    bytes: Vec::with_capacity(byte_len),
                });

                Ok(None)
            }
        }
    }

    /// Append one chunk and return its completed owning message.
    fn push_chunk(
        &mut self,
        id: PayloadId,
        bytes: &[u8],
    ) -> Result<Option<Message>, ConnectionError> {
        let pending = self.pending.as_mut().ok_or_else(|| {
            ConnectionError::Protocol("payload chunk has no deferred message".to_string())
        })?;
        if pending.id != id {
            return Err(ConnectionError::Protocol(
                "payload chunk identity does not match its message".to_string(),
            ));
        }
        let assembled = pending
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| {
                ConnectionError::Protocol("assembled payload length overflowed".to_string())
            })?;
        if assembled > pending.byte_len {
            return Err(ConnectionError::Protocol(
                "payload chunks exceed their declared length".to_string(),
            ));
        }
        pending.bytes.extend_from_slice(bytes);
        if assembled < pending.byte_len {
            return Ok(None);
        }

        let Some(mut pending) = self.pending.take() else {
            return Err(ConnectionError::Protocol(
                "completed payload is not pending".to_string(),
            ));
        };
        let payload = pending.message.payload_mut().ok_or_else(|| {
            ConnectionError::Protocol("deferred message has no payload".to_string())
        })?;
        payload.resolve(std::mem::take(&mut pending.bytes));

        Ok(Some(pending.message))
    }

    /// Enforce the negotiated assembled payload limit.
    fn validate_size(&self, actual: usize) -> Result<(), ConnectionError> {
        if actual > self.max_payload_bytes {
            Err(ConnectionError::Codec(CodecError::PayloadTooLarge {
                limit: self.max_payload_bytes,
                actual,
            }))
        } else {
            Ok(())
        }
    }
}

impl std::fmt::Debug for MessageReceiver {
    /// Format this receiver without exposing its transport implementation.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MessageReceiver")
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}

/// One message awaiting its declared payload bytes.
#[derive(Debug)]
struct PendingPayload {
    /// Message containing the deferred payload.
    message: Message,
    /// Expected direction-local payload identifier.
    id: PayloadId,
    /// Declared exact payload byte length.
    byte_len: usize,
    /// Contiguous bytes assembled so far.
    bytes: Vec<u8>,
}
