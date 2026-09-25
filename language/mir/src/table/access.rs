use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::AtomicAccess;

/// The operation performed by a memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum MemoryOperation {
    /// Reads memory.
    Read,
    /// Writes memory.
    Write,
    /// Reads and writes memory.
    ReadWrite,
}

/// Ordering constraints for one memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum MemoryAccessOrder {
    /// Ordinary memory access.
    Plain,
    /// Externally observable memory access.
    Volatile,
    /// Atomic memory access.
    Atomic(AtomicAccess),
}
