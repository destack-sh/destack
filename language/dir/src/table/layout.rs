use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{Arena, GlobalTypeId, SegmentView, StaticKey};

/// Cumulative layouts for one DIR module.
#[derive(Debug, Clone)]
pub struct LayoutTable<'a> {
    /// The module id of the layout table.
    pub module_id: ModuleId,
    /// The ordered layout table segments.
    segments: SegmentView<'a, LayoutSegment>,
}

impl LayoutTable<'static> {
    /// Create a layout table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<LayoutSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a layout table from one segment.
    pub fn from_segment(segment: Arc<LayoutSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> LayoutTable<'a> {
    /// Create a layout table from a segment view.
    pub fn from_view(segments: SegmentView<'a, LayoutSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("layout table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "layout table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create a layout table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b LayoutSegment) -> LayoutTable<'b> {
        LayoutTable::from_view(self.segments.with_tail(tail))
    }

    /// Return whether this table has no layouts.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }

    /// Iterate visible layouts.
    pub fn iter_layouts(&self) -> impl Iterator<Item = (LocalLayoutId, &Layout)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_layouts())
    }

    /// Iterate visible type layout bindings.
    pub fn type_layouts(&self) -> impl Iterator<Item = (GlobalTypeId, LocalLayoutId)> + '_ {
        self.segments
            .iter()
            .enumerate()
            .flat_map(move |(segment_index, segment)| {
                segment
                    .type_layouts
                    .iter()
                    .filter_map(move |(type_id, layout_id)| {
                        let is_shadowed = self
                            .segments
                            .iter()
                            .skip(segment_index + 1)
                            .any(|segment| segment.type_layouts.contains_key(type_id));

                        (!is_shadowed).then_some((*type_id, *layout_id))
                    })
            })
    }

    /// Return the layout id for one type.
    pub fn layout_id_for_type(&self, type_id: GlobalTypeId) -> Option<LocalLayoutId> {
        for segment in self.segments.iter().rev() {
            if let Some(layout_id) = segment.layout_id_for_type(type_id) {
                return Some(layout_id);
            }
        }

        None
    }

    /// Return the layout for one type.
    pub fn layout_for_type(&self, type_id: GlobalTypeId) -> Option<&Layout> {
        let layout_id = self.layout_id_for_type(type_id)?;

        Some(self.get_layout(layout_id))
    }

    /// Get a layout by id.
    pub fn get_layout(&self, layout_id: LocalLayoutId) -> &Layout {
        for segment in self.segments.iter() {
            if let Some(layout) = segment.get_local_layout(layout_id) {
                return layout;
            }
        }

        panic!("DIR layout {layout_id:?} is not visible")
    }

    /// Get the number of layouts in the table.
    pub fn layout_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.layout_count())
            .unwrap_or(0)
    }
}

/// Layouts added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutSegment {
    /// The module id of the layout segment.
    pub module_id: ModuleId,
    /// The first layout id owned by this table segment.
    pub(crate) first_layout_id: u32,
    /// Concrete layouts.
    pub(crate) layouts: Arena<Layout>,
    /// Layout ids keyed by canonical type id.
    pub(crate) type_layouts: IndexMap<GlobalTypeId, LocalLayoutId>,
}

