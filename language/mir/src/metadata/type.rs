use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use destack_core::StringId;

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
    /// Raw index into the vtable table.
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

/// Identifier for a vtable method slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VtableSlotId(
    /// Raw index into the vtable entry list.
    pub u32,
);

impl VtableSlotId {
    /// Create a vtable slot id from a raw index.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return the raw index for this id.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Identifier for an itab entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItabId(
    /// Raw index into the itab table.
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

/// Identifier for an interface dispatch slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InterfaceSlotId(
    /// Raw index into the interface dispatch entry list.
    pub u32,
);

impl InterfaceSlotId {
    /// Create an interface slot id from a raw index.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return the raw index for this id.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Entry in a class vtable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VtableEntry {
    /// Slot containing the runtime type descriptor.
    TypeDescriptor,
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
    /// Slot containing the runtime type descriptor.
    TypeDescriptor,
    /// Slot containing an interface field offset.
    FieldOffset {
        /// The canonical interface dispatch field id.
        field: LocalNodeId<Field>,
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

/// Slot descriptor for interface dispatch layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterfaceDispatchEntry {
    /// Slot containing the runtime type descriptor.
    TypeDescriptor,
    /// Slot containing an interface field offset.
    FieldOffset {
        /// The canonical interface dispatch field id.
        field: LocalNodeId<Field>,
        /// The interface field name.
        field_name: StringId,
    },
    /// Slot containing an interface method declaration.
    Method {
        /// The declared interface method.
        declared_method: LocalNodeId<Function>,
    },
}

/// Canonical interface dispatch shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceDispatchShape {
    /// The interface type owning this shape.
    pub interface: LocalNodeId<Type>,
    /// Entries in declaration order.
    pub entries: Vec<InterfaceDispatchEntry>,
}

/// Metadata for a class vtable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vtable {
    /// The class type owning this table.
    pub ty: LocalNodeId<Type>,
    /// Storage backing for this vtable.
    pub storage: VtableStorage,
    /// Entries in declaration order.
    pub entries: Vec<VtableEntry>,
}

/// Storage backing for a class vtable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VtableStorage {
    /// Global data symbol containing the vtable entries.
    Global(LocalNodeId<Global>),
}

/// Metadata for an interface itab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Itab {
    /// The concrete type providing the implementation.
    pub concrete: LocalNodeId<Type>,
    /// The interface type being dispatched.
    pub interface: LocalNodeId<Type>,
    /// Storage backing for this itab.
    pub storage: ItabStorage,
    /// Entries in declaration order.
    pub entries: Vec<ItabEntry>,
}

/// Storage backing for an interface itab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItabStorage {
    /// Immediate handle encoded as an itab id.
    Handle,
}

/// Primitive type cache for fast lookups.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TypeCache {
    /// Cached void type id.
    pub void: Option<LocalNodeId<Type>>,
    /// Cached boolean type id.
    pub boolean: Option<LocalNodeId<Type>>,
    /// Cached type descriptor type id.
    pub type_descriptor: Option<LocalNodeId<Type>>,
    /// Cached type id type id.
    pub type_id: Option<LocalNodeId<Type>>,
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
    /// Runtime type descriptor type.
    TypeDescriptor,
    /// Runtime type id type.
    TypeId,
    /// Pointer sized signed integer type.
    Isize,
    /// Pointer sized unsigned integer type.
    Usize,
    /// Integer type with width and signedness.
    Int { width: u16, signed: bool },
    /// Float type with width.
    Float { width: u16 },
}

