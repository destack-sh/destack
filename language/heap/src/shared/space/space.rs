use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use destack_memory::AddressSpace;
use destack_mir::{TraceMap, TraceTable};
use parking_lot::RwLock;

use super::{
    SharedAllocationCache, SharedHeapPageMapEntry, SharedHeapPlace, SharedLargeAllocation,
    SharedSmallSpan,
};
use crate::allocator::{Allocator, PageRun, PageRunCache, SizeClassTable};
use crate::shared::gc::{SharedGcPhase, SharedGcState};
use crate::{
    AllocationPlan, AllocationShape, GcState, HeapError, HeapResult, SharedHeapOptions,
    SharedHeapReference, SharedHeapSpaceUsage, SmallSpanClass, allocation_plan,
};

/// The first non-null shared heap large-allocation id.
const FIRST_LARGE_ALLOCATION_ID: u64 = 1;

/// One shared heap space over a shared allocator.
#[derive(Debug)]
pub struct SharedHeapSpace {
    /// The shared heap-space allocator for every allocation.
    pub(crate) allocator: Arc<Allocator>,
    /// The fixed live byte mapping for shared heap space.
    pub(crate) mapping: AddressSpace,
    /// The shared heap allocation state.
    pub(crate) state: RwLock<SharedHeapState>,
    /// The exact shared heap accounting state.
    pub(crate) accounting: SharedHeapAccounting,
    /// The active shared collection state.
    pub(crate) gc: SharedGcState,
}

impl SharedHeapSpace {
    /// Return the base native address for direct shared heap access.
    #[inline(always)]
    pub fn base_address(&self) -> usize {
        self.mapping.base_address()
    }

    /// Create a new empty shared heap space over one shared allocator.
    pub fn with_allocator(allocator: Arc<Allocator>) -> HeapResult<Self> {
        let options = SharedHeapOptions {
            page_bytes: allocator.page_bytes(),
            allocator_chunk_bytes: allocator.chunk_bytes(),
            ..SharedHeapOptions::default()
        };

        Self::with_options(allocator, &options)
    }

    /// Create a new empty shared heap space over one shared allocator and options.
    pub fn with_options(
        allocator: Arc<Allocator>,
        options: &SharedHeapOptions,
    ) -> HeapResult<Self> {
        // validate the heap and allocator contract
        options.validate()?;
        options.validate_allocator(&allocator)?;

        // reserve the shared address space
        let mapping = AddressSpace::reserve(options.heap_space_bytes, options.page_bytes)?;

        // initialize shared heap metadata
        let store = SharedHeapState {
            page_run_cache: PageRunCache::new(allocator.pages_per_chunk()),
            small: SharedSmallSpace {
                size_classes: options.size_classes.clone(),
                span_bytes: options.heap_small_bytes,
                spans: Vec::new(),
                partial_spans: BTreeMap::new(),
            },
            large: SharedLargeSpace {
                allocations: Vec::new(),
                free_large_allocation_ids: Vec::new(),
                next_unused_large_allocation_id: FIRST_LARGE_ALLOCATION_ID,
            },
            page_map: Vec::new(),
            next_offset: allocator.page_bytes(),
            gc: GcState::default(),
        };

        Ok(Self {
            allocator,
            mapping,
            state: RwLock::new(store),
            accounting: SharedHeapAccounting::default(),
            gc: SharedGcState::default(),
        })
    }

    /// Return the configured shared page size.
    pub fn page_bytes(&self) -> usize {
        self.allocator.page_bytes()
    }

