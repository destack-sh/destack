use std::sync::Arc;

use destack_memory::AddressSpace;
use destack_mir::TraceMap;

use super::{
    GcState, HeapPageMapEntry, HeapPlace, LargeAllocation, LargeAllocationId, LocalGcState,
    SmallSpan, YoungPlace, YoungRange, YoungSpace,
};
use crate::allocator::{Allocator, PageRun, PageRunCache, SizeClassTable};
use crate::{
    AllocationPlan, AllocationShape, AllocationUsage, CowTable, HeapError, HeapOptions,
    HeapReference, HeapResult, HeapSpaceUsage, SmallSpanClass, allocation_plan,
    allocation_trace_map, slot_trace_map,
};

/// The first non-null heap large-allocation id.
const FIRST_ALLOCATED_LARGE_ALLOCATION_ID: u64 = 1;

/// One heap space over a shared allocator.
#[derive(Debug)]
pub struct HeapSpace {
    /// The shared page allocator for every heap payload.
    pub(super) allocator: Arc<Allocator>,
    /// The local cache of reusable page runs.
    pub(crate) page_run_cache: PageRunCache,

    /// The heap young space.
    pub(crate) young: YoungSpace,
    /// The heap small space.
    pub(crate) small: SmallSpace,
    /// The heap large space.
    pub(crate) large: LargeSpace,
    /// The owning heap metadata for each visible allocator page.
    pub(crate) page_map: Vec<Option<HeapPageMapEntry>>,
    /// The next unused byte offset in heap space.
    pub(crate) next_offset: usize,
    /// The fixed live byte mapping for heap space.
    pub(crate) mapping: AddressSpace,
    /// The exact live managed heap usage.
    pub(crate) usage: AllocationUsage,
    /// The exact live managed young-space usage.
    pub(crate) young_usage: AllocationUsage,
    /// The exact retained allocator-page bytes owned by live heap metadata.
    pub(crate) retained_page_bytes: u64,

    /// The maximum payload size routed to young space.
    pub(crate) max_young_allocation_bytes: usize,
    /// The completed GC cycle summary.
    pub(crate) gc: GcState,
    /// The active collector state.
    pub(crate) collector: LocalGcState,
}

impl HeapSpace {
    /// Create one heap space with explicit options.
    pub fn with_options(
        allocator: Arc<Allocator>,
        options: &HeapOptions,
    ) -> Result<Self, HeapError> {
        options.validate_local()?;
        options.validate_allocator(&allocator)?;

        Self::build_with_options(allocator, options)
    }

    /// Create one heap space from one checked options set.
    pub(crate) fn build_with_options(
        allocator: Arc<Allocator>,
        options: &HeapOptions,
    ) -> Result<Self, HeapError> {
        let mut page_run_cache = PageRunCache::new(allocator.pages_per_chunk());
        let mapping = AddressSpace::reserve(options.heap_space_bytes, options.page_bytes)?;

        // reserve one fixed young-space page run up front
        let young = YoungSpace::new(
            &allocator,
            options.heap_young_bytes,
            options.page_bytes,
            options.small_allocation_alignment_bytes,
            SmallSpanClass::bucket_count(&options.size_classes),
            &mut page_run_cache,
        )?;
        let max_young_allocation_bytes = if options.heap_young_bytes == 0 {
            0
        } else {
            options.max_heap_young_allocation_bytes
        };

        let next_heap_offset = options.heap_young_bytes.max(options.page_bytes);
        let next_heap_offset = align_up(next_heap_offset, options.page_bytes);

        // build the live space over the shared allocator
        let young_pages = young.pages;
        let retained_page_bytes = young_pages.len() as u64 * options.page_bytes as u64;
        let mut space = Self {
            allocator,
            page_run_cache,
            max_young_allocation_bytes,
            young,
            small: SmallSpace {
                size_classes: options.size_classes.clone(),
                span_bytes: options.heap_small_bytes,
                spans: CowTable::new(),
                partial_spans: vec![
                    Vec::new();
                    SmallSpanClass::bucket_count(&options.size_classes)
                ],
            },
            large: LargeSpace {
                allocations: CowTable::new(),
                free_large_allocation_ids: Vec::new(),
                next_unused_large_allocation_id: FIRST_ALLOCATED_LARGE_ALLOCATION_ID,
            },
            page_map: Vec::new(),
            next_offset: next_heap_offset,
            mapping,
            usage: AllocationUsage::default(),
            young_usage: AllocationUsage::default(),
            retained_page_bytes,
            gc: GcState::default(),
            collector: LocalGcState::default(),
        };

        space.map_page_run(0, &young_pages, |logical_page_index| {
            HeapPageMapEntry::Young { logical_page_index }
        });

        Ok(space)
    }

