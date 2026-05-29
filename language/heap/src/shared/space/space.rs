use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use destack_memory::AddressSpace;
use destack_mir::{TraceMap, TraceTable};
use parking_lot::RwLock;

use super::{
    SharedAllocationCache, SharedHeapPageMapEntry, SharedHeapStorage, SharedLargeBlock,
    SharedSmallSpan,
};
use crate::allocator::{Allocator, PageSpan, PageSpanCache, SizeClassTable};
use crate::shared::gc::{SharedGcPhase, SharedGcState};
use crate::{
    AllocationPlan, AllocationShape, GcState, HeapError, HeapResult, SharedHeapOptions,
    SharedHeapReference, SharedHeapSpaceUsage, SmallSpanClass, allocation_plan,
};

/// The first non-null shared heap large-block id.
const FIRST_LARGE_ALLOCATION_ID: u64 = 1;

/// One shared heap space over a shared allocator.
#[derive(Debug)]
pub struct SharedHeapSpace {
    /// The shared heap-space allocator for every block.
    pub(crate) allocator: Arc<Allocator>,
    /// The fixed live byte mapping for shared heap space.
    pub(crate) mapping: AddressSpace,
    /// The shared heap block state.
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
            page_size_bytes: allocator.page_size_bytes(),
            allocator_chunk_size_bytes: allocator.chunk_size_bytes(),
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
        let mapping =
            AddressSpace::reserve(options.heap_space_size_bytes, options.page_size_bytes)?;

        // initialize shared heap metadata
        let store = SharedHeapState {
            page_span_cache: PageSpanCache::new(allocator.pages_per_chunk()),
            small: SharedSmallSpace {
                size_classes: options.size_classes.clone(),
                span_size_bytes: options.heap_small_size_bytes,
                spans: Vec::new(),
                partial_spans: BTreeMap::new(),
            },
            large: SharedLargeSpace {
                blocks: Vec::new(),
                free_large_block_ids: Vec::new(),
                next_unused_large_block_id: FIRST_LARGE_ALLOCATION_ID,
            },
            page_map: Vec::new(),
            next_offset: allocator.page_size_bytes(),
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
    pub fn page_size_bytes(&self) -> usize {
        self.allocator.page_size_bytes()
    }

    /// Resolve one allocation shape against this shared heap space.
    #[inline(always)]
    pub(crate) fn allocation_plan<'a>(&self, shape: AllocationShape<'a>) -> AllocationPlan<'a> {
        let store = self.state.read();

