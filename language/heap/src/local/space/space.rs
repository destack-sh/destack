use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use destack_mir::ReferenceMap;

use super::{
    GcState, HeapPageMapEntry, HeapPlace, LargeAllocation, LargeAllocationId, PinSet, SmallSpan,
    YoungPlace, YoungRange, YoungSpace,
};
use crate::allocator::{AddressSpace, Allocator, PageRun, PageRunCache, SizeClassTable};
use crate::{
    AllocationLayout, AllocationPlan, CowTable, HeapError, HeapOptions, HeapReference, HeapResult,
    HeapSpaceUsage, SmallSpanClass, TraceQueue, TraceReference, allocation_layout,
    allocation_reference_map, slot_reference_map,
};

/// Collector queue for heap references.
type HeapTraceQueue = TraceQueue<HeapReference>;

/// One queued unit of local major mark work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalTraceWork {
    /// One heap allocation to scan.
    Reference(HeapReference),
    /// One range of one large heap allocation to scan.
    LargeRange {
        /// The heap allocation reference.
        reference: HeapReference,
        /// The range start in bytes.
        start: usize,
    },
}

impl TraceReference for LocalTraceWork {
    /// Report whether this trace work points at null.
    fn is_null(self) -> bool {
        match self {
            Self::Reference(reference) | Self::LargeRange { reference, .. } => reference.is_null(),
        }
    }
}

/// Collector queue for local major mark work.
type LocalTraceQueue = TraceQueue<LocalTraceWork>;

/// The first non-null heap large-allocation id.
const FIRST_ALLOCATED_LARGE_ALLOCATION_ID: u64 = 1;

