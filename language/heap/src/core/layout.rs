use std::cmp::Ordering;

use crate::TraceView;
use serde::{Deserialize, Serialize};
use tspp_core::{Optional, SectionEntry};
use tspp_mir::{TraceId, TraceMap};
use tspp_serde::Reflect;

use crate::{
    DropCardinality, DropId, DropPlan, HeapConfigurationError, HeapError, HeapRepresentationError,
    HeapResult, SizeClassTable,
};

const ALLOCATION_CLASS_LARGE: u32 = 0;
const ALLOCATION_CLASS_SMALL: u32 = 1;
const ALLOCATION_FLAG_NOSCAN: u32 = 1 << 0;
const ALLOCATION_FLAG_SHARED_REFERENCE: u32 = 1 << 1;

/// The shape used to plan one heap allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocationShape {
    /// The exact payload byte length.
    pub byte_len: usize,
    /// The required block base alignment in bytes.
    pub alignment: usize,
    /// The canonical trace id when this allocation has table-backed metadata.
    pub trace_id: Option<TraceId>,
    /// The exact heap trace map.
    pub trace_map: TraceMap,
    /// Destruction required by this managed allocation.
    pub drop: Option<DropPlan>,
    /// Whether the allocation contains no heap references.
    pub is_noscan: bool,
    /// Whether the allocation may contain shared heap references.
    pub has_shared_reference: bool,
}

/// One compact heap allocation plan.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct AllocationPlan {
    /// The exact payload byte length.
    pub byte_len: u64,
    /// The required block base alignment in bytes.
    pub alignment: u32,
    /// The canonical trace id when table-backed tracing is required.
    pub trace_id: Optional<TraceId>,
    /// Packed allocation flags.
    pub flags: u32,
    /// Destruction required by this managed allocation.
    pub drop: Optional<DropPlan>,
    /// The heap class used by this plan.
    pub class: AllocationClass,
}

impl AllocationPlan {
    /// Create one allocation plan from one shape and class.
    #[inline(always)]
    pub const fn new(shape: &AllocationShape, class: AllocationClass) -> Self {
        Self {
            byte_len: shape.byte_len as u64,
            alignment: shape.alignment as u32,
            trace_id: match shape.trace_id {
                Some(trace_id) => Optional::some(trace_id),
                None => Optional::none(),
            },
            flags: allocation_flags(shape.is_noscan, shape.has_shared_reference),
            drop: match shape.drop {
                Some(drop) => Optional::some(drop),
                None => Optional::none(),
            },
            class,
        }
    }

    /// Return the exact payload byte length.
    #[inline(always)]
    pub const fn byte_len(self) -> usize {
        self.byte_len as usize
    }

    /// Return the required block base alignment in bytes.
    #[inline(always)]
    pub const fn alignment(self) -> usize {
        self.alignment as usize
    }

    /// Return the canonical trace id when this plan has table-backed metadata.
    #[inline(always)]
    pub const fn trace_id(self) -> Option<TraceId> {
        self.trace_id.get()
    }

    /// Return whether the allocation contains no heap references.
    #[inline(always)]
    pub const fn is_noscan(self) -> bool {
        self.flags & ALLOCATION_FLAG_NOSCAN != 0
    }

    /// Return whether the allocation may contain shared heap references.
    #[inline(always)]
    pub const fn has_shared_reference(self) -> bool {
        self.flags & ALLOCATION_FLAG_SHARED_REFERENCE != 0
    }

    /// Return the destruction required by this allocation.
    #[inline(always)]
    pub const fn drop_plan(self) -> Option<DropPlan> {
        self.drop.get()
    }

    /// Return whether this plan describes a valid non-empty heap block.
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.byte_len == 0
    }

    /// Return this plan as one complete heap allocation.
    #[inline(always)]
    pub(crate) fn allocation<'a>(&self, trace_map: &'a TraceMap) -> Allocation<'a> {
        Allocation {
            byte_len: self.byte_len(),
            alignment: self.alignment(),
            trace_id: self.trace_id(),
            trace_map,
            drop: self.drop_plan(),
            is_noscan: self.is_noscan(),
            has_shared_reference: self.has_shared_reference(),
            class: self.class,
        }
    }

    /// Resolve the trace map required by this allocation plan.
    pub fn trace_map(self, traces: TraceView<'_>) -> HeapResult<TraceMap> {
        match self.trace_id() {
            Some(trace_id) => Ok(traces.trace_map(trace_id)?),
            None => Ok(TraceMap::Empty),
        }
    }

    /// Build one dynamically sized repeated allocation from this element plan.
    pub fn repeat(self, element_trace_map: &TraceMap, count: usize) -> HeapResult<AllocationShape> {
        let (byte_len, trace_map) =
            repeated_layout(self.byte_len(), self.alignment(), element_trace_map, count)?;
        let mut shape = AllocationShape::new(byte_len, self.alignment(), None, trace_map);

        // carry element cleanup into the complete backing allocation
        if let Some(drop) = self.drop_plan() {
            shape.drop = Some(drop.repeated());
        }

        Ok(shape)
    }

    /// Return this plan as a small allocation.
    #[inline(always)]
    pub fn small_allocation(self) -> Option<SmallAllocationPlan> {
        let small = self.class.as_small()?;

        Some(SmallAllocationPlan {
            allocation: self,
            small,
        })
    }
}

