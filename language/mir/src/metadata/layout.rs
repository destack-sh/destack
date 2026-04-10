use std::collections::HashMap;
use std::mem::size_of;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::{Field, Global, LocalNodeId, Type};

/// Approximate per-entry overhead for one hash-map entry.
const HASH_MAP_ENTRY_OVERHEAD_BYTES: usize = size_of::<usize>() * 3;

/// Table of canonical well known MIR types.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WellKnownTypes {
    /// Canonical well known string reference type.
    pub string: Option<LocalNodeId<Type>>,
}

impl WellKnownTypes {
    /// Copy one canonical identity when the type id is remapped.
    pub fn remap_type(&mut self, from: LocalNodeId<Type>, to: LocalNodeId<Type>) {
        if self.string == Some(from) {
            self.string = Some(to);
        }
    }
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

/// Primitive type cache for fast lookups.
#[derive(Clone, Debug, Default)]
pub(crate) struct PrimitiveTypeCache {
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
pub(crate) enum PrimitiveTypeCacheEntry {
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

/// Canonical layout facts for one MIR module.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LayoutMetadata {
    /// Canonical module storage metadata.
    pub storage: Storage,
    /// Cached primitive type ids.
    #[serde(skip, default)]
    pub(crate) primitive_type_cache: PrimitiveTypeCache,
    /// Layout metadata table for aggregate types.
    pub layout_table: LayoutTable,
    /// Concrete layout ids keyed by type id.
    pub layout_by_type: HashMap<LocalNodeId<Type>, LayoutId>,
    /// Nominal lineage keyed by type id.
    pub lineage_by_type: HashMap<LocalNodeId<Type>, TypeLineage>,
    /// Union layout metadata keyed by type id.
    pub union_layout_by_type: HashMap<LocalNodeId<Type>, UnionLayout>,
    /// Runtime type descriptor globals keyed by type id.
    pub descriptor_by_type: HashMap<LocalNodeId<Type>, LocalNodeId<Global>>,
    /// Canonical display names keyed by type id.
    pub display_name_by_type: HashMap<LocalNodeId<Type>, StringId>,
    /// Canonical well known MIR type identities.
    pub well_known_types: WellKnownTypes,
}

impl Default for LayoutMetadata {
    fn default() -> Self {
        Self {
            storage: Storage::default(),
            primitive_type_cache: PrimitiveTypeCache::default(),
            layout_table: LayoutTable::default(),
            layout_by_type: HashMap::default(),
            lineage_by_type: HashMap::default(),
            union_layout_by_type: HashMap::default(),
            descriptor_by_type: HashMap::default(),
            display_name_by_type: HashMap::default(),
            well_known_types: WellKnownTypes::default(),
        }
    }
}

impl LayoutMetadata {
    /// Create a new empty layout table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the cache entry for a MIR type when applicable.
    pub(crate) fn cache_entry_for_type(ty: &Type) -> Option<PrimitiveTypeCacheEntry> {
        match ty {
            Type::Void => Some(PrimitiveTypeCacheEntry::Void),
            Type::Boolean => Some(PrimitiveTypeCacheEntry::Boolean),
            Type::TypeDescriptor => Some(PrimitiveTypeCacheEntry::TypeDescriptor),
            Type::TypeId => Some(PrimitiveTypeCacheEntry::TypeId),
            Type::Isize => Some(PrimitiveTypeCacheEntry::Isize),
            Type::Usize => Some(PrimitiveTypeCacheEntry::Usize),
            Type::Int {
                width,
                is_signed: signed,
            } => Some(PrimitiveTypeCacheEntry::Int {
                width: *width,
                signed: *signed,
            }),
            Type::Float { width } => Some(PrimitiveTypeCacheEntry::Float { width: *width }),
            _ => None,
        }
    }

