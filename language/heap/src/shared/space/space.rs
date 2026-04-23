use std::sync::Arc;

use destack_mir::{Layout, LayoutId, LayoutTable, ReferenceMap};
use parking_lot::RwLock;

use super::{
    SharedHeapLocation, SharedHeapPageOwner, SharedHeapStorage, SharedLargeEntry,
    SharedLargeEntryId, SharedSmallSpan,
};
use crate::allocator::{Allocator, PageId, PageRunCache, PageView, SizeClassTable, SpanSlot};
use crate::shared::gc::{SharedGcPhase, SharedGcState};
use crate::{
    AccountingRegion, Allocation, AllocationUsage, GcState, HeapError, HeapOptions, HeapResult,
    SharedHeapReference, SharedHeapSpaceUsage, SmallSpanClass, clear_slot_reference_bits,
    slot_reference_map, write_slot_reference_bits,
};

/// The first allocated shared heap large-entry id.
const FIRST_SHARED_MANAGED_LARGE_ENTRY_ID: u64 = 1;

/// One shared heap small space.
#[derive(Debug)]
pub(crate) struct SharedSmallSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span size.
    pub(crate) span_bytes: usize,
    /// The live shared heap spans.
    pub(crate) spans: Vec<Arc<RwLock<SharedSmallSpan>>>,
    /// The reusable non-full spans per layout id.
    pub(crate) available_spans: Vec<Vec<usize>>,
}

/// One shared heap large space.
#[derive(Debug)]
pub(crate) struct SharedLargeSpace {
    /// The configured page size for entries in large space.
    pub(crate) page_bytes: usize,
    /// The live shared heap entries.
    pub(crate) entries: Vec<Arc<RwLock<SharedLargeEntry>>>,
    /// The free shared heap entry ids available for reuse.
    pub(crate) free_large_entry_ids: Vec<u64>,
    /// The next shared heap entry id to allocate.
    pub(crate) next_unused_large_entry_id: u64,
}

/// Shared heap allocator metadata.
#[derive(Debug)]
pub(crate) struct SharedHeapState {
    /// The shared front-end cache of reusable page runs.
    pub(crate) page_run_cache: PageRunCache,
    /// The shared heap small space.
    pub(crate) small: SharedSmallSpace,
    /// The shared heap large space.
    pub(crate) large: SharedLargeSpace,
    /// The owning shared heap metadata for each visible allocator page.
    pub(crate) page_owners: Vec<Option<SharedHeapPageOwner>>,
    /// The exact live shared heap-space usage.
    pub(crate) usage: AllocationUsage,
    /// The live shared heap collector state.
    pub(crate) gc: GcState,
}

/// One shared heap space store over a shared allocator.
#[derive(Debug)]
pub struct SharedHeapSpace {
    /// The shared heap-space allocator for every entry.
    pub(crate) allocator: Arc<Allocator>,
    /// The managed layout table visible to this shared heap space.
    pub(crate) layouts: RwLock<LayoutTable>,
    /// The shared heap allocator state.
    pub(crate) state: RwLock<SharedHeapState>,
    /// The active shared collection state.
    pub(crate) gc: SharedGcState,
}

impl SharedHeapSpace {
    /// Create a new empty shared heap-space store over one shared allocator.
    pub fn with_allocator(allocator: Arc<Allocator>) -> HeapResult<Self> {
        let options = HeapOptions {
            page_bytes: allocator.page_bytes(),
            allocator_arena_bytes: allocator.arena_bytes(),
            ..HeapOptions::shared()
        };

        Self::with_layouts_and_options(allocator, Arc::new(LayoutTable::new()), &options)
    }

    /// Create a new empty shared heap-space store over one shared allocator and options.
    pub fn with_options(allocator: Arc<Allocator>, options: &HeapOptions) -> HeapResult<Self> {
        Self::with_layouts_and_options(allocator, Arc::new(LayoutTable::new()), options)
    }

    /// Create one shared heap-space store over one shared allocator, layouts, and options.
    pub fn with_layouts_and_options(
        allocator: Arc<Allocator>,
        layouts: Arc<LayoutTable>,
        options: &HeapOptions,
    ) -> HeapResult<Self> {
        options.validate_shared()?;
        options.validate_allocator(&allocator)?;

        let store = SharedHeapState {
            page_run_cache: PageRunCache::new(allocator.pages_per_arena()),
            small: SharedSmallSpace {
                size_classes: options.size_classes.clone(),
                span_bytes: options.heap_small_bytes,
                spans: Vec::new(),
                available_spans: vec![
                    Vec::new();
                    SmallSpanClass::bucket_count(&options.size_classes)
                ],
            },
            large: SharedLargeSpace {
                page_bytes: allocator.page_bytes(),
                entries: Vec::new(),
                free_large_entry_ids: Vec::new(),
                next_unused_large_entry_id: FIRST_SHARED_MANAGED_LARGE_ENTRY_ID,
            },
            page_owners: Vec::new(),
            usage: AllocationUsage::default(),
            gc: GcState::default(),
        };

        Ok(Self {
            allocator,
            layouts: RwLock::new((*layouts).clone()),
            state: RwLock::new(store),
            gc: SharedGcState::default(),
        })
    }

