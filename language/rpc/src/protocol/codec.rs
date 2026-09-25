use super::ProtocolVersion;
use super::message::Message;

/// RPC message encoder and decoder.
#[derive(Debug, Clone)]
pub(crate) struct MessageCodec {
    /// Exact negotiated wire grammar.
    version: ProtocolVersion,
    /// Largest encoded message in bytes.
    max_message_bytes: usize,
}

impl MessageCodec {
    /// Create one message codec with an exact byte limit.
    pub(crate) fn new(version: ProtocolVersion, max_message_bytes: usize) -> Self {
        Self {
            version,
            max_message_bytes,
        }
    }

    /// Return the exact encoded message byte limit.
    pub(crate) const fn max_message_bytes(&self) -> usize {
        self.max_message_bytes
    }

    /// Encode one RPC message.
    pub(crate) fn encode(&self, message: &Message) -> Result<Vec<u8>, CodecError> {
        self.validate_version()?;
        let bytes = tspp_serde::to_vec(message).map_err(CodecError::Encode)?;

        // reject messages beyond the negotiated limit
        if bytes.len() > self.max_message_bytes {
            return Err(CodecError::TooLarge {
                limit: self.max_message_bytes,
                actual: bytes.len(),
            });
        }

        Ok(bytes)
    }

    /// Decode one RPC message.
    pub(crate) fn decode(&self, bytes: &[u8]) -> Result<Message, CodecError> {
        self.validate_version()?;

        // reject messages beyond the negotiated limit
        if bytes.len() > self.max_message_bytes {
            return Err(CodecError::TooLarge {
                limit: self.max_message_bytes,
                actual: bytes.len(),
            });
        }

        tspp_serde::from_slice(bytes).map_err(CodecError::Decode)
    }

    /// Reject wire grammars not implemented by this codec.
    fn validate_version(&self) -> Result<(), CodecError> {
        if self.version == ProtocolVersion::CURRENT {
            Ok(())
        } else {
            Err(CodecError::UnsupportedVersion(self.version))
        }
    }
}

/// Failure to encode or decode one RPC message.
#[derive(Debug)]
pub enum CodecError {
    /// The selected wire grammar is not implemented by this codec.
    UnsupportedVersion(ProtocolVersion),
    /// Message encoding failed.
    Encode(tspp_serde::Error),
    /// Message decoding failed.
    Decode(tspp_serde::Error),
    /// Encoded message exceeded the negotiated limit.
    TooLarge {
        /// Negotiated byte limit.
        limit: usize,
        /// Actual encoded byte length.
        actual: usize,
    },
    /// One assembled value exceeded the negotiated payload limit.
    PayloadTooLarge {
        /// Negotiated byte limit.
        limit: usize,
        /// Actual encoded payload byte length.
        actual: usize,
    },
}

impl std::fmt::Display for CodecError {
    /// Format this codec failure.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => {
                write!(
                    formatter,
                    "RPC protocol version {version:?} is not implemented"
                )
            }
            Self::Encode(error) => write!(formatter, "RPC message encode failed: {error}"),
            Self::Decode(error) => write!(formatter, "RPC message decode failed: {error}"),
            Self::TooLarge { limit, actual } => {
                write!(formatter, "RPC message exceeds {limit} bytes: {actual}")
            }
            Self::PayloadTooLarge { limit, actual } => {
                write!(formatter, "RPC payload exceeds {limit} bytes: {actual}")
            }
        }
    }
}

impl std::error::Error for CodecError {
    /// Return the underlying serialization failure when one exists.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Encode(error) | Self::Decode(error) => Some(error),
            Self::UnsupportedVersion(_) | Self::TooLarge { .. } | Self::PayloadTooLarge { .. } => {
                None
            }
        }
    }
}
