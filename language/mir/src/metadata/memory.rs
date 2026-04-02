use std::collections::HashMap;
use std::mem::size_of;

use serde::{Deserialize, Serialize};

use crate::{Instruction, LocalNodeId, MemoryAccessMetadata};

use super::{AliasScopeId, AliasScopeTable, TbaaTable, TbaaTagId};

/// Capture behavior for a pointer argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CaptureKind {
    /// The argument does not escape the callee.
    NoCapture,
    /// The argument only escapes through the return value.
    ReturnOnly,
    /// The argument is stored somewhere reachable from the caller.
    Store,
    /// The argument may escape in an unknown way.
    #[default]
    Escape,
}

/// Allocation size information for functions returning newly allocated memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AllocSize {
    /// The parameter index containing the element size in bytes.
    pub element_size_index: u32,
    /// The parameter index containing the element count, if any.
    pub element_count_index: Option<u32>,
}

impl AllocSize {
    /// Create an alloc size description from parameter indices.
    pub fn new(element_size_index: u32, element_count_index: Option<u32>) -> Self {
        Self {
            element_size_index,
            element_count_index,
        }
    }
}

/// Attributes that refine pointer aliasing and memory access behavior.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PointerAttributes {
    /// The pointer is guaranteed to be non null.
    pub nonnull: bool,
    /// The pointer refers to a unique non aliasing allocation.
    pub noalias: bool,
    /// The capture behavior for this pointer.
    pub capture: CaptureKind,
    /// The pointee is only read through this pointer.
    pub readonly: bool,
    /// The pointee is only written through this pointer.
    pub writeonly: bool,
    /// The pointer value is fully defined.
    pub noundef: bool,
    /// The number of bytes guaranteed to be dereferenceable.
    pub dereferenceable_bytes: Option<u64>,
    /// The number of bytes dereferenceable when non null.
    pub dereferenceable_or_null_bytes: Option<u64>,
    /// The alignment guarantee for the pointer.
    pub alignment: Option<u32>,
    /// Return value aliases this parameter.
    pub returned: bool,
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

/// Attributes describing how a call argument may be accessed.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CallArgumentMetadata {
    /// Pointer specific attributes for this argument.
    pub attributes: PointerAttributes,
    /// Access mode for this argument.
    pub access: ArgumentAccess,
    /// Alias scopes applied to this argument.
    pub alias_scopes: Vec<AliasScopeId>,
    /// No alias scopes applied to this argument.
    pub noalias_scopes: Vec<AliasScopeId>,
    /// Optional TBAA tag for this argument.
    pub tbaa_tag: Option<TbaaTagId>,
}

/// Table of memory metadata entries.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MemoryTable {
    /// Memory access metadata keyed by instruction id.
    pub memory_accesses_by_instruction_id:
        HashMap<LocalNodeId<Instruction>, Vec<MemoryAccessMetadata>>,
    /// Alias scopes and domains used in metadata.
    pub alias_scopes: AliasScopeTable,
    /// TBAA nodes and tags used in metadata.
    pub tbaa: TbaaTable,
}

impl MemoryTable {
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

    /// Return the owned bytes for this memory metadata table.
    pub fn owned_bytes(&self) -> usize {
        let mut owned_bytes = size_of::<Self>();
        owned_bytes += self.memory_accesses_by_instruction_id.capacity()
            * size_of::<(LocalNodeId<Instruction>, Vec<MemoryAccessMetadata>)>();
        owned_bytes += self.alias_scopes.owned_bytes();
        owned_bytes += self.tbaa.owned_bytes();

        for accesses in self.memory_accesses_by_instruction_id.values() {
            owned_bytes += accesses.capacity() * size_of::<MemoryAccessMetadata>();

            for access in accesses {
                owned_bytes += access.alias_scopes.capacity() * size_of::<AliasScopeId>();
                owned_bytes += access.noalias_scopes.capacity() * size_of::<AliasScopeId>();
            }
        }

        owned_bytes
    }
}