    /// Return the configured shared page size.
    pub fn page_bytes(&self) -> usize {
        self.allocator.page_bytes()
    }

    /// Return one managed layout by id.
    pub(crate) fn layout(&self, layout_id: LayoutId) -> HeapResult<Layout> {
        self.layouts
            .read()
            .layouts
            .get(layout_id.index())
            .cloned()
            .ok_or(HeapError::InvalidLayoutId {
                index: layout_id.index(),
            })
    }

    /// Register one managed layout and return its stable id.
    pub(crate) fn register_layout(&self, layout: Layout) -> LayoutId {
        self.layouts.write().insert(layout)
    }

    /// Return the byte length for one managed layout.
    pub(crate) fn layout_byte_len(&self, layout_id: LayoutId) -> HeapResult<usize> {
        Ok(self.layout(layout_id)?.size as usize)
    }

    /// Return the reference map for one managed layout.
    pub(crate) fn reference_map(&self, layout_id: LayoutId) -> HeapResult<ReferenceMap> {
        Ok(self.layout(layout_id)?.reference_map)
    }

    /// Return the exact active shared heap-space bytes.
    pub fn active_bytes(&self) -> u64 {
        self.mapped_bytes()
    }

    /// Return the exact mapped shared page bytes.
    pub fn mapped_bytes(&self) -> u64 {
        let store = self.state.read();

        self.live_mapped_bytes(&store)
    }

    /// Return the exact borrowed shared image bytes.
    pub fn borrowed_bytes(&self) -> u64 {
        let store = self.state.read();
        let pages = self.live_pages(&store);

        self.allocator.borrowed_bytes_for_page_views(pages.iter())
    }

    /// Return the exact usage for this live shared heap-space store.
    pub fn usage(&self) -> SharedHeapSpaceUsage {
        // allocator summary
        let store = self.state.read();
        let allocation_count = store.usage.allocation_count();
        let allocated_bytes = store.usage.allocated_bytes();
        let mapped_bytes = self.live_mapped_bytes(&store);
        let active_bytes = mapped_bytes;

        drop(store);

        // borrowed bytes
        let borrowed_bytes = self.borrowed_bytes();

        SharedHeapSpaceUsage {
            allocation_count,
            allocated_bytes,
            active_bytes,
            mapped_bytes,
            borrowed_bytes,
        }
    }

    /// Return the number of live shared heap allocations.
    pub fn allocation_count(&self) -> usize {
        let store = self.state.read();

        store.usage.allocation_count()
    }

    /// Return the number of live shared heap bytes.
    pub fn allocated_bytes(&self) -> u64 {
        let store = self.state.read();

        store.usage.allocated_bytes()
    }

    /// Return the current shared heap collector state.
    pub fn gc_state(&self) -> GcState {
        let store = self.state.read();

        store.gc.clone()
    }

    /// Return the current shared heap collector phase.
    pub fn gc_phase(&self) -> SharedGcPhase {
        self.gc.phase()
    }

    /// Return whether one shared heap reference currently refers to one live entry.
    pub fn is_live(&self, reference: SharedHeapReference) -> bool {
        self.resolve_location(reference).is_some()
    }

    /// Allocate one shared managed heap entry.
    pub fn allocate(
        &self,
        layout_id: LayoutId,
        allocation: Allocation<'_>,
    ) -> HeapResult<SharedHeapReference> {
        let byte_len = self.layout_byte_len(layout_id)?;
        if let Some(bytes) = allocation.bytes()
            && bytes.len() != byte_len
        {
            return Err(HeapError::InvalidLayoutBytes {
                layout_id,
                expected: byte_len,
                actual: bytes.len(),
            });
        }

        let reference_map = self.reference_map(layout_id)?.clone();
        let mut store = self.state.write();

        // allocate the backing storage before publishing the live reference
        let storage = self.allocate_storage(&mut store, layout_id, byte_len, allocation)?;
        let reference = self.base_reference(&store, storage)?;

        store
            .usage
            .allocate(byte_len, AccountingRegion::SharedHeap)?;
        drop(store);

        let has_shared_reference =
            allocation.bytes().is_some() && reference_map.has_shared_reference();
        self.publish_shared_allocation(reference, has_shared_reference)?;

        Ok(reference)
    }

