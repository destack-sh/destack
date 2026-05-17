use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    Global, Instruction, Local, LocalNodeId, MemoryFlags, MemoryOrdering, MemoryScope, Space,
    SyncScope, Value,
};

/// Table of memory metadata entries.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MemoryMetadata {
    /// Memory access metadata keyed by instruction id.
    pub memory_accesses_by_instruction_id:
        HashMap<LocalNodeId<Instruction>, Vec<MemoryAccessMetadata>>,
}

impl MemoryMetadata {
    /// Create a new empty memory table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return memory access metadata for an instruction id.
    pub fn memory_accesses(
        &self,
        instruction: LocalNodeId<Instruction>,
    ) -> Option<&[MemoryAccessMetadata]> {
        self.memory_accesses_by_instruction_id
            .get(&instruction)
            .map(|accesses| accesses.as_slice())
    }

    /// Return mutable memory access metadata for an instruction id.
    pub fn memory_accesses_mut(
        &mut self,
        instruction: LocalNodeId<Instruction>,
    ) -> Option<&mut Vec<MemoryAccessMetadata>> {
        self.memory_accesses_by_instruction_id.get_mut(&instruction)
    }

    /// Insert memory access metadata for an instruction id.
    pub fn insert_memory_accesses(
        &mut self,
        instruction: LocalNodeId<Instruction>,
        accesses: Vec<MemoryAccessMetadata>,
    ) -> Option<Vec<MemoryAccessMetadata>> {
        self.memory_accesses_by_instruction_id
            .insert(instruction, accesses)
    }

    /// Remove memory access metadata for an instruction id.
    pub fn remove_memory_accesses(
        &mut self,
        instruction: LocalNodeId<Instruction>,
    ) -> Option<Vec<MemoryAccessMetadata>> {
        self.memory_accesses_by_instruction_id.remove(&instruction)
    }
}

/// Metadata describing a single memory access in an instruction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryAccessMetadata {
    /// The kind of access performed.
    pub kind: MemoryAccessKind,
    /// The memory target for the access.
    pub target: MemoryAccessTarget,
    /// The number of bytes accessed when known.
    pub size: Option<u64>,
    /// Alignment in bytes, when known.
    pub alignment: Option<u32>,
    /// Whether the access is volatile.
    pub is_volatile: bool,
    /// Whether repeated loads observe the same value.
    pub is_load_invariant: bool,
    /// Memory ordering for atomic accesses.
    pub ordering: Option<MemoryOrdering>,
    /// Synchronization scope for atomic accesses and fences.
    pub scope: Option<SyncScope>,
    /// Memory visibility scope for fences.
    pub memory_scope: Option<MemoryScope>,
    /// Memory flags for fences.
    pub flags: Option<MemoryFlags>,
    /// Space override for the access.
    pub space: Option<Space>,
}

/// The kind of memory access represented by metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryAccessKind {
    /// Reads memory.
    Read,
    /// Writes memory.
    Write,
    /// Reads and writes memory.
    ReadWrite,
    /// Read-modify-write memory access.
    ReadModifyWrite,
    /// Atomic memory fence.
    Fence,
    /// Prefetch hint for reading.
    PrefetchRead,
    /// Prefetch hint for writing.
    PrefetchWrite,
}

/// Target of a memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryAccessTarget {
    /// Access through a pointer value.
    Pointer(Value),
    /// Access through a local slot.
    Local(LocalNodeId<Local>),
    /// Access through a global.
    Global(LocalNodeId<Global>),
    /// Access with unknown target.
    Unknown,
}

/// Memory behavior for one call argument.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CallArgumentEffect {
    /// Access mode for this argument.
    pub access: ArgumentAccess,
    /// Escape behavior for this argument.
    pub escape: ArgumentEscape,
}

/// Access mode for a pointer argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ArgumentAccess {
    /// The argument is not accessed.
    None,
    /// The argument is only read.
    Read,
    /// The argument is only written.
    Write,
    /// The argument is read and written.
    #[default]
    ReadWrite,
}

/// Escape behavior for a call argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ArgumentEscape {
    /// The argument does not escape the callee.
    None,
    /// The argument only escapes through the return value.
    Return,
    /// The argument may escape in an unknown way.
    #[default]
    Escape,
}

/// Allocation size information for functions returning newly allocated memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AllocationSize {
    /// The parameter index containing the element size in bytes.
    pub stride_index: u32,
    /// The parameter index containing the element count, if any.
    pub element_count_index: Option<u32>,
}

impl AllocationSize {
    /// Create an alloc size description from parameter indices.
    pub fn new(stride_index: u32, element_count_index: Option<u32>) -> Self {
        Self {
            stride_index,
            element_count_index,
        }
    }
}
