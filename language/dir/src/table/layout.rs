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
    pub size: Option<u32>,
    /// The alignment in bytes.
    pub alignment: Option<u32>,
}

/// Aggregate layout shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutShape {
    /// No runtime storage.
    None,
    /// Builtin scalar storage.
    Scalar,
    /// Pointer-sized erased value storage.
    Dynamic,
    /// Struct or object storage.
    Struct(StructLayout),
    /// Tuple storage.
    Tuple(TupleLayout),
    /// Variant value storage.
    Variant(VariantLayout),
    /// Transparent nominal storage.
    Newtype(NewtypeLayout),
    /// Runtime function or closure storage.
    Function,
}

/// Concrete layout for a struct or object.
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

/// Concrete layout for a variant value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantLayout {
    /// The variant cases.
    pub variants: Vec<VariantCaseLayout>,
}

/// Concrete layout for a nominal newtype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewtypeLayout {
    /// The backing type layout.
    pub backing: LocalLayoutId,
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
    pub offset: Option<u32>,
    /// The size in bytes.
    pub size: Option<u32>,
    /// The alignment in bytes.
    pub alignment: Option<u32>,
}

/// Concrete layout for one variant case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantCaseLayout {
    /// The logical case type.
    pub ty: GlobalTypeId,
    /// The case layout.
    pub layout: LocalLayoutId,
}
