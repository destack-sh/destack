use std::collections::HashMap;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::{LocalNodeId, TensorFormat, TensorSharding, TensorViewFormat, TraceMap, Type};

/// Canonical layout metadata for one MIR module.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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
            .unwrap_or_else(|| unreachable!("missing layout entry {index}"))
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
            None => unreachable!(),
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Return the byte offset of a dynamic dispatch pointer.
    pub const fn dynamic_dispatch_offset(&self) -> Option<u32> {
        match &self.shape {
            LayoutShape::Dynamic => Some(self.alignment),
            _ => None,
        }
    }

    /// Return the byte width of this layout.
    pub const fn byte_len(&self) -> usize {
        self.size as usize
    }

    /// Return one field by layout index.
    pub fn field_at(&self, index: u32) -> Option<&LayoutField> {
        self.shape.fields().get(index as usize)
    }

    /// Return the field count for field-addressable layouts.
    pub fn field_count(&self) -> Option<usize> {
        match &self.shape {
            LayoutShape::Struct(layout) => Some(layout.fields.len()),
            LayoutShape::Tuple(layout) => Some(layout.elements.len()),
            LayoutShape::Object(layout) => Some(layout.fields.len()),
            _ => None,
        }
    }
}

/// Concrete memory layout shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LayoutShape {
    /// No runtime storage.
    None,
    /// Builtin scalar storage.
    Scalar,
    /// Struct storage.
    Struct(StructLayout),
    /// Tuple storage.
    Tuple(TupleLayout),
    /// Slice header storage.
    Slice,
    /// Fixed array storage.
    Array(ElementLayout),
    /// Vector value storage.
    Vector(ElementLayout),
    /// Tensor handle storage.
    Tensor(TensorLayout),
    /// Tensor view descriptor storage.
    TensorView(TensorViewLayout),
    /// Variant value storage.
    Variant(VariantLayout),
    /// Object storage with a dispatch table header.
    Object(ObjectLayout),
    /// Runtime dynamic value layout.
    Dynamic,
    /// Runtime function value storage.
    Function,
    /// Transparent nominal storage.
    Newtype(NewtypeLayout),
}

impl LayoutShape {
    /// Return element layout when this shape stores indexed elements inline.
    pub const fn elements(&self) -> Option<&ElementLayout> {
        match self {
            Self::Array(layout) | Self::Vector(layout) => Some(layout),
            _ => None,
        }
    }

    /// Return field layouts for field-addressable shapes.
    pub fn fields(&self) -> &[LayoutField] {
        match self {
            Self::Struct(layout) => &layout.fields,
            Self::Tuple(layout) => &layout.elements,
            Self::Object(layout) => &layout.fields,
            Self::None
            | Self::Scalar
            | Self::Slice
            | Self::Array(_)
            | Self::Vector(_)
            | Self::Tensor(_)
            | Self::TensorView(_)
            | Self::Variant(_)
            | Self::Dynamic
            | Self::Function
            | Self::Newtype(_) => &[],
        }
    }

    /// Return this shape with field layouts attached when supported.
    pub fn with_fields(self, fields: Vec<LayoutField>) -> Self {
        match self {
            Self::Struct(_) => Self::Struct(StructLayout { fields }),
            Self::Tuple(_) => Self::Tuple(TupleLayout { elements: fields }),
            Self::Object(_) => Self::Object(ObjectLayout { fields }),
            Self::None
            | Self::Scalar
            | Self::Slice
            | Self::Array(_)
            | Self::Vector(_)
            | Self::Tensor(_)
            | Self::TensorView(_)
            | Self::Variant(_)
            | Self::Dynamic
            | Self::Function
            | Self::Newtype(_) => self,
        }
    }
}

/// Concrete layout for a struct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructLayout {
    /// The fields in layout order.
    pub fields: Vec<LayoutField>,
}

/// Concrete layout for a tuple.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TupleLayout {
    /// The tuple elements in layout order.
    pub elements: Vec<LayoutField>,
}

/// Layout for inline indexed element storage.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ElementLayout {
    /// The stored element type.
    pub element: LocalNodeId<Type>,
    /// The byte stride between elements.
    pub stride: u32,
    /// The fixed element count.
    pub count: u32,
}

/// Concrete layout for a tensor handle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TensorLayout {
    /// The tensor element type.
    pub element: LocalNodeId<Type>,
    /// The tensor storage format.
    pub format: TensorFormat,
    /// The tensor placement.
    pub sharding: TensorSharding,
    /// The tensor rank.
    pub rank: u32,
}

/// Concrete layout for a tensor view descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TensorViewLayout {
    /// The viewed element type.
    pub element: LocalNodeId<Type>,
    /// The tensor view format.
    pub format: TensorViewFormat,
    /// The tensor placement.
    pub sharding: TensorSharding,
    /// The tensor rank.
    pub rank: u32,
}

/// Concrete layout for a variant value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariantLayout {
    /// The tag layout.
    pub tag: VariantTagLayout,
    /// The variant payload byte offset.
    pub payload_offset: u32,
    /// The variant cases.
    pub variants: Vec<VariantCaseLayout>,
}

/// Concrete layout for a variant tag.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VariantTagLayout {
    /// The tag type when it has been materialized.
    pub ty: Option<LocalNodeId<Type>>,
    /// The tag size in bytes.
    pub size: u32,
    /// The tag alignment in bytes.
    pub alignment: u32,
}

/// Concrete layout for an object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectLayout {
    /// The fields in layout order.
    pub fields: Vec<LayoutField>,
}

/// Concrete layout for a nominal newtype.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NewtypeLayout {
    /// The backing type.
    pub backing_type: LocalNodeId<Type>,
    /// The backing type layout.
    pub backing_layout: LayoutId,
}

/// Memory layout for a single field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Concrete layout for one variant case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariantCaseLayout {
    /// The logical case type.
    pub ty: LocalNodeId<Type>,
    /// The case layout.
    pub layout: LayoutId,
}