    /// Return the shared page allocator.
    pub fn allocator(&self) -> &Arc<Allocator> {
        &self.allocator
    }

    /// Return the exact retained heap allocator-page bytes.
    pub fn retained_bytes(&self) -> u64 {
        self.retained_page_bytes
            + self
                .page_run_cache
                .cached_bytes(self.allocator.page_bytes())
    }

    /// Return the current GC state.
    pub fn gc_state(&self) -> &GcState {
        &self.gc
    }

    /// Validate one heap reference for stable scoped access.
    pub fn stabilize(&mut self, reference: HeapReference) -> HeapResult<HeapReference> {
        let Some(_location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        Ok(reference)
    }

    /// Pin one heap reference against movement.
    pub fn pin(&mut self, reference: HeapReference) -> HeapResult<HeapReference> {
        // first validate the reference against live heap state
        let reference = self.stabilize(reference)?;

        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        // then record the active pin count
        self.collector.pins.pin(location.base)?;

        Ok(reference)
    }

    /// Release one heap pin.
    pub fn unpin(&mut self, reference: HeapReference) -> HeapResult<()> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        self.collector.pins.unpin(location.base)
    }

    /// Return the number of live heap allocations.
    pub fn allocation_count(&self) -> usize {
        self.usage.allocation_count()
    }

    /// Return the number of live heap bytes.
    pub fn allocated_bytes(&self) -> u64 {
        self.usage.allocated_bytes()
    }

    /// Return the exact live usage for this heap space.
    pub fn usage(&self) -> HeapSpaceUsage {
        HeapSpaceUsage {
            allocation_count: self.usage.allocation_count(),
            allocated_bytes: self.usage.allocated_bytes(),
            retained_bytes: self.retained_bytes(),
        }
    }

    /// Flush transient cache state before one exact branch boundary.
    pub(crate) fn flush_branch_boundary(&mut self) -> HeapResult<()> {
        self.page_run_cache.flush(&self.allocator)
    }

    /// Allocate one page run through the local page-run cache.
    pub(crate) fn allocate_page_run(&mut self, byte_len: usize) -> HeapResult<PageRun> {
        let page_run = self
            .page_run_cache
            .allocate_pages(&self.allocator, byte_len)?;
        self.retained_page_bytes += self.page_run_bytes(page_run);

        Ok(page_run)
    }

    /// Release one page run through the local page-run cache.
    pub(crate) fn release_page_run(&mut self, page_run: PageRun) -> HeapResult<()> {
        self.page_run_cache
            .release_page_run(&self.allocator, page_run)?;
        self.retained_page_bytes -= self.page_run_bytes(page_run);

        Ok(())
    }

    /// Rebuild the tracked local references that may contain shared edges.
    pub(crate) fn rebuild_shared_edge_roots(&mut self) -> HeapResult<()> {
        self.collector.clear_shared_edge_roots();

        for reference in self.live_references()? {
            if !self.reference_has_shared_roots(reference)? {
                continue;
            }

            self.collector.track_shared_edge_root(reference);
        }

        Ok(())
    }

    /// Record one young managed allocation.
    pub(crate) fn record_young_allocation(&mut self, byte_len: usize) {
        self.usage.allocate(byte_len);
        self.young_usage.allocate(byte_len);
    }

    /// Record one mature managed allocation.
    pub(crate) fn record_mature_allocation(&mut self, byte_len: usize) {
        self.usage.allocate(byte_len);
    }

    /// Record one young managed allocation free.
    pub(crate) fn record_young_free(&mut self, byte_len: usize) {
        let freed_bytes = byte_len as u64;

        self.usage.free(freed_bytes);
        self.young_usage.free(freed_bytes);
    }

    /// Record one mature managed allocation free.
    pub(crate) fn record_mature_free(&mut self, byte_len: usize) {
        self.usage.free(byte_len as u64);
    }

    /// Return the retained byte width for one page run.
    fn page_run_bytes(&self, page_run: PageRun) -> u64 {
        page_run.len() as u64 * self.allocator.page_bytes() as u64
    }

