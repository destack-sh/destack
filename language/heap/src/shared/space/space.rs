use std::sync::Arc;

use destack_memory::AddressSpace;
use destack_mir::ReferenceMap;
use parking_lot::RwLock;

use super::{
    SharedAllocator, SharedHeapPageMapEntry, SharedHeapPlace, SharedLargeAllocation,
    SharedSmallSpan,
};
use crate::allocator::{Allocator, PageRun, PageRunCache, SizeClassTable};
use crate::shared::gc::{SharedGcPhase, SharedGcState};
use crate::{
    AllocationLayout, AllocationShape, GcState, HeapError, HeapOptions, HeapResult,
    SharedHeapReference, SharedHeapSpaceUsage, SmallSpanClass, allocation_layout,
};

/// The first non-null shared heap large-allocation id.
const FIRST_LARGE_ALLOCATION_ID: u64 = 1;

/// One shared heap small space.
#[derive(Debug)]
pub(crate) struct SharedSmallSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span width.
    pub(crate) span_bytes: usize,
    /// The live shared heap spans.
    pub(crate) spans: Vec<Arc<SharedSmallSpan>>,
    /// The reusable non-full spans per size and scan class.
    pub(crate) partial_spans: Vec<Vec<usize>>,
}

/// One shared heap large space.
#[derive(Debug)]
pub(crate) struct SharedLargeSpace {
    /// The configured page width for allocations in large space.
    pub(crate) page_bytes: usize,
    /// The live shared heap allocations.
    pub(crate) allocations: Vec<Arc<RwLock<SharedLargeAllocation>>>,
    /// The free shared heap allocation ids available for reuse.
    pub(crate) free_large_allocation_ids: Vec<u64>,
    /// The next shared heap allocation id to allocate.
    pub(crate) next_unused_large_allocation_id: u64,
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

/// One shared heap space over a shared allocator.
#[derive(Debug)]
pub struct SharedHeapSpace {
    /// The shared heap-space allocator for every allocation.
    pub(crate) allocator: Arc<Allocator>,
    /// The fixed live byte mapping for shared heap space.
    pub(crate) mapping: AddressSpace,
    /// The shared heap allocator state.
    pub(crate) state: RwLock<SharedHeapState>,
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
        let options = HeapOptions {
            page_bytes: allocator.page_bytes(),
            allocator_chunk_bytes: allocator.chunk_bytes(),
            ..HeapOptions::shared()
        };

        Self::with_options(allocator, &options)
    }

    /// Create a new empty shared heap space over one shared allocator and options.
    pub fn with_options(allocator: Arc<Allocator>, options: &HeapOptions) -> HeapResult<Self> {
        // validate the heap and allocator contract
        options.validate_shared()?;
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
                partial_spans: vec![
                    Vec::new();
                    SmallSpanClass::bucket_count(&options.size_classes)
                ],
            },
            large: SharedLargeSpace {
                page_bytes: allocator.page_bytes(),
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
            gc: SharedGcState::default(),
        })
    }

    /// Return the configured shared page size.
    pub fn page_bytes(&self) -> usize {
        self.allocator.page_bytes()
    }

    /// Resolve one allocation shape against this shared heap space.
    #[inline(always)]
    pub(crate) fn allocation_layout<'a>(&self, shape: AllocationShape<'a>) -> AllocationLayout<'a> {
        let store = self.state.read();

        allocation_layout(
            shape,
            &store.small.size_classes,
            self.allocator.page_bytes(),
            store.small.span_bytes,
        )
    }

    /// Return the exact retained shared heap allocator-page bytes.
    pub fn retained_bytes(&self) -> u64 {
        let store = self.state.read();

        self.live_retained_bytes(&store)
    }

    /// Return the exact usage for this live shared heap space.
    pub fn usage(&self) -> SharedHeapSpaceUsage {
        let store = self.state.read();
        let retained_bytes = self.live_retained_bytes(&store);
        let (allocation_count, allocated_bytes) = self.live_allocated_usage(&store);

        SharedHeapSpaceUsage {
            allocation_count,
            allocated_bytes,
            retained_bytes,
        }
    }

    /// Return the number of live shared heap allocations.
    pub fn allocation_count(&self) -> usize {
        let store = self.state.read();
        let (allocation_count, _) = self.live_allocated_usage(&store);

        allocation_count
    }

    /// Return the number of live shared heap bytes.
    pub fn allocated_bytes(&self) -> u64 {
        let store = self.state.read();
        let (_, allocated_bytes) = self.live_allocated_usage(&store);

        allocated_bytes
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
            }
        }

        Ok(released_bytes)
    }

    /// Create one worker-local shared heap allocator.
    pub fn allocator(&self) -> SharedAllocator {
        // allocator shape follows shared heap options
        let store = self.state.read();

        SharedAllocator::new(
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

    /// Return the retained live bytes for the current shared heap state.
    pub(crate) fn live_retained_bytes(&self, store: &SharedHeapState) -> u64 {
        // retained pages include live page runs and cached reusable page runs
        let pages = self.live_page_runs(store);
        let cached_bytes = store
            .page_run_cache
            .cached_bytes(self.allocator.page_bytes());

        self.allocator.retained_bytes_for_page_runs(pages.iter()) + cached_bytes
    }

    /// Return exact live allocation usage from shared heap metadata.
    pub(crate) fn live_allocated_usage(&self, store: &SharedHeapState) -> (usize, u64) {
        let mut allocation_count = 0usize;
        let mut allocated_bytes = 0u64;

        // small spans charge one size-class slot per occupied slot
        for span in &store.small.spans {
            let occupied_count = span.occupied_count();

            allocation_count += occupied_count;
            allocated_bytes += occupied_count as u64 * span.class.size_class as u64;
        }

        // large allocations charge their logical allocation length
        for allocation in &store.large.allocations {
            let allocation = allocation.read();
            if !allocation.is_live {
                continue;
            }

            allocation_count += 1;
            allocated_bytes += allocation.len as u64;
        }

        (allocation_count, allocated_bytes)
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

    /// Return the exact reference map stored for one shared small slot.
    pub(crate) fn small_slot_reference_map(
        &self,
        span_index: usize,
        slot_index: usize,
    ) -> HeapResult<ReferenceMap> {
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

        Ok(span.reference_map(slot_index))
    }

    /// Return every live shared heap reference.
    pub(crate) fn live_references(&self) -> HeapResult<Vec<SharedHeapReference>> {
        let store = self.state.read();
        let mut references = Vec::new();

        // small spans
        for span in &store.small.spans {
            if span.occupied_count() == 0 || span.pages_empty() {
                continue;
            }

            for slot_index in 0..span.slot_count {
                if !span.contains_slot(slot_index) {
                    continue;
                }

                let slot_offset = span.class.size_class * slot_index;
                let base_offset = span.first_offset + slot_offset;

                references.push(SharedHeapReference::new(base_offset));
            }
        }

        // large allocations
        for allocation in &store.large.allocations {
            let allocation = allocation.read();
            if !allocation.is_live {
                continue;
            }

            references.push(SharedHeapReference::new(allocation.first_offset));
        }

        Ok(references)
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