impl LayoutSegment {
    /// Create an empty layout segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            first_layout_id: 0,
            layouts: Arena::new(),
            type_layouts: IndexMap::new(),
        }
    }

    /// Create a new empty segment after an existing layout table segment.
    pub fn from_base(base: &Self) -> Self {
        Self {
            module_id: base.module_id,
            first_layout_id: base.layout_count(),
            layouts: Arena::new(),
            type_layouts: IndexMap::new(),
        }
    }

    /// Insert one concrete layout.
    pub fn insert_layout(&mut self, layout: Layout) -> LocalLayoutId {
        let layout_id = LocalLayoutId::new(self.layout_count());
        self.layouts.allocate(layout);

        layout_id
    }

    /// Bind one type to a layout.
    pub fn set_type_layout(&mut self, type_id: GlobalTypeId, layout_id: LocalLayoutId) {
        self.type_layouts.insert(type_id, layout_id);
    }

    /// Return the layout id for one type.
    pub fn layout_id_for_type(&self, type_id: GlobalTypeId) -> Option<LocalLayoutId> {
        self.type_layouts.get(&type_id).copied()
    }

    /// Iterate type layout bindings.
    pub fn type_layouts(&self) -> impl Iterator<Item = (GlobalTypeId, LocalLayoutId)> + '_ {
        self.type_layouts
            .iter()
            .map(|(type_id, layout_id)| (*type_id, *layout_id))
    }

    /// Return the number of type layout bindings.
    pub fn type_layout_count(&self) -> usize {
        self.type_layouts.len()
    }

    /// Get a layout by id.
    pub fn get_layout(&self, layout_id: LocalLayoutId) -> &Layout {
        self.get_local_layout(layout_id)
            .unwrap_or_else(|| panic!("DIR layout {layout_id:?} is not allocated in this segment"))
    }

    /// Iterate layouts owned by this segment.
    pub fn iter_layouts(&self) -> impl Iterator<Item = (LocalLayoutId, &Layout)> + '_ {
        (self.first_layout_id..self.layout_count()).map(|index| {
            let layout_id = LocalLayoutId::new(index);
            (layout_id, self.get_layout(layout_id))
        })
    }

    /// Get the number of layouts in the segment.
    pub fn layout_count(&self) -> u32 {
        self.first_layout_id + self.layouts.len() as u32
    }

    /// Return whether this segment has no layouts.
    pub fn is_empty(&self) -> bool {
        self.layouts.is_empty() && self.type_layouts.is_empty()
    }

    /// Get a layout owned by this table segment.
    pub(crate) fn get_local_layout(&self, layout_id: LocalLayoutId) -> Option<&Layout> {
        self.contains_layout_id(layout_id)
            .then(|| self.layouts.get(layout_id.0 - self.first_layout_id))
    }

    /// Return whether this segment contains the given layout id.
    fn contains_layout_id(&self, layout_id: LocalLayoutId) -> bool {
        layout_id.0 >= self.first_layout_id && layout_id.0 < self.layout_count()
    }
}

/// Unique identifier for a concrete layout.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalLayoutId(pub u32);

impl LocalLayoutId {
    /// Wrap an id as a local layout id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Concrete memory layout for a checked type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layout {
    /// The layout shape.
    pub shape: LayoutShape,
    /// The size in bytes.
    pub size: u32,
    /// The alignment in bytes.
    pub alignment: u32,
    /// The largest niche of free scalar values, when one exists.
    pub niche: Option<Niche>,
}

/// One niche of free values inside a layout.
///
/// The niched scalar stores only values inside `start..=end`, leaving
/// every other bit pattern of its width free for enclosing layouts to
/// encode variant tags without extra storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Niche {
    /// The byte offset of the niched scalar.
    pub offset: u32,
    /// The niched scalar width in bytes.
    pub width: u32,
    /// The first valid value stored by the type.
    pub start: u128,
    /// The last valid value stored by the type.
    pub end: u128,
}

impl Layout {
    /// Return a storage-free layout.
    pub fn unit() -> Layout {
        Layout {
            shape: LayoutShape::None,
            size: 0,
            alignment: 1,
            niche: None,
        }
    }

    /// Return a builtin scalar layout.
    pub fn scalar(size: u32, alignment: u32, niche: Option<Niche>) -> Layout {
        Layout {
            shape: LayoutShape::Scalar,
            size,
            alignment,
            niche,
        }
    }

    /// Return a pointer layout, niched when the pointer cannot be null.
    pub fn pointer(pointer_bytes: u32, non_null: bool) -> Layout {
        let niche = non_null.then(|| Niche::non_null_pointer(pointer_bytes));

        Layout::scalar(pointer_bytes, pointer_bytes, niche)
    }

    /// Return a pointer storage slot layout naming its pointee.
    pub fn pointer_slot(pointee: GlobalTypeId, pointer_bytes: u32, non_null: bool) -> Layout {
        let niche = non_null.then(|| Niche::non_null_pointer(pointer_bytes));

        Layout {
            shape: LayoutShape::Pointer(PointerLayout { pointee }),
            size: pointer_bytes,
            alignment: pointer_bytes,
            niche,
        }
    }

