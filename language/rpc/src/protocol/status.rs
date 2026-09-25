use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::Metadata;

/// Terminal RPC call failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Status {
    /// Stable failure classification.
    pub code: Code,
    /// Human-readable failure description.
    pub message: String,
    /// Encoded application-specific failure details.
    pub details: Vec<u8>,
    /// Terminal failure metadata.
    pub metadata: Metadata,
}

impl Status {
    /// Create one call failure.
    pub fn new(code: Code, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: Vec::new(),
            metadata: Metadata::new(),
        }
    }

    /// Set encoded application-specific failure details.
    pub fn with_details(mut self, details: Vec<u8>) -> Self {
        self.details = details;

        self
    }

    /// Create one internal call failure.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(Code::Internal, message)
    }
}

impl std::fmt::Display for Status {
    /// Format this call failure.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for Status {}

/// Stable RPC call failure classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Code {
    /// The caller canceled the call.
    Canceled,
    /// An unknown failure occurred.
    Unknown,
    /// The request is not valid for the called method.
    InvalidArgument,
    /// The call exceeded its deadline.
    DeadlineExceeded,
    /// The requested entity does not exist.
    NotFound,
    /// The requested entity already exists.
    AlreadyExists,
    /// The caller does not have permission to perform the operation.
    PermissionDenied,
    /// A negotiated resource limit was exceeded.
    ResourceExhausted,
    /// The operation cannot run in the current system state.
    FailedPrecondition,
    /// The operation was aborted by a concurrent change.
    Aborted,
    /// The operation addressed a value outside its valid range.
    OutOfRange,
    /// The service does not implement the requested operation.
    Unimplemented,
    /// The service failed unexpectedly.
    Internal,
    /// The service is temporarily unavailable.
    Unavailable,
    /// Unrecoverable encoded data was lost or corrupted.
    DataLoss,
    /// The request does not carry valid authentication credentials.
    Unauthenticated,
}

impl Code {
    /// Return this status code's stable text name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Canceled => "canceled",
            Self::Unknown => "unknown",
            Self::InvalidArgument => "invalid_argument",
            Self::DeadlineExceeded => "deadline_exceeded",
            Self::NotFound => "not_found",
            Self::AlreadyExists => "already_exists",
            Self::PermissionDenied => "permission_denied",
            Self::ResourceExhausted => "resource_exhausted",
            Self::FailedPrecondition => "failed_precondition",
            Self::Aborted => "aborted",
            Self::OutOfRange => "out_of_range",
            Self::Unimplemented => "unimplemented",
            Self::Internal => "internal",
            Self::Unavailable => "unavailable",
            Self::DataLoss => "data_loss",
            Self::Unauthenticated => "unauthenticated",
        }
    }
}