    /// Resolve one allocation shape against this shared heap space.
    #[inline(always)]
    pub(crate) fn allocation_plan<'a>(&self, shape: AllocationShape<'a>) -> AllocationPlan<'a> {
        let store = self.state.read();

        allocation_plan(
            shape,
            &store.small.size_classes,
            self.allocator.page_bytes(),
            store.small.span_bytes,
        )
    }

    /// Return the exact retained shared heap allocator-page bytes.
    pub fn retained_bytes(&self) -> u64 {
        let store = self.state.read();

        self.accounting
            .retained_bytes(&store, self.allocator.page_bytes())
    }

    /// Return the exact usage for this live shared heap space.
    pub fn usage(&self) -> SharedHeapSpaceUsage {
        let store = self.state.read();

        self.accounting.usage(&store, self.allocator.page_bytes())
    }

    /// Return the number of live shared heap allocations.
    pub fn allocation_count(&self) -> usize {
        self.accounting.allocation_count()
    }

    /// Return the number of live shared heap bytes.
    pub fn allocated_bytes(&self) -> u64 {
        self.accounting.allocated_bytes()
    }

    /// Return the current shared heap collector state.
    pub fn gc_state(&self) -> GcState {
        let store = self.state.read();

        store.gc.clone()
    }

    /// Return the current shared heap collector phase.
    #[inline(always)]
    pub fn gc_phase(&self) -> SharedGcPhase {
        self.gc.phase()
    }

    /// Return whether one shared heap reference currently refers to one live allocation.
    pub fn is_live(&self, reference: SharedHeapReference) -> bool {
        self.resolve_location(reference).is_some()
    }

    /// Free one shared heap allocation immediately.
    pub(crate) fn free(&self, reference: SharedHeapReference) -> HeapResult<u64> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        };
        let released_bytes = location.byte_len as u64;
        let mut store = self.state.write();

        // release the live place
        match location.place {
            SharedHeapPlace::Small(slot) => {
                self.release_small_slot(&mut store, slot)?;
            }
            SharedHeapPlace::Large(allocation_id) => {
                let Some(allocation) = store.large.allocations.get(allocation_id.index()?).cloned()
                else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };
                let (first_offset, pages) = {
                    let mut allocation = allocation.write();
                    if !allocation.is_live {
                        return Err(HeapError::MissingLargeAllocation {
                            allocation_id: allocation_id.id(),
                        });
                    }

                    let first_offset = allocation.first_offset;
                    let pages = allocation.pages;
                    allocation.retire();

                    (first_offset, pages)
                };

                store
                    .large
                    .free_large_allocation_ids
                    .push(allocation_id.id());
                self.unmap_page_run(&mut store, first_offset, &pages);
                store
                    .page_run_cache
                    .release_page_run(&self.allocator, pages)?;
                self.accounting.release_pages(pages, self.page_bytes());
            }
        }
        self.accounting.free(released_bytes as usize);

        Ok(released_bytes)
    }

    /// Create one mutator-local shared allocation cache.
    pub fn allocation_cache(&self) -> SharedAllocationCache {
        // cache shape follows shared heap options
        let store = self.state.read();

        SharedAllocationCache::new(
            store.small.size_classes.clone(),
            store.small.span_bytes,
            self.allocator.page_bytes(),
        )
    }

    /// Return the current live heap page runs.
    fn live_page_runs(&self, store: &SharedHeapState) -> Vec<PageRun> {
        let mut pages = Vec::with_capacity(store.small.spans.len() + store.large.allocations.len());

        // live small spans
        for span in &store.small.spans {
            if span.occupied_count() == 0 || span.pages_empty() {
                continue;
            }

            pages.push(span.pages());
        }

        // live large allocations
        for allocation in &store.large.allocations {
            let allocation = allocation.read();

            if !allocation.is_live {
                continue;
            }

            pages.push(allocation.pages);
        }

        pages
    }

    /// Release allocator page runs owned by this shared heap space.
    fn close(&mut self) -> HeapResult<()> {
        // release live page runs through the page-run cache
        let mut store = self.state.write();
        let page_runs = self.live_page_runs(&store);

        for page_run in page_runs {
            store
                .page_run_cache
                .release_page_run(&self.allocator, page_run)?;
        }

        // release cached page runs
        store.page_run_cache.flush(&self.allocator)
    }

    /// Return the exact trace map stored for one shared small slot.
    pub(crate) fn small_slot_trace_map(
        &self,
        span_index: usize,
        slot_index: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<TraceMap> {
        // resolve the small span
        let store = self.state.read();
        let Some(span) = store.small.spans.get(span_index).cloned() else {
            return Err(HeapError::MissingSpan { span_index });
        };

        // validate the slot belongs to the span
        if !span.contains_slot(slot_index) {
            return Err(HeapError::MissingSmallSlot {
                span_index,
                slot_index,
            });
        }

        span.trace_map(slot_index, trace_table)
    }

    /// Return the base reference for one shared heap place.
    pub(super) fn base_reference_for_place(
        &self,
        place: SharedHeapPlace,
    ) -> HeapResult<SharedHeapReference> {
        let store = self.state.read();

        self.base_reference(&store, place)
    }

    /// Return the base reference for one shared heap place in the current state.
    fn base_reference(
        &self,
        store: &SharedHeapState,
        place: SharedHeapPlace,
    ) -> HeapResult<SharedHeapReference> {
        // dispatch by physical shared heap place
        let base_offset = match place {
            SharedHeapPlace::Small(slot) => {
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let slot_offset = span.class.size_class * slot.slot_index();

                span.first_offset + slot_offset
            }
            SharedHeapPlace::Large(allocation_id) => {
                let Some(allocation) = store.large.allocations.get(allocation_id.index()?).cloned()
                else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };
                let allocation = allocation.read();
                if !allocation.is_live {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                }

                allocation.first_offset
            }
        };

        Ok(SharedHeapReference::new(base_offset))
    }
}

