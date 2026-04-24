use std::collections::HashMap;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::tree::compute_type_layout;
use crate::{
    AddressSpace, Global, LocalNodeId, NodeTree, PrimitiveTypeIndex, ReferenceKind, Type,
    TypeLineage, TypeReference, UnionLayout, slice_header_types,
};

/// Heap-reference metadata for one runtime payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReferenceMap {
    /// Payload contains no heap references.
    None,
    /// Payload stores direct heap-reference words at fixed byte offsets.
    Reference {
        /// Byte offsets of encoded local heap references.
        local_offsets: Box<[u32]>,
        /// Byte offsets of encoded shared heap references.
        shared_offsets: Box<[u32]>,
    },
    /// Payload stores repeated elements with heap-reference words at fixed element offsets.
    RepeatedReference {
        /// The number of elements in the payload.
        count: u32,
        /// The element byte stride.
        stride: u32,
        /// Local heap-reference byte offsets within each element.
        local_offsets: Box<[u32]>,
        /// Shared heap-reference byte offsets within each element.
        shared_offsets: Box<[u32]>,
    },
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
            Self::Reference { local_offsets, .. } => !local_offsets.is_empty(),
            Self::RepeatedReference {
                count,
                local_offsets,
                ..
            } => *count > 0 && !local_offsets.is_empty(),
        }
    }

    /// Report whether this map can reach shared heap references.
    pub fn has_shared_reference(&self) -> bool {
        match self {
            Self::None => false,
            Self::Reference { shared_offsets, .. } => !shared_offsets.is_empty(),
            Self::RepeatedReference {
                count,
                shared_offsets,
                ..
            } => *count > 0 && !shared_offsets.is_empty(),
        }
    }
}

/// Canonical module storage metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Storage {
    /// Native pointer size in bytes for this module.
    pub native_pointer_bytes: u8,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            native_pointer_bytes: 8,
        }
    }
}

impl Storage {
    /// Create storage metadata with a specific pointer size.
    pub fn with_pointer_bytes(pointer_bytes: u8) -> Self {
        Self {
            native_pointer_bytes: pointer_bytes,
        }
    }

    /// Return pointer width in bits.
    pub fn pointer_bits(self) -> u16 {
        u16::from(self.native_pointer_bytes) * 8
    }
}

/// Canonical layout facts for one MIR module.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LayoutMetadata {
    /// Canonical module storage metadata.
    pub storage: Storage,
    /// Cached primitive type ids.
    #[serde(skip, default)]
    pub(crate) primitive_type_index: PrimitiveTypeIndex,
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
}

impl LayoutMetadata {
    /// Create a new empty layout table.
    pub fn new() -> Self {
        Self::default()
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
    }
}

/// One layout metadata completion result.
pub(crate) type LayoutMetadataResult<T> = Result<T, LayoutMetadataError>;

/// One layout metadata completion failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LayoutMetadataError {
    /// One array length did not fit in the metadata representation.
    ArrayLengthOverflow,
    /// One aggregate layout id was missing unexpectedly.
    MissingLayoutId { type_id: LocalNodeId<Type> },
    /// One layout entry was missing unexpectedly.
    MissingLayoutEntry { index: usize },
    /// One layout computation overflowed.
    Overflow { context: &'static str },
}

impl std::fmt::Display for LayoutMetadataError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArrayLengthOverflow => {
                write!(formatter, "array length exceeds layout metadata")
            }
            Self::MissingLayoutId { type_id } => {
                write!(
                    formatter,
                    "missing layout id during layout metadata completion: {type_id:?}"
                )
            }
            Self::MissingLayoutEntry { index } => {
                write!(
                    formatter,
                    "missing layout entry during layout metadata completion: index={index}"
                )
            }
            Self::Overflow { context } => {
                write!(formatter, "layout metadata completion overflow: {context}")
            }
        }
    }
}

impl std::error::Error for LayoutMetadataError {}

/// Complete canonical layout metadata for every concrete MIR aggregate type.
pub(crate) fn complete_layout_metadata(tree: &mut NodeTree) -> LayoutMetadataResult<()> {
    let type_ids: Vec<_> = tree
        .iter_nodes::<Type>()
        .map(|(type_id, _)| type_id)
        .collect();
    let mut complete = LayoutMetadataCompletion { tree };

    for type_id in type_ids {
        complete.record_layout_for_type(type_id)?;
    }

    Ok(())
}

