use std::collections::HashMap;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::tree::compute_type_layout;
use crate::{
    AddressSpace, Field, LocalNodeId, Mutability, ReferenceKind, Tree, Type, TypeReference,
    UnionLayout, UnionPayloadKind, slice_header_types,
};

/// Canonical layout facts for one MIR module.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LayoutMetadata {
    /// Layout metadata table for aggregate types.
    pub layout_table: LayoutTable,
    /// Concrete layout ids keyed by type id.
    pub layout_by_type: HashMap<LocalNodeId<Type>, LayoutId>,
    /// Union layout metadata keyed by type id.
    pub union_layout_by_type: HashMap<LocalNodeId<Type>, UnionLayout>,
}

impl LayoutMetadata {
    /// Create a new empty layout table.
    pub fn new() -> Self {
        Self::default()
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

    /// Copy structural layout metadata from one type id to another.
    pub fn copy_type_metadata(&mut self, from: LocalNodeId<Type>, to: LocalNodeId<Type>) {
        if let Some(layout_id) = self.layout_id(from) {
            self.set_layout_id(to, layout_id);
        }

        if let Some(union_layout) = self.union_layout(from).cloned() {
            self.set_union_layout(to, union_layout);
        }
    }
}

/// Shared layout table for all aggregate types.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LayoutTable {
    /// Layout entries indexed by LayoutId.
    pub layouts: Vec<Layout>,
}

impl LayoutTable {
    /// Create an empty layout table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a layout entry and return its id.
    pub fn insert(&mut self, layout: Layout) -> LayoutId {
        let next_index = self.layouts.len() + 1;
        let id = LayoutId::new(next_index as u32);
        self.layouts.push(layout);
        id
    }

    /// Return a layout entry for an id.
    pub fn layout(&self, id: LayoutId) -> &Layout {
        let index = id.index();
        self.layouts
            .get(index)
            .unwrap_or_else(|| panic!("missing layout entry {index}"))
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

/// Concrete memory layout for an aggregate type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    /// The layout kind and kind specific data.
    pub kind: LayoutKind,
    /// Total size in bytes, including trailing padding.
    pub size: u32,
    /// Alignment requirement in bytes.
    pub alignment: u32,
    /// Managed-reference metadata for this layout.
    pub reference_map: ReferenceMap,
    /// Field layouts in concrete memory order.
    pub fields: Vec<LayoutField>,
}

/// Aggregate layout kinds with kind specific data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutKind {
    /// Plain struct layout.
    Struct,
    /// Heap object layout with a virtual dispatch table header.
    Object {
        /// The byte offset of the virtual dispatch table pointer.
        vtable_offset: u32,
    },
    /// Tuple layout with ordered elements.
    Tuple,
    /// Slice header layout with data and length fields.
    Slice,
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
    CallableEnvironment,
    /// Function value layout.
    Callable,
}

/// Memory layout for a single field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutField {
    /// Field name for lookup and debugging.
    pub name: Option<StringId>,
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

/// Heap-reference metadata for one runtime payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReferenceMap {
    /// Payload contains no heap references.
    None,
    /// Payload stores direct heap-reference words at fixed byte offsets.
    Direct {
        /// Byte offsets of encoded local heap references.
        local_offsets: Box<[u32]>,
        /// Byte offsets of encoded shared heap references.
        shared_offsets: Box<[u32]>,
    },
    /// Payload stores one nested map at a byte offset.
    Offset {
        /// The byte offset of the nested payload.
        byte_offset: u32,
        /// The nested reference map.
        map: Box<ReferenceMap>,
    },
    /// Payload stores multiple nested maps.
    Group {
        /// The nested reference maps.
        maps: Box<[ReferenceMap]>,
    },
    /// Payload stores repeated elements with one nested reference map.
    Repeat {
        /// The number of elements in the payload.
        count: u32,
        /// The element byte stride.
        stride: u32,
        /// The per-element reference map.
        element: Box<ReferenceMap>,
    },
    /// Payload stores a tagged union with variant-specific reference maps.
    Tagged {
        /// The byte offset of the union tag.
        tag_offset: u32,
        /// The byte width of the union tag.
        tag_bytes: u8,
        /// Variant reference maps keyed by tag value.
        variants: Box<[ReferenceVariant]>,
    },
}

