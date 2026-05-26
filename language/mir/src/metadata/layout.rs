use std::collections::HashMap;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::{LocalNodeId, Type};

/// Canonical layout metadata for one MIR module.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LayoutMetadata {
    /// Layout metadata table for aggregate types.
    pub layout_table: LayoutTable,
    /// Concrete layout ids keyed by type id.
    pub layout_by_type: HashMap<LocalNodeId<Type>, LayoutId>,
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

    /// Copy structural layout metadata from one type id to another.
    pub fn copy_type_metadata(&mut self, from: LocalNodeId<Type>, to: LocalNodeId<Type>) {
        if let Some(layout_id) = self.layout_id(from) {
            self.set_layout_id(to, layout_id);
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
    /// The layout shape.
    pub shape: LayoutShape,
    /// Total size in bytes, including trailing padding.
    pub size: u32,
    /// Alignment requirement in bytes.
    pub alignment: u32,
    /// Managed-reference metadata for this layout.
    pub trace_map: TraceMap,
}

impl Layout {
    /// Return the byte offset of a dynamic table pointer.
    pub const fn dynamic_table_offset(&self) -> Option<u32> {
        match self.shape {
            LayoutShape::Dynamic => Some(self.alignment),
            _ => None,
        }
    }
}

/// Aggregate layout shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutShape {
    /// Plain struct layout.
    Struct {
        /// Field layouts in concrete memory order.
        fields: Vec<LayoutField>,
    },
    /// Tuple layout with ordered elements.
    Tuple {
        /// Element layouts in concrete memory order.
        fields: Vec<LayoutField>,
    },
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
    /// Variant layout with tag and storage offsets.
    Variant {
        /// The byte offset of the tag field.
        tag_offset: u32,
        /// The byte offset of the storage field.
        storage_offset: u32,
    },
    /// Object layout with a class dispatch table header.
    Object {
        /// The byte offset of the class dispatch table pointer.
        table_offset: u32,
        /// Field layouts in concrete memory order.
        fields: Vec<LayoutField>,
    },
    /// Runtime dynamic value layout.
    Dynamic,
    /// Closure object layout.
    Closure,
}

impl LayoutShape {
    /// Return field layouts for field-addressable shapes.
    pub fn fields(&self) -> &[LayoutField] {
        match self {
            Self::Struct { fields } | Self::Tuple { fields } | Self::Object { fields, .. } => {
                fields
            }
            Self::Slice
            | Self::Array { .. }
            | Self::Variant { .. }
            | Self::Dynamic
            | Self::Closure => &[],
        }
    }

    /// Return this shape with field layouts attached when supported.
    pub fn with_fields(self, fields: Vec<LayoutField>) -> Self {
        match self {
            Self::Struct { .. } => Self::Struct { fields },
            Self::Tuple { .. } => Self::Tuple { fields },
            Self::Object { table_offset, .. } => Self::Object {
                table_offset,
                fields,
            },
            Self::Slice
            | Self::Array { .. }
            | Self::Variant { .. }
            | Self::Dynamic
            | Self::Closure => self,
        }
    }
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

/// Heap trace metadata for one runtime payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TraceMap {
    /// Payload contains no heap references.
    Empty,
    /// Payload stores heap-reference words at fixed byte offsets.
    Fixed {
        /// Byte offsets of encoded local heap references.
        local_offsets: Box<[u32]>,
        /// Byte offsets of encoded shared heap references.
        shared_offsets: Box<[u32]>,
    },
    /// Payload stores one nested map at a byte offset.
    Nested {
        /// The byte offset of the nested payload.
        byte_offset: u32,
        /// The nested trace map.
        map: Box<TraceMap>,
    },
    /// Payload stores multiple nested maps.
    Composite {
        /// The nested trace maps.
        maps: Box<[TraceMap]>,
    },
    /// Payload stores repeated elements with one nested trace map.
    Repeated {
        /// The number of elements in the payload.
        count: u32,
        /// The element byte stride.
        stride: u32,
        /// The per-element trace map.
        element: Box<TraceMap>,
    },
    /// Payload stores a tagged variant with variant-specific trace maps.
    Tagged {
        /// The byte offset of the variant tag.
        tag_offset: u32,
        /// The byte width of the variant tag.
        tag_bytes: u8,
        /// Variant trace maps keyed by normalized tag value.
        variants: Box<[TraceVariant]>,
    },
}

/// One tag-selected trace variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceVariant {
    /// The normalized numeric tag value selecting this variant.
    pub tag: u64,
    /// The byte offset of the variant storage.
    pub storage_offset: u32,
    /// The storage trace map for this variant.
    pub map: TraceMap,
}

impl TraceMap {
    /// Return the empty trace map.
    pub const fn empty() -> Self {
        Self::Empty
    }

    /// Report whether this map can reach heap references.
    pub fn has_reference(&self) -> bool {
        self.has_local_reference() || self.has_shared_reference()
    }

    /// Report whether this map can reach local heap references.
    pub fn has_local_reference(&self) -> bool {
        match self {
            Self::Empty => false,
            Self::Fixed { local_offsets, .. } => !local_offsets.is_empty(),
            Self::Nested { map, .. } => map.has_local_reference(),
            Self::Composite { maps } => maps.iter().any(Self::has_local_reference),
            Self::Repeated { count, element, .. } => *count > 0 && element.has_local_reference(),
            Self::Tagged { variants, .. } => variants
                .iter()
                .any(|variant| variant.map.has_local_reference()),
        }
    }

    /// Report whether this map can reach shared heap references.
    pub fn has_shared_reference(&self) -> bool {
        match self {
            Self::Empty => false,
            Self::Fixed { shared_offsets, .. } => !shared_offsets.is_empty(),
            Self::Nested { map, .. } => map.has_shared_reference(),
            Self::Composite { maps } => maps.iter().any(Self::has_shared_reference),
            Self::Repeated { count, element, .. } => *count > 0 && element.has_shared_reference(),
            Self::Tagged { variants, .. } => variants
                .iter()
                .any(|variant| variant.map.has_shared_reference()),
        }
    }

    /// Report whether this map requires reading payload tags while scanning.
    pub fn has_tagged_reference(&self) -> bool {
        match self {
            Self::Empty | Self::Fixed { .. } => false,
            Self::Nested { map, .. } => map.has_tagged_reference(),
            Self::Composite { maps } => maps.iter().any(Self::has_tagged_reference),
            Self::Repeated { element, .. } => element.has_tagged_reference(),
            Self::Tagged { variants, .. } => {
                variants.iter().any(|variant| variant.map.has_reference())
            }
        }
    }
}