    /// Return one live large allocation by id.
    pub(crate) fn large_allocation(
        &self,
        allocation_id: LargeAllocationId,
    ) -> Option<&LargeAllocation> {
        // resolve the dense table slot first
        let index = allocation_id.index().ok()?;
        let allocation = self.large.allocations.get(index)?;

        // skip free allocations
        allocation.is_live.then_some(allocation)
    }

    /// Return one live large allocation mutably by id.
    pub(crate) fn large_allocation_mut(
        &mut self,
        allocation_id: LargeAllocationId,
    ) -> Option<&mut LargeAllocation> {
        // resolve the dense table slot first
        let index = allocation_id.index().ok()?;
        let allocation = self.large.allocations.get_mut(index)?;

        // skip free allocations
        allocation.is_live.then_some(allocation)
    }

    /// Return one live young range by object-start index.
    pub(crate) fn young_range(&self, start_index: usize) -> Option<YoungRange> {
        self.young.range(start_index)
    }

    /// Return one live young range by base offset.
    pub(crate) fn young_range_by_offset(&self, first_offset: usize) -> Option<(usize, YoungRange)> {
        self.young.range_by_offset(first_offset)
    }

    /// Return one live heap span by index.
    pub(crate) fn span(&self, span_index: usize) -> Option<&SmallSpan> {
        self.small.spans.get(span_index)
    }

    /// Return one live heap span mutably by index.
    pub(crate) fn span_mut(&mut self, span_index: usize) -> Option<&mut SmallSpan> {
        self.small.spans.get_mut(span_index)
    }