    /// Register a type id in the primitive cache.
    pub(crate) fn register_type_entry(
        &mut self,
        type_id: LocalNodeId<Type>,
        entry: PrimitiveTypeCacheEntry,
    ) {
        match entry {
            PrimitiveTypeCacheEntry::Void => {
                self.primitive_type_cache.void.get_or_insert(type_id);
            }
            PrimitiveTypeCacheEntry::Boolean => {
                self.primitive_type_cache.boolean.get_or_insert(type_id);
            }
            PrimitiveTypeCacheEntry::TypeDescriptor => {
                self.primitive_type_cache
                    .type_descriptor
                    .get_or_insert(type_id);
            }
            PrimitiveTypeCacheEntry::TypeId => {
                self.primitive_type_cache.type_id.get_or_insert(type_id);
            }
            PrimitiveTypeCacheEntry::Isize => {
                self.primitive_type_cache.isize.get_or_insert(type_id);
            }
            PrimitiveTypeCacheEntry::Usize => {
                self.primitive_type_cache.usize.get_or_insert(type_id);
            }
            PrimitiveTypeCacheEntry::Int { width, signed } => {
                self.primitive_type_cache
                    .ints
                    .entry((width, signed))
                    .or_insert(type_id);
            }
            PrimitiveTypeCacheEntry::Float { width } => {
                self.primitive_type_cache
                    .floats
                    .entry(width)
                    .or_insert(type_id);
            }
        }
    }

    /// Return the cached boolean type id.
    pub fn boolean_type(&self) -> Option<LocalNodeId<Type>> {
        self.primitive_type_cache.boolean
    }

    /// Return the cached void type id.
    pub fn void_type(&self) -> Option<LocalNodeId<Type>> {
        self.primitive_type_cache.void
    }

    /// Return the cached type descriptor type id.
    pub fn type_descriptor_type(&self) -> Option<LocalNodeId<Type>> {
        self.primitive_type_cache.type_descriptor
    }

    /// Return the cached type id type id.
    pub fn type_id_type(&self) -> Option<LocalNodeId<Type>> {
        self.primitive_type_cache.type_id
    }

    /// Return the cached isize type id.
    pub fn isize_type(&self) -> Option<LocalNodeId<Type>> {
        self.primitive_type_cache.isize
    }

    /// Return the cached usize type id.
    pub fn usize_type(&self) -> Option<LocalNodeId<Type>> {
        self.primitive_type_cache.usize
    }

    /// Return the cached integer type id for a width and signedness.
    pub fn int_type(&self, width: u16, signed: bool) -> Option<LocalNodeId<Type>> {
        self.primitive_type_cache
            .ints
            .get(&(width, signed))
            .copied()
    }

    /// Return the cached float type id for a width.
    pub fn float_type(&self, width: u16) -> Option<LocalNodeId<Type>> {
        self.primitive_type_cache.floats.get(&width).copied()
    }

    /// Clear the primitive type cache.
    pub fn clear_type_cache(&mut self) {
        self.primitive_type_cache = PrimitiveTypeCache::default();
    }