        allocation_plan(
            shape,
            &store.small.size_classes,
            self.allocator.page_size_bytes(),
            store.small.span_size_bytes,
        )
    }

    /// Return the exact retained shared heap allocator-page bytes.
    pub fn retained_bytes(&self) -> u64 {
        let store = self.state.read();

        self.accounting
            .retained_bytes(&store, self.allocator.page_size_bytes())
    }

    /// Return the exact usage for this live shared heap space.
    pub fn usage(&self) -> SharedHeapSpaceUsage {
        let store = self.state.read();

        self.accounting
            .usage(&store, self.allocator.page_size_bytes())
    }

    /// Return the number of live shared heap blocks.
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

    /// Return whether one shared heap reference currently refers to one live block.
    pub fn is_live(&self, reference: SharedHeapReference) -> bool {
        self.resolve_extent(reference).is_some()
    }

    /// Free one shared heap block immediately.
    pub(crate) fn free(&self, reference: SharedHeapReference) -> HeapResult<u64> {
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        };
        let released_bytes = extent.byte_len as u64;
        let mut store = self.state.write();

        // release the live storage
        match extent.storage {
            SharedHeapStorage::SmallSlot(slot) => {
                self.release_small_slot(&mut store, slot)?;
            }
            SharedHeapStorage::LargeBlock(block_id) => {
                let Some(block) = store.large.blocks.get(block_id.index()?).cloned() else {
                    return Err(HeapError::internal("missing large block"));
                };
                let (first_offset, pages) = {
                    let mut block = block.write();
                    if !block.is_live {
                        return Err(HeapError::internal("missing large block"));
                    }

                    let first_offset = block.first_offset;
                    let pages = block.pages;
                    block.retire();

                    (first_offset, pages)
                };

                store.large.free_large_block_ids.push(block_id.id());
                self.unmap_page_span(&mut store, first_offset, &pages);
                store
                    .page_span_cache
                    .release_page_span(&self.allocator, pages)?;
                self.accounting.release_pages(pages, self.page_size_bytes());
            }
        }
        self.accounting.free(released_bytes as usize);

        Ok(released_bytes)
    }

    /// Create one mutator-local shared allocation cache.
    pub fn allocation_cache(&self) -> SharedAllocationCache {
        SharedAllocationCache::new()
    }

    /// Return the current live heap page spans.
    fn live_page_spans(&self, store: &SharedHeapState) -> Vec<PageSpan> {
        let mut pages = Vec::with_capacity(store.small.spans.len() + store.large.blocks.len());

        // live small spans
        for span in &store.small.spans {
            if span.occupied_count() == 0 || span.pages_empty() {
                continue;
            }

            pages.push(span.pages());
        }

        // live large blocks
        for block in &store.large.blocks {
            let block = block.read();

            if !block.is_live {
                continue;
            }

            pages.push(block.pages);
        }

        pages
    }

    /// Release allocator page spans owned by this shared heap space.
    fn close(&mut self) -> HeapResult<()> {
        // release live page spans through the page-span cache
        let mut store = self.state.write();
        let page_spans = self.live_page_spans(&store);

        for page_span in page_spans {
            store
                .page_span_cache
                .release_page_span(&self.allocator, page_span)?;
        }

        // release cached page spans
        store.page_span_cache.flush(&self.allocator)
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
            return Err(HeapError::internal("missing span"));
        };

        // validate the slot belongs to the span
        if !span.contains_slot(slot_index) {
            return Err(HeapError::internal("missing small slot"));
        }

        span.trace_map(slot_index, trace_table)
    }

    /// Return the base reference for one shared heap storage.
    pub(super) fn base_reference_for_place(
        &self,
        storage: SharedHeapStorage,
    ) -> HeapResult<SharedHeapReference> {
        let store = self.state.read();

        self.base_reference(&store, storage)
    }

    /// Return the base reference for one shared heap storage in the current state.
    fn base_reference(
        &self,
        store: &SharedHeapState,
        storage: SharedHeapStorage,
    ) -> HeapResult<SharedHeapReference> {
        // dispatch by physical shared heap storage
        let base_offset = match storage {
            SharedHeapStorage::SmallSlot(slot) => {
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::internal("missing span"));
                };
                let slot_offset = span.class.size_class * slot.slot_index();

                span.first_offset + slot_offset
            }
            SharedHeapStorage::LargeBlock(block_id) => {
                let Some(block) = store.large.blocks.get(block_id.index()?).cloned() else {
                    return Err(HeapError::internal("missing large block"));
                };
                let block = block.read();
                if !block.is_live {
                    return Err(HeapError::internal("missing large block"));
                }

                block.first_offset
            }
        };

        Ok(SharedHeapReference::new(base_offset))
    }
}

/// Shared heap allocator metadata.
#[derive(Debug)]
pub(crate) struct SharedHeapState {
    /// The shared cache of reusable page spans.
    pub(crate) page_span_cache: PageSpanCache,
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
    /// The number of live shared heap blocks.
    allocation_count: AtomicUsize,
    /// The number of live shared heap payload bytes.
    allocated_bytes: AtomicU64,
    /// The retained bytes for live shared heap page spans.
    live_retained_bytes: AtomicU64,
}

