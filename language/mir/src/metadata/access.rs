use crate::{AddressSpace, Global, Local, LocalNodeId, MemoryOrdering, Value};

use super::{AliasScopeId, TbaaTagId};

/// The kind of memory access represented by metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryAccessKind {
    /// Reads memory.
    Read,
    /// Writes memory.
    Write,
    /// Reads and writes memory.
    ReadWrite,
    /// Read modify write memory access.
    ReadModifyWrite,
    /// Memory fence or barrier.
    Fence,
    /// Prefetch hint for reading.
    PrefetchRead,
    /// Prefetch hint for writing.
    PrefetchWrite,
}

/// Target of a memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// Metadata describing a single memory access in an instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Whether the access is invariant for the duration of the program.
    pub is_invariant: bool,
    /// Whether the access should avoid temporal locality optimizations.
    pub is_non_temporal: bool,
    /// Memory ordering for atomic accesses.
    pub ordering: Option<MemoryOrdering>,
    /// Address space override for the access.
    pub address_space: Option<AddressSpace>,
    /// Alias scopes that the access participates in.
    pub alias_scopes: Vec<AliasScopeId>,
    /// No alias scopes that the access participates in.
    pub noalias_scopes: Vec<AliasScopeId>,
    /// Optional TBAA tag for the access.
    pub tbaa_tag: Option<TbaaTagId>,
}
