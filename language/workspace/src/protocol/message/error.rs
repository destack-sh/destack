use destack_serde::Schema;
use serde::{Deserialize, Serialize};

/// Protocol errors returned in responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct ProtocolError {
    /// Error code classification.
    pub code: ProtocolErrorCode,
    /// Human readable error message.
    pub message: String,
    /// Optional structured detail string.
    pub detail: Option<String>,
    /// Whether the request can be retried safely.
    pub retryable: bool,
    /// Optional retry delay in milliseconds.
    pub retry_after_ms: Option<u64>,
}

impl ProtocolError {
    /// Build a protocol error.
    pub fn new(code: ProtocolErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            detail: None,
            retryable: false,
            retry_after_ms: None,
        }
    }

    /// Build an internal protocol error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ProtocolErrorCode::Internal, message)
    }
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // format protocol errors
        match &self.detail {
            Some(detail) => write!(
                formatter,
                "{}: {} ({})",
                self.code.as_str(),
                self.message,
                detail
            ),
            None => write!(formatter, "{}: {}", self.code.as_str(), self.message),
        }
    }
}

impl std::error::Error for ProtocolError {}

/// Error code classification for protocol errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub enum ProtocolErrorCode {
    /// The request could not be parsed or validated.
    InvalidRequest,
    /// The payload format was invalid.
    InvalidPayload,
    /// The requested protocol version is unsupported.
    UnsupportedVersion,
    /// The requested resource was not found.
    NotFound,
    /// The request conflicts with the current state.
    Conflict,
    /// The server is busy and cannot service the request.
    Busy,
    /// The server is not ready for the request.
    NotReady,
    /// The request timed out.
    Timeout,
    /// The request was canceled.
    Canceled,
    /// The payload exceeded negotiated limits.
    TooLarge,
    /// The caller lacks permission.
    Unauthorized,
    /// The caller is forbidden from performing the action.
    Forbidden,
    /// An internal error occurred.
    Internal,
}

impl ProtocolErrorCode {
    /// Return the string identifier for this error code.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProtocolErrorCode::InvalidRequest => "invalid_request",
            ProtocolErrorCode::InvalidPayload => "invalid_payload",
            ProtocolErrorCode::UnsupportedVersion => "unsupported_version",
            ProtocolErrorCode::NotFound => "not_found",
            ProtocolErrorCode::Conflict => "conflict",
            ProtocolErrorCode::Busy => "busy",
            ProtocolErrorCode::NotReady => "not_ready",
            ProtocolErrorCode::Timeout => "timeout",
            ProtocolErrorCode::Canceled => "canceled",
            ProtocolErrorCode::TooLarge => "too_large",
            ProtocolErrorCode::Unauthorized => "unauthorized",
            ProtocolErrorCode::Forbidden => "forbidden",
            ProtocolErrorCode::Internal => "internal",
        }
    }
}