impl AllocationShape {
    /// Create one allocation shape.
    #[inline(always)]
    pub fn new(
        byte_len: usize,
        alignment: usize,
        trace_id: Option<TraceId>,
        trace_map: TraceMap,
    ) -> Self {
        debug_assert!(alignment == 0 || alignment.is_power_of_two());
        let is_noscan = !trace_map.has_heap_reference();
        let has_shared_reference = trace_map.has_shared_reference();

        Self {
            byte_len,
            alignment: alignment.max(1),
            trace_id,
            trace_map,
            drop: None,
            is_noscan,
            has_shared_reference,
        }
    }

    /// Return this shape with one single-value drop plan.
    pub fn with_drop(mut self, drop: DropId) -> HeapResult<Self> {
        self.drop = Some(DropPlan::one(drop, self.byte_len)?);

        Ok(self)
    }

    /// Return whether this shape describes a valid non-empty heap block.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.byte_len == 0
    }

    /// Return whether this shape requires metadata stored per allocation.
    #[inline(always)]
    pub fn requires_individual_metadata(&self) -> bool {
        let has_dynamic_trace = !self.is_noscan && self.trace_id.is_none();
        let has_repeated_drop = self
            .drop
            .is_some_and(|drop| drop.cardinality == DropCardinality::Repeated);

        self.trace_map.has_variant_reference() || has_dynamic_trace || has_repeated_drop
    }
}

/// One heap allocation class.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct AllocationClass {
    /// Allocation class tag.
    pub tag: u32,
    /// Small allocation payload when tag names a small class.
    pub small: SmallAllocationClass,
}

impl AllocationClass {
    /// Create a large allocation class.
    pub const fn large() -> Self {
        Self {
            tag: ALLOCATION_CLASS_LARGE,
            small: SmallAllocationClass::empty(),
        }
    }

    /// Create a small allocation class.
    pub const fn small(small: SmallAllocationClass) -> Self {
        Self {
            tag: ALLOCATION_CLASS_SMALL,
            small,
        }
    }

    /// Return the small allocation class when this class uses small spans.
    pub fn as_small(self) -> Option<SmallAllocationClass> {
        match self.tag {
            ALLOCATION_CLASS_LARGE => None,
            ALLOCATION_CLASS_SMALL => Some(self.small),
            tag => unreachable!("invalid allocation class tag: {tag}"),
        }
    }

    /// Select one allocation class from concrete heap allocation parameters.
    pub(crate) fn select(
        byte_len: usize,
        alignment: usize,
        trace_id: Option<TraceId>,
        drop: Option<DropPlan>,
        is_noscan: bool,
        size_classes: &SizeClassTable,
        page_size_bytes: usize,
        span_size_bytes: usize,
    ) -> Self {
        if !is_noscan && trace_id.is_none() {
            return Self::large();
        }

        let Some(class_index) = size_classes.class_index_for_layout(byte_len, alignment) else {
            return Self::large();
        };
        let size_class = size_classes.classes[class_index];
        let minimum_byte_len = size_classes.class_minimum_byte_len(class_index, alignment);
        let span_size_bytes = size_class
            .span_size_bytes(page_size_bytes, span_size_bytes)
            .max(span_size_bytes);

        // share no-scan spans without trace identity
        let trace_id = if is_noscan { None } else { trace_id };
        let class = SmallSpanClass::new(size_class.bytes, span_size_bytes, trace_id, drop);

        // derive one dense cache index from allocation metadata and size class
        let metadata_slot = if let Some(drop) = drop {
            drop.drop.index() * 2 + 1
        } else {
            trace_id.map_or(0, |trace_id| (trace_id.index() + 1) * 2)
        };
        let cache_index = (metadata_slot * size_classes.classes.len() + class_index) * 2;
        let cache_index = SmallCacheIndex::from_class_index(cache_index + is_noscan as usize);

        Self::small(SmallAllocationClass {
            cache_index,
            minimum_byte_len: minimum_byte_len as u64,
            class,
        })
    }
}

