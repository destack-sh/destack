use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_base::StringId;

use crate::{
    Field, Function, Global, Layout, LayoutId, LayoutTable, LocalNodeId, Type, UnionLayout,
};

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

/// Identifier for a vtable entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VtableId(
    /// Raw index into the vtable registry.
    u32,
);

impl VtableId {
    /// Create a vtable id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Identifier for an itab entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItabId(
    /// Raw index into the itab registry.
    u32,
);

impl ItabId {
    /// Create an itab id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Entry in a class vtable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VtableEntry {
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
}

/// Entry in an interface itab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItabEntry {
    /// Slot containing a type tag handle.
    TypeTag,
    /// Slot containing an interface field offset.
    FieldOffset {
        /// The interface field name.
        field_name: StringId,
        /// The field offset in bytes.
        offset: u32,
    },
    /// Slot mapping interface method declaration to target method.
    Method {
        /// The declared interface method.
        declared_method: LocalNodeId<Function>,
        /// The concrete method implementation.
        target_method: LocalNodeId<Function>,
    },
}

/// Metadata for a class vtable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vtable {
    /// The class type owning this table.
    pub ty: LocalNodeId<Type>,
    /// Optional global symbol containing the table.
    pub global: Option<LocalNodeId<Global>>,
    /// Entries in declaration order.
    pub entries: Vec<VtableEntry>,
}

/// Metadata for an interface itab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Itab {
    /// The concrete type providing the implementation.
    pub concrete: LocalNodeId<Type>,
    /// The interface type being dispatched.
    pub interface: LocalNodeId<Type>,
    /// Optional global symbol containing the table.
    pub global: Option<LocalNodeId<Global>>,
    /// Entries in declaration order.
    pub entries: Vec<ItabEntry>,
}

/// Registry of class vtables.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VtableRegistry {
    /// Registered class vtables.
    pub tables: Vec<Option<Vtable>>,
}

impl VtableRegistry {
    /// Create a new empty vtable registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a vtable and return its id.
    pub fn insert(&mut self, table: Vtable) -> VtableId {
        let id = VtableId::new(self.tables.len() as u32);
        self.tables.push(Some(table));
        id
    }

    /// Insert a vtable at a specific id.
    pub fn insert_at(&mut self, id: VtableId, table: Vtable) {
        let index = id.index();
        if self.tables.len() <= index {
            self.tables.resize_with(index + 1, || None);
        }
        if self.tables[index].is_some() {
            panic!("vtable slot {index} already populated");
        }
        self.tables[index] = Some(table);
    }

    /// Return the vtable for an id.
    pub fn table(&self, id: VtableId) -> &Vtable {
        self.tables[id.index()]
            .as_ref()
            .expect("missing vtable entry")
    }
}

/// Registry of interface itabs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ItabRegistry {
    /// Registered interface itabs.
    pub tables: Vec<Option<Itab>>,
}

impl ItabRegistry {
    /// Create a new empty itab registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert an itab and return its id.
    pub fn insert(&mut self, table: Itab) -> ItabId {
        let id = ItabId::new(self.tables.len() as u32);
        self.tables.push(Some(table));
        id
    }

    /// Insert an itab at a specific id.
    pub fn insert_at(&mut self, id: ItabId, table: Itab) {
        let index = id.index();
        if self.tables.len() <= index {
            self.tables.resize_with(index + 1, || None);
        }
        if self.tables[index].is_some() {
            panic!("itab slot {index} already populated");
        }
        self.tables[index] = Some(table);
    }

    /// Return the itab for an id.
    pub fn table(&self, id: ItabId) -> &Itab {
        self.tables[id.index()]
            .as_ref()
            .expect("missing itab entry")
    }
}

/// Type metadata available for optimization and codegen.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TypeMetadata {
    /// The display name of the type for diagnostics and debugging only.
    pub name: Option<StringId>,
    /// Layout identifier for the type when materialized.
    pub layout_id: Option<LayoutId>,
    /// Lineage metadata for the type.
    pub lineage: Option<TypeLineage>,
    /// Vtable for class virtual dispatch.
    pub vtable: Option<VtableId>,
    /// Itabs for interface dispatch.
    pub itabs: Vec<ItabId>,
    /// Interface to itab mapping for this concrete type.
    pub itab_by_interface: HashMap<LocalNodeId<Type>, ItabId>,
    /// Runtime type descriptor global.
    pub type_descriptor: Option<LocalNodeId<Global>>,
    /// Field map for property layout lookup.
    pub field_map: HashMap<StringId, LocalNodeId<Field>>,
    /// Union layout metadata for tagged unions.
    pub union_layout: Option<UnionLayout>,
}

