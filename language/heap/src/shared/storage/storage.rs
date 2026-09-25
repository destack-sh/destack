use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use parking_lot::RwLock;
use tspp_memory::{MemoryMap, MemoryRange};

use super::{AllocationCache, HeapPlace, LargeBlock, PageOwner, SmallSpan};
use crate::shared::gc::CollectorState;
use crate::shared::heap::SharedHeapUsage;
use crate::{
    GcPhase, GcState, HeapError, HeapResult, PageTable, SharedHeapOptions, SharedHeapReference,
    SizeClassTable, SmallSpanClass,
};

/// The first non-null shared heap large-block id.
const FIRST_LARGE_ALLOCATION_ID: u64 = 1;

/// One shared heap storage over a shared memory.
#[derive(Debug)]
pub(crate) struct HeapStorage {
    /// The shared heap storage memory for every block.
    pub(crate) memory: Arc<MemoryMap>,
    /// The logical shared heap page size.
    pub(crate) page_size_bytes: usize,
    /// The shared heap block state.
    pub(crate) state: RwLock<HeapState>,
    /// The constant object range inside world memory, which tracing skips.
    pub(crate) constant: MemoryRange,
    /// The exact shared heap accounting state.
    pub(crate) accounting: HeapAccounting,
    /// The active shared collection state.
    pub(crate) gc: CollectorState,
}

impl HeapStorage {
    /// Return the base native address for direct shared heap access.
    #[inline(always)]
    pub(crate) fn base_address(&self) -> usize {
        self.memory.base_address()
    }

    /// Create a new empty shared heap storage over one shared memory and options.
    pub(crate) fn new(memory: Arc<MemoryMap>, options: &SharedHeapOptions) -> HeapResult<Self> {
        // validate the heap configuration
        options.validate()?;

        // initialize shared heap metadata
        let store = HeapState {
            small: SmallStorage {
                size_classes: options.size_classes.clone(),
                span_size_bytes: options.heap_small_size_bytes,
                spans: Vec::new(),
                partial_spans: BTreeMap::new(),
            },
            large: LargeStorage {
                blocks: Vec::new(),
                free_large_block_ids: Vec::new(),
                next_unused_large_block_id: FIRST_LARGE_ALLOCATION_ID,
            },
            page_table: PageTable::new(),
            gc: GcState::default(),
        };

        Ok(Self {
            memory,
            page_size_bytes: options.page_size_bytes,
            state: RwLock::new(store),
            constant: MemoryRange::default(),
            accounting: HeapAccounting::default(),
            gc: CollectorState::default(),
        })
    }

    /// Return the configured shared page size.
    pub(crate) fn page_size_bytes(&self) -> usize {
        self.page_size_bytes
    }

    /// Return the exact retained shared heap bytes.
    pub(crate) fn retained_bytes(&self) -> u64 {
        self.accounting.retained_bytes()
    }

    /// Return the exact usage for this live shared heap storage.
    pub(crate) fn usage(&self) -> SharedHeapUsage {
        self.accounting.usage()
    }

    /// Return the number of live shared heap blocks.
    pub(crate) fn allocation_count(&self) -> usize {
        self.accounting.allocation_count()
    }

    /// Return the number of live shared heap bytes.
    pub(crate) fn allocated_bytes(&self) -> u64 {
        self.accounting.allocated_bytes()
    }

    /// Return the current shared heap collector state.
    pub(crate) fn gc_state(&self) -> GcState {
        let store = self.state.read();

        store.gc.clone()
    }

    /// Return the current shared heap collector phase.
    #[inline(always)]
    pub(crate) fn gc_phase(&self) -> GcPhase {
        self.gc.phase()
    }

    /// Return whether one shared heap reference currently refers to one live block.
    pub(crate) fn is_live(&self, reference: SharedHeapReference) -> bool {
        self.resolve_extent(reference).is_some()
    }

    /// Free one shared heap block immediately.
    pub(crate) fn free(&self, reference: SharedHeapReference) -> HeapResult<u64> {
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        };
        if extent.byte_offset != 0 {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        }

        let released_bytes = extent.byte_len as u64;
        let mut store = self.state.write();

        // release the live storage
        match extent.place {
            HeapPlace::SmallSlot(slot) => {
                self.release_small_slot(&mut store, slot)?;
            }
            HeapPlace::LargeBlock(block_id) => {
                let index = block_id.index()?;
                let Some(block) = store.large.blocks.get_mut(index) else {
                    return Err(HeapError::internal("missing large block"));
                };
                let Some(block) = block.take() else {
                    return Err(HeapError::internal("missing large block"));
                };
                let pages = {
                    let block = block.read();

                    block.pages
                };

                store.large.free_large_block_ids.push(block_id.id());
                self.unmap_page_span(&mut store, &pages);
                self.memory.release(pages)?;
                self.accounting.release_pages(pages);
            }
        }
        self.accounting.free(released_bytes as usize);

