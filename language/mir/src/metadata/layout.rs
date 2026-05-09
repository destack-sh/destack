use std::collections::HashMap;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::{LocalNodeId, Type, UnionLayout};

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
