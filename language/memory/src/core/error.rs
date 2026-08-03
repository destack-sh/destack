use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

/// One memory result.
pub type MemoryResult<T> = Result<T, MemoryError>;

/// Memory operation failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryError {
    /// One platform memory operation failed.
    System {
        /// The failed platform operation.
        operation: MemoryOperation,
        /// The platform error code when available.
        code: Option<i32>,
        /// The requested byte length.
        byte_len: Option<usize>,
    },
    /// Too many write watched memory ranges are active.
    WatchLimitExceeded {
        /// The maximum number of entries.
        capacity: usize,
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
    /// One byte range did not satisfy its required alignment.
    UnalignedRange {
        /// The requested byte offset.
        offset: usize,
        /// The requested byte length.
        byte_len: usize,
        /// The required byte alignment.
        alignment: usize,
    },
    /// One logical memory range could not fit in the reserved map.
    RangeExhausted {
        /// The requested byte length.
        byte_len: usize,
        /// The requested byte alignment.
        alignment: usize,
        /// The reserved map capacity.
        capacity: usize,
    },
    /// One claimed logical range overlaps live storage.
    RangeOccupied {
        /// The claimed byte offset.
        offset: usize,
        /// The claimed byte length.
        byte_len: usize,
    },
    /// One released logical range was not allocated by this map.
    InvalidRelease {
        /// The released byte offset.
        offset: usize,
        /// The released byte length.
        byte_len: usize,
    },
    /// One write targeted immutable memory.
    ImmutableRange {
        /// The immutable byte offset.
        offset: usize,
        /// The immutable byte length.
        byte_len: usize,
    },
    /// One serialized memory image is malformed or incompatible.
    InvalidImage {
        /// The invalid image context.
        context: String,
    },
    /// One internal memory error occurred.
    Internal {
        /// The internal error context.
        context: String,
    },
}

impl MemoryError {
    /// Create one system error.
    pub const fn system(
        operation: MemoryOperation,
        code: Option<i32>,
        byte_len: Option<usize>,
    ) -> Self {
        Self::System {
            operation,
            code,
            byte_len,
        }
    }

    /// Create one system error with byte context.
    pub const fn system_bytes(
        operation: MemoryOperation,
        code: Option<i32>,
        byte_len: usize,
    ) -> Self {
        Self::system(operation, code, Some(byte_len))
    }

    /// Create one internal memory error.
    pub fn internal(context: impl Into<String>) -> Self {
        Self::Internal {
            context: context.into(),
        }
    }

    /// Create one invalid memory image error.
    pub fn invalid_image(context: impl Into<String>) -> Self {
        Self::InvalidImage {
            context: context.into(),
        }
    }
}

/// A platform memory operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryOperation {
    /// Create backing storage for page frames.
    CreateFrameAllocator,
    /// Extend backing storage for page frames.
    ExtendFrameAllocator,
    /// Copy mapped bytes into backing frame storage.
    CopyFrameStorage,
    /// Reserve one virtual memory map.
    ReserveMemoryMap,
    /// Map one page frame range into virtual address space.
    MapFrameRange,
    /// Change virtual page protection.
    ProtectPages,
    /// Install the process wide write watch handler.
    InstallWriteWatch,
}

impl Display for MemoryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::System {
                operation,
                code,
                byte_len,
            } => match (code, byte_len) {
                (Some(code), Some(byte_len)) => {
                    write!(
                        formatter,
                        "memory system operation failed: {operation}, code {code}, {byte_len} bytes"
                    )
                }
                (Some(code), None) => {
                    write!(
                        formatter,
                        "memory system operation failed: {operation}, code {code}"
                    )
                }
                (None, Some(byte_len)) => {
                    write!(
                        formatter,
                        "memory system operation failed: {operation}, {byte_len} bytes"
                    )
                }
                (None, None) => write!(formatter, "memory system operation failed: {operation}"),
            },
            Self::WatchLimitExceeded { capacity } => {
                write!(
                    formatter,
                    "too many write watched memory ranges: capacity {capacity}"
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
            Self::UnalignedRange {
                offset,
                byte_len,
                alignment,
            } => {
                write!(
                    formatter,
                    "unaligned memory range: offset {offset}, length {byte_len}, alignment {alignment}"
                )
            }
            Self::RangeExhausted {
                byte_len,
                alignment,
                capacity,
            } => {
                write!(
                    formatter,
                    "memory range exhausted: {byte_len} bytes aligned to {alignment}, capacity {capacity}"
                )
            }
            Self::RangeOccupied { offset, byte_len } => {
                write!(
                    formatter,
                    "memory range occupied: offset {offset}, length {byte_len}"
                )
            }
            Self::InvalidRelease { offset, byte_len } => {
                write!(
                    formatter,
                    "invalid memory range release: offset {offset}, length {byte_len}"
                )
            }
            Self::ImmutableRange { offset, byte_len } => {
                write!(
                    formatter,
                    "immutable memory range: offset {offset}, length {byte_len}"
                )
            }
            Self::InvalidImage { context } => {
                write!(formatter, "invalid memory image: {context}")
            }
            Self::Internal { context } => {
                write!(formatter, "internal memory error: {context}")
            }
        }
    }
}

impl Error for MemoryError {}

impl Display for MemoryOperation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateFrameAllocator => write!(formatter, "create frame allocator"),
            Self::ExtendFrameAllocator => write!(formatter, "extend frame allocator"),
            Self::CopyFrameStorage => write!(formatter, "copy frame storage"),
            Self::ReserveMemoryMap => write!(formatter, "reserve memory map"),
            Self::MapFrameRange => write!(formatter, "map frame range"),
            Self::ProtectPages => write!(formatter, "protect pages"),
            Self::InstallWriteWatch => write!(formatter, "install write watch"),
        }
    }
}
