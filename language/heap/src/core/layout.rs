use std::borrow::Cow;
use std::cmp::Ordering;

use destack_mir::{TraceId, TraceMap, TraceTable};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    HeapConfigurationError, HeapError, HeapRepresentationError, HeapResult, SizeClassTable,
};

/// The payload shape used to plan one heap allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayloadShape<'a> {
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
}

/// One allocator-ready allocation plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AllocationPlan {
    /// The exact payload byte length.
    pub byte_len: usize,
    /// The required block base alignment in bytes.
    pub alignment: usize,
    /// The canonical trace id when this plan has table-backed metadata.
    pub trace_id: Option<TraceId>,
    /// Whether the payload contains no heap references.
    pub is_noscan: bool,
    /// Whether the payload may contain shared heap references.
    pub has_shared_reference: bool,
    /// The allocator class used by this plan.
    pub class: AllocationClass,
}

impl AllocationPlan {
    /// Create one allocation plan from one shape and class.
    #[inline(always)]
    pub const fn new(shape: PayloadShape<'_>, class: AllocationClass) -> Self {
        Self {
            byte_len: shape.byte_len,
            alignment: shape.alignment,
            trace_id: shape.trace_id,
            is_noscan: shape.is_noscan,
            has_shared_reference: shape.has_shared_reference,
            class,
        }
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
            byte_len: self.byte_len,
            alignment: self.alignment,
            trace_id: self.trace_id,
            trace_map,
            is_noscan: self.is_noscan,
            has_shared_reference: self.has_shared_reference,
            class: self.class,
        }
    }

    /// Resolve the trace map required by this allocation plan.
    pub fn trace_map<'a>(&self, traces: &'a TraceTable) -> Option<Cow<'a, TraceMap>> {
        match self.trace_id {
            Some(trace_id) => traces.trace(trace_id).map(Cow::Borrowed),
            None => Some(Cow::Owned(TraceMap::Empty)),
        }
    }

    /// Return this plan as a small allocation.
    #[inline(always)]
    pub const fn small_allocation(self) -> Option<SmallAllocationPlan> {
        match self.class {
            AllocationClass::Small(small) => Some(SmallAllocationPlan {
                allocation: self,
                small,
            }),
            AllocationClass::Large => None,
        }
    }
}

impl<'a> PayloadShape<'a> {
    /// Create one payload shape.
    #[inline(always)]
    pub fn new(
        byte_len: usize,
        alignment: usize,
        trace_id: Option<TraceId>,
        trace_map: &'a TraceMap,
    ) -> Self {
        debug_assert!(alignment == 0 || alignment.is_power_of_two());

        Self {
            byte_len,
            alignment: alignment.max(1),
            trace_id,
            trace_map,
            is_noscan: !trace_map.has_reference(),
            has_shared_reference: trace_map.has_shared_reference(),
        }
    }

    /// Return whether this shape describes a valid non-empty heap block.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.byte_len == 0
    }
}

/// One allocator-ready allocation class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum AllocationClass {
    /// One block backed by a small-span slot.
    Small(SmallAllocationClass),
    /// One block backed by a dedicated page span.
    Large,
}

impl AllocationClass {
    /// Return the small allocation class when this class uses small spans.
    pub const fn small(self) -> Option<SmallAllocationClass> {
        match self {
            Self::Small(small) => Some(small),
            Self::Large => None,
        }
    }
}

/// One concrete heap allocation with a borrowed trace map.
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

impl<'a> Allocation<'a> {
    /// Return whether this allocation describes a valid non-empty heap block.
    #[inline(always)]
    pub(crate) fn is_empty(&self) -> bool {
        self.byte_len == 0
    }
}

/// One small allocation routed through a mutator cursor.
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
        self.allocation.byte_len
    }

    /// Return the small-span class for this allocation.
    #[inline(always)]
    pub const fn span_class(self) -> SmallSpanClass {
        self.small.span_class()
    }
}

/// Dense mutator cache index for one small allocation class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub(crate) struct SmallCacheIndex(usize);

impl SmallCacheIndex {
    /// Create one class-derived small allocation cache index.
    #[inline(always)]
    pub(crate) const fn from_class_index(index: usize) -> Self {
        Self(index)
    }

    /// Return this cache index as a usize.
    #[inline(always)]
    pub(crate) const fn index(self) -> usize {
        self.0
    }
}

/// One allocator-ready small allocation class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SmallAllocationClass {
    /// The exact mutator-cache index for this class.
    pub(crate) cache_index: SmallCacheIndex,
    /// The smallest payload byte length routed to this class.
    pub(crate) minimum_byte_len: usize,
    /// The small-span class used by local and shared spaces.
    pub(crate) class: SmallSpanClass,
}

/// One small-span size and scan class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct SmallSpanClass {
    /// The slot payload size in bytes.
    pub(crate) size_class: usize,
    /// The span byte width for this size class.
    pub(crate) span_size_bytes: usize,
    /// The table-backed trace id shared by every slot in this class.
    pub(crate) trace_id: Option<TraceId>,
    /// Whether every slot in this span has no references.
    pub(crate) is_noscan: bool,
}

impl SmallSpanClass {
    /// Create one small-span class from a validated size class.
    pub(crate) const fn new(
        size_class: usize,
        span_size_bytes: usize,
        trace_id: Option<TraceId>,
        is_noscan: bool,
    ) -> Self {
        Self {
            size_class,
            span_size_bytes,
            trace_id,
            is_noscan,
        }
    }

    /// Validate this class against one size-class table and span policy.
    pub(crate) fn validate(
        &self,
        size_classes: &SizeClassTable,
        page_size_bytes: usize,
        span_size_bytes: usize,
    ) -> HeapResult<()> {
        let Some(class_index) = size_classes.class_index_for(self.size_class) else {
            return Err(HeapError::configuration(
                HeapConfigurationError::InvalidSizeClass {
                    class_bytes: self.size_class,
                },
            ));
        };

        let size_class = size_classes.classes[class_index];
        if size_class.bytes != self.size_class {
            return Err(HeapError::configuration(
                HeapConfigurationError::InvalidSizeClass {
                    class_bytes: self.size_class,
                },
            ));
        }

        let configured_span_bytes = size_class
            .span_size_bytes(page_size_bytes, span_size_bytes)
            .max(span_size_bytes);
        if configured_span_bytes != self.span_size_bytes {
            return Err(HeapError::configuration(
                HeapConfigurationError::InvalidSizeClass {
                    class_bytes: self.size_class,
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
        self.class.size_class
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
        return AllocationClass::Large;
    }

    let Some(class_index) = size_classes.class_index_for_layout(byte_len, alignment) else {
        return AllocationClass::Large;
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

    AllocationClass::Small(SmallAllocationClass {
        cache_index,
        minimum_byte_len,
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
        self.size_class
            .cmp(&other.size_class)
            .then_with(|| self.span_size_bytes.cmp(&other.span_size_bytes))
            .then_with(|| self.trace_id.cmp(&other.trace_id))
            .then_with(|| self.is_noscan.cmp(&other.is_noscan))
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