/// One tagged reference-map variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReferenceVariant {
    /// The numeric tag value selecting this variant.
    pub tag: u64,
    /// The byte offset of the variant payload.
    pub payload_offset: u32,
    /// The payload reference map for this variant.
    pub map: ReferenceMap,
}

impl ReferenceMap {
    /// Return the empty reference map.
    pub const fn empty() -> Self {
        Self::None
    }

    /// Report whether this map can reach heap references.
    pub fn has_reference(&self) -> bool {
        self.has_local_reference() || self.has_shared_reference()
    }

    /// Report whether this map can reach local heap references.
    pub fn has_local_reference(&self) -> bool {
        match self {
            Self::None => false,
            Self::Direct { local_offsets, .. } => !local_offsets.is_empty(),
            Self::Offset { map, .. } => map.has_local_reference(),
            Self::Group { maps } => maps.iter().any(Self::has_local_reference),
            Self::Repeat { count, element, .. } => *count > 0 && element.has_local_reference(),
            Self::Tagged { variants, .. } => variants
                .iter()
                .any(|variant| variant.map.has_local_reference()),
        }
    }

    /// Report whether this map can reach shared heap references.
    pub fn has_shared_reference(&self) -> bool {
        match self {
            Self::None => false,
            Self::Direct { shared_offsets, .. } => !shared_offsets.is_empty(),
            Self::Offset { map, .. } => map.has_shared_reference(),
            Self::Group { maps } => maps.iter().any(Self::has_shared_reference),
            Self::Repeat { count, element, .. } => *count > 0 && element.has_shared_reference(),
            Self::Tagged { variants, .. } => variants
                .iter()
                .any(|variant| variant.map.has_shared_reference()),
        }
    }

    /// Report whether this map requires reading payload tags while scanning.
    pub fn has_tagged_reference(&self) -> bool {
        match self {
            Self::None | Self::Direct { .. } => false,
            Self::Offset { map, .. } => map.has_tagged_reference(),
            Self::Group { maps } => maps.iter().any(Self::has_tagged_reference),
            Self::Repeat { element, .. } => element.has_tagged_reference(),
            Self::Tagged { variants, .. } => {
                variants.iter().any(|variant| variant.map.has_reference())
            }
        }
    }
}

/// One layout metadata recording result.
pub(crate) type LayoutMetadataResult<T> = Result<T, LayoutMetadataError>;

/// One layout metadata recording failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LayoutMetadataError {
    /// One array length did not fit in the metadata representation.
    ArrayLengthOverflow,
    /// One layout computation overflowed.
    Overflow { context: &'static str },
}

impl std::fmt::Display for LayoutMetadataError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArrayLengthOverflow => {
                write!(formatter, "array length exceeds layout metadata")
            }
            Self::Overflow { context } => {
                write!(formatter, "layout metadata recording overflow: {context}")
            }
        }
    }
}

impl std::error::Error for LayoutMetadataError {}

/// Record canonical layout metadata for one concrete MIR type.
pub(crate) fn record_type_layout(
    tree: &mut Tree,
    type_id: LocalNodeId<Type>,
) -> LayoutMetadataResult<()> {
    let mut recorder = LayoutRecorder { tree };

    recorder.record_layout_for_type(type_id)
}

/// One layout metadata recorder.
struct LayoutRecorder<'a> {
    /// The MIR tree receiving layout metadata.
    tree: &'a mut Tree,
}

