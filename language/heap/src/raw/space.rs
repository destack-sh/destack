use std::sync::Arc;

use super::{Allocation, AllocationId, RawPointerRecord, Span};
use crate::alloc::{Arena, SizeClassTable};
use crate::heap::{HeapLayout, RawSpaceUsage};

/// The first non-null raw allocation id.
const FIRST_ALLOCATED_RAW_ID: u64 = 1;

/// The first non-null raw allocation id in large space.
const FIRST_ALLOCATED_ALLOCATION_ID: u64 = 1;

/// One raw small-allocation space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmallSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span width.
    pub(crate) span_bytes: usize,
    /// The live raw spans.
    pub(crate) spans: Vec<Span>,
    /// The reusable non-full spans per size class.
    pub(crate) available_spans: Vec<Vec<usize>>,
}

/// One raw large space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeSpace {
    /// The configured page width for allocations in large space.
    pub(crate) page_bytes: usize,
    /// The live raw allocations.
    pub(crate) allocations: Vec<Allocation>,
    /// The free raw allocation ids available for reuse.
    pub(crate) free_allocation_ids: Vec<u64>,
    /// The next raw allocation id to allocate.
    pub(crate) next_unused_allocation_id: u64,
}

/// One live raw allocation space rooted in one arena.
#[derive(Debug, Clone)]
pub struct RawSpace {
    /// The shared page arena for every raw payload.
    pub(super) arena: Arc<Arena>,
    /// The raw small-allocation space.
    pub(crate) small: SmallSpace,
    /// The raw large space.
    pub(crate) large: LargeSpace,
    /// Dense raw pointer metadata keyed by allocation id minus one.
    pub(crate) pointers: Vec<RawPointerRecord>,
    /// The free raw allocation ids available for reuse.
    pub(crate) free_pointer_ids: Vec<u64>,
    /// The next raw allocation id to allocate.
    pub(crate) next_unused_pointer_id: u64,
    /// The number of live raw allocations.
    pub(crate) allocated_count: usize,
    /// The number of live raw bytes.
    pub(crate) allocated_bytes: u64,
}

impl Default for RawSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl RawSpace {
    /// Create one raw space with the default layout.
    pub fn new() -> Self {
        let options = HeapLayout::default();

        Self::with_layout(
            Arc::new(Arena::with_page_bytes(options.page_bytes)),
            &options,
        )
    }

    /// Create one raw space with explicit layout.
    pub fn with_layout(arena: Arc<Arena>, options: &HeapLayout) -> Self {
        // build the live root over the shared arena
        Self {
            arena,
            small: SmallSpace {
                size_classes: options.size_classes.clone(),
                span_bytes: options.raw_small_bytes,
                spans: Vec::new(),
                available_spans: vec![Vec::new(); options.size_classes.classes.len()],
            },
            large: LargeSpace {
                page_bytes: options.page_bytes,
                allocations: Vec::new(),
                free_allocation_ids: Vec::new(),
                next_unused_allocation_id: FIRST_ALLOCATED_ALLOCATION_ID,
            },
            pointers: Vec::new(),
            free_pointer_ids: Vec::new(),
            next_unused_pointer_id: FIRST_ALLOCATED_RAW_ID,
            allocated_count: 0,
            allocated_bytes: 0,
        }
    }

    /// Return the shared page arena.
    pub fn arena(&self) -> &Arc<Arena> {
        &self.arena
    }

    /// Return the number of live raw allocations.
    pub fn allocation_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the exact retained raw bytes.
    pub fn active_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the exact mapped raw page bytes.
    pub fn mapped_bytes(&self) -> u64 {
        0
    }

    /// Return the exact borrowed raw image bytes.
    pub fn borrowed_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the exact live usage for this raw space.
    pub fn usage(&self) -> RawSpaceUsage {
        RawSpaceUsage {
            allocation_count: self.allocated_count,
            allocated_bytes: self.allocated_bytes,
            active_bytes: self.active_bytes(),
            mapped_bytes: self.mapped_bytes(),
            borrowed_bytes: self.borrowed_bytes(),
        }
    }

    /// Return one live raw allocation by id.
    pub(super) fn allocation(&self, allocation_id: AllocationId) -> Option<&Allocation> {
        // resolve the dense table slot first
        let index = allocation_id.id().checked_sub(1)? as usize;
        let allocation = self.large.allocations.get(index)?;

        // skip free allocations
        if allocation.is_allocated {
            Some(allocation)
        } else {
            None
        }
    }

    /// Return one live raw allocation mutably by id.
    pub(super) fn allocation_mut(
        &mut self,
        allocation_id: AllocationId,
    ) -> Option<&mut Allocation> {
        // resolve the dense table slot first
        let index = allocation_id.id().checked_sub(1)? as usize;
        let allocation = self.large.allocations.get_mut(index)?;

        // skip free allocations
        if allocation.is_allocated {
            Some(allocation)
        } else {
            None
        }
    }

    /// Return one live raw span by index.
    pub(super) fn span(&self, span_index: usize) -> Option<&Span> {
        self.small.spans.get(span_index)
    }

    /// Return one live raw span mutably by index.
    pub(super) fn span_mut(&mut self, span_index: usize) -> Option<&mut Span> {
        self.small.spans.get_mut(span_index)
    }
}