        Ok(released_bytes)
    }

    /// Create one mutator-local shared allocation cache.
    pub(crate) fn allocation_cache(&self) -> AllocationCache {
        AllocationCache::new()
    }

    /// Release memory page spans owned by this shared heap storage.
    fn close(&mut self) -> HeapResult<()> {
        let store = self.state.read();

        // release every small span range
        for span in &store.small.spans {
            self.memory.release(span.pages())?;
        }

        // release every live large block range
        for block in store.large.blocks.iter().flatten() {
            let block = block.read();
            self.memory.release(block.pages)?;
        }

        Ok(())
    }

    /// Return the base reference for one shared heap place.
    pub(super) fn base_reference(&self, place: HeapPlace) -> HeapResult<SharedHeapReference> {
        let store = self.state.read();

        store.base_reference(place)
    }
}

/// Shared heap memory metadata.
#[derive(Debug)]
pub(crate) struct HeapState {
    /// The shared heap small space.
    pub(crate) small: SmallStorage,
    /// The shared heap large space.
    pub(crate) large: LargeStorage,
    /// The owning shared heap metadata for each visible memory page.
    pub(crate) page_table: PageTable<PageOwner>,
    /// The live shared heap collector state.
    pub(crate) gc: GcState,
}

impl HeapState {
    /// Return the base reference for one shared heap place in the current state.
    fn base_reference(&self, place: HeapPlace) -> HeapResult<SharedHeapReference> {
        // dispatch by physical shared heap place
        let base_offset = match place {
            HeapPlace::SmallSlot(slot) => {
                let Some(span) = self.small.spans.get(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };
                let slot_offset = span.class.size_class() * slot.slot_index();

                span.first_offset + slot_offset
            }
            HeapPlace::LargeBlock(block_id) => {
                let Some(block) = self
                    .large
                    .blocks
                    .get(block_id.index()?)
                    .and_then(Option::as_ref)
                else {
                    return Err(HeapError::internal("missing large block"));
                };
                let block = block.read();

                block.first_offset
            }
        };

        Ok(SharedHeapReference::new(base_offset))
    }
}

/// Exact shared heap accounting.
#[derive(Debug, Default)]
pub(crate) struct HeapAccounting {
    /// The number of live shared heap blocks.
    allocation_count: AtomicUsize,
    /// The number of live shared heap payload bytes.
    allocated_bytes: AtomicU64,
    /// The retained bytes for live shared heap page spans.
    live_retained_bytes: AtomicU64,
}

impl HeapAccounting {
    /// Rebuild exact accounting from shared heap metadata.
    pub(crate) fn from_state(store: &HeapState) -> Self {
        let accounting = Self::default();

        // small spans
        for span in &store.small.spans {
            let occupied_count = span.occupied_count();
            accounting.allocate_many(
                occupied_count,
                occupied_count as u64 * span.class.size_class() as u64,
            );
            accounting.retain_pages(span.pages());
        }

        // large blocks
        for block in store.large.blocks.iter().flatten() {
            let block = block.read();
            accounting.allocate(block.byte_len);
            accounting.retain_pages(block.pages);
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
    pub(crate) fn retained_bytes(&self) -> u64 {
        self.live_retained_bytes.load(Ordering::Acquire)
    }

    /// Return exact shared heap usage.
    pub(crate) fn usage(&self) -> SharedHeapUsage {
        SharedHeapUsage {
            allocation_count: self.allocation_count(),
            allocated_bytes: self.allocated_bytes(),
            retained_bytes: self.retained_bytes(),
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
    pub(crate) fn retain_pages(&self, pages: MemoryRange) {
        let retained_bytes = pages.byte_len as u64;
        self.live_retained_bytes
            .fetch_add(retained_bytes, Ordering::AcqRel);
    }

    /// Record released live pages.
    pub(crate) fn release_pages(&self, pages: MemoryRange) {
        let retained_bytes = pages.byte_len as u64;
        self.live_retained_bytes
            .fetch_sub(retained_bytes, Ordering::AcqRel);
    }
}

/// One shared heap small space.
#[derive(Debug)]
pub(crate) struct SmallStorage {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span width.
    pub(crate) span_size_bytes: usize,
    /// The live shared heap spans.
    pub(crate) spans: Vec<Arc<SmallSpan>>,
    /// The reusable non-full spans per exact small-span class.
    pub(crate) partial_spans: BTreeMap<SmallSpanClass, Vec<usize>>,
}

/// One shared heap large space.
#[derive(Debug)]
pub(crate) struct LargeStorage {
    /// The live shared heap blocks.
    pub(crate) blocks: Vec<Option<Arc<RwLock<LargeBlock>>>>,
    /// The free shared heap block ids available for reuse.
    pub(crate) free_large_block_ids: Vec<u64>,
    /// The next shared heap block id to allocate.
    pub(crate) next_unused_large_block_id: u64,
}

impl Drop for HeapStorage {
    fn drop(&mut self) {
        // abort rather than continue with corrupted shared range metadata
        if self.close().is_err() {
            std::process::abort();
        }
    }
}
