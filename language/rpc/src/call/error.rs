use std::sync::Arc;

use crate::{ConnectionError, Status};

/// Failure while using one active RPC call.
#[derive(Debug)]
pub enum CallError {
    /// The request or stream value could not be encoded.
    Encode(tspp_serde::Error),
    /// The response or stream value could not be decoded.
    Decode(tspp_serde::Error),
    /// The RPC connection failed.
    Connection(Arc<ConnectionError>),
    /// The service completed the call unsuccessfully.
    Status(Status),
    /// The peer canceled the call.
    Canceled,
    /// The call is already complete.
    Complete,
    /// Output items must be consumed before reading the terminal response.
    OutputPending,
}

impl std::fmt::Display for CallError {
    /// Format this call failure.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encode(error) => write!(formatter, "RPC value encode failed: {error}"),
            Self::Decode(error) => write!(formatter, "RPC value decode failed: {error}"),
            Self::Connection(error) => write!(formatter, "{error}"),
            Self::Status(status) => write!(formatter, "{status}"),
            Self::Canceled => write!(formatter, "RPC call was canceled"),
            Self::Complete => write!(formatter, "RPC call is complete"),
            Self::OutputPending => write!(formatter, "RPC call has unread output"),
        }
    }
}

impl std::error::Error for CallError {
    /// Return the underlying call failure when one exists.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Encode(error) | Self::Decode(error) => Some(error),
            Self::Connection(error) => Some(error.as_ref()),
            Self::Status(status) => Some(status),
            Self::Canceled | Self::Complete | Self::OutputPending => None,
        }
    }
}
