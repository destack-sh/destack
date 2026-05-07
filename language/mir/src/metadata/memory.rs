use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::{
    AddressSpace, Global, Instruction, Local, LocalNodeId, MemoryFlags, MemoryOrdering,
    MemoryScope, SyncScope, Value,
};

/// Table of memory metadata entries.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MemoryMetadata {
    /// Memory access metadata keyed by instruction id.
    pub memory_accesses_by_instruction_id:
        HashMap<LocalNodeId<Instruction>, Vec<MemoryAccessMetadata>>,
    /// Alias scopes and domains used in metadata.
    pub alias_scopes: MemoryAliasTable,
    /// Type-alias nodes and tags used in metadata.
    pub type_alias: TypeAliasTable,
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
    /// Address space override for the access.
    pub address_space: Option<AddressSpace>,
    /// Alias scopes that the access participates in.
    pub alias_scopes: Vec<MemoryAliasScopeId>,
    /// No alias scopes that the access participates in.
    pub noalias_scopes: Vec<MemoryAliasScopeId>,
    /// Optional type-alias tag for the access.
    pub type_alias_tag: Option<TypeAliasTagId>,
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

/// Table of alias scopes and domains.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryAliasTable {
    /// Registered alias domains.
    pub domains: Vec<MemoryAliasDomain>,
    /// Registered alias scopes.
    pub scopes: Vec<MemoryAliasScope>,
}

impl MemoryAliasTable {
    /// Create a new empty alias scope table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new alias domain.
    pub fn create_domain(&mut self, name: Option<StringId>) -> MemoryAliasDomainId {
        let id = MemoryAliasDomainId::new(self.domains.len() as u32);
        self.domains.push(MemoryAliasDomain { name });
        id
    }

    /// Create a new alias scope within a domain.
    pub fn create_scope(
        &mut self,
        domain: MemoryAliasDomainId,
        name: Option<StringId>,
    ) -> MemoryAliasScopeId {
        let id = MemoryAliasScopeId::new(self.scopes.len() as u32);
        self.scopes.push(MemoryAliasScope { domain, name });
        id
    }

    /// Return the alias domain for an id.
    pub fn domain(&self, id: MemoryAliasDomainId) -> &MemoryAliasDomain {
        &self.domains[id.index()]
    }

    /// Return the alias scope for an id.
    pub fn scope(&self, id: MemoryAliasScopeId) -> &MemoryAliasScope {
        &self.scopes[id.index()]
    }
}

/// Identifier for a memory alias domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryAliasDomainId(u32);

impl MemoryAliasDomainId {
    /// Create a domain id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Get the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Identifier for a memory alias scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryAliasScopeId(u32);

impl MemoryAliasScopeId {
    /// Create a scope id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Get the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Alias analysis domain for grouping alias scopes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryAliasDomain {
    /// Optional name for diagnostics or debugging.
    pub name: Option<StringId>,
}

/// Alias scope for noalias or scoped aliasing metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryAliasScope {
    /// The domain this scope belongs to.
    pub domain: MemoryAliasDomainId,
    /// Optional name for diagnostics or debugging.
    pub name: Option<StringId>,
}

/// Table of type-alias nodes and tags.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeAliasTable {
    /// Registered type-alias nodes.
    pub nodes: Vec<TypeAliasNode>,
    /// Registered type-alias tags.
    pub tags: Vec<TypeAliasTag>,
}

impl TypeAliasTable {
    /// Create a new empty type-alias table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new type-alias node.
    pub fn create_node(
        &mut self,
        name: Option<StringId>,
        parent: Option<TypeAliasNodeId>,
        is_constant: bool,
    ) -> TypeAliasNodeId {
        let id = TypeAliasNodeId::new(self.nodes.len() as u32);
        self.nodes.push(TypeAliasNode {
            name,
            parent,
            is_constant,
        });
        id
    }

    /// Create a new type-alias tag.
    pub fn create_tag(
        &mut self,
        base: TypeAliasNodeId,
        access: TypeAliasNodeId,
        offset: u64,
        size: u64,
        is_immutable: bool,
    ) -> TypeAliasTagId {
        let id = TypeAliasTagId::new(self.tags.len() as u32);
        self.tags.push(TypeAliasTag {
            base,
            access,
            offset,
            size,
            is_immutable,
        });
        id
    }

    /// Return the type-alias node for an id.
    pub fn node(&self, id: TypeAliasNodeId) -> &TypeAliasNode {
        &self.nodes[id.index()]
    }

    /// Return the type-alias tag for an id.
    pub fn tag(&self, id: TypeAliasTagId) -> &TypeAliasTag {
        &self.tags[id.index()]
    }
}

/// Identifier for a type-alias node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeAliasNodeId(u32);

impl TypeAliasNodeId {
    /// Create a node id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Get the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Identifier for a type-alias tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeAliasTagId(u32);

impl TypeAliasTagId {
    /// Create a tag id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Get the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Type-alias node describing one class in the alias tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeAliasNode {
    /// Optional name for diagnostics or debugging.
    pub name: Option<StringId>,
    /// Parent node in the type-alias tree.
    pub parent: Option<TypeAliasNodeId>,
    /// Whether this node represents immutable memory.
    pub is_constant: bool,
}

/// Type-alias tag describing one access in the alias tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeAliasTag {
    /// Base type node for the access.
    pub base: TypeAliasNodeId,
    /// Access type node for the access.
    pub access: TypeAliasNodeId,
    /// Byte offset within the base type.
    pub offset: u64,
    /// Size of the access in bytes.
    pub size: u64,
    /// Whether this access is to immutable memory.
    pub is_immutable: bool,
}

/// Attributes describing how a call argument may be accessed.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ArgumentAttribute {
    /// Pointer specific attributes for this argument.
    pub attributes: PointerAttribute,
    /// Access mode for this argument.
    pub access: ArgumentAccess,
    /// Alias scopes applied to this argument.
    pub alias_scopes: Vec<MemoryAliasScopeId>,
    /// No alias scopes applied to this argument.
    pub noalias_scopes: Vec<MemoryAliasScopeId>,
    /// Optional type-alias tag for this argument.
    pub type_alias_tag: Option<TypeAliasTagId>,
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

/// Attributes that refine pointer aliasing and memory access behavior.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PointerAttribute {
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