    /// Return the layout entry for a type id when available.
    pub fn type_layout(&self, ty: LocalNodeId<Type>) -> Option<&crate::Layout> {
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

    /// Return the canonical well known string type.
    pub fn string_type(&self) -> Option<LocalNodeId<Type>> {
        self.well_known_types.string
    }

    /// Record the canonical well known string type.
    pub fn set_string_type(&mut self, type_id: LocalNodeId<Type>) -> Option<LocalNodeId<Type>> {
        self.well_known_types.string.replace(type_id)
    }

    /// Copy structural layout metadata from one type id to another.
    pub fn copy_type_metadata(&mut self, from: LocalNodeId<Type>, to: LocalNodeId<Type>) {
        if let Some(layout_id) = self.layout_id(from) {
            self.set_layout_id(to, layout_id);
        }

        if let Some(lineage) = self.lineage(from).cloned() {
            self.set_lineage(to, lineage);
        }

        if let Some(union_layout) = self.union_layout(from).cloned() {
            self.set_union_layout(to, union_layout);
        }

        if let Some(descriptor) = self.descriptor_global(from) {
            self.set_descriptor_global(to, descriptor);
        }

        if let Some(display_name) = self.display_name(from) {
            self.set_display_name(to, display_name);
        }

        self.well_known_types.remap_type(from, to);
    }

    /// Return the owned bytes for this layout metadata.
    pub fn owned_bytes(&self) -> usize {
        let mut owned_bytes = size_of::<Self>();
        owned_bytes += self.layout_table.owned_bytes();
        owned_bytes += hash_map_bytes(&self.layout_by_type);
        owned_bytes += hash_map_bytes(&self.lineage_by_type);
        owned_bytes += hash_map_bytes(&self.union_layout_by_type);
        owned_bytes += hash_map_bytes(&self.descriptor_by_type);
        owned_bytes += hash_map_bytes(&self.display_name_by_type);

        for lineage in self.lineage_by_type.values() {
            owned_bytes += lineage.interfaces.capacity() * size_of::<LocalNodeId<Type>>();
        }

        owned_bytes
    }
}

/// Return the approximate owned bytes for one hash map table.
fn hash_map_bytes<K, V>(map: &HashMap<K, V>) -> usize {
    size_of::<HashMap<K, V>>()
        + map.capacity() * (size_of::<K>() + size_of::<V>() + HASH_MAP_ENTRY_OVERHEAD_BYTES)
}

/// Canonical module storage metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Storage {
    /// Native pointer size in bytes for this module.
    pub native_pointer_bytes: u8,
    /// Managed reference representation for this module.
    pub managed_reference_layout: ManagedReferenceLayout,
}

/// Managed reference representation metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedReferenceLayout {
    /// Managed reference size in bytes.
    pub bytes: u8,
    /// Managed reference alignment in bytes.
    pub alignment: u8,
    /// Managed reference encoding.
    pub representation: ManagedReferenceRepresentation,
}

/// Managed reference encoding strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManagedReferenceRepresentation {
    /// Native machine pointer.
    NativePointer,
    /// Offset from a managed heap base.
    CompressedOffset32,
    /// Indirect 32 bit handle.
    Handle32,
    /// Indirect 64 bit handle.
    Handle64,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            native_pointer_bytes: 8,
            managed_reference_layout: ManagedReferenceLayout::default(),
        }
    }
}

impl Default for ManagedReferenceLayout {
    fn default() -> Self {
        Self {
            bytes: 8,
            alignment: 8,
            representation: ManagedReferenceRepresentation::NativePointer,
        }
    }
}

impl Storage {
    /// Create storage metadata with a specific pointer size.
    pub fn with_pointer_bytes(pointer_bytes: u8) -> Self {
        Self {
            native_pointer_bytes: pointer_bytes,
            managed_reference_layout: ManagedReferenceLayout {
                bytes: pointer_bytes,
                alignment: pointer_bytes,
                representation: ManagedReferenceRepresentation::NativePointer,
            },
        }
    }

    /// Return pointer width in bits.
    pub fn pointer_bits(self) -> u16 {
        u16::from(self.native_pointer_bytes) * 8
    }

    /// Return managed reference width in bits.
    pub fn managed_reference_bits(self) -> u16 {
        u16::from(self.managed_reference_layout.bytes) * 8
    }
}

/// Opaque identifier for a concrete memory layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LayoutId(NonZeroU32);

impl LayoutId {
    /// Create a layout identifier from one raw value.
    #[inline]
    pub const fn new(raw: u32) -> Self {
        match NonZeroU32::new(raw) {
            Some(raw) => Self(raw),
            None => panic!("layout identifiers must be non-zero"),
        }
    }

    /// Return the raw layout identifier value.
    #[inline]
    pub const fn raw(self) -> u32 {
        self.0.get()
    }

    /// Return the zero based layout-table index.
    #[inline]
    pub const fn index(self) -> usize {
        self.raw() as usize - 1
    }
}

/// Shared layout table for all aggregate types.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LayoutTable {
    /// Layout entries indexed by LayoutId.
    pub layouts: Vec<crate::Layout>,
}

impl LayoutTable {
    /// Create an empty layout table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a layout entry and return its id.
    pub fn insert(&mut self, layout: crate::Layout) -> LayoutId {
        let next_index = self.layouts.len() + 1;
        let id = LayoutId::new(next_index as u32);
        self.layouts.push(layout);
        id
    }

    /// Return a layout entry for an id.
    pub fn layout(&self, id: LayoutId) -> &crate::Layout {
        let index = id.index();
        self.layouts
            .get(index)
            .unwrap_or_else(|| panic!("missing layout entry {index}"))
    }

