use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_serde::Reflect;

use crate::{AtomicAccess, Global, Instruction, Local, LocalNodeId, Tree, Value};

/// Table of explicit memory accesses.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Reflect)]
pub struct MemoryTable {
    /// Memory accesses keyed by instruction id.
    pub memory_accesses_by_instruction_id: HashMap<LocalNodeId<Instruction>, Vec<MemoryAccess>>,
}

impl MemoryTable {
    /// Create a new empty memory table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return memory accesses for an instruction id.
    pub fn memory_accesses(
        &self,
        instruction: LocalNodeId<Instruction>,
    ) -> Option<&[MemoryAccess]> {
        self.memory_accesses_by_instruction_id
            .get(&instruction)
            .map(|accesses| accesses.as_slice())
    }

    /// Return mutable memory accesses for an instruction id.
    pub fn memory_accesses_mut(
        &mut self,
        instruction: LocalNodeId<Instruction>,
    ) -> Option<&mut Vec<MemoryAccess>> {
        self.memory_accesses_by_instruction_id.get_mut(&instruction)
    }

    /// Insert memory accesses for an instruction id.
    pub fn insert_memory_accesses(
        &mut self,
        instruction: LocalNodeId<Instruction>,
        accesses: Vec<MemoryAccess>,
    ) -> Option<Vec<MemoryAccess>> {
        self.memory_accesses_by_instruction_id
            .insert(instruction, accesses)
    }

    /// Remove memory accesses for an instruction id.
    pub fn remove_memory_accesses(
        &mut self,
        instruction: LocalNodeId<Instruction>,
    ) -> Option<Vec<MemoryAccess>> {
        self.memory_accesses_by_instruction_id.remove(&instruction)
    }

    /// Return whether one instruction has ordered memory behavior.
    pub fn instruction_has_atomic_ordering(
        &self,
        tree: &Tree,
        instruction: LocalNodeId<Instruction>,
    ) -> bool {
        if matches!(
            tree.get(instruction),
            Instruction::AtomicLoad { .. }
                | Instruction::AtomicStore { .. }
                | Instruction::AtomicCompareExchange { .. }
                | Instruction::AtomicRmw { .. }
                | Instruction::AtomicFence { .. }
        ) {
            return true;
        }

        let Some(accesses) = self.memory_accesses(instruction) else {
            return false;
        };

        accesses.iter().any(MemoryAccess::is_atomic)
    }

    /// Return whether one instruction must keep exact memory position.
    pub fn instruction_requires_exact_access(
        &self,
        tree: &Tree,
        instruction: LocalNodeId<Instruction>,
    ) -> bool {
        if self.instruction_has_atomic_ordering(tree, instruction) {
            return true;
        }

        let Some(accesses) = self.memory_accesses(instruction) else {
            return false;
        };

        accesses.iter().any(MemoryAccess::requires_exact_position)
    }
}

/// Explicit memory access attached to one instruction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemoryAccess {
    /// The operation performed.
    pub operation: MemoryOperation,
    /// The access target.
    pub target: MemoryTarget,
    /// The number of bytes accessed when known.
    pub byte_len: Option<u64>,
    /// Alignment in bytes, when known.
    pub alignment_bytes: Option<u32>,
    /// The ordering constraints on this access.
    pub order: MemoryAccessOrder,
}

impl MemoryAccess {
    /// Create one plain memory access.
    pub fn plain(
        operation: MemoryOperation,
        target: MemoryTarget,
        byte_len: Option<u64>,
        alignment_bytes: Option<u32>,
    ) -> Self {
        Self {
            operation,
            target,
            byte_len,
            alignment_bytes,
            order: MemoryAccessOrder::Plain,
        }
    }

    /// Create one volatile memory access.
    pub fn volatile(
        operation: MemoryOperation,
        target: MemoryTarget,
        byte_len: Option<u64>,
        alignment_bytes: Option<u32>,
    ) -> Self {
        Self {
            operation,
            target,
            byte_len,
            alignment_bytes,
            order: MemoryAccessOrder::Volatile,
        }
    }

    /// Return whether this access is atomic.
    pub fn is_atomic(&self) -> bool {
        matches!(self.order, MemoryAccessOrder::Atomic(_))
    }

    /// Return whether this access is volatile.
    pub fn is_volatile(&self) -> bool {
        matches!(self.order, MemoryAccessOrder::Volatile)
    }

    /// Return whether this access must remain at its exact program position.
    pub fn requires_exact_position(&self) -> bool {
        !matches!(self.order, MemoryAccessOrder::Plain)
    }
}

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

/// Target of one memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum MemoryTarget {
    /// Access through a reference value.
    Reference(Value),
    /// Access through a local slot.
    Local(LocalNodeId<Local>),
    /// Access through a global.
    Global(LocalNodeId<Global>),
}

/// Summary behavior for one argument passed to a bodyless call.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Reflect)]
pub struct CallArgumentEffect {
    /// Access mode for this argument.
    pub access: ArgumentAccess,
    /// Escape behavior for this argument.
    pub escape: ArgumentEscape,
}

/// Access mode for a bodyless call pointer argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
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

/// Escape behavior for a bodyless call argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
pub enum ArgumentEscape {
    /// The argument does not escape the callee.
    None,
    /// The argument only escapes through the return value.
    Return,
    /// The argument may escape in an unknown way.
    #[default]
    Escape,
}