/// Shared heap allocator metadata.
#[derive(Debug)]
pub(crate) struct SharedHeapState {
    /// The shared cache of reusable page runs.
    pub(crate) page_run_cache: PageRunCache,
    /// The shared heap small space.
    pub(crate) small: SharedSmallSpace,
    /// The shared heap large space.
    pub(crate) large: SharedLargeSpace,
    /// The owning shared heap metadata for each visible allocator page.
    pub(crate) page_map: Vec<Option<SharedHeapPageMapEntry>>,
    /// The next unused byte offset in shared heap space.
    pub(crate) next_offset: usize,
    /// The live shared heap collector state.
    pub(crate) gc: GcState,
}

/// Exact shared heap accounting.
#[derive(Debug, Default)]
pub(crate) struct SharedHeapAccounting {
    /// The number of live shared heap allocations.
    allocation_count: AtomicUsize,
    /// The number of live shared heap payload bytes.
    allocated_bytes: AtomicU64,
    /// The retained bytes for live shared heap page runs.
    live_retained_bytes: AtomicU64,
}

impl SharedHeapAccounting {
    /// Rebuild exact accounting from shared heap metadata.
    pub(crate) fn from_state(store: &SharedHeapState, page_bytes: usize) -> Self {
        let accounting = Self::default();

        // small spans
        for span in &store.small.spans {
            let occupied_count = span.occupied_count();
            accounting.allocate_many(
                occupied_count,
                occupied_count as u64 * span.class.size_class as u64,
            );
            if occupied_count > 0 && !span.pages_empty() {
                accounting.retain_pages(span.pages(), page_bytes);
            }
        }

        // large allocations
        for allocation in &store.large.allocations {
            let allocation = allocation.read();
            if !allocation.is_live {
                continue;
            }

            accounting.allocate(allocation.byte_len);
            accounting.retain_pages(allocation.pages, page_bytes);
        }

        accounting
    }

    /// Return the number of live shared heap allocations.
    pub(crate) fn allocation_count(&self) -> usize {
        self.allocation_count.load(Ordering::Acquire)
    }