impl LayoutRecorder<'_> {
    /// Record layout metadata for one concrete aggregate type.
    fn record_layout_for_type(&mut self, type_id: LocalNodeId<Type>) -> LayoutMetadataResult<()> {
        if self.tree.metadata.layout.layout_id(type_id).is_some() {
            return Ok(());
        }

        match self.tree.get(type_id) {
            Type::Struct { fields, .. } => {
                let fields = fields.clone();
                self.record_struct_layout(type_id, &fields)
            }
            Type::Tuple { elements, .. } => {
                let elements = elements.clone();
                self.record_tuple_layout(type_id, &elements)
            }
            Type::Array {
                element, length, ..
            } => self.record_array_layout(type_id, *element, *length),
            Type::Slice {
                kind,
                element,
                address_space,
                mutability,
            } => {
                let kind = *kind;
                let element = *element;
                let address_space = address_space.clone();
                let mutability = *mutability;

                self.record_slice_layout(type_id, kind, element, &address_space, mutability)
            }
            Type::Callable { signature } => self.record_callable_layout(type_id, *signature),
            _ => Ok(()),
        }
    }

    /// Record layout metadata for one struct type.
    fn record_struct_layout(
        &mut self,
        type_id: LocalNodeId<Type>,
        fields: &[LocalNodeId<Field>],
    ) -> LayoutMetadataResult<()> {
        let mut layout_fields = Vec::with_capacity(fields.len());
        let mut offset = 0u32;

        // field layouts
        for (index, field_id) in fields.iter().enumerate() {
            let (field_name, field_type) = {
                let field = self.tree.get(*field_id);
                (field.name, field.ty)
            };
            let Some(field_type) = concrete_type(field_type) else {
                return Ok(());
            };
            let field_layout =
                compute_type_layout(self.tree, field_type, self.tree.pointer_bytes());
            offset = field_layout.align_offset(offset);

            layout_fields.push(LayoutField {
                name: field_name,
                ty: field_type,
                offset,
                size: field_layout.size,
                alignment: field_layout.alignment,
                source_index: Some(index as u32),
            });
            offset += field_layout.size;
        }

        let layout = compute_type_layout(self.tree, type_id, self.tree.pointer_bytes());
        let layout_entry = Layout {
            kind: LayoutKind::Struct,
            size: layout.size,
            alignment: layout.alignment,
            reference_map: ReferenceMap::empty(),
            fields: layout_fields,
        };

        self.insert_layout_entry(type_id, layout_entry);
        self.record_reference_map(type_id)?;

        Ok(())
    }

    /// Record layout metadata for one tuple type.
    fn record_tuple_layout(
        &mut self,
        type_id: LocalNodeId<Type>,
        elements: &[TypeReference],
    ) -> LayoutMetadataResult<()> {
        let mut layout_fields = Vec::with_capacity(elements.len());
        let mut offset = 0u32;
        let mut alignment = 1u32;

        // element layouts
        for (index, element_id) in elements.iter().copied().enumerate() {
            let Some(element_id) = concrete_type(element_id) else {
                return Ok(());
            };

            let element_layout =
                compute_type_layout(self.tree, element_id, self.tree.pointer_bytes());
            offset = element_layout.align_offset(offset);

            layout_fields.push(LayoutField {
                name: None,
                ty: element_id,
                offset,
                size: element_layout.size,
                alignment: element_layout.alignment,
                source_index: Some(index as u32),
            });

            offset += element_layout.size;
            alignment = alignment.max(element_layout.alignment);
        }

        let size = compute_type_layout(self.tree, type_id, self.tree.pointer_bytes()).size;
        let layout_entry = Layout {
            kind: LayoutKind::Tuple,
            size,
            alignment,
            reference_map: ReferenceMap::empty(),
            fields: layout_fields,
        };

        self.insert_layout_entry(type_id, layout_entry);
        self.record_reference_map(type_id)?;

        Ok(())
    }

    /// Record layout metadata for one array type.
    fn record_array_layout(
        &mut self,
        type_id: LocalNodeId<Type>,
        element: TypeReference,
        length: u64,
    ) -> LayoutMetadataResult<()> {
        let Some(element) = concrete_type(element) else {
            return Ok(());
        };

        let element_layout = compute_type_layout(self.tree, element, self.tree.pointer_bytes());
        let stride = align_up(element_layout.size, element_layout.alignment);
        let count = u32::try_from(length).map_err(|_| LayoutMetadataError::ArrayLengthOverflow)?;
        let size = stride
            .checked_mul(count)
            .ok_or(LayoutMetadataError::Overflow {
                context: "array layout size",
            })?;

        let layout_entry = Layout {
            kind: LayoutKind::Array {
                element_type: element,
                element_stride: stride,
                element_count: Some(count),
            },
            size,
            alignment: element_layout.alignment,
            reference_map: ReferenceMap::empty(),
            fields: Vec::new(),
        };

        self.insert_layout_entry(type_id, layout_entry);
        self.record_reference_map(type_id)?;

        Ok(())
    }

    /// Record layout metadata for one slice type.
    fn record_slice_layout(
        &mut self,
        type_id: LocalNodeId<Type>,
        kind: ReferenceKind,
        element: TypeReference,
        address_space: &AddressSpace,
        mutability: Mutability,
    ) -> LayoutMetadataResult<()> {
        let (data, length) = slice_header_types(kind, element, mutability, address_space.clone());
        let data = ensure_type(self.tree, data);
        let length = ensure_type(self.tree, length);

        let components = [data, length];
        let mut layout_fields = Vec::with_capacity(components.len());
        let mut offset = 0u32;
        let mut alignment = 1u32;

        // component layouts
        for (index, component_type) in components.into_iter().enumerate() {
            let component_layout =
                compute_type_layout(self.tree, component_type, self.tree.pointer_bytes());
            offset = component_layout.align_offset(offset);

            layout_fields.push(LayoutField {
                name: None,
                ty: component_type,
                offset,
                size: component_layout.size,
                alignment: component_layout.alignment,
                source_index: Some(index as u32),
            });

            offset += component_layout.size;
            alignment = alignment.max(component_layout.alignment);
        }

        let layout = compute_type_layout(self.tree, type_id, self.tree.pointer_bytes());
        let layout_entry = Layout {
            kind: LayoutKind::Slice,
            size: layout.size,
            alignment,
            reference_map: ReferenceMap::empty(),
            fields: layout_fields,
        };

        self.insert_layout_entry(type_id, layout_entry);
        self.record_reference_map(type_id)?;

        Ok(())
    }

    /// Record layout metadata for one callable type.
    fn record_callable_layout(
        &mut self,
        type_id: LocalNodeId<Type>,
        signature: TypeReference,
    ) -> LayoutMetadataResult<()> {
        let Some(signature) = concrete_type(signature) else {
            return Ok(());
        };

        let environment = self.tree.ensure_callable_environment_type();
        let components = [signature, environment];
        let mut layout_fields = Vec::with_capacity(components.len());
        let mut offset = 0u32;
        let mut alignment = 1u32;

        // component layouts
        for (index, component_type) in components.into_iter().enumerate() {
            let component_layout =
                compute_type_layout(self.tree, component_type, self.tree.pointer_bytes());
            offset = component_layout.align_offset(offset);

            layout_fields.push(LayoutField {
                name: None,
                ty: component_type,
                offset,
                size: component_layout.size,
                alignment: component_layout.alignment,
                source_index: Some(index as u32),
            });

            offset += component_layout.size;
            alignment = alignment.max(component_layout.alignment);
        }

        let layout = compute_type_layout(self.tree, type_id, self.tree.pointer_bytes());
        let layout_entry = Layout {
            kind: LayoutKind::Callable,
            size: layout.size,
            alignment,
            reference_map: ReferenceMap::empty(),
            fields: layout_fields,
        };

        self.insert_layout_entry(type_id, layout_entry);
        self.record_reference_map(type_id)?;

        Ok(())
    }

    /// Insert one layout entry and attach it to the type table.
    fn insert_layout_entry(&mut self, type_id: LocalNodeId<Type>, layout: Layout) {
        let layout_id = self.tree.metadata.layout.layout_table.insert(layout);
        self.tree.metadata.layout.set_layout_id(type_id, layout_id);
    }

    /// Compute and record the reference-map metadata for one aggregate layout.
    fn record_reference_map(&mut self, type_id: LocalNodeId<Type>) -> LayoutMetadataResult<()> {
        let reference_map = self.build_reference_map(type_id)?;
        let layout_id = self
            .tree
            .metadata
            .layout
            .layout_id(type_id)
            .unwrap_or_else(|| panic!("missing layout id for recorded type {type_id:?}"));
        let layout = self
            .tree
            .metadata
            .layout
            .layout_table
            .layouts
            .get_mut(layout_id.index())
            .unwrap_or_else(|| panic!("missing layout entry {}", layout_id.index()));

        layout.reference_map = reference_map;

        Ok(())
    }

    /// Build the reference-map metadata for one concrete type.
    fn build_reference_map(
        &mut self,
        type_id: LocalNodeId<Type>,
    ) -> LayoutMetadataResult<ReferenceMap> {
        self.build_reference_map_at(type_id, 0)
    }

    /// Build the reference map for one concrete type at one byte offset.
    fn build_reference_map_at(
        &mut self,
        type_id: LocalNodeId<Type>,
        base_offset: u32,
    ) -> LayoutMetadataResult<ReferenceMap> {
        if let Some(reference_map) = self.build_union_reference_map(type_id, base_offset)? {
            return Ok(reference_map);
        }

        match self.tree.get(type_id).clone() {
            Type::Reference {
                kind: ReferenceKind::Managed | ReferenceKind::Owned | ReferenceKind::Borrowed,
                address_space,
                ..
            } => {
                let local_offsets = match address_space {
                    AddressSpace::Local => vec![base_offset].into_boxed_slice(),
                    _ => Box::default(),
                };
                let shared_offsets = match address_space {
                    AddressSpace::Shared => vec![base_offset].into_boxed_slice(),
                    _ => Box::default(),
                };

                Ok(ReferenceMap::Direct {
                    local_offsets,
                    shared_offsets,
                })
            }
            Type::Struct { .. }
            | Type::Tuple { .. }
            | Type::Slice { .. }
            | Type::Callable { .. } => {
                self.record_layout_for_type(type_id)?;
                let layout_id = self
                    .tree
                    .metadata
                    .layout
                    .layout_id(type_id)
                    .unwrap_or_else(|| panic!("missing layout id for recorded type {type_id:?}"));
                let fields = self
                    .tree
                    .metadata
                    .layout
                    .layout_table
                    .layout(layout_id)
                    .fields
                    .clone();
                let mut maps = Vec::new();

                for field in fields {
                    let field_offset = base_offset + field.offset;
                    let reference_map = self.build_reference_map_at(field.ty, field_offset)?;

                    if reference_map.has_reference() {
                        maps.push(reference_map);
                    }
                }

                Ok(group_reference_map(maps))
            }
            Type::Array {
                element, length, ..
            } => {
                let Some(element) = concrete_type(element) else {
                    return Ok(ReferenceMap::None);
                };
                let element_layout =
                    compute_type_layout(self.tree, element, self.tree.pointer_bytes());
                let stride = align_up(element_layout.size, element_layout.alignment);
                let count =
                    u32::try_from(length).map_err(|_| LayoutMetadataError::ArrayLengthOverflow)?;
                let element = self.build_reference_map_at(element, 0)?;

                if !element.has_reference() || count == 0 {
                    return Ok(ReferenceMap::None);
                }

                let reference_map = ReferenceMap::Repeat {
                    count,
                    stride,
                    element: Box::new(element),
                };

                Ok(offset_reference_map(base_offset, reference_map))
            }
            _ => Ok(ReferenceMap::None),
        }
    }

    /// Build a tagged reference map for one union type when available.
    fn build_union_reference_map(
        &mut self,
        type_id: LocalNodeId<Type>,
        base_offset: u32,
    ) -> LayoutMetadataResult<Option<ReferenceMap>> {
        let Some(union_layout) = self.tree.metadata.layout.union_layout(type_id).cloned() else {
            return Ok(None);
        };
        let layout_id = self
            .tree
            .metadata
            .layout
            .layout_id(type_id)
            .unwrap_or_else(|| panic!("missing layout id for union type {type_id:?}"));
        let layout = self.tree.metadata.layout.layout_table.layout(layout_id);
        let LayoutKind::Union {
            tag_offset,
            payload_offset,
            ..
        } = layout.kind
        else {
            return Ok(None);
        };

        match union_layout.payload_kind {
            UnionPayloadKind::Inline => {
                let tag_layout = compute_type_layout(
                    self.tree,
                    union_layout.tag_type,
                    self.tree.pointer_bytes(),
                );
                let mut variants = Vec::new();

                for (tag, element) in union_layout.element_types.iter().copied().enumerate() {
                    let map = self.build_reference_map_at(element, 0)?;
                    if map.has_reference() {
                        variants.push(ReferenceVariant {
                            tag: tag as u64,
                            payload_offset,
                            map,
                        });
                    }
                }

                let reference_map = ReferenceMap::Tagged {
                    tag_offset,
                    tag_bytes: tag_layout.size as u8,
                    variants: variants.into_boxed_slice(),
                };
                if !reference_map.has_reference() {
                    return Ok(Some(ReferenceMap::None));
                }

                Ok(Some(offset_reference_map(base_offset, reference_map)))
            }
            UnionPayloadKind::Boxed => {
                let reference_map =
                    self.build_reference_map_at(union_layout.payload_type, base_offset)?;

                Ok(Some(reference_map))
            }
        }
    }
}