impl SharedHeapAccounting {
    /// Rebuild exact accounting from shared heap metadata.
    pub(crate) fn from_state(store: &SharedHeapState, page_size_bytes: usize) -> Self {
        let accounting = Self::default();

        // small spans
        for span in &store.small.spans {
            let occupied_count = span.occupied_count();
            accounting.allocate_many(
                occupied_count,
                occupied_count as u64 * span.class.size_class as u64,
            );
            if occupied_count > 0 && !span.pages_empty() {
                accounting.retain_pages(span.pages(), page_size_bytes);
            }
        }

        // large blocks
        for block in &store.large.blocks {
            let block = block.read();
            if !block.is_live {
                continue;
            }

            accounting.allocate(block.byte_len);
            accounting.retain_pages(block.pages, page_size_bytes);
        }

        accounting
    }

    /// Return the number of live shared heap blocks.
    pub(crate) fn allocation_count(&self) -> usize {
        self.allocation_count.load(Ordering::Acquire)
    }

    /// Return the number of live shared heap payload bytes.
    pub(crate) fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes.load(Ordering::Acquire)
    }

    /// Return exact retained shared heap bytes.
    pub(crate) fn retained_bytes(&self, store: &SharedHeapState, page_size_bytes: usize) -> u64 {
        let live_bytes = self.live_retained_bytes.load(Ordering::Acquire);
        let cached_bytes = store.page_span_cache.cached_bytes(page_size_bytes);

        live_bytes + cached_bytes
    }

    /// Return exact shared heap usage.
    pub(crate) fn usage(
        &self,
        store: &SharedHeapState,
        page_size_bytes: usize,
    ) -> SharedHeapSpaceUsage {
        SharedHeapSpaceUsage {
            allocation_count: self.allocation_count(),
            allocated_bytes: self.allocated_bytes(),
            retained_bytes: self.retained_bytes(store, page_size_bytes),
        }
    }

    /// Record one live block.
    pub(crate) fn allocate(&self, byte_len: usize) {
        self.allocation_count.fetch_add(1, Ordering::AcqRel);
        self.allocated_bytes
            .fetch_add(byte_len as u64, Ordering::AcqRel);
    }

    /// Record many live blocks.
    pub(crate) fn allocate_many(&self, allocation_count: usize, allocated_bytes: u64) {
        self.allocation_count
            .fetch_add(allocation_count, Ordering::AcqRel);
        self.allocated_bytes
            .fetch_add(allocated_bytes, Ordering::AcqRel);
    }

    /// Record one freed block.
    pub(crate) fn free(&self, byte_len: usize) {
        self.allocation_count.fetch_sub(1, Ordering::AcqRel);
        self.allocated_bytes
            .fetch_sub(byte_len as u64, Ordering::AcqRel);
    }

    /// Record live retained pages.
    pub(crate) fn retain_pages(&self, pages: PageSpan, page_size_bytes: usize) {
        let retained_bytes = pages.len() as u64 * page_size_bytes as u64;
        self.live_retained_bytes
            .fetch_add(retained_bytes, Ordering::AcqRel);
    }

    /// Record released live pages.
    pub(crate) fn release_pages(&self, pages: PageSpan, page_size_bytes: usize) {
        let retained_bytes = pages.len() as u64 * page_size_bytes as u64;
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
    pub(crate) span_size_bytes: usize,
    /// The live shared heap spans.
    pub(crate) spans: Vec<Arc<SharedSmallSpan>>,
    /// The reusable non-full spans per exact small-span class.
    pub(crate) partial_spans: BTreeMap<SmallSpanClass, Vec<usize>>,
}

/// One shared heap large space.
#[derive(Debug)]
pub(crate) struct SharedLargeSpace {
    /// The live shared heap blocks.
    pub(crate) blocks: Vec<Arc<RwLock<SharedLargeBlock>>>,
    /// The free shared heap block ids available for reuse.
    pub(crate) free_large_block_ids: Vec<u64>,
    /// The next shared heap block id to allocate.
    pub(crate) next_unused_large_block_id: u64,
}

/// Return one block-local byte offset for one visible range.
pub(super) fn block_byte_offset(
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
