use std::sync::Arc;

use parking_lot::RwLock;

use super::{
    SharedHeapLocation, SharedHeapPageOwner, SharedHeapStorage, SharedLargeEntry,
    SharedLargeEntryId, SharedSmallSpan,
};
use crate::allocator::{
    Allocator, PageId, PageRunCache, PageView, SizeClassTable, SpanAllocationPath, SpanSlot,
};
use crate::shared::gc::{SharedGcPhase, SharedGcState};
use crate::{
    AccountingRegion, AllocationUsage, GcSummary, HeapError, HeapOptions, HeapResult, LayoutId,
    Shape, ShapeId, ShapeTable, SharedHeapReference, SharedHeapSpaceUsage, TracePlan,
};

/// The first allocated shared heap large-entry id.
const FIRST_SHARED_MANAGED_LARGE_ENTRY_ID: u64 = 1;

/// One shared heap small space.
#[derive(Debug)]
pub(crate) struct SharedSmallSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span width.
    pub(crate) span_bytes: usize,
    /// The live shared heap spans.
    pub(crate) spans: Vec<Arc<RwLock<SharedSmallSpan>>>,
    /// The reusable non-full spans per size class.
    pub(crate) available_spans: Vec<Vec<usize>>,
}

/// One shared heap large space.
#[derive(Debug)]
pub(crate) struct SharedLargeSpace {
    /// The configured page width for entries in large space.
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
    /// The interned shared heap entry shapes.
    pub(crate) shape_table: ShapeTable,
    /// The exact live shared heap-space usage.
    pub(crate) usage: AllocationUsage,
    /// The live shared heap collector state.
    pub(crate) gc_state: GcSummary,
}

/// One shared heap space store over a shared allocator.
#[derive(Debug)]
pub struct SharedHeapSpace {
    /// The shared heap-space allocator for every entry.
    pub(crate) allocator: Arc<Allocator>,
    /// The shared heap allocator control plane.
    pub(crate) store: RwLock<SharedHeapState>,
    /// The active shared collection state.
    pub(crate) gc: SharedGcState,
}

impl Default for SharedHeapSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedHeapSpace {
    /// Create a new empty shared heap-space store.
    pub fn new() -> Self {
        Self::with_options(Arc::new(Allocator::new()), &HeapOptions::shared())
    }

    /// Create a new empty shared heap-space store over one shared allocator.
    pub fn with_allocator(allocator: Arc<Allocator>) -> Self {
        let options = HeapOptions {
            page_bytes: allocator.page_bytes(),
            allocator_segment_bytes: allocator.segment_bytes(),
            ..HeapOptions::shared()
        };

        Self::with_options(allocator, &options)
    }

    /// Create a new empty shared heap-space store over one shared allocator and options.
    pub fn with_options(allocator: Arc<Allocator>, options: &HeapOptions) -> Self {
        options
            .validate_shared()
            .expect("shared heap options should validate");
        options
            .validate_allocator(&allocator)
            .expect("shared heap allocator should match options");

        let store = SharedHeapState {
            page_run_cache: PageRunCache::new(allocator.pages_per_segment()),
            small: SharedSmallSpace {
                size_classes: options.size_classes.clone(),
                span_bytes: options.heap_small_bytes,
                spans: Vec::new(),
                available_spans: vec![Vec::new(); options.size_classes.classes.len()],
            },
            large: SharedLargeSpace {
                page_bytes: allocator.page_bytes(),
                entries: Vec::new(),
                free_large_entry_ids: Vec::new(),
                next_unused_large_entry_id: FIRST_SHARED_MANAGED_LARGE_ENTRY_ID,
            },
            page_owners: Vec::new(),
            shape_table: ShapeTable::new(),
            usage: AllocationUsage::default(),
            gc_state: GcSummary::default(),
        };

        Self {
            allocator,
            store: RwLock::new(store),
            gc: SharedGcState::default(),
        }
    }

    /// Return the configured shared page width.
    pub fn page_bytes(&self) -> usize {
        self.allocator.page_bytes()
    }

    /// Return the exact active shared heap-space bytes.
    pub fn active_bytes(&self) -> u64 {
        self.mapped_bytes()
    }

    /// Return the exact mapped shared page bytes.
    pub fn mapped_bytes(&self) -> u64 {
        let store = self.store.read();

        self.live_mapped_bytes(&store)
    }

    /// Return the exact borrowed shared image bytes.
    pub fn borrowed_bytes(&self) -> u64 {
        let store = self.store.read();
        let pages = self.live_pages(&store);

        self.allocator.borrowed_bytes_for_page_views(pages.iter())
    }

    /// Return the exact usage for this live shared heap-space store.
    pub fn usage(&self) -> SharedHeapSpaceUsage {
        // allocator summary
        let store = self.store.read();
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
        let store = self.store.read();

        store.usage.allocation_count()
    }