/// Return one concrete type when present.
fn concrete_type(reference: TypeReference) -> Option<LocalNodeId<Type>> {
    match reference {
        TypeReference::Type(ty) => Some(ty),
        TypeReference::Missing | TypeReference::Error => None,
    }
}

/// Return an existing type id for a shape or insert it.
fn ensure_type(tree: &mut Tree, ty: Type) -> LocalNodeId<Type> {
    if let Some(type_id) = tree
        .iter_nodes::<Type>()
        .find_map(|(type_id, existing)| (existing == &ty).then_some(type_id))
    {
        return type_id;
    }

    tree.insert_type(ty)
}

/// Align one size up to the requested alignment.
fn align_up(value: u32, alignment: u32) -> u32 {
    if alignment == 0 {
        return value;
    }

    let misalignment = value % alignment;
    if misalignment == 0 {
        value
    } else {
        value + (alignment - misalignment)
    }
}

/// Return one normalized offset reference map.
fn offset_reference_map(byte_offset: u32, map: ReferenceMap) -> ReferenceMap {
    if byte_offset == 0 || !map.has_reference() {
        return map;
    }

    ReferenceMap::Offset {
        byte_offset,
        map: Box::new(map),
    }
}

/// Return one normalized grouped reference map.
fn group_reference_map(maps: Vec<ReferenceMap>) -> ReferenceMap {
    match maps.len() {
        0 => ReferenceMap::None,
        1 => maps.into_iter().next().unwrap_or(ReferenceMap::None),
        _ => ReferenceMap::Group {
            maps: maps.into_boxed_slice(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{ParseOptions, Parser};
    use crate::{DataLayout, TypeAlias};
    use destack_core::ImmutableStringPool;
    use destack_source::FileId;

    /// Parse one MIR module with layout metadata.
    fn parse_tree_with_layout(
        mir_text: &str,
        data_layout: DataLayout,
    ) -> (Tree, ImmutableStringPool) {
        let (mut tree, strings) = Parser::parse(
            FileId::new(0),
            mir_text,
            ParseOptions {
                pointer_bytes: data_layout.pointer_bytes,
            },
        )
        .validate()
        .expect("failed to parse MIR");
        tree.metadata.data_layout = data_layout;

        (tree, strings)
    }

    /// Look up one aliased type by name.
    fn lookup_type_alias(
        tree: &Tree,
        strings: &ImmutableStringPool,
        name: &str,
    ) -> LocalNodeId<Type> {
        for (_, type_alias) in tree.iter_nodes::<TypeAlias>() {
            if strings.get(type_alias.name) == name {
                return type_alias
                    .ty
                    .ty()
                    .expect("type alias should be concrete after validation");
            }
        }

        panic!("missing type alias {name}");
    }

    /// Canonical layout metadata should match one parsed struct layout.
    #[test]
    fn test_record_type_layout_imports_struct_layout() {
        let mir_text = r#"
type Mixed {
    first: uint8;
    second: int64;
    third: uint8;
}"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, DataLayout::default());
        let ty = lookup_type_alias(&tree, &strings, "Mixed");
        let raw_layout = tree.type_layout(ty).expect("missing MIR raw layout");

        // top-level facts
        assert_eq!(raw_layout.size, 24);
        assert_eq!(raw_layout.alignment, 8);

        // field facts
        assert_eq!(raw_layout.fields.len(), 3);
        assert_eq!(raw_layout.fields[0].offset, 0);
        assert_eq!(raw_layout.fields[1].offset, 8);
        assert_eq!(raw_layout.fields[2].offset, 16);
    }

    /// Canonical layout metadata should record one heap-reference trace.
    #[test]
    fn test_record_type_layout_records_struct_trace() {
        let mir_text = r#"
type Packed {
    first: uint8;
    inner: ref<int32, managed, readonly>;
    third: uint8;
}"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, DataLayout::default());
        let ty = lookup_type_alias(&tree, &strings, "Packed");
        let layout = tree.type_layout(ty).expect("missing MIR raw layout");

        // field layout
        assert_eq!(layout.fields.len(), 3);
        assert_eq!(layout.fields[0].offset, 0);
        assert_eq!(layout.fields[1].offset, 8);
        assert_eq!(layout.fields[2].offset, 16);

        // heap-reference trace
        assert_eq!(
            layout.reference_map,
            ReferenceMap::Direct {
                local_offsets: vec![8].into_boxed_slice(),
                shared_offsets: Vec::new().into_boxed_slice(),
            }
        );
    }
}