    /// Return an erased value layout.
    pub fn dynamic(pointer_bytes: u32) -> Layout {
        Layout {
            shape: LayoutShape::Dynamic,
            size: pointer_bytes * 2,
            alignment: pointer_bytes,
            niche: None,
        }
    }

    /// Return a runtime function layout.
    pub fn function(pointer_bytes: u32) -> Layout {
        Layout {
            shape: LayoutShape::Function,
            size: pointer_bytes * 2,
            alignment: pointer_bytes,
            niche: None,
        }
    }
}

impl Niche {
    /// Return the niche of one non-null pointer scalar.
    pub fn non_null_pointer(pointer_bytes: u32) -> Niche {
        Niche {
            offset: 0,
            width: pointer_bytes,
            start: 1,
            end: Niche::scalar_max(pointer_bytes),
        }
    }

    /// Return the largest value one scalar width can store.
    pub fn scalar_max(bytes: u32) -> u128 {
        match bytes {
            16.. => u128::MAX,
            bytes => (1u128 << (bytes * 8)) - 1,
        }
    }

    /// Return the number of free values below and above the valid range.
    pub fn free_values(&self) -> u128 {
        let span = match self.width {
            16.. => u128::MAX,
            width => (1u128 << (u32::from(width as u8) * 8)).saturating_sub(1),
        };
        let valid = self.end.saturating_sub(self.start).saturating_add(1);

        span.saturating_sub(valid).saturating_add(1)
    }
}

/// Concrete memory layout shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Runtime dynamic value storage.
    Dynamic,
    /// Runtime function value storage.
    Function,
    /// Transparent nominal storage.
    Newtype(NewtypeLayout),
    /// Pointer storage slot for an indirectly stored value.
    Pointer(PointerLayout),
}

/// Concrete layout for a struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructLayout {
    /// The fields in layout order.
    pub fields: Vec<LayoutField>,
}

/// Concrete layout for a tuple.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TupleLayout {
    /// The tuple elements in layout order.
    pub elements: Vec<LayoutField>,
}

/// Layout for inline indexed element storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementLayout {
    /// The stored element type.
    pub element: GlobalTypeId,
    /// The byte stride between elements.
    pub stride: u32,
    /// The fixed element count when known.
    pub count: u32,
}

/// Concrete layout for a tensor handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TensorLayout {
    /// The tensor element type.
    pub element: GlobalTypeId,
    /// The tensor storage format.
    pub format: TensorFormat,
    /// The tensor placement.
    pub sharding: TensorSharding,
    /// The tensor rank.
    pub rank: u32,
}

/// Concrete layout for a tensor view descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TensorViewLayout {
    /// The viewed element type.
    pub element: GlobalTypeId,
    /// The tensor view format.
    pub format: TensorViewFormat,
    /// The tensor placement.
    pub sharding: TensorSharding,
    /// The tensor rank.
    pub rank: u32,
}

/// Dimension order for dense tensor storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TensorDimensionOrder {
    /// Last dimension is contiguous.
    RowMajor,
    /// First dimension is contiguous.
    ColumnMajor,
}

impl TensorDimensionOrder {
    /// Return the dimension order represented by one `MemoryOrder` discriminant.
    pub fn from_discriminant(value: i64) -> Option<Self> {
        match value {
            1 => Some(Self::RowMajor),
            2 => Some(Self::ColumnMajor),
            _ => None,
        }
    }
}

/// Format for an owning tensor value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TensorFormat {
    /// Dense contiguous format.
    Dense {
        /// The dimension order.
        order: TensorDimensionOrder,
    },
}

impl TensorFormat {
    /// Return the default dense row-major tensor format.
    pub fn dense_row_major() -> Self {
        Self::Dense {
            order: TensorDimensionOrder::RowMajor,
        }
    }
}

/// Format descriptor for a tensor view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TensorViewFormat {
    /// Dense contiguous view.
    Dense {
        /// The dimension order.
        order: TensorDimensionOrder,
    },
    /// Explicit strided view.
    Strided,
}