    /// Return the number of live shared heap bytes.
    pub fn allocated_bytes(&self) -> u64 {
        let store = self.store.read();

        store.usage.allocated_bytes()
    }

    /// Return the current shared heap collector state.
    pub fn gc_state(&self) -> GcSummary {
        let store = self.store.read();

        store.gc_state.clone()
    }

    /// Return the current shared heap collector phase.
    pub fn gc_phase(&self) -> SharedGcPhase {
        self.gc.phase()
    }

    /// Return whether one shared heap reference currently refers to one live entry.
    pub fn is_live(&self, reference: SharedHeapReference) -> bool {
        self.resolve_location(reference).is_some()
    }

    /// Allocate one shared heap byte entry.
    pub fn allocate_bytes(
        &self,
        bytes: &[u8],
        scan: impl Into<TracePlan>,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<SharedHeapReference> {
        let path = self.allocation_path(bytes.len());

        self.place_bytes(bytes, scan, layout_id, path)
    }

    /// Allocate one zeroed shared heap-space entry.
    pub fn allocate_zeroed(
        &self,
        byte_len: usize,
        scan: impl Into<TracePlan>,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<SharedHeapReference> {
        let path = self.allocation_path(byte_len);

        self.place_zeroed(byte_len, scan, layout_id, path)
    }

    /// Allocate one shared heap byte entry through one precomputed allocation path.
    pub(crate) fn place_bytes(
        &self,
        bytes: &[u8],
        scan: impl Into<TracePlan>,
        layout_id: Option<LayoutId>,
        path: SpanAllocationPath,
    ) -> HeapResult<SharedHeapReference> {
        let byte_len = bytes.len();
        let scan = scan.into();
        let mut store = self.store.write();

        // allocate the backing location before publishing the live reference
        let storage = self.allocate_location(
            &mut store,
            byte_len,
            scan.clone(),
            layout_id,
            Some(bytes),
            path,
        )?;
        let reference = self.base_reference(&store, storage)?;

        store
            .usage
            .allocate(byte_len, AccountingRegion::SharedHeap)?;
        drop(store);

        self.publish_shared_allocation(reference, scan.has_shared_reference())?;

        Ok(reference)
    }

    /// Allocate one zeroed shared heap entry through one precomputed allocation path.
    pub(crate) fn place_zeroed(
        &self,
        byte_len: usize,
        scan: impl Into<TracePlan>,
        layout_id: Option<LayoutId>,
        path: SpanAllocationPath,
    ) -> HeapResult<SharedHeapReference> {
        let scan = scan.into();
        let mut store = self.store.write();

        // allocate the zeroed backing location before publishing the live reference
        let storage = self.allocate_location(&mut store, byte_len, scan, layout_id, None, path)?;
        let reference = self.base_reference(&store, storage)?;

        store
            .usage
            .allocate(byte_len, AccountingRegion::SharedHeap)?;
        drop(store);

        self.publish_shared_allocation(reference, false)?;

        Ok(reference)
    }

    /// Return one exact shared heap allocation path for the requested byte length.
    pub(crate) fn allocation_path(&self, byte_len: usize) -> SpanAllocationPath {
        let store = self.store.read();

        store.small.size_classes.span_allocation_path(
            byte_len,
            store.small.span_bytes,
            |class_index| self.has_available_small_slot(&store, class_index),
            |large_bytes| self.round_up_allocation_bytes(large_bytes),
        )
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

    /// Return the scan metadata for one shared heap reference.
    pub fn scan(&self, reference: SharedHeapReference) -> HeapResult<TracePlan> {
        let (location, _) = self.checked_location_range(reference, 0, 0)?;

        self.location_scan(location.storage)
    }

    /// Return the storage layout id for one shared heap reference.
    pub fn layout_id(&self, reference: SharedHeapReference) -> HeapResult<Option<LayoutId>> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        };

        self.location_layout_id(location.storage)
    }

    /// Set the storage layout id for one shared heap reference.
    pub fn set_layout_id(
        &self,
        reference: SharedHeapReference,
        layout_id: LayoutId,
    ) -> HeapResult<()> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        };

        self.set_location_layout_id(location.storage, layout_id)
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
        let store = self.store.read();

