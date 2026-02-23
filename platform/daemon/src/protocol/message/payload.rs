use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Binary payload wrapper for protocol message bodies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BinaryPayload {
    /// Payload format.
    pub format: PayloadFormat,
    /// Payload body.
    pub body: PayloadBody,
}

impl BinaryPayload {
    /// Build a JSON payload from a serde_json value.
    pub fn from_json_value(value: &serde_json::Value) -> Result<Self, serde_json::Error> {
        let bytes = serde_json::to_vec(value)?;
        Ok(Self {
            format: PayloadFormat::Json,
            body: PayloadBody::Inline { bytes },
        })
    }

    /// Decode a JSON payload into a serde_json value.
    pub fn to_json_value(&self) -> Result<serde_json::Value, BinaryPayloadDecodeError> {
        if self.format != PayloadFormat::Json {
            return Err(BinaryPayloadDecodeError::UnexpectedFormat {
                format: self.format,
            });
        }

        match &self.body {
            PayloadBody::Inline { bytes } => {
                serde_json::from_slice(bytes).map_err(BinaryPayloadDecodeError::InvalidJsonPayload)
            }
            PayloadBody::Deferred { .. } => Err(BinaryPayloadDecodeError::PayloadDeferred),
        }
    }
}

/// Error returned when decoding a binary payload into json.
#[derive(Debug)]
pub enum BinaryPayloadDecodeError {
    /// Payload format is not json.
    UnexpectedFormat {
        /// Actual payload format.
        format: PayloadFormat,
    },
    /// Payload body is deferred and cannot be decoded yet.
    PayloadDeferred,
    /// Payload bytes are not valid json.
    InvalidJsonPayload(serde_json::Error),
}

impl fmt::Display for BinaryPayloadDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedFormat { format } => {
                write!(formatter, "expected json payload, got {format:?}")
            }
            Self::PayloadDeferred => write!(formatter, "payload is deferred"),
            Self::InvalidJsonPayload(source) => write!(formatter, "invalid json payload: {source}"),
        }
    }
}

impl Error for BinaryPayloadDecodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidJsonPayload(source) => Some(source),
            _ => None,
        }
    }
}

/// Payload format identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PayloadFormat {
    /// Postcard binary payload.
    Postcard,
    /// Json payload.
    Json,
}

/// Payload body representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PayloadBody {
    /// Inline bytes.
    Inline { bytes: Vec<u8> },
    /// Deferred payload identified by id.
    Deferred { id: PayloadId, total_bytes: u64 },
}

/// Unique identifier for payload transfers.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PayloadId(pub u64);

impl PayloadId {
    /// Wrap a raw payload id.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Notification for chunked payload data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PayloadChunkNotification {
    /// Payload id for the chunk stream.
    pub id: PayloadId,
    /// Payload format.
    pub format: PayloadFormat,
    /// Zero based chunk index.
    pub index: u32,
    /// Total chunks expected.
    pub total: u32,
    /// Chunk bytes.
    pub bytes: Vec<u8>,
    /// Whether this chunk is the final chunk.
    pub done: bool,
}