impl TensorViewFormat {
    /// Return the default dense row-major tensor view format.
    pub fn dense_row_major() -> Self {
        Self::Dense {
            order: TensorDimensionOrder::RowMajor,
        }
    }

    /// Return the pointer-sized descriptor slot count for a view of `rank`.
    pub fn descriptor_slots(self, rank: u32) -> u32 {
        match self {
            Self::Dense { .. } => 1u32.saturating_add(rank),
            Self::Strided => 1u32.saturating_add(rank.saturating_mul(2)),
        }
    }
}

impl From<TensorFormat> for TensorViewFormat {
    /// Convert an owning tensor format into its view descriptor format.
    fn from(format: TensorFormat) -> Self {
        match format {
            TensorFormat::Dense { order } => TensorViewFormat::Dense { order },
        }
    }
}

/// Placement descriptor for tensor storage.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TensorSharding {
    /// Tensor storage is not partitioned across a mesh.
    Unsharded,
    /// Tensor storage is mapped across a mesh axis by axis.
    Sharding {
        /// The per-axis placement descriptors.
        axes: Vec<TensorShardingAxis>,
    },
}

impl TensorSharding {
    /// Return the default unsharded tensor placement.
    pub fn unsharded() -> Self {
        Self::Unsharded
    }
}

/// Per-axis placement descriptor for a sharded tensor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TensorShardingAxis {
    /// Split one tensor axis across one mesh axis.
    Shard {
        /// The tensor axis being split.
        axis: i32,
    },
    /// Replicate values across one mesh axis.
    Replicate,
    /// Store partial results across one mesh axis.
    Partial {
        /// The reduction used to combine partial values.
        reduction: TensorReduction,
    },
}

/// Reduction used when partial tensor shards are combined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TensorReduction {
    /// Add partial values.
    Add,
    /// Multiply partial values.
    Multiply,
    /// Keep the minimum partial value.
    Minimum,
    /// Keep the maximum partial value.
    Maximum,
    /// Combine partial boolean values with AND.
    And,
    /// Combine partial boolean values with OR.
    Or,
}

impl TensorReduction {
    /// Return the reduction represented by one `TensorReduction` discriminant.
    pub fn from_discriminant(value: i64) -> Option<Self> {
        match value {
            1 => Some(Self::Add),
            2 => Some(Self::Multiply),
            3 => Some(Self::Minimum),
            4 => Some(Self::Maximum),
            5 => Some(Self::And),
            6 => Some(Self::Or),
            _ => None,
        }
    }
}

/// Concrete layout for a variant value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantLayout {
    /// The tag layout.
    pub tag: VariantTagLayout,
    /// The variant payload byte offset.
    pub payload_offset: Option<u32>,
    /// The variant cases.
    pub variants: Vec<VariantCaseLayout>,
}

/// Concrete layout for a variant tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantTagLayout {
    /// The tag type when it has been materialized.
    pub ty: Option<GlobalTypeId>,
    /// The tag size in bytes.
    pub size: u32,
    /// The tag alignment in bytes.
    pub alignment: u32,
}

/// Concrete layout for an object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectLayout {
    /// The fields in layout order.
    pub fields: Vec<LayoutField>,
}

/// Concrete layout for a nominal newtype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewtypeLayout {
    /// The backing type.
    pub backing_type: GlobalTypeId,
    /// The backing type layout.
    pub backing_layout: LocalLayoutId,
}

/// Concrete layout for a pointer storage slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointerLayout {
    /// The pointed-to value type.
    pub pointee: GlobalTypeId,
}

/// Concrete field or tuple-element layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutField {
    /// The field key.
    pub key: Option<StaticKey>,
    /// The field type.
    pub ty: GlobalTypeId,
    /// The field layout.
    pub layout: LocalLayoutId,
    /// The offset in bytes.
    pub offset: u32,
    /// The size in bytes.
    pub size: u32,
    /// The alignment in bytes.
    pub alignment: u32,
}

/// Concrete layout for one variant case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantCaseLayout {
    /// The logical case type.
    pub ty: GlobalTypeId,
    /// The case layout.
    pub layout: LocalLayoutId,
}