    /// Return the projected mapped-byte delta for one typed shared heap allocation.
    pub(crate) fn mapped_byte_delta(&self, layout_id: LayoutId) -> HeapResult<i64> {
        let store = self.state.read();
        let byte_len = self.layout_byte_len(layout_id)?;
        let reference_map = self.reference_map(layout_id)?;

        if let Some(class) = self.small_span_class(&store, byte_len, &reference_map) {
            if self.has_available_small_slot(&store, &class)? {
                return Ok(0);
            }

            return Ok(store.small.span_bytes as i64);
        }

        Ok(self.round_up_allocation_bytes(byte_len) as i64)
    }

    /// Return the remaining byte length for one shared heap reference.
    pub fn byte_len(&self, reference: SharedHeapReference) -> HeapResult<usize> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        };

        checked_remaining_byte_len(location.byte_offset, location.byte_len)
    }

    /// Return the bytes for one shared heap reference.
    pub fn read_bytes(&self, reference: SharedHeapReference) -> HeapResult<Vec<u8>> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        };

        let byte_len = checked_remaining_byte_len(location.byte_offset, location.byte_len)?;

        self.location_bytes(location, location.byte_offset, byte_len)
    }

    /// Fill one caller-provided buffer from one shared heap entry at one offset.
    pub fn read_bytes_into(
        &self,
        reference: SharedHeapReference,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        let (location, byte_offset) =
            self.checked_location_range(reference, start, target.len())?;

        self.fill_location_bytes(location, byte_offset, target)
    }

    /// Return the reference map for one shared heap reference.
    pub fn scan(&self, reference: SharedHeapReference) -> HeapResult<ReferenceMap> {
        let (location, _) = self.checked_location_range(reference, 0, 0)?;

        self.location_reference_map(location.storage)
    }

    /// Overwrite one shared heap byte range.
    pub fn write_bytes(
        &self,
        reference: SharedHeapReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let (location, byte_offset) = self.checked_location_range(reference, start, bytes.len())?;
        self.write_location_bytes(location, byte_offset, bytes)
    }

    /// Record one shared heap write barrier.
    pub fn write_barrier(
        &self,
        reference: SharedHeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        let (_, byte_offset) = self.checked_location_range(reference, start, byte_len)?;
        self.write_shared_barrier(reference, byte_offset, byte_len)
    }

    /// Publish one exact shared heap edge after one completed store.
    pub fn publish_edge(&self, reference: SharedHeapReference) -> HeapResult<()> {
        self.publish_shared_edge(reference)
    }

    /// Return one checked live location and byte offset for one shared heap range.
    fn checked_location_range(
        &self,
        reference: SharedHeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<(SharedHeapLocation, usize)> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        };
        let byte_offset =
            checked_byte_range(location.byte_offset, start, byte_len, location.byte_len)?;

        Ok((location, byte_offset))
    }

    /// Return one shared heap large entry by entry id.
    pub(crate) fn large_entry_ref(
        &self,
        entry_id: SharedLargeEntryId,
    ) -> Option<Arc<RwLock<SharedLargeEntry>>> {
        let index = entry_id.index().ok()?;
        let store = self.state.read();

        store.large.entries.get(index).cloned()
    }

    /// Allocate one shared heap storage partition for the given payload.
    fn allocate_storage(
        &self,
        store: &mut SharedHeapState,
        layout_id: LayoutId,
        byte_len: usize,
        allocation: Allocation<'_>,
    ) -> HeapResult<SharedHeapStorage> {
        let reference_map = self.reference_map(layout_id)?.clone();

        // allocate from one homogeneous small span when the payload still fits
        if let Some(class) = self.small_span_class(store, byte_len, &reference_map) {
            let slot = self.allocate_small(store, &class, byte_len, &reference_map, allocation)?;

            Ok(SharedHeapStorage::Small(slot))
        }
        // otherwise allocate one dedicated large entry
        else {
            let pages = match allocation {
                Allocation::Bytes(bytes) => store
                    .page_run_cache
                    .allocate_bytes(&self.allocator, bytes)?,
                Allocation::Zeroed => store
                    .page_run_cache
                    .allocate_zeroed(&self.allocator, byte_len)?,
            };
            let entry_id = self.store_large_entry(store, byte_len, pages, layout_id)?;

            Ok(SharedHeapStorage::Large(entry_id))
        }
    }

    /// Return whether one size class still has one live reusable slot.
    fn has_available_small_slot(
        &self,
        store: &SharedHeapState,
        class: &SmallSpanClass,
    ) -> HeapResult<bool> {
        let bucket_index = class.bucket_index(&store.small.size_classes)?;

        Ok(store
            .small
            .available_spans
            .get(bucket_index)
            .is_some_and(|spans| !spans.is_empty()))
    }

    /// Return one homogeneous small-span class for the given layout when it fits.
    fn small_span_class(
        &self,
        store: &SharedHeapState,
        byte_len: usize,
        reference_map: &ReferenceMap,
    ) -> Option<SmallSpanClass> {
        let class_index = store.small.size_classes.class_index_for(byte_len)?;

        Some(SmallSpanClass {
            size_class: store.small.size_classes.classes[class_index].bytes,
            is_noscan: !reference_map.has_reference(),
        })
    }

    /// Allocate or reuse one non-full shared heap span for the given size class.
    fn allocate_small_span(
        &self,
        store: &mut SharedHeapState,
        class: &SmallSpanClass,
    ) -> HeapResult<usize> {
        let bucket_index = class.bucket_index(&store.small.size_classes)?;

        while let Some(span_index) = store.small.available_spans[bucket_index].pop() {
            let Some(span) = store.small.spans.get(span_index).cloned() else {
                return Err(HeapError::MissingSpan { span_index });
            };
            let mut span = span.write();

            if span.occupied_count < span.slot_count {
                if span.occupied_count == 0 && span.pages.is_empty() {
                    let pages = store
                        .page_run_cache
                        .allocate_zeroed(&self.allocator, store.small.span_bytes)?;
                    self.map_page_view(store, &pages, |logical_page_index| {
                        SharedHeapPageOwner::Small {
                            span_index,
                            logical_page_index,
                        }
                    })?;
                    span.pages = pages.clone();
                }

                return Ok(span_index);
            }
        }

        let slot_count = (store.small.span_bytes / class.size_class).max(1);
        let scan_word_count = class.size_class.div_ceil(std::mem::size_of::<usize>());
        let pages = store
            .page_run_cache
            .allocate_zeroed(&self.allocator, store.small.span_bytes)?;
        let span = SharedSmallSpan {
            class: class.clone(),
            slot_count,
            byte_lens: vec![0; slot_count].into_boxed_slice(),
            occupied_count: 0,
            free_cursor: 0,
            occupied: crate::Bitmap::with_capacity(slot_count),
            local_reference_bits: crate::Bitmap::with_capacity(slot_count * scan_word_count),
            shared_reference_bits: crate::Bitmap::with_capacity(slot_count * scan_word_count),
            marked: crate::Bitmap::with_capacity(slot_count),
            pages: pages.clone(),
        };
        let span_index = store.small.spans.len();
        self.map_page_view(store, &span.pages, |logical_page_index| {
            SharedHeapPageOwner::Small {
                span_index,
                logical_page_index,
            }
        })?;

        store.small.spans.push(Arc::new(RwLock::new(span)));

        Ok(span_index)
    }

    /// Allocate one shared heap small slot from one explicit initialization source.
    fn allocate_small(
        &self,
        store: &mut SharedHeapState,
        class: &SmallSpanClass,
        byte_len: usize,
        reference_map: &ReferenceMap,
        allocation: Allocation<'_>,
    ) -> HeapResult<SpanSlot> {
        let span_index = self.allocate_small_span(store, &class)?;
        let Some(span) = store.small.spans.get(span_index).cloned() else {
            return Err(HeapError::MissingSpan { span_index });
        };
        let mut span = span.write();
        let slot_index = span.free_cursor;

        // slot initialization
        let slot = SpanSlot::new(span_index, slot_index)?;
        let slot_offset = checked_slot_offset(span.class.size_class, slot_index)?;

        if let Allocation::Bytes(bytes) = allocation {
            let write_offset = checked_storage_offset(slot_offset, 0, store.small.span_bytes)?;

            self.allocator
                .set_bytes(&mut span.pages, write_offset, bytes)?;
        }

        // slot metadata
        span.occupied.set(slot_index);
        span.marked.clear(slot_index);
        span.byte_lens[slot_index] = byte_len;
        {
            let super::SharedSmallSpan {
                class,
                local_reference_bits,
                shared_reference_bits,
                ..
            } = &mut *span;
            let size_class = class.size_class;

            write_slot_reference_bits(
                reference_map,
                local_reference_bits,
                shared_reference_bits,
                slot_index,
                size_class,
            );
        }
        span.occupied_count += 1;
        span.free_cursor = find_free_cursor(&span.occupied, slot_index + 1, span.slot_count);

        if span.occupied_count < span.slot_count {
            let bucket_index = class.bucket_index(&store.small.size_classes)?;
            store.small.available_spans[bucket_index].push(span_index);
        }

        Ok(slot)
    }

    /// Store one large entry in shared heap large space and return its entry id.
    fn store_large_entry(
        &self,
        store: &mut SharedHeapState,
        len: usize,
        pages: PageView,
        layout_id: LayoutId,
    ) -> HeapResult<SharedLargeEntryId> {
        let (entry_id, reused_entry_id) = if let Some(entry_id) =
            store.large.free_large_entry_ids.pop()
        {
            (entry_id, true)
        } else {
            let entry_id = store.large.next_unused_large_entry_id;
            let Some(next_entry_id) = store.large.next_unused_large_entry_id.checked_add(1) else {
                store
                    .page_run_cache
                    .release_page_view(&self.allocator, pages)?;

                return Err(HeapError::InvalidLargeEntryId { id: entry_id });
            };

            store.large.next_unused_large_entry_id = next_entry_id;
            (entry_id, false)
        };

        if entry_id == 0 {
            if reused_entry_id {
                store.large.free_large_entry_ids.push(entry_id);
            }

            store
                .page_run_cache
                .release_page_view(&self.allocator, pages)?;

            return Err(HeapError::InvalidLargeEntryId { id: entry_id });
        }

        let entry_pages = pages.clone();
        let entry = SharedLargeEntry {
            is_live: true,
            len,
            pages: pages.clone(),
            layout_id,
            is_marked: false,
        };
        let index = SharedLargeEntryId::new(entry_id).index()?;

        if index > store.large.entries.len() {
            if reused_entry_id {
                store.large.free_large_entry_ids.push(entry_id);
            }

            store
                .page_run_cache
                .release_page_view(&self.allocator, pages)?;

            return Err(HeapError::InvalidLargeEntryId { id: entry_id });
        }

        let entry = Arc::new(RwLock::new(entry));

        if index == store.large.entries.len() {
            store.large.entries.push(entry);
        } else {
            store.large.entries[index] = entry;
        }

        let entry_id = SharedLargeEntryId::new(entry_id);
        self.map_page_view(store, &entry_pages, |logical_page_index| {
            SharedHeapPageOwner::Large {
                entry_id,
                logical_page_index,
            }
        })?;

        Ok(entry_id)
    }

    /// Release one shared heap small slot.
    pub(crate) fn release_small_slot(
        &self,
        store: &mut SharedHeapState,
        slot: SpanSlot,
    ) -> HeapResult<()> {
        let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
            return Err(HeapError::MissingSpan {
                span_index: slot.span_index(),
            });
        };

        let pages = {
            let mut span = span.write();
            let slot_index = slot.slot_index();
            let was_full = span.occupied_count == span.slot_count;

            if !span.occupied.contains(slot_index) || span.occupied_count == 0 {
                return Err(HeapError::MissingSmallSlot {
                    span_index: slot.span_index(),
                    slot_index,
                });
            }

            span.occupied.clear(slot_index);
            span.marked.clear(slot_index);
            span.byte_lens[slot_index] = 0;
            {
                let super::SharedSmallSpan {
                    class,
                    local_reference_bits,
                    shared_reference_bits,
                    ..
                } = &mut *span;
                let size_class = class.size_class;

                clear_slot_reference_bits(
                    local_reference_bits,
                    shared_reference_bits,
                    slot_index,
                    size_class,
                );
            }
            span.occupied_count -= 1;
            span.free_cursor = span.free_cursor.min(slot_index);

            if span.occupied_count == 0 {
                span.free_cursor = 0;

                let pages = span.pages.clone();
                span.pages = PageView::empty();

                Some(pages)
            } else {
                let should_requeue = was_full && span.occupied_count < span.slot_count;

                if should_requeue {
                    let bucket_index = span.class.bucket_index(&store.small.size_classes)?;
                    store.small.available_spans[bucket_index].push(slot.span_index());
                }

                None
            }
        };

        if let Some(pages) = pages {
            self.unmap_page_view(store, &pages)?;
            store
                .page_run_cache
                .release_page_view(&self.allocator, pages)?;
        }

        Ok(())
    }

    /// Return the reference map for one shared heap location.
    fn location_reference_map(&self, storage: SharedHeapStorage) -> HeapResult<ReferenceMap> {
        match storage {
            SharedHeapStorage::Small(slot) => {
                self.small_slot_reference_map(slot.span_index(), slot.slot_index())
            }
            SharedHeapStorage::Large(entry_id) => {
                let layout_id = self
                    .state
                    .read()
                    .large
                    .entries
                    .get(entry_id.index()?)
                    .cloned()
                    .ok_or(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    })?
                    .read()
                    .layout_id;

                self.reference_map(layout_id)
            }
        }
    }

    /// Return the bytes for one shared heap location as one owned vector.
    fn location_bytes(
        &self,
        location: SharedHeapLocation,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<Vec<u8>> {
        match location.storage {
            SharedHeapStorage::Small(slot) => {
                let store = self.state.read();
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span = span.read();

                let slot_offset = checked_slot_offset(span.class.size_class, slot.slot_index())?;
                let read_offset =
                    checked_storage_offset(slot_offset, byte_offset, store.small.span_bytes)?;

                self.allocator
                    .bytes_to_vec_from(&span.pages, read_offset, byte_len)
            }
            SharedHeapStorage::Large(entry_id) => {
                let Some(entry) = self.large_entry_ref(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };
                let entry = entry.read();

                if !entry.is_live {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                }

                self.allocator
                    .bytes_to_vec_from(&entry.pages, byte_offset, byte_len)
            }
        }
    }

    /// Fill one caller-provided buffer from one shared heap location.
    fn fill_location_bytes(
        &self,
        location: SharedHeapLocation,
        byte_offset: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        match location.storage {
            SharedHeapStorage::Small(slot) => {
                let store = self.state.read();
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span = span.read();

                let slot_offset = checked_slot_offset(span.class.size_class, slot.slot_index())?;
                let read_offset =
                    checked_storage_offset(slot_offset, byte_offset, store.small.span_bytes)?;

                self.allocator
                    .fill_bytes_from(&span.pages, read_offset, target)
            }
            SharedHeapStorage::Large(entry_id) => {
                let Some(entry) = self.large_entry_ref(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };
                let entry = entry.read();

                if !entry.is_live {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                }

                self.allocator
                    .fill_bytes_from(&entry.pages, byte_offset, target)
            }
        }
    }

    /// Overwrite one byte range for one shared heap location.
    fn write_location_bytes(
        &self,
        location: SharedHeapLocation,
        byte_offset: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        match location.storage {
            SharedHeapStorage::Small(slot) => {
                let store = self.state.read();
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span_bytes = store.small.span_bytes;
                drop(store);

                let (previous_pages, next_pages) = {
                    let mut span = span.write();
                    let previous_pages = span.pages.clone();
                    let slot_offset =
                        checked_slot_offset(span.class.size_class, slot.slot_index())?;
                    let write_offset =
                        checked_storage_offset(slot_offset, byte_offset, span_bytes)?;

                    self.allocator
                        .set_bytes(&mut span.pages, write_offset, bytes)?;

                    (previous_pages, span.pages.clone())
                };

                if next_pages != previous_pages {
                    let mut store = self.state.write();

                    self.unmap_page_view(&mut store, &previous_pages)?;
                    self.map_page_view(&mut store, &next_pages, |logical_page_index| {
                        SharedHeapPageOwner::Small {
                            span_index: slot.span_index(),
                            logical_page_index,
                        }
                    })?;
                }

                Ok(())
            }
            SharedHeapStorage::Large(entry_id) => {
                let Some(entry) = self.large_entry_ref(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };
                let mut entry = entry.write();

                if !entry.is_live {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                }

                let previous_pages = entry.pages.clone();

                self.allocator
                    .set_bytes(&mut entry.pages, byte_offset, bytes)?;

                if entry.pages != previous_pages {
                    let next_pages = entry.pages.clone();
                    drop(entry);

                    let mut store = self.state.write();

                    self.unmap_page_view(&mut store, &previous_pages)?;
                    self.map_page_view(&mut store, &next_pages, |logical_page_index| {
                        SharedHeapPageOwner::Large {
                            entry_id,
                            logical_page_index,
                        }
                    })?;

                    return Ok(());
                }

                Ok(())
            }
        }
    }

    /// Return the current live heap page views.
    fn live_pages(&self, store: &SharedHeapState) -> Vec<PageView> {
        let mut pages = Vec::with_capacity(store.small.spans.len() + store.large.entries.len());

        for span in &store.small.spans {
            let span = span.read();

            if span.occupied_count == 0 || span.pages.is_empty() {
                continue;
            }

            pages.push(span.pages.clone());
        }

        for entry in &store.large.entries {
            let entry = entry.read();

            if !entry.is_live {
                continue;
            }

            pages.push(entry.pages.clone());
        }

        pages
    }

    /// Return the mapped live bytes for the current shared heap state.
    pub(crate) fn live_mapped_bytes(&self, store: &SharedHeapState) -> u64 {
        let pages = self.live_pages(store);
        let cached_bytes = store
            .page_run_cache
            .cached_bytes(self.allocator.page_bytes());

        self.allocator
            .mapped_bytes_for_page_views(pages.iter())
            .saturating_add(cached_bytes)
    }

    /// Return the page-rounded mapped bytes for one shared heap-space entry.
    fn round_up_allocation_bytes(&self, byte_len: usize) -> u64 {
        let page_bytes = self.page_bytes() as u64;
        let byte_len = byte_len as u64;

        byte_len.div_ceil(page_bytes) * page_bytes
    }

    /// Return the visible owner for one physical page.
    fn page_owner(&self, store: &SharedHeapState, page_id: PageId) -> Option<SharedHeapPageOwner> {
        store.page_owners.get(page_id.index()).copied().flatten()
    }

    /// Record one visible owner for every page in one logical page view.
    fn map_page_view(
        &self,
        store: &mut SharedHeapState,
        page_view: &PageView,
        mut owner: impl FnMut(usize) -> SharedHeapPageOwner,
    ) -> HeapResult<()> {
        for logical_page_index in 0..page_view.len() {
            let Some(page_id) = page_view.page(logical_page_index) else {
                return Err(HeapError::MissingLogicalPage {
                    page_index: logical_page_index,
                });
            };
            let page_index = page_id.index();

            if store.page_owners.len() <= page_index {
                store.page_owners.resize(page_index + 1, None);
            }

            store.page_owners[page_index] = Some(owner(logical_page_index));
        }

        Ok(())
    }

    /// Clear every visible owner for one logical page view.
    pub(crate) fn unmap_page_view(
        &self,
        store: &mut SharedHeapState,
        page_view: &PageView,
    ) -> HeapResult<()> {
        for logical_page_index in 0..page_view.len() {
            let Some(page_id) = page_view.page(logical_page_index) else {
                return Err(HeapError::MissingLogicalPage {
                    page_index: logical_page_index,
                });
            };

            if let Some(owner) = store.page_owners.get_mut(page_id.index()) {
                *owner = None;
            }
        }

        Ok(())
    }

    /// Return the resolved location for one live shared heap reference.
    pub(crate) fn resolve_location(
        &self,
        reference: SharedHeapReference,
    ) -> Option<SharedHeapLocation> {
        let (page_id, page_offset) = self.allocator.address_page_position(reference.address())?;
        let store = self.state.read();
        let owner = self.page_owner(&store, page_id)?;

        match owner {
            SharedHeapPageOwner::Small {
                span_index,
                logical_page_index,
            } => {
                let span = store.small.spans.get(span_index)?.clone();
                let span = span.read();
                let logical_byte_offset = logical_page_index
                    .checked_mul(self.allocator.page_bytes())?
                    .checked_add(page_offset)?;
                let slot_index = logical_byte_offset / span.class.size_class;
                let slot_offset = logical_byte_offset % span.class.size_class;
                if slot_index >= span.slot_count || !span.occupied.contains(slot_index) {
                    return None;
                }

                let byte_len = *span.byte_lens.get(slot_index)?;
                if slot_offset >= byte_len {
                    return None;
                }

                let slot_base_offset = slot_index.checked_mul(span.class.size_class)?;
                let base_address = self
                    .allocator
                    .page_view_ptr(&span.pages, slot_base_offset)
                    .ok()? as *mut u8 as usize;
                let slot = SpanSlot::new(span_index, slot_index).ok()?;

                Some(SharedHeapLocation {
                    storage: SharedHeapStorage::Small(slot),
                    base: SharedHeapReference::new(base_address),
                    byte_offset: slot_offset,
                    byte_len,
                })
            }
            SharedHeapPageOwner::Large {
                entry_id,
                logical_page_index,
            } => {
                let entry = store.large.entries.get(entry_id.index().ok()?)?.clone();
                let entry = entry.read();
                if !entry.is_live {
                    return None;
                }

                let logical_byte_offset = logical_page_index
                    .checked_mul(self.allocator.page_bytes())?
                    .checked_add(page_offset)?;
                if logical_byte_offset >= entry.len {
                    return None;
                }

                let base_address =
                    self.allocator.page_view_ptr(&entry.pages, 0).ok()? as *mut u8 as usize;

                Some(SharedHeapLocation {
                    storage: SharedHeapStorage::Large(entry_id),
                    base: SharedHeapReference::new(base_address),
                    byte_offset: logical_byte_offset,
                    byte_len: entry.len,
                })
            }
        }
    }

    /// Return the exact reference map stored for one shared small slot.
    pub(crate) fn small_slot_reference_map(
        &self,
        span_index: usize,
        slot_index: usize,
    ) -> HeapResult<ReferenceMap> {
        let store = self.state.read();
        let Some(span) = store.small.spans.get(span_index).cloned() else {
            return Err(HeapError::MissingSpan { span_index });
        };
        let span = span.read();
        let Some(&byte_len) = span.byte_lens.get(slot_index) else {
            return Err(HeapError::MissingSmallSlot {
                span_index,
                slot_index,
            });
        };

        Ok(slot_reference_map(
            &span.local_reference_bits,
            &span.shared_reference_bits,
            slot_index,
            span.class.size_class,
            byte_len,
        ))
    }

    /// Return every live shared heap reference.
    pub(crate) fn live_references(&self) -> HeapResult<Vec<SharedHeapReference>> {
        let store = self.state.read();
        let mut references = Vec::new();

        for span in &store.small.spans {
            let span = span.read();
            if span.occupied_count == 0 || span.pages.is_empty() {
                continue;
            }

            for slot_index in 0..span.slot_count {
                if !span.occupied.contains(slot_index) {
                    continue;
                }

                let slot_offset = span.class.size_class.checked_mul(slot_index).ok_or(
                    HeapError::InvariantOverflow {
                        context: "shared heap slot base offset",
                    },
                )?;
                let base_address =
                    self.allocator.page_view_ptr(&span.pages, slot_offset)? as *mut u8 as usize;

                references.push(SharedHeapReference::new(base_address));
            }
        }

        for entry in &store.large.entries {
            let entry = entry.read();
            if !entry.is_live {
                continue;
            }

            let base_address = self.allocator.page_view_ptr(&entry.pages, 0)? as *mut u8 as usize;
            references.push(SharedHeapReference::new(base_address));
        }

        Ok(references)
    }

    /// Return the base reference for one shared heap storage partition.
    fn base_reference(
        &self,
        store: &SharedHeapState,
        storage: SharedHeapStorage,
    ) -> HeapResult<SharedHeapReference> {
        let base_address = match storage {
            SharedHeapStorage::Small(slot) => {
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span = span.read();
                let slot_offset = span.class.size_class.checked_mul(slot.slot_index()).ok_or(
                    HeapError::InvariantOverflow {
                        context: "shared heap slot base offset",
                    },
                )?;

                self.allocator.page_view_ptr(&span.pages, slot_offset)? as *mut u8 as usize
            }
            SharedHeapStorage::Large(entry_id) => {
                let Some(entry) = store.large.entries.get(entry_id.index()?).cloned() else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };
                let entry = entry.read();
                if !entry.is_live {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                }

                self.allocator.page_view_ptr(&entry.pages, 0)? as *mut u8 as usize
            }
        };

        Ok(SharedHeapReference::new(base_address))
    }
}