/// One concrete heap allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Allocation<'a> {
    /// The exact payload byte length.
    pub byte_len: usize,
    /// The required block base alignment in bytes.
    pub alignment: usize,
    /// The canonical trace id when this payload has table-backed metadata.
    pub trace_id: Option<TraceId>,
    /// The exact heap trace map.
    pub trace_map: &'a TraceMap,
    /// Destruction required by this managed allocation.
    pub drop: Option<DropPlan>,
    /// Whether the payload contains no heap references.
    pub is_noscan: bool,
    /// Whether the payload may contain shared heap references.
    pub has_shared_reference: bool,
    /// The heap class used by this allocation.
    pub class: AllocationClass,
}

impl Allocation<'_> {
    /// Return whether this allocation describes a valid non-empty heap block.
    #[inline(always)]
    pub(crate) fn is_empty(&self) -> bool {
        self.byte_len == 0
    }
}

/// One small allocation routed through a mutator cursor.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct SmallAllocationPlan {
    /// The complete allocation plan used when cursor reservation misses.
    pub allocation: AllocationPlan,
    /// The resolved small allocation class.
    pub small: SmallAllocationClass,
}

impl SmallAllocationPlan {
    /// Return the exact payload byte length.
    #[inline(always)]
    pub const fn byte_len(self) -> usize {
        self.allocation.byte_len()
    }

    /// Return the small-span class for this allocation.
    #[inline(always)]
    pub const fn span_class(self) -> SmallSpanClass {
        self.small.span_class()
    }
}

/// Dense mutator cache index for one small allocation class.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub(crate) struct SmallCacheIndex(u32);

impl SmallCacheIndex {
    /// Create one class-derived small allocation cache index.
    #[inline(always)]
    pub(crate) const fn from_class_index(index: usize) -> Self {
        Self(index as u32)
    }

    /// Return this cache index as a usize.
    #[inline(always)]
    pub(crate) const fn index(self) -> usize {
        self.0 as usize
    }
}

/// One small heap allocation class.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct SmallAllocationClass {
    /// The exact mutator-cache index for this class.
    pub(crate) cache_index: SmallCacheIndex,
    /// The smallest payload byte length routed to this class.
    pub(crate) minimum_byte_len: u64,
    /// The small-span class used by local and shared spaces.
    pub(crate) class: SmallSpanClass,
}

impl SmallAllocationClass {
    /// Create an empty small allocation class.
    pub const fn empty() -> Self {
        Self {
            cache_index: SmallCacheIndex(0),
            minimum_byte_len: 0,
            class: SmallSpanClass::empty(),
        }
    }
}

/// One small-span size and scan class.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct SmallSpanClass {
    /// The slot payload size in bytes.
    pub(crate) size_class: u64,
    /// The span byte width for this size class.
    pub(crate) span_size_bytes: u64,
    /// The trace id shared by every slot when tracing is required.
    pub(crate) trace_id: Optional<TraceId>,
    /// Destruction shared by every managed slot.
    pub(crate) drop: Optional<DropPlan>,
}

impl SmallSpanClass {
    /// Create an empty small-span class.
    pub const fn empty() -> Self {
        Self {
            size_class: 0,
            span_size_bytes: 0,
            trace_id: Optional::none(),
            drop: Optional::none(),
        }
    }

    /// Create one small-span class from a validated size class.
    pub(crate) const fn new(
        size_class: usize,
        span_size_bytes: usize,
        trace_id: Option<TraceId>,
        drop: Option<DropPlan>,
    ) -> Self {
        Self {
            size_class: size_class as u64,
            span_size_bytes: span_size_bytes as u64,
            trace_id: match trace_id {
                Some(trace_id) => Optional::some(trace_id),
                None => Optional::none(),
            },
            drop: match drop {
                Some(drop) => Optional::some(drop),
                None => Optional::none(),
            },
        }
    }

    /// Return the slot payload size in bytes.
    pub const fn size_class(self) -> usize {
        self.size_class as usize
    }

    /// Return the span byte width for this size class.
    pub const fn span_size_bytes(self) -> usize {
        self.span_size_bytes as usize
    }

    /// Return the table-backed trace id shared by every slot in this class.
    pub const fn trace_id(self) -> Option<TraceId> {
        self.trace_id.get()
    }