    /// Return the number of live shared heap payload bytes.
    pub(crate) fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes.load(Ordering::Acquire)
    }

    /// Return exact retained shared heap bytes.
    pub(crate) fn retained_bytes(&self, store: &SharedHeapState, page_bytes: usize) -> u64 {
        let live_bytes = self.live_retained_bytes.load(Ordering::Acquire);
        let cached_bytes = store.page_run_cache.cached_bytes(page_bytes);

        live_bytes + cached_bytes
    }

    /// Return exact shared heap usage.
    pub(crate) fn usage(&self, store: &SharedHeapState, page_bytes: usize) -> SharedHeapSpaceUsage {
        SharedHeapSpaceUsage {
            allocation_count: self.allocation_count(),
            allocated_bytes: self.allocated_bytes(),
            retained_bytes: self.retained_bytes(store, page_bytes),
        }
    }

    /// Record one live allocation.
    pub(crate) fn allocate(&self, byte_len: usize) {
        self.allocation_count.fetch_add(1, Ordering::AcqRel);
        self.allocated_bytes
            .fetch_add(byte_len as u64, Ordering::AcqRel);
    }

    /// Record many live allocations.
    fn allocate_many(&self, allocation_count: usize, allocated_bytes: u64) {
        self.allocation_count
            .fetch_add(allocation_count, Ordering::AcqRel);
        self.allocated_bytes
            .fetch_add(allocated_bytes, Ordering::AcqRel);
    }

    /// Record one freed allocation.
    pub(crate) fn free(&self, byte_len: usize) {
        self.allocation_count.fetch_sub(1, Ordering::AcqRel);
        self.allocated_bytes
            .fetch_sub(byte_len as u64, Ordering::AcqRel);
    }

    /// Record live retained pages.
    pub(crate) fn retain_pages(&self, pages: PageRun, page_bytes: usize) {
        let retained_bytes = pages.len() as u64 * page_bytes as u64;
        self.live_retained_bytes
            .fetch_add(retained_bytes, Ordering::AcqRel);
    }

    /// Record released live pages.
    pub(crate) fn release_pages(&self, pages: PageRun, page_bytes: usize) {
        let retained_bytes = pages.len() as u64 * page_bytes as u64;
        self.live_retained_bytes
            .fetch_sub(retained_bytes, Ordering::AcqRel);
    }
}

/// One shared heap small space.
#[derive(Debug)]
pub(crate) struct SharedSmallSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span width.
    pub(crate) span_bytes: usize,
    /// The live shared heap spans.
    pub(crate) spans: Vec<Arc<SharedSmallSpan>>,
    /// The reusable non-full spans per exact small-span class.
    pub(crate) partial_spans: BTreeMap<SmallSpanClass, Vec<usize>>,
}

/// One shared heap large space.
#[derive(Debug)]
pub(crate) struct SharedLargeSpace {
    /// The live shared heap allocations.
    pub(crate) allocations: Vec<Arc<RwLock<SharedLargeAllocation>>>,
    /// The free shared heap allocation ids available for reuse.
    pub(crate) free_large_allocation_ids: Vec<u64>,
    /// The next shared heap allocation id to allocate.
    pub(crate) next_unused_large_allocation_id: u64,
}

/// Return one allocation-local byte offset for one visible range.
pub(super) fn allocation_byte_offset(
    base_offset: usize,
    start: usize,
    len: usize,
    capacity: usize,
) -> HeapResult<usize> {
    debug_assert!(base_offset <= capacity);

    // validate the caller start relative to the visible payload
    let remaining = capacity - base_offset;
    if start > remaining {
        return Err(HeapError::InvalidByteRange {
            start,
            len,
            capacity,
        });
    }

    // validate the caller length after projecting the start
    let byte_offset = base_offset + start;
    let remaining = capacity - byte_offset;
    if len > remaining {
        return Err(HeapError::InvalidByteRange {
            start: byte_offset,
            len,
            capacity,
        });
    }

    Ok(byte_offset)
}

/// Return the byte offset for one slot payload inside one span.
pub(crate) fn small_slot_offset(size_class: usize, slot_index: usize) -> usize {
    size_class * slot_index
}

impl Drop for SharedHeapSpace {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