/// One in-place layout metadata completion pass.
struct LayoutMetadataCompletion<'a> {
    /// The MIR tree being completed.
    tree: &'a mut NodeTree,
}

impl LayoutMetadataCompletion<'_> {
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
            Type::Closure { signature } => self.record_closure_layout(type_id, *signature),
            _ => Ok(()),
        }
    }

    /// Record layout metadata for one struct type.
    fn record_struct_layout(
        &mut self,
        type_id: LocalNodeId<Type>,
        fields: &[LocalNodeId<crate::Field>],
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
        kind: crate::ReferenceKind,
        element: TypeReference,
        address_space: &AddressSpace,
        mutability: crate::Mutability,
    ) -> LayoutMetadataResult<()> {
        let (data, length) = slice_header_types(kind, element, mutability, address_space.clone());
        let data = self.tree.insert_type(data);
        let length = self.tree.insert_type(length);

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
            kind: LayoutKind::Tuple,
            size: layout.size,
            alignment,
            reference_map: ReferenceMap::empty(),
            fields: layout_fields,
        };

        self.insert_layout_entry(type_id, layout_entry);
        self.record_reference_map(type_id)?;

        Ok(())
    }

    /// Record layout metadata for one closure type.
    fn record_closure_layout(
        &mut self,
        type_id: LocalNodeId<Type>,
        signature: TypeReference,
    ) -> LayoutMetadataResult<()> {
        let Some(signature) = concrete_type(signature) else {
            return Ok(());
        };

        let environment = self.tree.ensure_function_value_environment_type();
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
            kind: LayoutKind::Closure,
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
            .ok_or(LayoutMetadataError::MissingLayoutId { type_id })?;
        let Some(layout) = self
            .tree
            .metadata
            .layout
            .layout_table
            .layouts
            .get_mut(layout_id.index())
        else {
            return Err(LayoutMetadataError::MissingLayoutEntry {
                index: layout_id.index(),
            });
        };

        layout.reference_map = reference_map;

        Ok(())
    }

    /// Build the reference-map metadata for one concrete type.
    fn build_reference_map(
        &mut self,
        type_id: LocalNodeId<Type>,
    ) -> LayoutMetadataResult<ReferenceMap> {
        if let Type::Array {
            element, length, ..
        } = self.tree.get(type_id)
        {
            let Some(element) = concrete_type(*element) else {
                return Ok(ReferenceMap::empty());
            };

            let element_layout = compute_type_layout(self.tree, element, self.tree.pointer_bytes());
            let stride = align_up(element_layout.size, element_layout.alignment);
            let count =
                u32::try_from(*length).map_err(|_| LayoutMetadataError::ArrayLengthOverflow)?;
            let mut local_offsets = Vec::new();
            let mut shared_offsets = Vec::new();
            self.append_reference_map_offsets(element, 0, &mut local_offsets, &mut shared_offsets)?;

            if local_offsets.is_empty() && shared_offsets.is_empty() {
                return Ok(ReferenceMap::empty());
            }

            return Ok(ReferenceMap::RepeatedReference {
                count,
                stride,
                local_offsets: local_offsets.into_boxed_slice(),
                shared_offsets: shared_offsets.into_boxed_slice(),
            });
        }

        let mut local_offsets = Vec::new();
        let mut shared_offsets = Vec::new();
        self.append_reference_map_offsets(type_id, 0, &mut local_offsets, &mut shared_offsets)?;

        if local_offsets.is_empty() && shared_offsets.is_empty() {
            Ok(ReferenceMap::empty())
        } else {
            Ok(ReferenceMap::Reference {
                local_offsets: local_offsets.into_boxed_slice(),
                shared_offsets: shared_offsets.into_boxed_slice(),
            })
        }
    }

    /// Append heap-reference offsets for one concrete type.
    fn append_reference_map_offsets(
        &mut self,
        type_id: LocalNodeId<Type>,
        base_offset: u32,
        local_offsets: &mut Vec<u32>,
        shared_offsets: &mut Vec<u32>,
    ) -> LayoutMetadataResult<()> {
        match self.tree.get(type_id) {
            Type::Reference {
                kind: ReferenceKind::Managed | ReferenceKind::Owned | ReferenceKind::Borrowed,
                address_space,
                ..
            } => {
                match address_space {
                    AddressSpace::Local => local_offsets.push(base_offset),
                    AddressSpace::Shared => shared_offsets.push(base_offset),
                    _ => {}
                }

                Ok(())
            }
            Type::Struct { .. }
            | Type::Tuple { .. }
            | Type::Slice { .. }
            | Type::Closure { .. } => {
                self.record_layout_for_type(type_id)?;
                let layout_id = self
                    .tree
                    .metadata
                    .layout
                    .layout_id(type_id)
                    .ok_or(LayoutMetadataError::MissingLayoutId { type_id })?;
                let layout = self
                    .tree
                    .metadata
                    .layout
                    .layout_table
                    .layout(layout_id)
                    .clone();

                for field in layout.fields {
                    let field_offset = base_offset.checked_add(field.offset).ok_or(
                        LayoutMetadataError::Overflow {
                            context: "layout trace field offset",
                        },
                    )?;

                    self.append_reference_map_offsets(
                        field.ty,
                        field_offset,
                        local_offsets,
                        shared_offsets,
                    )?;
                }

                Ok(())
            }
            Type::Array {
                element, length, ..
            } => {
                let Some(element) = concrete_type(*element) else {
                    return Ok(());
                };
                let element_layout =
                    compute_type_layout(self.tree, element, self.tree.pointer_bytes());
                let stride = align_up(element_layout.size, element_layout.alignment);
                let count =
                    u32::try_from(*length).map_err(|_| LayoutMetadataError::ArrayLengthOverflow)?;

                let mut element_local_offsets = Vec::new();
                let mut element_shared_offsets = Vec::new();
                self.append_reference_map_offsets(
                    element,
                    0,
                    &mut element_local_offsets,
                    &mut element_shared_offsets,
                )?;

                if element_local_offsets.is_empty() && element_shared_offsets.is_empty() {
                    return Ok(());
                }

                for index in 0..count {
                    let delta = stride
                        .checked_mul(index)
                        .ok_or(LayoutMetadataError::Overflow {
                            context: "layout trace array stride",
                        })?;

                    for element_offset in &element_local_offsets {
                        let offset = base_offset
                            .checked_add(delta)
                            .and_then(|value| value.checked_add(*element_offset))
                            .ok_or(LayoutMetadataError::Overflow {
                                context: "layout trace array offset",
                            })?;
                        local_offsets.push(offset);
                    }

                    for element_offset in &element_shared_offsets {
                        let offset = base_offset
                            .checked_add(delta)
                            .and_then(|value| value.checked_add(*element_offset))
                            .ok_or(LayoutMetadataError::Overflow {
                                context: "layout trace array offset",
                            })?;
                        shared_offsets.push(offset);
                    }
                }

                Ok(())
            }
            _ => Ok(()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{ParseOptions, Parser};
    use crate::{Storage, TypeAlias};
    use destack_core::ImmutableStringPool;
    use destack_source::FileId;

    /// Parse one MIR module and complete its layout metadata.
    fn parse_tree_with_layout(mir_text: &str, storage: Storage) -> (NodeTree, ImmutableStringPool) {
        let (mut tree, strings) = Parser::parse(
            FileId::new(0),
            mir_text,
            ParseOptions {
                pointer_bytes: storage.native_pointer_bytes,
            },
        )
        .validate()
        .expect("failed to parse MIR");
        tree.metadata.layout.storage = storage;
        complete_layout_metadata(&mut tree).expect("failed to complete layout metadata");

        (tree, strings)
    }

    /// Look up one aliased type by name.
    fn lookup_type_alias(
        tree: &NodeTree,
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
    fn test_complete_layout_metadata_imports_struct_layout() {
        let mir_text = r#"
type Mixed {
    first: uint8;
    second: int64;
    third: uint8;
}"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, Storage::default());
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
    fn test_complete_layout_metadata_records_struct_trace() {
        let mir_text = r#"
type Packed {
    first: uint8;
    inner: ref<int32, managed, readonly>;
    third: uint8;
}"#;
        let (tree, strings) = parse_tree_with_layout(mir_text, Storage::default());
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
            ReferenceMap::Reference {
                local_offsets: vec![8].into_boxed_slice(),
                shared_offsets: Vec::new().into_boxed_slice(),
            }
        );
    }
}
