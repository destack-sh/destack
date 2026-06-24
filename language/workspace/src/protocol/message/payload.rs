use destack_serde::{Codec, Reflect};
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Binary payload wrapper for protocol message bodies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct BinaryPayload {
    /// Payload body.
    pub body: PayloadBody,
}

impl BinaryPayload {
    /// Encode one value as a binary payload.
    pub fn from_value<T: Codec>(value: &T) -> Result<Self, destack_serde::Error> {
        let bytes = destack_serde::to_vec(value)?;

        Ok(Self {
            body: PayloadBody::Inline { bytes },
        })
    }

    /// Decode this payload as one value.
    pub fn to_value<T: Codec>(&self) -> Result<T, BinaryPayloadDecodeError> {
        match &self.body {
            PayloadBody::Inline { bytes } => {
                destack_serde::from_slice(bytes).map_err(BinaryPayloadDecodeError::InvalidPayload)
            }
            PayloadBody::Deferred { .. } => Err(BinaryPayloadDecodeError::PayloadDeferred),
        }
    }
}

/// Error returned when decoding a binary payload.
#[derive(Debug)]
pub enum BinaryPayloadDecodeError {
    /// Payload body is deferred and cannot be decoded yet.
    PayloadDeferred,
    /// Payload bytes are not valid.
    InvalidPayload(destack_serde::Error),
}

impl fmt::Display for BinaryPayloadDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PayloadDeferred => write!(formatter, "payload is deferred"),
            Self::InvalidPayload(source) => write!(formatter, "invalid binary payload: {source}"),
        }
    }
}

impl Error for BinaryPayloadDecodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidPayload(source) => Some(source),
            _ => None,
        }
    }
}

/// Payload body representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum PayloadBody {
    /// Inline bytes.
    Inline { bytes: Vec<u8> },
    /// Deferred payload identified by id.
    Deferred { id: PayloadId, total_bytes: u64 },
}

/// Unique identifier for payload transfers.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct PayloadId(pub u64);

impl PayloadId {
    /// Wrap a raw payload id.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Notification for chunked payload data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct PayloadChunkNotification {
    /// Payload id for the chunk stream.
    pub id: PayloadId,
    /// Zero based chunk index.
    pub index: u32,
    /// Total chunks expected.
    pub total: u32,
    /// Chunk bytes.
    pub bytes: Vec<u8>,
    /// Whether this chunk is the final chunk.
    pub done: bool,
}