/// Return the visible byte length for one shared heap reference.
fn checked_remaining_byte_len(byte_offset: usize, byte_len: usize) -> HeapResult<usize> {
    if byte_offset > byte_len {
        return Err(HeapError::InvalidByteRange {
            start: byte_offset,
            len: 0,
            capacity: byte_len,
        });
    }

    Ok(byte_len - byte_offset)
}

/// Validate one byte range inside one shared heap entry.
fn checked_byte_range(
    pointer_offset: usize,
    start: usize,
    len: usize,
    capacity: usize,
) -> HeapResult<usize> {
    let byte_offset = pointer_offset
        .checked_add(start)
        .ok_or(HeapError::InvalidByteRange {
            start,
            len,
            capacity,
        })?;
    let byte_end = byte_offset
        .checked_add(len)
        .ok_or(HeapError::InvalidByteRange {
            start,
            len,
            capacity,
        })?;

    if byte_end > capacity {
        return Err(HeapError::InvalidByteRange {
            start: byte_offset,
            len,
            capacity,
        });
    }

    Ok(byte_offset)
}

/// Return the byte offset for one slot payload inside one span.
pub(crate) fn checked_slot_offset(size_class: usize, slot_index: usize) -> HeapResult<usize> {
    size_class
        .checked_mul(slot_index)
        .ok_or(HeapError::InvariantOverflow {
            context: "shared small-slot byte offset",
        })
}

/// Return one nested storage offset inside one bounded page view.
pub(crate) fn checked_storage_offset(
    base: usize,
    byte_offset: usize,
    capacity: usize,
) -> HeapResult<usize> {
    let storage_offset = base
        .checked_add(byte_offset)
        .ok_or(HeapError::InvalidByteRange {
            start: base,
            len: byte_offset,
            capacity,
        })?;

    if storage_offset > capacity {
        return Err(HeapError::InvalidByteRange {
            start: storage_offset,
            len: 0,
            capacity,
        });
    }

    Ok(storage_offset)
}

/// Return the next clear slot starting at the given cursor.
pub(crate) fn find_free_cursor(occupied: &crate::Bitmap, start: usize, slot_count: usize) -> usize {
    for slot_index in start..slot_count {
        if !occupied.contains(slot_index) {
            return slot_index;
        }
    }

    slot_count
}

impl Drop for SharedHeapSpace {
    fn drop(&mut self) {
        let mut store = self.state.write();

        store.page_run_cache.flush(&self.allocator);
    }
}