/// The current local major collection phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalGcPhase {
    /// No major collection is active.
    Idle,
    /// The major collector is marking reachable allocations.
    Mark,
    /// The major collector is reclaiming unreachable allocations.
    Sweep,
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
    /// The configured page width for allocations in large space.
    pub(crate) page_bytes: usize,
    /// The live heap allocations.
    pub(crate) allocations: CowTable<LargeAllocation>,
    /// The free heap allocation ids available for reuse.
    pub(crate) free_large_allocation_ids: Vec<u64>,
    /// The next heap allocation id to allocate.
    pub(crate) next_unused_large_allocation_id: u64,
}

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

    /// The maximum payload size routed to young space.
    pub(crate) max_young_allocation_bytes: usize,
    /// The live GC state.
    pub(crate) gc: GcState,
    /// The reusable collector trace queue.
    pub(crate) trace_queue: HeapTraceQueue,
    /// The current local major collection phase.
    pub(crate) major_phase: LocalGcPhase,
    /// The persistent trace queue for an active local major cycle.
    pub(crate) major_trace_queue: LocalTraceQueue,
    /// The stable sweep reference snapshot for an active local major cycle.
    pub(crate) major_sweep_references: Vec<HeapReference>,
    /// The next sweep snapshot index to visit.
    pub(crate) major_sweep_cursor: usize,
    /// The number of allocations freed by the active local major cycle.
    pub(crate) major_freed_allocations: usize,
    /// The number of bytes freed by the active local major cycle.
    pub(crate) major_freed_bytes: u64,
    /// Whether a heap collection is currently running.
    pub(crate) is_collecting: bool,
    /// The scoped heap pins that keep stable addresses and block branch boundaries.
    pub(crate) pins: PinSet,
    /// Mature spans queued for dirty-card scanning.
    pub(crate) dirty_spans: Vec<usize>,
    /// Mature large allocations queued for dirty-card scanning.
    pub(crate) dirty_large_allocations: Vec<LargeAllocationId>,
    /// Live local references whose layouts may contain shared heap references.
    pub(crate) shared_edge_roots: Vec<HeapReference>,
    /// Reverse index into tracked shared-edge roots.
    pub(crate) shared_edge_index: BTreeMap<HeapReference, usize>,
    /// Whether one local-to-shared edge scan is currently active.
    pub(crate) is_scanning_shared_edges: bool,
    /// The next dense reference slot to scan for shared edges.
    pub(crate) shared_edge_cursor: usize,
    /// The pending local references whose shared edges need rescanning.
    pub(crate) shared_edge_queue: HeapTraceQueue,
    /// Queue membership for pending shared-edge rescans.
    pub(crate) shared_edge_pending: BTreeSet<HeapReference>,
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
                page_bytes: options.page_bytes,
                allocations: CowTable::new(),
                free_large_allocation_ids: Vec::new(),
                next_unused_large_allocation_id: FIRST_ALLOCATED_LARGE_ALLOCATION_ID,
            },
            page_map: Vec::new(),
            next_offset: next_heap_offset,
            mapping,
            gc: GcState::default(),
            trace_queue: TraceQueue::default(),
            major_phase: LocalGcPhase::Idle,
            major_trace_queue: TraceQueue::default(),
            major_sweep_references: Vec::new(),
            major_sweep_cursor: 0,
            major_freed_allocations: 0,
            major_freed_bytes: 0,
            is_collecting: false,
            pins: PinSet::default(),
            dirty_spans: Vec::new(),
            dirty_large_allocations: Vec::new(),
            shared_edge_roots: Vec::new(),
            shared_edge_index: BTreeMap::new(),
            is_scanning_shared_edges: false,
            shared_edge_cursor: 0,
            shared_edge_queue: TraceQueue::default(),
            shared_edge_pending: BTreeSet::new(),
        };

        space.map_page_run(0, &young_pages, |logical_page_index| {
            HeapPageMapEntry::Young { logical_page_index }
        });

        // materialize the nursery before object allocation starts
        if options.heap_young_bytes > 0 {
            space.mapping.materialize(0, options.heap_young_bytes)?;
            space.young.mapped_until = options.heap_young_bytes;
        }

        Ok(space)
    }

    /// Return the shared page allocator.
    pub fn allocator(&self) -> &Arc<Allocator> {
        &self.allocator
    }

    /// Return the exact retained heap allocator-page bytes.
    pub fn retained_bytes(&self) -> u64 {
        self.allocator.retained_bytes_for_page_runs(
            std::iter::once(&self.young.pages)
                .chain(self.small.spans.iter().map(|span| &span.pages))
                .chain(
                    self.large
                        .allocations
                        .iter()
                        .map(|allocation| &allocation.pages),
                ),
        ) + self
            .page_run_cache
            .cached_bytes(self.allocator.page_bytes())
    }

    /// Return the current GC state.
    pub fn gc_state(&self) -> &GcState {
        &self.gc
    }

    /// Stabilize one heap reference in mature place.
    pub fn stabilize(&mut self, reference: HeapReference) -> HeapResult<HeapReference> {
        self.promote_reference(reference)
    }

    /// Pin one heap reference against movement.
    pub fn pin(&mut self, reference: HeapReference) -> HeapResult<HeapReference> {
        // first ensure the reference already points at stable mature place
        let reference = self.stabilize(reference)?;

        // then record the active pin count
        self.pins.pin(reference)?;

        Ok(reference)
    }

    /// Release one heap pin.
    pub fn unpin(&mut self, reference: HeapReference) -> HeapResult<()> {
        self.pins.unpin(reference)
    }

    /// Return the number of live heap allocations.
    pub fn allocation_count(&self) -> usize {
        let (allocation_count, _) = self.live_allocated_usage();

        allocation_count
    }

    /// Return the number of live heap bytes.
    pub fn allocated_bytes(&self) -> u64 {
        let (_, allocated_bytes) = self.live_allocated_usage();

        allocated_bytes
    }

    /// Return the exact live usage for this heap space.
    pub fn usage(&self) -> HeapSpaceUsage {
        let (allocation_count, allocated_bytes) = self.live_allocated_usage();

        HeapSpaceUsage {
            allocation_count,
            allocated_bytes,
            retained_bytes: self.retained_bytes(),
        }
    }

    /// Return exact live allocation usage from heap metadata.
    pub(crate) fn live_allocated_usage(&self) -> (usize, u64) {
        let mut allocation_count = 0usize;
        let mut allocated_bytes = 0u64;

        // young ranges charge their exact logical payload width
        let mut start = 0;
        while let Some(start_index) = self.young.live.first_set_from(start) {
            allocation_count += 1;
            allocated_bytes += self.young.byte_lens[start_index] as u64;
            start = start_index + 1;
        }

        // young runs charge one size-class slot per live slot
        for (run_index, run) in self.young.runs.iter().enumerate() {
            let reserved_offset = if self.young.run_cursor.is_active()
                && self.young.run_cursor.run_index == run_index
            {
                self.young.run_cursor.next_offset
            } else {
                run.next_offset
            };
            let reserved_count = run.reserved_slot_count_with(reserved_offset);
            let freed_count = self.young.run_bits[run_index].freed.count_ones();
            let occupied_count = reserved_count - freed_count;

            allocation_count += occupied_count;
            allocated_bytes += occupied_count as u64 * run.size_class as u64;
        }

        // small spans charge one size-class slot per occupied slot
        for span in self.small.spans.iter() {
            allocation_count += span.occupied_count;
            allocated_bytes += span.occupied_count as u64 * span.class.size_class as u64;
        }

        // large allocations charge their logical allocation length
        for allocation in self.large.allocations.iter() {
            if !allocation.is_live {
                continue;
            }

            allocation_count += 1;
            allocated_bytes += allocation.len as u64;
        }

        (allocation_count, allocated_bytes)
    }

    /// Flush transient cache state before one exact branch boundary.
    pub(crate) fn flush_branch_boundary(&mut self) -> HeapResult<()> {
        self.page_run_cache.flush(&self.allocator)
    }

    /// Allocate one zeroed page run through the local page-run cache.
    pub(crate) fn allocate_page_run_zeroed(&mut self, byte_len: usize) -> HeapResult<PageRun> {
        self.page_run_cache
            .allocate_pages(&self.allocator, byte_len)
    }

    /// Release one page run through the local page-run cache.
    pub(crate) fn release_page_run(&mut self, page_run: PageRun) -> HeapResult<()> {
        self.page_run_cache
            .release_page_run(&self.allocator, page_run)
    }

    /// Rebuild the tracked local references that may contain shared edges.
    pub(crate) fn rebuild_shared_edge_roots(&mut self) -> HeapResult<()> {
        self.shared_edge_roots.clear();
        self.shared_edge_index.clear();

        for reference in self.live_references()? {
            if !self.reference_has_shared_roots(reference)? {
                continue;
            }

            self.track_shared_edge_root(reference)?;
        }

        Ok(())
    }

    /// Record one live reference whose layout may contain shared edges.
    pub(crate) fn track_shared_edge_root(&mut self, reference: HeapReference) -> HeapResult<()> {
        if self.shared_edge_index.contains_key(&reference) {
            return Ok(());
        }

        let tracked_index = self.shared_edge_roots.len();
        self.shared_edge_roots.push(reference);
        self.shared_edge_index.insert(reference, tracked_index);

        Ok(())
    }

    /// Remove one live reference from the tracked shared-edge set.
    pub(crate) fn remove_shared_edge_root(&mut self, reference: HeapReference) -> HeapResult<()> {
        let Some(tracked_index) = self.shared_edge_index.remove(&reference) else {
            return Ok(());
        };

        self.shared_edge_pending.remove(&reference);

        // active scans need stable cursor ordering
        if self.is_scanning_shared_edges {
            self.shared_edge_roots[tracked_index] = HeapReference::NULL;

            return Ok(());
        }

        let Some(moved_reference) = self.shared_edge_roots.pop() else {
            return Ok(());
        };

        if tracked_index == self.shared_edge_roots.len() {
            return Ok(());
        }

        self.shared_edge_roots[tracked_index] = moved_reference;
        if !moved_reference.is_null() {
            self.shared_edge_index
                .insert(moved_reference, tracked_index);
        }

        Ok(())
    }

    /// Compact removed roots after one active shared-edge scan.
    pub(crate) fn compact_shared_edge_roots(&mut self) {
        self.shared_edge_roots
            .retain(|reference| !reference.is_null());
        self.shared_edge_index.clear();

        for (index, reference) in self.shared_edge_roots.iter().copied().enumerate() {
            self.shared_edge_index.insert(reference, index);
        }
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

    /// Return the byte offset for one young range.
    pub(crate) fn young_range_offset(&self, range: YoungRange) -> usize {
        range.first_offset
    }

    /// Return the reference map for one heap place.
    pub(crate) fn reference_map_for_place(&self, place: HeapPlace) -> HeapResult<ReferenceMap> {
        match place {
            HeapPlace::Young(YoungPlace::Range { first_offset }) => {
                self.young_range_reference_map(first_offset)
            }
            HeapPlace::Young(YoungPlace::Slot(_)) => Ok(ReferenceMap::None),
            HeapPlace::Small(slot) => {
                self.small_slot_reference_map(slot.span_index(), slot.slot_index())
            }
            HeapPlace::Large(allocation_id) => {
                let reference_map = self
                    .large_allocation(allocation_id)
                    .ok_or(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    })?
                    .reference_map
                    .clone();

                Ok(reference_map)
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
                .len),
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
                self.young_range_offset(allocation)
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

    /// Return the exact reference map stored for one small slot.
    pub(crate) fn small_slot_reference_map(
        &self,
        span_index: usize,
        slot_index: usize,
    ) -> HeapResult<ReferenceMap> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::MissingSpan { span_index });
        };
        if !span.occupied.contains(slot_index) {
            return Err(HeapError::MissingSmallSlot {
                span_index,
                slot_index,
            });
        }

        Ok(slot_reference_map(
            &span.local_reference_bits,
            &span.shared_reference_bits,
            slot_index,
            span.class.size_class,
            span.class.size_class,
        ))
    }

    /// Return the exact reference map stored for one young range.
    pub(crate) fn young_range_reference_map(
        &self,
        first_offset: usize,
    ) -> HeapResult<ReferenceMap> {
        let Some((_range_index, range)) = self.young_range_by_offset(first_offset) else {
            return Err(HeapError::MissingYoungRange { first_offset });
        };
        let range_offset = self.young_range_offset(range);

        Ok(allocation_reference_map(
            &self.young.local_reference_bits,
            &self.young.shared_reference_bits,
            range_offset,
            range.byte_len,
        ))
    }

    /// Return the reusable-span bucket index for one small-span class.
    pub(crate) fn small_span_bucket(&self, class: &SmallSpanClass) -> HeapResult<usize> {
        class.bucket_index(&self.small.size_classes)
    }

    /// Resolve one allocation plan against this heap space.
    #[inline(always)]
    pub(crate) fn allocation_layout<'a>(&self, plan: AllocationPlan<'a>) -> AllocationLayout<'a> {
        allocation_layout(
            plan,
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

/// Return the offset rounded up to one allocation boundary.
fn align_up(byte_len: usize, alignment_bytes: usize) -> usize {
    let alignment_bytes = alignment_bytes.max(1);

    byte_len.div_ceil(alignment_bytes) * alignment_bytes
}
