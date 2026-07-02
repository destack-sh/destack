use std::cmp::Ordering;

use crate::TraceView;
use destack_core::SectionEntry;
use destack_mir::{TraceId, TraceMap};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    HeapConfigurationError, HeapError, HeapRepresentationError, HeapResult, SizeClassTable,
};

const ALLOCATION_CLASS_LARGE: u32 = 0;
const ALLOCATION_CLASS_SMALL: u32 = 1;

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
    /// Whether the allocation contains no heap references.
    pub is_noscan: bool,
    /// Whether the allocation may contain shared heap references.
    pub has_shared_reference: bool,
}

/// One allocator-ready allocation plan.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AllocationPlan {
    /// The exact payload byte length.
    pub byte_len: u64,
    /// The required block base alignment in bytes.
    pub alignment: u32,
    /// The raw non-zero trace id, or zero when no table trace is used.
    pub trace_id: u32,
    /// Whether the allocation contains no heap references.
    pub is_noscan: u32,
    /// Whether the allocation may contain shared heap references.
    pub has_shared_reference: u32,
    /// The allocator class used by this plan.
    pub class: AllocationClass,
}

impl AllocationPlan {
    /// Create one allocation plan from one shape and class.
    #[inline(always)]
    pub const fn new(shape: &AllocationShape, class: AllocationClass) -> Self {
        Self {
            byte_len: shape.byte_len as u64,
            alignment: shape.alignment as u32,
            trace_id: trace_id_raw(shape.trace_id),
            is_noscan: shape.is_noscan as u32,
            has_shared_reference: shape.has_shared_reference as u32,
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
        TraceId::from_raw(self.trace_id)
    }

    /// Return whether the allocation contains no heap references.
    #[inline(always)]
    pub const fn is_noscan(self) -> bool {
        self.is_noscan != 0
    }

    /// Return whether the allocation may contain shared heap references.
    #[inline(always)]
    pub const fn has_shared_reference(self) -> bool {
        self.has_shared_reference != 0
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

    /// Return this plan as a small allocation.
    #[inline(always)]
    pub const fn small_allocation(self) -> Option<SmallAllocationPlan> {
        match self.class.as_small() {
            Some(small) => Some(SmallAllocationPlan {
                allocation: self,
                small,
            }),
            None => None,
        }
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
        let is_noscan = !trace_map.has_reference();
        let has_shared_reference = trace_map.has_shared_reference();

        Self {
            byte_len,
            alignment: alignment.max(1),
            trace_id,
            trace_map,
            is_noscan,
            has_shared_reference,
        }
    }

    /// Return whether this shape describes a valid non-empty heap block.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.byte_len == 0
    }
}

/// One allocator-ready allocation class.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
    pub const fn as_small(self) -> Option<SmallAllocationClass> {
        if self.tag == ALLOCATION_CLASS_SMALL {
            Some(self.small)
        } else {
            None
        }
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
    /// Whether the payload contains no heap references.
    pub is_noscan: bool,
    /// Whether the payload may contain shared heap references.
    pub has_shared_reference: bool,
    /// The allocator class used by this allocation.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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

/// One allocator-ready small allocation class.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct SmallSpanClass {
    /// The slot payload size in bytes.
    pub(crate) size_class: u64,
    /// The span byte width for this size class.
    pub(crate) span_size_bytes: u64,
    /// The raw non-zero trace id shared by every slot, or zero when no trace is used.
    pub(crate) trace_id: u32,
    /// Whether every slot in this span has no references.
    pub(crate) is_noscan: u32,
}

impl SmallSpanClass {
    /// Create an empty small-span class.
    pub const fn empty() -> Self {
        Self {
            size_class: 0,
            span_size_bytes: 0,
            trace_id: 0,
            is_noscan: 0,
        }
    }

    /// Create one small-span class from a validated size class.
    pub(crate) const fn new(
        size_class: usize,
        span_size_bytes: usize,
        trace_id: Option<TraceId>,
        is_noscan: bool,
    ) -> Self {
        Self {
            size_class: size_class as u64,
            span_size_bytes: span_size_bytes as u64,
            trace_id: trace_id_raw(trace_id),
            is_noscan: is_noscan as u32,
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
        TraceId::from_raw(self.trace_id)
    }

    /// Return whether every slot in this span has no references.
    pub const fn is_noscan(self) -> bool {
        self.is_noscan != 0
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

/// Resolve one allocation class against a concrete small allocation table.
pub(crate) fn allocation_class(
    byte_len: usize,
    alignment: usize,
    trace_id: Option<TraceId>,
    is_noscan: bool,
    size_classes: &SizeClassTable,
    page_size_bytes: usize,
    span_size_bytes: usize,
) -> AllocationClass {
    if !is_noscan && trace_id.is_none() {
        return AllocationClass::large();
    }

    let Some(class_index) = size_classes.class_index_for_layout(byte_len, alignment) else {
        return AllocationClass::large();
    };
    let size_class = size_classes.classes[class_index];
    let minimum_byte_len = size_classes.class_minimum_byte_len(class_index, alignment);
    let span_size_bytes = size_class
        .span_size_bytes(page_size_bytes, span_size_bytes)
        .max(span_size_bytes);

    // no-scan classes share spans regardless of their trace id
    let trace_id = if is_noscan { None } else { trace_id };
    let class = SmallSpanClass::new(size_class.bytes, span_size_bytes, trace_id, is_noscan);
    let trace_slot = trace_id.map_or(0, |trace_id| trace_id.index() + 1);
    let cache_index = (trace_slot * size_classes.classes.len() + class_index) * 2;
    let cache_index = SmallCacheIndex::from_class_index(cache_index + is_noscan as usize);

    AllocationClass::small(SmallAllocationClass {
        cache_index,
        minimum_byte_len: minimum_byte_len as u64,
        class,
    })
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
            .then_with(|| self.is_noscan().cmp(&other.is_noscan()))
    }
}

// SAFETY: allocation plans are fixed executable entries.
unsafe impl SectionEntry for AllocationPlan {}
unsafe impl SectionEntry for AllocationClass {}
unsafe impl SectionEntry for SmallAllocationPlan {}
unsafe impl SectionEntry for SmallAllocationClass {}
unsafe impl SectionEntry for SmallSpanClass {}

/// Return the raw optional trace id encoding.
const fn trace_id_raw(trace_id: Option<TraceId>) -> u32 {
    match trace_id {
        Some(trace_id) => trace_id.raw(),
        None => 0,
    }
}

/// Return the allocation plan for one repeated element layout.
pub fn repeated_layout(
    element_byte_len: usize,
    element_alignment: usize,
    element_trace_map: &TraceMap,
    count: usize,
) -> HeapResult<(usize, TraceMap)> {
    let element_stride = align_up(element_byte_len, element_alignment);
    let byte_len = element_stride
        .checked_mul(count)
        .ok_or(HeapError::representation(
            HeapRepresentationError::LimitExceeded {
                context: "repeated payload byte length",
            },
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
        _ if !element_map.has_reference() => Ok(TraceMap::Empty),
        _ => Ok(TraceMap::Repeated {
            count: u32::try_from(count).map_err(|_| {
                HeapError::representation(HeapRepresentationError::LimitExceeded {
                    context: "repeated payload element count",
                })
            })?,
            stride: u32::try_from(element_stride).map_err(|_| {
                HeapError::representation(HeapRepresentationError::LimitExceeded {
                    context: "repeated payload element stride",
                })
            })?,
            element: Box::new(element_map.clone()),
        }),
    }
}
