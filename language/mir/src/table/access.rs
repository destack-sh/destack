use destack_core::FxIndexMap;

use serde::{Deserialize, Serialize};

use destack_serde::Reflect;

use crate::{AtomicAccess, Global, Instruction, Local, LocalNodeId, Tree, Value};

/// Table of explicit memory accesses.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Reflect)]
pub struct AccessTable {
    /// Memory accesses keyed by instruction id.
    accesses: FxIndexMap<LocalNodeId<Instruction>, Vec<MemoryAccess>>,
}

impl AccessTable {
    /// Return memory accesses for an instruction id.
    pub fn get(&self, instruction: LocalNodeId<Instruction>) -> Option<&[MemoryAccess]> {
        self.accesses
            .get(&instruction)
            .map(|accesses| accesses.as_slice())
    }

    /// Insert memory accesses for an instruction id.
    pub fn insert(
        &mut self,
        instruction: LocalNodeId<Instruction>,
        accesses: Vec<MemoryAccess>,
    ) -> Option<Vec<MemoryAccess>> {
        self.accesses.insert(instruction, accesses)
    }

    /// Remove memory accesses for an instruction id.
    pub fn remove(&mut self, instruction: LocalNodeId<Instruction>) -> Option<Vec<MemoryAccess>> {
        self.accesses.shift_remove(&instruction)
    }

    /// Return whether one instruction has ordered memory behavior.
    pub fn is_ordered(&self, instruction: LocalNodeId<Instruction>, tree: &Tree) -> bool {
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

        let Some(accesses) = self.get(instruction) else {
            return false;
        };

        accesses.iter().any(MemoryAccess::is_atomic)
    }

    /// Return whether one instruction must keep exact memory position.
    pub fn requires_exact_position(
        &self,
        instruction: LocalNodeId<Instruction>,
        tree: &Tree,
    ) -> bool {
        if self.is_ordered(instruction, tree) {
            return true;
        }

        let Some(accesses) = self.get(instruction) else {
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
    /// Access through an address-bearing value.
    Address(Value),
    /// Access through a local slot.
    Local(LocalNodeId<Local>),
    /// Access through a global.
    Global(LocalNodeId<Global>),
}
