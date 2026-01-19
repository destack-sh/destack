use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_base::StringId;

use crate::{Field, Function, Global, LocalNodeId, Type};

/// Layout policy for composite types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LayoutPolicy {
    /// Default layout policy for the target.
    Default,
    /// C ABI layout policy.
    C,
    /// Preserve source declaration order.
    Source,
    /// Packed layout with minimal padding.
    Packed,
}

/// Concrete layout details for a MIR type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeLayout {
    /// The total size of the type in bytes.
    pub size: u64,
    /// The required alignment of the type in bytes.
    pub alignment: u32,
    /// The stride between elements when used in arrays.
    pub stride: u64,
    /// The layout policy used for this type.
    pub policy: LayoutPolicy,
    /// Field offsets for structs or tuples in declaration order.
    pub field_offsets: Vec<u32>,
}

/// Lineage metadata for nominal types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeLineage {
    /// Optional parent type for class inheritance.
    pub parent: Option<LocalNodeId<Type>>,
    /// Interfaces implemented by this type.
    pub interfaces: Vec<LocalNodeId<Type>>,
    /// True when the type is sealed to external extension.
    pub is_sealed: bool,
    /// True when the type is final and cannot be subclassed.
    pub is_final: bool,
    /// True when the type is abstract and cannot be instantiated.
    pub is_abstract: bool,
    /// True when the type represents an interface.
    pub is_interface: bool,
}

/// Identifier for a dispatch table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DispatchTableId(
    /// Raw index into the dispatch table registry.
    u32,
);

impl DispatchTableId {
    /// Create a dispatch table id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Get the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Kind of dispatch table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DispatchTableKind {
    /// Class vtable for virtual dispatch.
    Class {
        /// The class type owning the vtable.
        ty: LocalNodeId<Type>,
    },
    /// Interface itab for a concrete type and interface pair.
    Interface {
        /// The concrete type providing the implementation.
        concrete: LocalNodeId<Type>,
        /// The interface type being dispatched.
        interface: LocalNodeId<Type>,
    },
}

/// Entry in a dispatch table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DispatchSlot {
    /// Slot containing a type tag handle.
    TypeTag,
    /// Slot containing a drop glue function.
    Destructor {
        /// The drop glue function when present.
        function: Option<LocalNodeId<Function>>,
    },
    /// Slot containing a method implementation.
    Method {
        /// The concrete method implementation.
        function: LocalNodeId<Function>,
    },
    /// Slot mapping interface method to a concrete implementation.
    InterfaceMethod {
        /// The interface method signature.
        interface_method: LocalNodeId<Function>,
        /// The concrete method implementation.
        target: LocalNodeId<Function>,
    },
    /// Slot containing an interface field offset.
    FieldOffset {
        /// The interface field name.
        field_name: StringId,
        /// The field offset in bytes.
        offset: u32,
    },
}

/// Metadata for a dispatch table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchTable {
    /// The kind of dispatch table.
    pub kind: DispatchTableKind,
    /// Optional global symbol containing the table.
    pub global: Option<LocalNodeId<Global>>,
    /// Slots in declaration order.
    pub slots: Vec<DispatchSlot>,
}

/// Registry of dispatch metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DispatchRegistry {
    /// Registered dispatch tables.
    pub tables: Vec<DispatchTable>,
}

impl DispatchRegistry {
    /// Create a new empty dispatch table registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a dispatch table and return its id.
    pub fn insert(&mut self, table: DispatchTable) -> DispatchTableId {
        let id = DispatchTableId::new(self.tables.len() as u32);
        self.tables.push(table);
        id
    }

    /// Return the dispatch table for an id.
    pub fn table(&self, id: DispatchTableId) -> &DispatchTable {
        &self.tables[id.index()]
    }
}

/// Type metadata available for optimization and codegen.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TypeMetadata {
    /// The display name of the type for diagnostics and debugging only.
    pub name: Option<StringId>,
    /// Layout metadata for the type.
    pub layout: Option<TypeLayout>,
    /// Lineage metadata for the type.
    pub lineage: Option<TypeLineage>,
    /// Dispatch table for class virtual dispatch.
    pub vtable: Option<DispatchTableId>,
    /// Dispatch tables for interface dispatch.
    pub itabs: Vec<DispatchTableId>,
    /// Runtime type descriptor global.
    pub type_descriptor: Option<LocalNodeId<Global>>,
    /// Field map for property layout lookup.
    pub field_map: HashMap<StringId, LocalNodeId<Field>>,
}

/// Table of type metadata entries.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TypeTable {
    /// Drop function for each type that implements Drop.
    /// Maps type id → drop function id.
    pub drop_function_by_type_id: HashMap<LocalNodeId<Type>, LocalNodeId<Function>>,
    /// Type metadata keyed by type id.
    pub type_metadata_by_id: HashMap<LocalNodeId<Type>, TypeMetadata>,
    /// Dispatch tables for virtual and interface calls.
    pub dispatch_registry: DispatchRegistry,
}

impl TypeTable {
    /// Create a new empty type table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return type metadata for a type id.
    pub fn type_metadata(&self, ty: LocalNodeId<Type>) -> Option<&TypeMetadata> {
        self.type_metadata_by_id.get(&ty)
    }

    /// Return mutable type metadata for a type id.
    pub fn type_metadata_mut(&mut self, ty: LocalNodeId<Type>) -> Option<&mut TypeMetadata> {
        self.type_metadata_by_id.get_mut(&ty)
    }

    /// Insert type metadata for a type id.
    pub fn insert_type_metadata(
        &mut self,
        ty: LocalNodeId<Type>,
        metadata: TypeMetadata,
    ) -> Option<TypeMetadata> {
        self.type_metadata_by_id.insert(ty, metadata)
    }

    /// Remove type metadata for a type id.
    pub fn remove_type_metadata(&mut self, ty: LocalNodeId<Type>) -> Option<TypeMetadata> {
        self.type_metadata_by_id.remove(&ty)
    }
}