    /// Return the owned bytes for this layout table.
    pub fn owned_bytes(&self) -> usize {
        let mut owned_bytes = size_of::<Self>();
        owned_bytes += self.layouts.capacity() * size_of::<crate::Layout>();

        for layout in &self.layouts {
            owned_bytes += layout.fields.capacity() * size_of::<crate::LayoutField>();
        }

        owned_bytes
    }
}

/// Concrete memory layout for an aggregate type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    /// The layout kind and kind specific data.
    pub kind: LayoutKind,
    /// Total size in bytes, including trailing padding.
    pub size: u32,
    /// Alignment requirement in bytes.
    pub alignment: u32,
    /// Field layouts in concrete memory order.
    pub fields: Vec<LayoutField>,
}

/// Memory layout for a single field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutField {
    /// Field name for lookup and debugging.
    pub name: StringId,
    /// MIR type of the field.
    pub ty: LocalNodeId<Type>,
    /// Byte offset from the start of the aggregate.
    pub offset: u32,
    /// Size of the field in bytes.
    pub size: u32,
    /// Alignment requirement of the field in bytes.
    pub alignment: u32,
    /// Original source index for stable mapping.
    pub source_index: Option<u32>,
}

/// Aggregate layout kinds with kind specific data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutKind {
    /// Plain struct layout.
    Struct,
    /// Tuple layout with ordered elements.
    Tuple,
    /// Array layout with stride and optional fixed count.
    Array {
        /// The array element type.
        element_type: LocalNodeId<Type>,
        /// The stride between array elements in bytes.
        element_stride: u32,
        /// The fixed element count when known.
        element_count: Option<u32>,
    },
    /// Union layout with tag and payload offsets.
    Union {
        /// The tag type used for discriminants.
        tag_type: LocalNodeId<Type>,
        /// The byte offset of the tag field.
        tag_offset: u32,
        /// The byte offset of the payload field.
        payload_offset: u32,
    },
    /// Interface layout with object and table offsets.
    Interface {
        /// The byte offset of the object pointer.
        object_offset: u32,
        /// The byte offset of the table pointer.
        table_offset: u32,
    },
    /// Function environment layout.
    FunctionEnvironment,
    /// Function value layout.
    Closure,
}

/// Payload storage strategy for a union layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnionPayloadKind {
    /// Store the payload inline inside the union struct.
    Inline,
    /// Store the payload as a managed box.
    Boxed,
}

/// Canonical discriminant values for tagged unions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnionDiscriminantValue {
    /// Null literal value.
    Null,
    /// Undefined literal value.
    Undefined,
    /// Boolean literal value.
    Boolean(bool),
    /// Number literal value stored as f64 bits.
    Number { bits: u64 },
    /// Bigint literal value.
    Bigint(i64),
    /// String literal value.
    String(StringId),
    /// Unique symbol literal value.
    UniqueSymbol,
}

/// Discriminant values for a field in tag order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnionDiscriminantField {
    /// The discriminant field for each union element in tag order.
    pub field_by_element: Vec<LocalNodeId<Field>>,
    /// The field name shared by all union variants.
    pub field_name: StringId,
    /// Literal values ordered by tag value.
    pub values: Vec<UnionDiscriminantValue>,
}

/// Discriminant metadata for a tagged union.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnionDiscriminant {
    /// The primary discriminant field index in `fields`.
    pub primary_field_index: u32,
    /// Discriminant fields indexed by field name.
    pub fields: Vec<UnionDiscriminantField>,
}

/// Layout metadata for a lowered union type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnionLayout {
    /// The tag field type.
    pub tag_type: LocalNodeId<Type>,
    /// The payload field type.
    pub payload_type: LocalNodeId<Type>,
    /// The payload storage strategy.
    pub payload_kind: UnionPayloadKind,
    /// The union element type ids in tag order.
    pub element_types: Vec<LocalNodeId<Type>>,
    /// The tag field name in the lowered layout.
    pub tag_field_name: StringId,
    /// The payload field name in the lowered layout.
    pub payload_field_name: StringId,
    /// Discriminant field metadata when present.
    pub discriminant: Option<UnionDiscriminant>,
}
