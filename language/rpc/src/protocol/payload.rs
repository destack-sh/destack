use serde::{Deserialize, Serialize};
use tspp_serde::{Codec, Reflect};

/// Direction-local deferred payload identifier.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub(crate) struct PayloadId(pub(crate) u64);

impl PayloadId {
    /// Create one payload identifier.
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }
}

/// One encoded call value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) enum Payload {
    /// Bytes carried by their owning message.
    Inline(Vec<u8>),
    /// Bytes carried by immediately following chunk messages.
    Deferred {
        /// Direction-local payload identifier.
        id: PayloadId,
        /// Exact encoded byte length.
        byte_len: u64,
    },
}

impl Payload {
    /// Encode one typed value into an inline payload.
    pub(crate) fn encode<T: Codec>(value: &T) -> Result<Self, tspp_serde::Error> {
        let bytes = tspp_serde::to_vec(value)?;

        Ok(Self::Inline(bytes))
    }

    /// Decode one complete inline payload.
    pub(crate) fn decode<T: Codec>(&self) -> Result<T, PayloadError> {
        let Self::Inline(bytes) = self else {
            return Err(PayloadError::Deferred);
        };

        tspp_serde::from_slice(bytes).map_err(PayloadError::Decode)
    }

    /// Return the inline bytes when this payload is complete.
    pub(crate) fn inline(&self) -> Result<&[u8], PayloadError> {
        let Self::Inline(bytes) = self else {
            return Err(PayloadError::Deferred);
        };

        Ok(bytes)
    }

    /// Return this deferred payload's identity and exact byte length.
    pub(crate) fn deferred(&self) -> Option<(PayloadId, u64)> {
        match self {
            Self::Deferred { id, byte_len } => Some((*id, *byte_len)),
            Self::Inline(_) => None,
        }
    }

    /// Move inline bytes into one deferred payload declaration.
    pub(crate) fn defer(&mut self, id: PayloadId) -> Result<Vec<u8>, PayloadError> {
        let Self::Inline(bytes) = self else {
            return Err(PayloadError::Deferred);
        };
        let byte_len = bytes.len() as u64;
        let bytes = std::mem::take(bytes);
        *self = Self::Deferred { id, byte_len };

        Ok(bytes)
    }

    /// Replace this payload with its completely assembled inline bytes.
    pub(crate) fn resolve(&mut self, bytes: Vec<u8>) {
        *self = Self::Inline(bytes);
    }
}

/// One contiguous deferred payload chunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) struct PayloadChunk {
    /// Deferred payload receiving these bytes.
    pub(crate) payload: PayloadId,
    /// Next contiguous encoded bytes.
    pub(crate) bytes: Vec<u8>,
}

/// Failure to consume one encoded payload.
#[derive(Debug)]
pub(crate) enum PayloadError {
    /// The payload has not been assembled yet.
    Deferred,
    /// The payload could not be decoded as its method type.
    Decode(tspp_serde::Error),
}

impl std::fmt::Display for PayloadError {
    /// Format this payload failure.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Deferred => write!(formatter, "payload is deferred"),
            Self::Decode(error) => write!(formatter, "payload decode failed: {error}"),
        }
    }
}

impl std::error::Error for PayloadError {
    /// Return the underlying decode failure when one exists.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Decode(error) => Some(error),
            Self::Deferred => None,
        }
    }
}
