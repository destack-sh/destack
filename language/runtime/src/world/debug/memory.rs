use destack_program as program;
use serde::{Deserialize, Serialize};

/// Memory access operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryAccess {
    /// Memory read.
    Read,
    /// Memory write.
    Write,
    /// Allocation event.
    Allocate,
    /// Free event.
    Free,
}

/// Memory target selected by one watchpoint or probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryTarget {
    /// Any memory target.
    Any,
    /// Exact program address.
    Address(program::ProgramAddress),
    /// Program type.
    Type(program::TypeId),
    /// Program point that performs an allocation.
    Allocation(program::ProgramPoint),
}