    /// Return the drop plan shared by every slot in this class.
    pub const fn drop_plan(self) -> Option<DropPlan> {
        self.drop.get()
    }

    /// Return whether every slot in this class contains no heap references.
    pub const fn is_noscan(self) -> bool {
        self.trace_id.get().is_none()
    }

    /// Validate this class against one size-class table and span policy.
    pub(crate) fn validate(
        &self,
        size_classes: &SizeClassTable,
        page_size_bytes: usize,
        span_size_bytes: usize,
    ) -> HeapResult<()> {
        let Some(class_index) = size_classes.class_index_for(self.size_class()) else {
            return Err(HeapError::configuration(
                HeapConfigurationError::InvalidSizeClass {
                    class_bytes: self.size_class(),
                },
            ));
        };

        let size_class = size_classes.classes[class_index];
        if size_class.bytes != self.size_class() {
            return Err(HeapError::configuration(
                HeapConfigurationError::InvalidSizeClass {
                    class_bytes: self.size_class(),
                },
            ));
        }
        let configured_span_bytes = size_class
            .span_size_bytes(page_size_bytes, span_size_bytes)
            .max(span_size_bytes);
        if configured_span_bytes != self.span_size_bytes() {
            return Err(HeapError::configuration(
                HeapConfigurationError::InvalidSizeClass {
                    class_bytes: self.size_class(),
                },
            ));
        }

        Ok(())
    }
}

impl SmallAllocationClass {
    /// Return the exact mutator-cache index for this small allocation.
    #[inline(always)]
    pub const fn cache_index(self) -> usize {
        self.cache_index.index()
    }

    /// Return the small-span class for this small allocation.
    #[inline(always)]
    pub const fn span_class(self) -> SmallSpanClass {
        self.class
    }

    /// Return the slot payload size in bytes for this small allocation.
    #[inline(always)]
    pub const fn slot_bytes(self) -> usize {
        self.class.size_class()
    }
}

impl PartialOrd for SmallSpanClass {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SmallSpanClass {
    fn cmp(&self, other: &Self) -> Ordering {
        self.size_class()
            .cmp(&other.size_class())
            .then_with(|| self.span_size_bytes().cmp(&other.span_size_bytes()))
            .then_with(|| self.trace_id().cmp(&other.trace_id()))
            .then_with(|| self.drop_plan().cmp(&other.drop_plan()))
    }
}

/// Pack allocation booleans into fixed-width plan flags.
const fn allocation_flags(is_noscan: bool, has_shared_reference: bool) -> u32 {
    (if is_noscan { ALLOCATION_FLAG_NOSCAN } else { 0 })
        | (if has_shared_reference {
            ALLOCATION_FLAG_SHARED_REFERENCE
        } else {
            0
        })
}

/// Return the allocation plan for one repeated element layout.
fn repeated_layout(
    element_byte_len: usize,
    element_alignment: usize,
    element_trace_map: &TraceMap,
    count: usize,
) -> HeapResult<(usize, TraceMap)> {
    let element_stride = align_up(element_byte_len, element_alignment);
    let byte_len = element_stride
        .checked_mul(count)
        .ok_or(HeapError::representation(
            HeapRepresentationError::limit_exceeded("repeated payload byte length"),
        ))?;
    let trace_map = repeated_trace_map(element_trace_map, element_stride, count)?;

    Ok((byte_len, trace_map))
}

/// Return the offset rounded up to the nearest alignment boundary.
pub(crate) fn align_up(bytes: usize, alignment_bytes: usize) -> usize {
    let alignment_bytes = alignment_bytes.max(1);

    bytes.div_ceil(alignment_bytes) * alignment_bytes
}

/// Return the repeated trace map for one repeated element layout.
fn repeated_trace_map(
    element_map: &TraceMap,
    element_stride: usize,
    count: usize,
) -> HeapResult<TraceMap> {
    if count == 0 {
        return Ok(TraceMap::Empty);
    }

    match element_map {
        TraceMap::Empty => Ok(TraceMap::Empty),
        _ if !element_map.has_heap_reference() => Ok(TraceMap::Empty),
        _ => Ok(TraceMap::Repeated {
            count: u32::try_from(count).map_err(|_| {
                HeapError::representation(HeapRepresentationError::limit_exceeded(
                    "repeated payload element count",
                ))
            })?,
            stride: u32::try_from(element_stride).map_err(|_| {
                HeapError::representation(HeapRepresentationError::limit_exceeded(
                    "repeated payload element stride",
                ))
            })?,
            element: Box::new(element_map.clone()),
        }),
    }
}