/// Primitive type cache for fast lookups.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TypeCache {
    /// Cached void type id.
    pub void: Option<LocalNodeId<Type>>,
    /// Cached boolean type id.
    pub boolean: Option<LocalNodeId<Type>>,
    /// Cached type tag type id.
    pub type_tag: Option<LocalNodeId<Type>>,
    /// Cached isize type id.
    pub isize: Option<LocalNodeId<Type>>,
    /// Cached usize type id.
    pub usize: Option<LocalNodeId<Type>>,
    /// Cached integer type ids keyed by width and signedness.
    pub ints: HashMap<(u16, bool), LocalNodeId<Type>>,
    /// Cached float type ids keyed by width.
    pub floats: HashMap<u16, LocalNodeId<Type>>,
}

/// Cache entry describing a primitive type.
#[derive(Clone, Debug)]
pub enum TypeCacheEntry {
    /// Void primitive type.
    Void,
    /// Boolean primitive type.
    Boolean,
    /// Runtime type tag type.
    TypeTag,
    /// Pointer sized signed integer type.
    Isize,
    /// Pointer sized unsigned integer type.
    Usize,
    /// Integer type with width and signedness.
    Int { width: u16, signed: bool },
    /// Float type with width.
    Float { width: u16 },
}

/// Table of type metadata entries.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TypeTable {
    /// Cached primitive type ids.
    pub type_cache: TypeCache,
    /// Layout metadata table for aggregate types.
    pub layout_table: LayoutTable,
    /// Type metadata keyed by type id.
    pub type_metadata_by_id: HashMap<LocalNodeId<Type>, TypeMetadata>,
    /// Class vtables.
    pub vtable_registry: VtableRegistry,
    /// Interface itabs.
    pub itab_registry: ItabRegistry,
}

impl TypeTable {
    /// Create a new empty type table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the cache entry for a MIR type when applicable.
    pub fn cache_entry_for_type(ty: &Type) -> Option<TypeCacheEntry> {
        match ty {
            Type::Void => Some(TypeCacheEntry::Void),
            Type::Boolean => Some(TypeCacheEntry::Boolean),
            Type::Type => Some(TypeCacheEntry::TypeTag),
            Type::Isize => Some(TypeCacheEntry::Isize),
            Type::Usize => Some(TypeCacheEntry::Usize),
            Type::Int {
                width,
                is_signed: signed,
            } => Some(TypeCacheEntry::Int {
                width: *width,
                signed: *signed,
            }),
            Type::Float { width } => Some(TypeCacheEntry::Float { width: *width }),
            _ => None,
        }
    }

    /// Register a type id in the primitive cache.
    pub fn register_type_entry(&mut self, type_id: LocalNodeId<Type>, entry: TypeCacheEntry) {
        match entry {
            TypeCacheEntry::Void => {
                self.type_cache.void.get_or_insert(type_id);
            }
            TypeCacheEntry::Boolean => {
                self.type_cache.boolean.get_or_insert(type_id);
            }
            TypeCacheEntry::TypeTag => {
                self.type_cache.type_tag.get_or_insert(type_id);
            }
            TypeCacheEntry::Isize => {
                self.type_cache.isize.get_or_insert(type_id);
            }
            TypeCacheEntry::Usize => {
                self.type_cache.usize.get_or_insert(type_id);
            }
            TypeCacheEntry::Int { width, signed } => {
                self.type_cache
                    .ints
                    .entry((width, signed))
                    .or_insert(type_id);
            }
            TypeCacheEntry::Float { width } => {
                self.type_cache.floats.entry(width).or_insert(type_id);
            }
        }
    }

    /// Return the cached boolean type id.
    pub fn boolean_type(&self) -> Option<LocalNodeId<Type>> {
        self.type_cache.boolean
    }

    /// Return the cached void type id.
    pub fn void_type(&self) -> Option<LocalNodeId<Type>> {
        self.type_cache.void
    }

    /// Return the cached type tag type id.
    pub fn type_tag_type(&self) -> Option<LocalNodeId<Type>> {
        self.type_cache.type_tag
    }

    /// Return the cached isize type id.
    pub fn isize_type(&self) -> Option<LocalNodeId<Type>> {
        self.type_cache.isize
    }

    /// Return the cached usize type id.
    pub fn usize_type(&self) -> Option<LocalNodeId<Type>> {
        self.type_cache.usize
    }

    /// Return the cached integer type id for a width and signedness.
    pub fn int_type(&self, width: u16, signed: bool) -> Option<LocalNodeId<Type>> {
        self.type_cache.ints.get(&(width, signed)).copied()
    }

    /// Return the cached float type id for a width.
    pub fn float_type(&self, width: u16) -> Option<LocalNodeId<Type>> {
        self.type_cache.floats.get(&width).copied()
    }

    /// Return type metadata for a type id.
    pub fn type_metadata(&self, ty: LocalNodeId<Type>) -> Option<&TypeMetadata> {
        self.type_metadata_by_id.get(&ty)
    }

    /// Return the layout entry for a type id when available.
    pub fn type_layout(&self, ty: LocalNodeId<Type>) -> Option<&Layout> {
        let metadata = self.type_metadata_by_id.get(&ty)?;
        let layout_id = metadata.layout_id?;
        self.layout_table.layouts.get(layout_id.0 as usize)
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