    /// Return the trace map for one heap place.
    pub(crate) fn trace_map_for_place(&self, place: HeapPlace) -> HeapResult<TraceMap> {
        match place {
            HeapPlace::Young(YoungPlace::Range { first_offset }) => {
                self.young_range_trace_map(first_offset)
            }
            HeapPlace::Young(YoungPlace::Slot(_)) => Ok(TraceMap::Empty),
            HeapPlace::Small(slot) => {
                self.small_slot_trace_map(slot.span_index(), slot.slot_index())
            }
            HeapPlace::Large(allocation_id) => {
                let trace_map = self
                    .large_allocation(allocation_id)
                    .ok_or(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    })?
                    .trace_map
                    .clone();

                Ok(trace_map)
            }
        }
    }

    /// Return the byte length for one heap place.
    pub(crate) fn byte_len_for_place(&self, place: HeapPlace) -> HeapResult<usize> {
        match place {
            HeapPlace::Young(YoungPlace::Range { first_offset }) => {
                let Some((_range_index, range)) = self.young_range_by_offset(first_offset) else {
                    return Err(HeapError::MissingYoungRange { first_offset });
                };

                Ok(range.byte_len)
            }
            HeapPlace::Young(YoungPlace::Slot(slot)) => {
                let Some(run) = self.young.run(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                Ok(run.size_class)
            }
            HeapPlace::Small(slot) => {
                let span = self.span(slot.span_index()).ok_or(HeapError::MissingSpan {
                    span_index: slot.span_index(),
                })?;

                if !span.occupied.contains(slot.slot_index()) {
                    return Err(HeapError::MissingSmallSlot {
                        span_index: slot.span_index(),
                        slot_index: slot.slot_index(),
                    });
                }

                Ok(span.class.size_class)
            }
            HeapPlace::Large(allocation_id) => Ok(self
                .large_allocation(allocation_id)
                .ok_or(HeapError::MissingLargeAllocation {
                    allocation_id: allocation_id.id(),
                })?
                .byte_len),
        }
    }

    /// Return the base reference for one heap place.
    pub(crate) fn base_reference(&self, place: HeapPlace) -> HeapResult<HeapReference> {
        let base_offset = match place {
            HeapPlace::Young(YoungPlace::Range { first_offset }) => {
                let Some((_allocation_index, allocation)) =
                    self.young_range_by_offset(first_offset)
                else {
                    return Err(HeapError::MissingYoungRange { first_offset });
                };
                allocation.first_offset
            }
            HeapPlace::Young(YoungPlace::Slot(slot)) => {
                let Some(run) = self.young.run(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                run.slot_offset(slot.slot_index())
            }
            HeapPlace::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let slot_offset = span.class.size_class * slot.slot_index();

                span.first_offset + slot_offset
            }
            HeapPlace::Large(allocation_id) => {
                let Some(allocation) = self.large_allocation(allocation_id) else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };

                allocation.first_offset
            }
        };

        Ok(HeapReference::new(base_offset))
    }

    /// Reserve one logical heap-space byte range.
    pub(crate) fn reserve_space_range(&mut self, byte_len: usize) -> HeapResult<usize> {
        self.reserve_space_range_aligned(byte_len, self.allocator.page_bytes())
    }

    /// Reserve one logical heap-space byte range with the given alignment.
    pub(crate) fn reserve_space_range_aligned(
        &mut self,
        byte_len: usize,
        alignment: usize,
    ) -> HeapResult<usize> {
        debug_assert!(self.next_offset <= self.mapping.byte_len());

        let alignment = alignment.max(self.allocator.page_bytes());
        let first_offset = align_up(self.next_offset, alignment);
        let next_offset = first_offset + byte_len;
        if next_offset > self.mapping.byte_len() {
            return Err(HeapError::InvalidByteRange {
                start: first_offset,
                len: byte_len,
                capacity: self.mapping.byte_len(),
            });
        }

        self.next_offset = next_offset;

        Ok(first_offset)
    }

    /// Return the exact trace map stored for one small slot.
    pub(crate) fn small_slot_trace_map(
        &self,
        span_index: usize,
        slot_index: usize,
    ) -> HeapResult<TraceMap> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::MissingSpan { span_index });
        };
        if !span.occupied.contains(slot_index) {
            return Err(HeapError::MissingSmallSlot {
                span_index,
                slot_index,
            });
        }

        Ok(slot_trace_map(
            &span.local_reference_bits,
            &span.shared_reference_bits,
            slot_index,
            span.class.size_class,
            span.class.size_class,
        ))
    }

    /// Return the exact trace map stored for one young range.
    pub(crate) fn young_range_trace_map(&self, first_offset: usize) -> HeapResult<TraceMap> {
        let Some((_range_index, range)) = self.young_range_by_offset(first_offset) else {
            return Err(HeapError::MissingYoungRange { first_offset });
        };
        Ok(allocation_trace_map(
            &self.young.local_reference_bits,
            &self.young.shared_reference_bits,
            range.first_offset,
            range.byte_len,
        ))
    }

    /// Return the reusable-span bucket index for one small-span class.
    pub(crate) fn small_span_bucket(&self, class: &SmallSpanClass) -> HeapResult<usize> {
        class.bucket_index(&self.small.size_classes)
    }

    /// Resolve one allocation shape against this heap space.
    #[inline(always)]
    pub(crate) fn allocation_plan<'a>(&self, shape: AllocationShape<'a>) -> AllocationPlan<'a> {
        allocation_plan(
            shape,
            &self.small.size_classes,
            self.allocator.page_bytes(),
            self.small.span_bytes,
        )
    }

    /// Return the page runs reachable from this live heap space.
    pub(crate) fn live_page_runs(&self) -> Vec<PageRun> {
        let mut page_runs = Vec::new();

        // collect span page runs first
        page_runs.extend(self.small.spans.iter().map(|span| span.pages));

        // collect large-allocation page runs next
        page_runs.extend(
            self.large
                .allocations
                .iter()
                .map(|allocation| allocation.pages),
        );

        // collect the young-space page run last
        page_runs.push(self.young.pages);

        page_runs
    }

    /// Release allocator page runs owned by this heap space.
    fn close(&mut self) -> HeapResult<()> {
        for page_run in self.live_page_runs() {
            self.release_page_run(page_run)?;
        }

        self.page_run_cache.flush(&self.allocator)
    }
}

impl Drop for HeapSpace {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// One heap small space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmallSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span width.
    pub(crate) span_bytes: usize,
    /// The live heap spans.
    pub(crate) spans: CowTable<SmallSpan>,
    /// The reusable non-full spans per size and scan class.
    pub(crate) partial_spans: Vec<Vec<usize>>,
}

/// One heap large space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeSpace {
    /// The live heap allocations.
    pub(crate) allocations: CowTable<LargeAllocation>,
    /// The free heap allocation ids available for reuse.
    pub(crate) free_large_allocation_ids: Vec<u64>,
    /// The next heap allocation id to allocate.
    pub(crate) next_unused_large_allocation_id: u64,
}

/// Return the offset rounded up to one allocation boundary.
fn align_up(byte_len: usize, alignment_bytes: usize) -> usize {
    let alignment_bytes = alignment_bytes.max(1);

    byte_len.div_ceil(alignment_bytes) * alignment_bytes
}
