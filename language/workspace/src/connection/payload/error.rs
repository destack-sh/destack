use crate::connection::TransportError;
use crate::protocol::ProtocolCodecError;

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
    /// Chunk total differs from the existing stream.
    ChunkTotalMismatch,
    /// Chunk was already received.
    DuplicatePayloadChunk,
    /// Payload state is missing at finalize.
    PayloadStateMissing,
    /// Payload chunks are missing at finalize.
    PayloadChunksMissingAtFinalize,
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
            Self::ChunkTotalMismatch => write!(formatter, "chunk total mismatch"),
            Self::DuplicatePayloadChunk => write!(formatter, "duplicate payload chunk"),
            Self::PayloadStateMissing => write!(formatter, "payload state missing"),
            Self::PayloadChunksMissingAtFinalize => {
                write!(formatter, "payload chunks missing at finalize")
            }
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
    /// Transport error.
    Transport(TransportError),
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
            Self::Transport(error) => write!(formatter, "transport error: {error}"),
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
            Self::Transport(error) => Some(error),
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
