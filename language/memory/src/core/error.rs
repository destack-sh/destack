use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// One memory result.
pub type MemoryResult<T> = Result<T, MemoryError>;

/// Memory operation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryError {
    /// One address-space operation failed.
    AddressSpaceFailed {
        /// The requested address-space byte length.
        byte_len: usize,
    },
    /// One byte range was outside one reserved address space.
    InvalidByteRange {
        /// The requested byte offset.
        start: usize,
        /// The requested byte length.
        len: usize,
        /// The reserved capacity in bytes.
        capacity: usize,
    },
    /// One internal memory invariant was violated.
    InvariantViolation {
        /// The violated invariant context.
        context: &'static str,
    },
}

impl Display for MemoryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::AddressSpaceFailed { byte_len } => {
                write!(
                    formatter,
                    "address space operation failed: {byte_len} bytes"
                )
            }
            Self::InvalidByteRange {
                start,
                len,
                capacity,
            } => {
                write!(
                    formatter,
                    "invalid memory byte range: start {start}, length {len}, capacity {capacity}"
                )
            }
            Self::InvariantViolation { context } => {
                write!(formatter, "memory invariant violation: {context}")
            }
        }
    }
}

impl Error for MemoryError {}