        store.large.entries.get(index).cloned()
    }

    /// Allocate one shared heap location for the given payload.
    fn allocate_location(
        &self,
        store: &mut SharedHeapState,
        byte_len: usize,
        scan: impl Into<TracePlan>,
        layout_id: Option<LayoutId>,
        bytes: Option<&[u8]>,
        path: SpanAllocationPath,
    ) -> HeapResult<SharedHeapStorage> {
        let scan = scan.into();

        match path {
            // execute the precomputed small path directly
            SpanAllocationPath::Small {
                class_index,
                size_class,
                ..
            } => {
                let slot = self.allocate_small(
                    store,
                    byte_len,
                    class_index,
                    size_class,
                    scan,
                    layout_id,
                    bytes,
                )?;

                Ok(SharedHeapStorage::Small(slot))
            }

            // execute the precomputed large path directly
            SpanAllocationPath::Large { .. } => {
                let pages = match bytes {
                    Some(bytes) => store
                        .page_run_cache
                        .allocate_bytes(&self.allocator, bytes)?,
                    None => store
                        .page_run_cache
                        .allocate_zeroed(&self.allocator, byte_len)?,
                };
                let entry_id = self.store_large_entry(store, byte_len, pages, scan, layout_id)?;

                Ok(SharedHeapStorage::Large(entry_id))
            }
        }
    }

    /// Return whether one size class still has one live reusable slot.
    fn has_available_small_slot(&self, store: &SharedHeapState, class_index: usize) -> bool {
        !store.small.available_spans[class_index].is_empty()
    }

    /// Allocate or reuse one non-full shared heap span for the given size class.
    fn allocate_small_span(
        &self,
        store: &mut SharedHeapState,
        class_index: usize,
        size_class: usize,
    ) -> HeapResult<usize> {
        while let Some(span_index) = store.small.available_spans[class_index].pop() {
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

        let slot_count = (store.small.span_bytes / size_class).max(1);
        let pages = store
            .page_run_cache
            .allocate_zeroed(&self.allocator, store.small.span_bytes)?;
        let span = SharedSmallSpan {
            size_class,
            slot_count,
            occupied_count: 0,
            next_free_slot: 0,
            occupied: crate::Bitmap::with_capacity(slot_count),
            marked: crate::Bitmap::with_capacity(slot_count),
            lengths: vec![0; slot_count].into_boxed_slice(),
            shape_ids: vec![None; slot_count].into_boxed_slice(),
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
        byte_len: usize,
        class_index: usize,
        size_class: usize,
        trace: TracePlan,
        layout_id: Option<LayoutId>,
        bytes: Option<&[u8]>,
    ) -> HeapResult<SpanSlot> {
        let shape_id = store.shape_table.intern(Shape { trace, layout_id })?;
        let span_index = self.allocate_small_span(store, class_index, size_class)?;
        let Some(span) = store.small.spans.get(span_index).cloned() else {
            return Err(HeapError::MissingSpan { span_index });
        };
        let mut span = span.write();
        let slot_index = span.next_free_slot;

        // slot initialization
        let slot = SpanSlot::new(span_index, slot_index)?;
        let slot_offset = checked_slot_offset(span.size_class, slot_index)?;

        if let Some(bytes) = bytes {
            let write_offset = checked_storage_offset(slot_offset, 0, store.small.span_bytes)?;

            self.allocator
                .set_bytes(&mut span.pages, write_offset, bytes)?;
        }

        // slot metadata
        span.occupied.set(slot_index);
        span.marked.clear(slot_index);
        span.occupied_count += 1;
        span.next_free_slot = find_next_free_slot(&span.occupied, slot_index + 1, span.slot_count);
        span.set_length(slot_index, byte_len);
        span.set_shape_id(slot_index, Some(shape_id));

        if span.occupied_count < span.slot_count {
            store.small.available_spans[class_index].push(span_index);
        }

        Ok(slot)
    }

    /// Store one large entry in shared heap large space and return its entry id.
    fn store_large_entry(
        &self,
        store: &mut SharedHeapState,
        len: usize,
        pages: PageView,
        trace: TracePlan,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<SharedLargeEntryId> {
        let shape_id = store.shape_table.intern(Shape { trace, layout_id })?;
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
            shape_id,
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
            let size_class = span.size_class;

            if !span.occupied.contains(slot_index) || span.occupied_count == 0 {
                return Err(HeapError::MissingSmallSlot {
                    span_index: slot.span_index(),
                    slot_index,
                });
            }

            span.occupied.clear(slot_index);
            span.marked.clear(slot_index);
            span.occupied_count -= 1;
            span.next_free_slot = span.next_free_slot.min(slot_index);
            span.set_length(slot_index, 0);
            span.set_shape_id(slot_index, None);

            if span.occupied_count == 0 {
                span.next_free_slot = 0;

                let pages = span.pages.clone();
                span.pages = PageView::empty();

                Some(pages)
            } else {
                let should_requeue = was_full && span.occupied_count < span.slot_count;

                if should_requeue
                    && let Some(class_index) = store.small.size_classes.class_index_for(size_class)
                {
                    store.small.available_spans[class_index].push(slot.span_index());
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

    /// Return the scan metadata for one shared heap location.
    fn location_scan(&self, storage: SharedHeapStorage) -> HeapResult<TracePlan> {
        let shape = self.location_shape(storage)?;

        Ok(shape.trace)
    }

    /// Return the storage layout id for one shared heap location.
    fn location_layout_id(&self, storage: SharedHeapStorage) -> HeapResult<Option<LayoutId>> {
        let shape = self.location_shape(storage)?;

        Ok(shape.layout_id)
    }

    /// Set the storage layout id for one shared heap location.
    fn set_location_layout_id(
        &self,
        storage: SharedHeapStorage,
        layout_id: LayoutId,
    ) -> HeapResult<()> {
        let mut store = self.store.write();

        match storage {
            SharedHeapStorage::Small(slot) => {
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let shape_id = span
                    .read()
                    .shape_ids
                    .get(slot.slot_index())
                    .copied()
                    .flatten()
                    .ok_or(HeapError::MissingSmallSlot {
                        span_index: slot.span_index(),
                        slot_index: slot.slot_index(),
                    })?;
                let shape_id = self.shape_with_layout(&mut store, shape_id, Some(layout_id))?;
                let mut span = span.write();

                span.set_shape_id(slot.slot_index(), Some(shape_id));

                Ok(())
            }
            SharedHeapStorage::Large(entry_id) => {
                let Some(entry) = store.large.entries.get(entry_id.index()?).cloned() else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };
                let shape_id = entry.read().shape_id;
                let shape_id = self.shape_with_layout(&mut store, shape_id, Some(layout_id))?;
                let mut entry = entry.write();

                if !entry.is_live {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                }

                entry.shape_id = shape_id;

                Ok(())
            }
        }
    }

    /// Return one interned shape with one replacement layout id.
    fn shape_with_layout(
        &self,
        store: &mut SharedHeapState,
        shape_id: ShapeId,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<ShapeId> {
        let shape =
            store
                .shape_table
                .shape(shape_id)
                .cloned()
                .ok_or(HeapError::InvalidShapeId {
                    index: shape_id.index(),
                })?;

        store.shape_table.intern(Shape {
            trace: shape.trace,
            layout_id,
        })
    }

    /// Return the entry shape for one live shared heap location.
    fn location_shape(&self, storage: SharedHeapStorage) -> HeapResult<Shape> {
        let store = self.store.read();
        let shape_id = match storage {
            SharedHeapStorage::Small(slot) => {
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span = span.read();
                let Some(shape_id) = span.shape_ids.get(slot.slot_index()).copied().flatten()
                else {
                    return Err(HeapError::MissingSmallSlot {
                        span_index: slot.span_index(),
                        slot_index: slot.slot_index(),
                    });
                };

                shape_id
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

                entry.shape_id
            }
        };

        store
            .shape_table
            .shape(shape_id)
            .cloned()
            .ok_or(HeapError::InvalidShapeId {
                index: shape_id.index(),
            })
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
                let store = self.store.read();
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span = span.read();

                let slot_offset = checked_slot_offset(span.size_class, slot.slot_index())?;
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
                let store = self.store.read();
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span = span.read();

                let slot_offset = checked_slot_offset(span.size_class, slot.slot_index())?;
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
                let store = self.store.read();
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
                    let slot_offset = checked_slot_offset(span.size_class, slot.slot_index())?;
                    let write_offset =
                        checked_storage_offset(slot_offset, byte_offset, span_bytes)?;

                    self.allocator
                        .set_bytes(&mut span.pages, write_offset, bytes)?;

                    (previous_pages, span.pages.clone())
                };

                if next_pages != previous_pages {
                    let mut store = self.store.write();

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

                    let mut store = self.store.write();

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
        let store = self.store.read();
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
                let slot_index = logical_byte_offset / span.size_class;
                let slot_offset = logical_byte_offset % span.size_class;
                if slot_index >= span.slot_count || !span.occupied.contains(slot_index) {
                    return None;
                }

                let byte_len = *span.lengths.get(slot_index)?;
                if slot_offset >= byte_len {
                    return None;
                }

                let slot_base_offset = slot_index.checked_mul(span.size_class)?;
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

    /// Return every live shared heap reference.
    pub(crate) fn live_references(&self) -> HeapResult<Vec<SharedHeapReference>> {
        let store = self.store.read();
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

                let slot_offset = span.size_class.checked_mul(slot_index).ok_or(
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
                let slot_offset = span.size_class.checked_mul(slot.slot_index()).ok_or(
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
pub(crate) fn find_next_free_slot(
    occupied: &crate::Bitmap,
    start: usize,
    slot_count: usize,
) -> usize {
    for slot_index in start..slot_count {
        if !occupied.contains(slot_index) {
            return slot_index;
        }
    }

    slot_count
}

impl Drop for SharedHeapSpace {
    fn drop(&mut self) {
        let mut store = self.store.write();

        store.page_run_cache.flush(&self.allocator);
    }
}
