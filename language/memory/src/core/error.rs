use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// One memory result.
pub type MemoryResult<T> = Result<T, MemoryError>;

/// Memory operation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryError {
    /// One platform memory operation failed.
    SystemError {
        /// The failed platform operation.
        operation: MemoryOperation,
        /// The platform error code when available.
        code: Option<i32>,
        /// The requested byte length.
        byte_len: usize,
    },
    /// One process-wide memory table reached capacity.
    CapacityExceeded {
        /// The table that reached capacity.
        table: MemoryTable,
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
    /// One internal memory invariant was violated.
    InvariantViolation {
        /// The violated invariant context.
        context: &'static str,
    },
}

impl MemoryError {
    /// Create one system error without a platform error code.
    pub const fn system(operation: MemoryOperation, byte_len: usize) -> Self {
        Self::SystemError {
            operation,
            code: None,
            byte_len,
        }
    }

    /// Create one system error with a platform error code.
    pub const fn system_with_code(
        operation: MemoryOperation,
        code: Option<i32>,
        byte_len: usize,
    ) -> Self {
        Self::SystemError {
            operation,
            code,
            byte_len,
        }
    }
}

/// A platform memory operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryOperation {
    /// Create backing storage for page frames.
    CreateFrameAllocator,
    /// Extend backing storage for page frames.
    ExtendFrameAllocator,
    /// Copy mapped bytes into backing frame storage.
    CopyFrameStorage,
    /// Reserve virtual address space.
    ReserveAddressSpace,
    /// Map one page frame range into virtual address space.
    MapFrameRange,
    /// Change virtual page protection.
    ProtectPages,
}

/// A bounded process-wide memory table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryTable {
    /// The write-watch registration table.
    WriteWatch,
}

impl Display for MemoryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::SystemError {
                operation,
                code,
                byte_len,
            } => {
                if let Some(code) = code {
                    write!(
                        formatter,
                        "memory system operation failed: {operation}, code {code}, {byte_len} bytes"
                    )
                } else {
                    write!(
                        formatter,
                        "memory system operation failed: {operation}, {byte_len} bytes"
                    )
                }
            }
            Self::CapacityExceeded { table, capacity } => {
                write!(
                    formatter,
                    "memory table capacity exceeded: {table}, capacity {capacity}"
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

impl Display for MemoryOperation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateFrameAllocator => write!(formatter, "create frame allocator"),
            Self::ExtendFrameAllocator => write!(formatter, "extend frame allocator"),
            Self::CopyFrameStorage => write!(formatter, "copy frame storage"),
            Self::ReserveAddressSpace => write!(formatter, "reserve address space"),
            Self::MapFrameRange => write!(formatter, "map frame range"),
            Self::ProtectPages => write!(formatter, "protect pages"),
        }
    }
}

impl Display for MemoryTable {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WriteWatch => write!(formatter, "write watch"),
        }
    }
}