/// Table of canonical type facts.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TypeTable {
    /// Cached primitive type ids.
    #[serde(skip, default)]
    pub type_cache: TypeCache,
    /// Layout metadata table for aggregate types.
    pub layout_table: LayoutTable,
    /// Concrete layout ids keyed by type id.
    pub layout_by_type: HashMap<LocalNodeId<Type>, LayoutId>,
    /// Nominal lineage keyed by type id.
    pub lineage_by_type: HashMap<LocalNodeId<Type>, TypeLineage>,
    /// Union layout metadata keyed by type id.
    pub union_layout_by_type: HashMap<LocalNodeId<Type>, UnionLayout>,
    /// Class vtable ids keyed by type id.
    pub vtable_by_type: HashMap<LocalNodeId<Type>, VtableId>,
    /// Interface itab ids keyed by concrete type id, then interface type id.
    pub itabs_by_type: HashMap<LocalNodeId<Type>, HashMap<LocalNodeId<Type>, ItabId>>,
    /// Field lookup tables keyed by type id.
    pub field_map_by_type: HashMap<LocalNodeId<Type>, HashMap<StringId, LocalNodeId<Field>>>,
    /// Runtime type descriptor globals keyed by type id.
    pub descriptor_by_type: HashMap<LocalNodeId<Type>, LocalNodeId<Global>>,
    /// Display names keyed by type id.
    pub display_name_by_type: HashMap<LocalNodeId<Type>, StringId>,
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
            Type::TypeDescriptor => Some(TypeCacheEntry::TypeDescriptor),
            Type::TypeId => Some(TypeCacheEntry::TypeId),
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
            TypeCacheEntry::TypeDescriptor => {
                self.type_cache.type_descriptor.get_or_insert(type_id);
            }
            TypeCacheEntry::TypeId => {
                self.type_cache.type_id.get_or_insert(type_id);
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

    /// Return the cached type descriptor type id.
    pub fn type_descriptor_type(&self) -> Option<LocalNodeId<Type>> {
        self.type_cache.type_descriptor
    }

    /// Return the cached type id type id.
    pub fn type_id_type(&self) -> Option<LocalNodeId<Type>> {
        self.type_cache.type_id
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

    /// Return the layout entry for a type id when available.
    pub fn type_layout(&self, ty: LocalNodeId<Type>) -> Option<&Layout> {
        let layout_id = self.layout_by_type.get(&ty)?;
        self.layout_table.layouts.get(layout_id.index())
    }

    /// Return the layout id for a type when present.
    pub fn layout_id(&self, ty: LocalNodeId<Type>) -> Option<LayoutId> {
        self.layout_by_type.get(&ty).copied()
    }

    /// Record the layout id for a type.
    pub fn set_layout_id(
        &mut self,
        ty: LocalNodeId<Type>,
        layout_id: LayoutId,
    ) -> Option<LayoutId> {
        self.layout_by_type.insert(ty, layout_id)
    }

    /// Return lineage metadata for a type when present.
    pub fn lineage(&self, ty: LocalNodeId<Type>) -> Option<&TypeLineage> {
        self.lineage_by_type.get(&ty)
    }

    /// Record lineage metadata for a type.
    pub fn set_lineage(
        &mut self,
        ty: LocalNodeId<Type>,
        lineage: TypeLineage,
    ) -> Option<TypeLineage> {
        self.lineage_by_type.insert(ty, lineage)
    }

    /// Return union layout metadata for a type when present.
    pub fn union_layout(&self, ty: LocalNodeId<Type>) -> Option<&UnionLayout> {
        self.union_layout_by_type.get(&ty)
    }

    /// Record union layout metadata for a type.
    pub fn set_union_layout(
        &mut self,
        ty: LocalNodeId<Type>,
        union_layout: UnionLayout,
    ) -> Option<UnionLayout> {
        self.union_layout_by_type.insert(ty, union_layout)
    }

    /// Return the class vtable id for a type when present.
    pub fn vtable_id(&self, ty: LocalNodeId<Type>) -> Option<VtableId> {
        self.vtable_by_type.get(&ty).copied()
    }

    /// Record the class vtable id for a type.
    pub fn set_vtable_id(
        &mut self,
        ty: LocalNodeId<Type>,
        vtable_id: VtableId,
    ) -> Option<VtableId> {
        self.vtable_by_type.insert(ty, vtable_id)
    }

    /// Return interface itab mappings for a concrete type when present.
    pub fn itabs(&self, ty: LocalNodeId<Type>) -> Option<&HashMap<LocalNodeId<Type>, ItabId>> {
        self.itabs_by_type.get(&ty)
    }

    /// Return the itab id for a concrete type and interface when present.
    pub fn itab_id(
        &self,
        concrete: LocalNodeId<Type>,
        interface: LocalNodeId<Type>,
    ) -> Option<ItabId> {
        self.itabs_by_type
            .get(&concrete)
            .and_then(|itabs| itabs.get(&interface))
            .copied()
    }

    /// Record the itab id for a concrete type and interface.
    pub fn set_itab_id(
        &mut self,
        concrete: LocalNodeId<Type>,
        interface: LocalNodeId<Type>,
        itab_id: ItabId,
    ) -> Option<ItabId> {
        self.itabs_by_type
            .entry(concrete)
            .or_default()
            .insert(interface, itab_id)
    }

    /// Return the field lookup map for a type when present.
    pub fn field_map(
        &self,
        ty: LocalNodeId<Type>,
    ) -> Option<&HashMap<StringId, LocalNodeId<Field>>> {
        self.field_map_by_type.get(&ty)
    }

    /// Record a field lookup map for a type.
    pub fn set_field_map(
        &mut self,
        ty: LocalNodeId<Type>,
        field_map: HashMap<StringId, LocalNodeId<Field>>,
    ) -> Option<HashMap<StringId, LocalNodeId<Field>>> {
        self.field_map_by_type.insert(ty, field_map)
    }

    /// Return the runtime type descriptor global for a type when present.
    pub fn descriptor_global(&self, ty: LocalNodeId<Type>) -> Option<LocalNodeId<Global>> {
        self.descriptor_by_type.get(&ty).copied()
    }

    /// Record the runtime type descriptor global for a type.
    pub fn set_descriptor_global(
        &mut self,
        ty: LocalNodeId<Type>,
        descriptor: LocalNodeId<Global>,
    ) -> Option<LocalNodeId<Global>> {
        self.descriptor_by_type.insert(ty, descriptor)
    }

    /// Return the display name for a type when present.
    pub fn display_name(&self, ty: LocalNodeId<Type>) -> Option<StringId> {
        self.display_name_by_type.get(&ty).copied()
    }

    /// Record the display name for a type.
    pub fn set_display_name(&mut self, ty: LocalNodeId<Type>, name: StringId) -> Option<StringId> {
        self.display_name_by_type.insert(ty, name)
    }

    /// Return the existing display name for a type or insert the provided one.
    pub fn ensure_display_name(&mut self, ty: LocalNodeId<Type>, name: StringId) -> StringId {
        *self.display_name_by_type.entry(ty).or_insert(name)
    }

    /// Copy structural metadata from one type id to another.
    pub fn copy_type_metadata(&mut self, from: LocalNodeId<Type>, to: LocalNodeId<Type>) {
        // copy layout metadata
        if let Some(layout_id) = self.layout_id(from) {
            self.set_layout_id(to, layout_id);
        }

        // copy lineage metadata
        if let Some(lineage) = self.lineage(from).cloned() {
            self.set_lineage(to, lineage);
        }

        // copy union metadata
        if let Some(union_layout) = self.union_layout(from).cloned() {
            self.set_union_layout(to, union_layout);
        }

        // copy dispatch metadata
        if let Some(vtable_id) = self.vtable_id(from) {
            self.set_vtable_id(to, vtable_id);
        }

        if let Some(itabs) = self.itabs(from).cloned() {
            self.itabs_by_type.insert(to, itabs);
        }

        // copy field metadata
        if let Some(field_map) = self.field_map(from).cloned() {
            self.set_field_map(to, field_map);
        }

        // copy runtime descriptor metadata
        if let Some(descriptor) = self.descriptor_global(from) {
            self.set_descriptor_global(to, descriptor);
        }
    }
}
