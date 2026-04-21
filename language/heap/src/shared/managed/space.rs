use std::sync::Arc;

use parking_lot::RwLock;

use super::{
    SharedLargeEntry, SharedLargeEntryId, SharedManagedLocation, SharedManagedReferenceEntry,
    SharedSmallSpan,
};
use crate::arena::{Arena, PageRunCache, PageView, SizeClassTable, SpanAllocationPath, SpanSlot};
use crate::shared::gc::{SharedGcPhase, SharedGcState};
use crate::{
    AllocationUsage, GcSummary, HeapError, HeapOptions, HeapResult, HeapScan, HeapSpace, LayoutId,
    Shape, ShapeId, ShapeTable, SharedManagedReference, SharedManagedSpaceUsage,
};

/// The first allocated shared managed reference id.
const FIRST_SHARED_MANAGED_REFERENCE_ID: u64 = 1;

/// The first allocated shared managed large-entry id.
const FIRST_SHARED_MANAGED_LARGE_ENTRY_ID: u64 = 1;

/// One shared managed small space.
#[derive(Debug)]
pub(crate) struct SharedSmallSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span width.
    pub(crate) span_bytes: usize,
    /// The live shared managed spans.
    pub(crate) spans: Vec<Arc<RwLock<SharedSmallSpan>>>,
    /// The reusable non-full spans per size class.
    pub(crate) available_spans: Vec<Vec<usize>>,
}

/// One shared managed large space.
#[derive(Debug)]
pub(crate) struct SharedLargeSpace {
    /// The configured page width for entries in large space.
    pub(crate) page_bytes: usize,
    /// The live shared managed entries.
    pub(crate) entries: Vec<Arc<RwLock<SharedLargeEntry>>>,
    /// The free shared managed entry ids available for reuse.
    pub(crate) free_large_entry_ids: Vec<u64>,
    /// The next shared managed entry id to allocate.
    pub(crate) next_unused_large_entry_id: u64,
}

/// Shared managed allocator metadata.
#[derive(Debug)]
pub(crate) struct SharedManagedState {
    /// The shared front-end cache of reusable page runs.
    pub(crate) page_run_cache: PageRunCache,
    /// The shared managed small space.
    pub(crate) small: SharedSmallSpace,
    /// The shared managed large space.
    pub(crate) large: SharedLargeSpace,
    /// Stable shared managed reference metadata keyed by reference id minus one.
    pub(crate) references: Vec<SharedManagedReferenceEntry>,
    /// The interned shared managed entry shapes.
    pub(crate) shape_table: ShapeTable,
    /// Free shared managed reference ids available for reuse.
    pub(crate) free_reference_ids: Vec<u64>,
    /// The next shared managed reference id to allocate.
    pub(crate) next_unused_reference_id: u64,
    /// The exact live shared managed-space usage.
    pub(crate) usage: AllocationUsage,
    /// The live shared managed collector state.
    pub(crate) gc_state: GcSummary,
}

/// Convert a stable shared managed reference id into its packed representation.
fn checked_packed_reference_id(reference_id: u64) -> HeapResult<u32> {
    if reference_id == 0 || reference_id > u32::MAX as u64 {
        return Err(HeapError::InvalidSharedManagedReferenceId { id: reference_id });
    }

    Ok(reference_id as u32)
}

/// One shared managed space store over a shared arena.
#[derive(Debug)]
pub struct SharedManagedSpace {
    /// The shared managed-space arena for every entry.
    pub(crate) arena: Arc<Arena>,
    /// The shared managed allocator control plane.
    pub(crate) store: RwLock<SharedManagedState>,
    /// The active shared collection state.
    pub(crate) gc: SharedGcState,
}

impl Default for SharedManagedSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedManagedSpace {
    /// Create a new empty shared managed-space store.
    pub fn new() -> Self {
        Self::with_options(Arc::new(Arena::new()), &HeapOptions::shared())
    }

    /// Create a new empty shared managed-space store over one shared arena.
    pub fn with_arena(arena: Arc<Arena>) -> Self {
        let options = HeapOptions {
            page_bytes: arena.page_bytes(),
            arena_segment_bytes: arena.segment_bytes(),
            ..HeapOptions::shared()
        };

        Self::with_options(arena, &options)
    }

    /// Create a new empty shared managed-space store over one shared arena and options.
    pub fn with_options(arena: Arc<Arena>, options: &HeapOptions) -> Self {
        options
            .validate_shared()
            .expect("shared managed options should validate");
        options
            .validate_arena(&arena)
            .expect("shared managed arena should match options");

        let store = SharedManagedState {
            page_run_cache: PageRunCache::new(arena.pages_per_segment()),
            small: SharedSmallSpace {
                size_classes: options.size_classes.clone(),
                span_bytes: options.managed_small_bytes,
                spans: Vec::new(),
                available_spans: vec![Vec::new(); options.size_classes.classes.len()],
            },
            large: SharedLargeSpace {
                page_bytes: arena.page_bytes(),
                entries: Vec::new(),
                free_large_entry_ids: Vec::new(),
                next_unused_large_entry_id: FIRST_SHARED_MANAGED_LARGE_ENTRY_ID,
            },
            references: Vec::new(),
            shape_table: ShapeTable::new(),
            free_reference_ids: Vec::new(),
            next_unused_reference_id: FIRST_SHARED_MANAGED_REFERENCE_ID,
            usage: AllocationUsage::default(),
            gc_state: GcSummary::default(),
        };

        Self {
            arena,
            store: RwLock::new(store),
            gc: SharedGcState::default(),
        }
    }

    /// Return the configured shared page width.
    pub fn page_bytes(&self) -> usize {
        self.arena.page_bytes()
    }

    /// Return the exact active shared managed-space bytes.
    pub fn active_bytes(&self) -> u64 {
        self.mapped_bytes()
    }

    /// Return the exact mapped shared page bytes.
    pub fn mapped_bytes(&self) -> u64 {
        let store = self.store.read();

        self.live_mapped_bytes(&store)
    }

    /// Return the exact borrowed shared image bytes.
    pub fn borrowed_bytes(&self) -> HeapResult<u64> {
        let store = self.store.read();
        let pages = self.live_pages(&store);

        self.arena.borrowed_bytes_for_page_views(pages.iter())
    }

    /// Return the exact usage for this live shared managed-space store.
    pub fn usage(&self) -> HeapResult<SharedManagedSpaceUsage> {
        // allocator summary
        let store = self.store.read();
        let allocation_count = store.usage.allocation_count();
        let allocated_bytes = store.usage.allocated_bytes();
        let mapped_bytes = self.live_mapped_bytes(&store);
        let active_bytes = mapped_bytes;

        drop(store);

        // borrowed bytes
        let borrowed_bytes = self.borrowed_bytes()?;

        Ok(SharedManagedSpaceUsage {
            allocation_count,
            allocated_bytes,
            active_bytes,
            mapped_bytes,
            borrowed_bytes,
        })
    }

    /// Return the number of live shared managed allocations.
    pub fn allocation_count(&self) -> usize {
        let store = self.store.read();

        store.usage.allocation_count()
    }

    /// Return the number of live shared managed bytes.
    pub fn allocated_bytes(&self) -> u64 {
        let store = self.store.read();

        store.usage.allocated_bytes()
    }

    /// Return the current shared managed collector state.
    pub fn gc_state(&self) -> GcSummary {
        let store = self.store.read();

        store.gc_state.clone()
    }

    /// Return the current shared managed collector phase.
    pub fn gc_phase(&self) -> SharedGcPhase {
        self.gc.phase()
    }

    /// Return whether one shared managed reference currently refers to one live entry.
    pub fn is_live(&self, reference: SharedManagedReference) -> bool {
        self.reference_entry(reference).is_some()
    }

    /// Allocate one shared managed byte entry.
    pub fn allocate_bytes(
        &self,
        bytes: &[u8],
        scan: impl Into<HeapScan>,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<SharedManagedReference> {
        let path = self.allocation_path(bytes.len());

        self.place_bytes(bytes, scan, layout_id, path)
    }

    /// Allocate one zeroed shared managed-space entry.
    pub fn allocate_zeroed(
        &self,
        byte_len: usize,
        scan: impl Into<HeapScan>,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<SharedManagedReference> {
        let path = self.allocation_path(byte_len);

        self.place_zeroed(byte_len, scan, layout_id, path)
    }

    /// Allocate one shared managed byte entry through one precomputed allocation path.
    pub(crate) fn place_bytes(
        &self,
        bytes: &[u8],
        scan: impl Into<HeapScan>,
        layout_id: Option<LayoutId>,
        path: SpanAllocationPath,
    ) -> HeapResult<SharedManagedReference> {
        let byte_len = bytes.len();
        let scan = scan.into();
        let mut store = self.store.write();
        let reference_id = self.allocate_reference_id(&mut store)?;
        let reference = SharedManagedReference::new(reference_id);

        // allocate the backing location before installing the live reference
        let location = self.allocate_location(
            &mut store,
            reference_id,
            byte_len,
            scan.clone(),
            layout_id,
            Some(bytes),
            path,
        )?;

        let index = shared_reference_index(reference_id)?;
        let record = SharedManagedReferenceEntry::new(location, byte_len);
        if index > store.references.len() {
            return Err(HeapError::InvalidSharedManagedReferenceId {
                id: reference_id.into(),
            });
        }

        if index == store.references.len() {
            store.references.push(record);
        } else {
            store.references[index] = record;
        }

        store.usage.allocate(byte_len, HeapSpace::SharedManaged)?;
        drop(store);

        self.publish_shared_allocation(reference, scan.has_shared_reference())?;

        Ok(reference)
    }

    /// Allocate one zeroed shared managed entry through one precomputed allocation path.
    pub(crate) fn place_zeroed(
        &self,
        byte_len: usize,
        scan: impl Into<HeapScan>,
        layout_id: Option<LayoutId>,
        path: SpanAllocationPath,
    ) -> HeapResult<SharedManagedReference> {
        let scan = scan.into();
        let mut store = self.store.write();
        let reference_id = self.allocate_reference_id(&mut store)?;
        let reference = SharedManagedReference::new(reference_id);

        // allocate the zeroed backing location before installing the live reference
        let location = self.allocate_location(
            &mut store,
            reference_id,
            byte_len,
            scan,
            layout_id,
            None,
            path,
        )?;

        let index = shared_reference_index(reference_id)?;
        let record = SharedManagedReferenceEntry::new(location, byte_len);

        if index > store.references.len() {
            return Err(HeapError::InvalidSharedManagedReferenceId {
                id: reference_id.into(),
            });
        }

        if index == store.references.len() {
            store.references.push(record);
        } else {
            store.references[index] = record;
        }

        store.usage.allocate(byte_len, HeapSpace::SharedManaged)?;
        drop(store);

        self.publish_shared_allocation(reference, false)?;

        Ok(reference)
    }

    /// Return one exact shared managed allocation path for the requested byte length.
    pub(crate) fn allocation_path(&self, byte_len: usize) -> SpanAllocationPath {
        let store = self.store.read();

        store.small.size_classes.span_allocation_path(
            byte_len,
            store.small.span_bytes,
            |class_index| self.has_available_small_slot(&store, class_index),
            |large_bytes| self.round_up_allocation_bytes(large_bytes),
        )
    }

    /// Return the remaining byte length for one shared managed reference.
    pub fn byte_len(&self, reference: SharedManagedReference) -> HeapResult<usize> {
        let Some(record) = self.reference_entry(reference) else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };

        checked_remaining_byte_len(reference, record.byte_len())
    }

    /// Return the bytes for one shared managed reference.
    pub fn read_bytes(&self, reference: SharedManagedReference) -> HeapResult<Vec<u8>> {
        let Some(record) = self.reference_entry(reference) else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };

        let byte_offset = reference.byte_offset();
        let byte_len = checked_remaining_byte_len(reference, record.byte_len())?;

        self.location_bytes(location, byte_offset, byte_len)
    }

    /// Fill one caller-provided buffer from one shared managed entry at one offset.
    pub fn read_bytes_into(
        &self,
        reference: SharedManagedReference,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        let (location, byte_offset) =
            self.checked_location_range(reference, start, target.len())?;

        self.fill_location_bytes(location, byte_offset, target)
    }

    /// Return the scan metadata for one shared managed reference.
    pub fn scan(&self, reference: SharedManagedReference) -> HeapResult<HeapScan> {
        let (location, _) = self.checked_location_range(reference, 0, 0)?;

        self.location_scan(location)
    }

    /// Return the storage layout id for one shared managed reference.
    pub fn layout_id(&self, reference: SharedManagedReference) -> HeapResult<Option<LayoutId>> {
        let Some(record) = self.reference_entry(reference) else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };

        self.location_layout_id(location)
    }

    /// Set the storage layout id for one shared managed reference.
    pub fn set_layout_id(
        &self,
        reference: SharedManagedReference,
        layout_id: LayoutId,
    ) -> HeapResult<()> {
        let Some(record) = self.reference_entry(reference) else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };

        self.set_location_layout_id(location, layout_id)
    }

    /// Overwrite one shared managed byte range.
    pub fn write_bytes(
        &self,
        reference: SharedManagedReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let (location, byte_offset) = self.checked_location_range(reference, start, bytes.len())?;
        self.write_location_bytes(location, byte_offset, bytes)
    }

    /// Record one shared managed write barrier.
    pub fn write_barrier(
        &self,
        reference: SharedManagedReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        let (_, byte_offset) = self.checked_location_range(reference, start, byte_len)?;
        self.write_shared_barrier(reference, byte_offset, byte_len)
    }

    /// Publish one exact shared managed edge after one completed store.
    pub fn publish_edge(&self, reference: SharedManagedReference) -> HeapResult<()> {
        self.publish_shared_edge(reference)
    }

    /// Return one checked live location and byte offset for one shared managed range.
    fn checked_location_range(
        &self,
        reference: SharedManagedReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<(SharedManagedLocation, usize)> {
        let Some(record) = self.reference_entry(reference) else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };

        let byte_offset =
            checked_byte_range(reference.byte_offset(), start, byte_len, record.byte_len())?;

        Ok((location, byte_offset))
    }

    /// Return one allocated shared managed entry by reference.
    pub(crate) fn reference_entry(
        &self,
        reference: SharedManagedReference,
    ) -> Option<SharedManagedReferenceEntry> {
        let index = reference.id().checked_sub(1)? as usize;
        let store = self.store.read();
        let record = store.references.get(index).copied()?;

        (!record.is_vacant()).then_some(record)
    }

    /// Return one shared managed large entry by stable id.
    pub(crate) fn large_entry_ref(
        &self,
        entry_id: SharedLargeEntryId,
    ) -> Option<Arc<RwLock<SharedLargeEntry>>> {
        let index = entry_id.index().ok()?;
        let store = self.store.read();

        store.large.entries.get(index).cloned()
    }

    /// Allocate one shared managed location for the given payload.
    fn allocate_location(
        &self,
        store: &mut SharedManagedState,
        reference_id: u32,
        byte_len: usize,
        scan: impl Into<HeapScan>,
        layout_id: Option<LayoutId>,
        bytes: Option<&[u8]>,
        path: SpanAllocationPath,
    ) -> HeapResult<SharedManagedLocation> {
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
                    reference_id,
                    class_index,
                    size_class,
                    scan,
                    layout_id,
                    bytes,
                )?;

                Ok(SharedManagedLocation::Small(slot))
            }

            // execute the precomputed large path directly
            SpanAllocationPath::Large { .. } => {
                let pages = match bytes {
                    Some(bytes) => store.page_run_cache.allocate_bytes(&self.arena, bytes)?,
                    None => store
                        .page_run_cache
                        .allocate_zeroed(&self.arena, byte_len)?,
                };
                let entry_id = self.store_large_entry(store, byte_len, pages, scan, layout_id)?;

                Ok(SharedManagedLocation::Large(entry_id))
            }
        }
    }

    /// Allocate one shared managed reference id from the intrusive free list or the unused tail.
    fn allocate_reference_id(&self, store: &mut SharedManagedState) -> HeapResult<u32> {
        if let Some(reference_id) = store.free_reference_ids.pop() {
            checked_packed_reference_id(reference_id)
        } else {
            let reference_id = store.next_unused_reference_id;
            let reference_id = checked_packed_reference_id(reference_id)?;
            store.next_unused_reference_id = store.next_unused_reference_id.checked_add(1).ok_or(
                HeapError::InvalidSharedManagedReferenceId {
                    id: store.next_unused_reference_id,
                },
            )?;

            Ok(reference_id)
        }
    }

    /// Retire one shared managed reference record and queue its id for reuse.
    pub(crate) fn retire_reference(
        &self,
        store: &mut SharedManagedState,
        reference_id: u32,
    ) -> HeapResult<()> {
        let index = shared_reference_index(reference_id)?;
        let Some(record) = store.references.get_mut(index) else {
            return Err(HeapError::InvalidSharedManagedReferenceId {
                id: reference_id.into(),
            });
        };

        *record = SharedManagedReferenceEntry::vacant();
        store.free_reference_ids.push(reference_id.into());

        Ok(())
    }

    /// Return whether one size class still has one live reusable slot.
    fn has_available_small_slot(&self, store: &SharedManagedState, class_index: usize) -> bool {
        !store.small.available_spans[class_index].is_empty()
    }

    /// Allocate or reuse one non-full shared managed span for the given size class.
    fn allocate_small_span(
        &self,
        store: &mut SharedManagedState,
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
                        .allocate_zeroed(&self.arena, store.small.span_bytes)?;
                    span.pages = pages;
                }

                return Ok(span_index);
            }
        }

        let slot_count = (store.small.span_bytes / size_class).max(1);
        let pages = store
            .page_run_cache
            .allocate_zeroed(&self.arena, store.small.span_bytes)?;
        let span = SharedSmallSpan {
            size_class,
            slot_count,
            occupied_count: 0,
            next_free_slot: 0,
            occupied: crate::Bitmap::with_capacity(slot_count),
            reference_ids: vec![None; slot_count].into_boxed_slice(),
            shape_ids: vec![None; slot_count].into_boxed_slice(),
            pages,
        };
        let span_index = store.small.spans.len();

        store.small.spans.push(Arc::new(RwLock::new(span)));

        Ok(span_index)
    }

    /// Allocate one shared managed small slot from one explicit initialization source.
    fn allocate_small(
        &self,
        store: &mut SharedManagedState,
        reference_id: u32,
        class_index: usize,
        size_class: usize,
        scan: HeapScan,
        layout_id: Option<LayoutId>,
        bytes: Option<&[u8]>,
    ) -> HeapResult<SpanSlot> {
        let shape_id = store.shape_table.intern(Shape { scan, layout_id })?;
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

            self.arena.set_bytes(&mut span.pages, write_offset, bytes)?;
        }

        // slot metadata
        span.occupied.set(slot_index);
        span.occupied_count += 1;
        span.next_free_slot = find_next_free_slot(&span.occupied, slot_index + 1, span.slot_count);
        span.set_reference_id(slot_index, Some(reference_id));
        span.set_shape_id(slot_index, Some(shape_id));

        if span.occupied_count < span.slot_count {
            store.small.available_spans[class_index].push(span_index);
        }

        Ok(slot)
    }

    /// Store one large entry in shared managed large space and return its stable id.
    fn store_large_entry(
        &self,
        store: &mut SharedManagedState,
        len: usize,
        pages: PageView,
        scan: HeapScan,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<SharedLargeEntryId> {
        let shape_id = store.shape_table.intern(Shape { scan, layout_id })?;
        let (entry_id, reused_entry_id) = if let Some(entry_id) =
            store.large.free_large_entry_ids.pop()
        {
            (entry_id, true)
        } else {
            let entry_id = store.large.next_unused_large_entry_id;
            let Some(next_entry_id) = store.large.next_unused_large_entry_id.checked_add(1) else {
                store.page_run_cache.release_page_view(&self.arena, pages)?;

                return Err(HeapError::InvalidLargeEntryId { id: entry_id });
            };

            store.large.next_unused_large_entry_id = next_entry_id;
            (entry_id, false)
        };

        if entry_id == 0 {
            if reused_entry_id {
                store.large.free_large_entry_ids.push(entry_id);
            }

            store.page_run_cache.release_page_view(&self.arena, pages)?;

            return Err(HeapError::InvalidLargeEntryId { id: entry_id });
        }

        let entry = SharedLargeEntry {
            is_live: true,
            len,
            pages,
            shape_id,
        };
        let index = SharedLargeEntryId::new(entry_id).index()?;

        if index > store.large.entries.len() {
            if reused_entry_id {
                store.large.free_large_entry_ids.push(entry_id);
            }

            store.page_run_cache.release_page_view(&self.arena, pages)?;

            return Err(HeapError::InvalidLargeEntryId { id: entry_id });
        }

        let entry = Arc::new(RwLock::new(entry));

        if index == store.large.entries.len() {
            store.large.entries.push(entry);
        } else {
            store.large.entries[index] = entry;
        }

        Ok(SharedLargeEntryId::new(entry_id))
    }

    /// Release one shared managed small slot without retiring its stable reference id.
    pub(crate) fn release_small_slot(
        &self,
        store: &mut SharedManagedState,
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
            span.occupied_count -= 1;
            span.next_free_slot = span.next_free_slot.min(slot_index);
            span.set_reference_id(slot_index, None);
            span.set_shape_id(slot_index, None);

            if span.occupied_count == 0 {
                span.next_free_slot = 0;

                let pages = span.pages;
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
            store.page_run_cache.release_page_view(&self.arena, pages)?;
        }

        Ok(())
    }

    /// Return the scan metadata for one shared managed location.
    fn location_scan(&self, location: SharedManagedLocation) -> HeapResult<HeapScan> {
        let shape = self.location_shape(location)?;

        Ok(shape.scan)
    }

    /// Return the storage layout id for one shared managed location.
    fn location_layout_id(&self, location: SharedManagedLocation) -> HeapResult<Option<LayoutId>> {
        let shape = self.location_shape(location)?;

        Ok(shape.layout_id)
    }

    /// Set the storage layout id for one shared managed location.
    fn set_location_layout_id(
        &self,
        location: SharedManagedLocation,
        layout_id: LayoutId,
    ) -> HeapResult<()> {
        let mut store = self.store.write();

        match location {
            SharedManagedLocation::Small(slot) => {
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
            SharedManagedLocation::Large(entry_id) => {
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
        store: &mut SharedManagedState,
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
            scan: shape.scan,
            layout_id,
        })
    }

    /// Return the entry shape for one live shared managed location.
    fn location_shape(&self, location: SharedManagedLocation) -> HeapResult<Shape> {
        let store = self.store.read();
        let shape_id = match location {
            SharedManagedLocation::Small(slot) => {
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
            SharedManagedLocation::Large(entry_id) => {
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

    /// Return the bytes for one shared managed location as one owned vector.
    fn location_bytes(
        &self,
        location: SharedManagedLocation,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<Vec<u8>> {
        match location {
            SharedManagedLocation::Small(slot) => {
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

                self.arena
                    .bytes_to_vec_from(&span.pages, read_offset, byte_len)
            }
            SharedManagedLocation::Large(entry_id) => {
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

                self.arena
                    .bytes_to_vec_from(&entry.pages, byte_offset, byte_len)
            }
        }
    }

    /// Fill one caller-provided buffer from one shared managed location.
    fn fill_location_bytes(
        &self,
        location: SharedManagedLocation,
        byte_offset: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        match location {
            SharedManagedLocation::Small(slot) => {
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

                self.arena.fill_bytes_from(&span.pages, read_offset, target)
            }
            SharedManagedLocation::Large(entry_id) => {
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

                self.arena
                    .fill_bytes_from(&entry.pages, byte_offset, target)
            }
        }
    }

    /// Overwrite one byte range for one shared managed location.
    fn write_location_bytes(
        &self,
        location: SharedManagedLocation,
        byte_offset: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        match location {
            SharedManagedLocation::Small(slot) => {
                let store = self.store.read();
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span_bytes = store.small.span_bytes;
                drop(store);

                let mut span = span.write();
                let slot_offset = checked_slot_offset(span.size_class, slot.slot_index())?;
                let write_offset = checked_storage_offset(slot_offset, byte_offset, span_bytes)?;

                self.arena.set_bytes(&mut span.pages, write_offset, bytes)
            }
            SharedManagedLocation::Large(entry_id) => {
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

                self.arena.set_bytes(&mut entry.pages, byte_offset, bytes)
            }
        }
    }

    /// Return the current live managed page views.
    fn live_pages(&self, store: &SharedManagedState) -> Vec<PageView> {
        let mut pages = Vec::with_capacity(store.small.spans.len() + store.large.entries.len());

        for span in &store.small.spans {
            let span = span.read();

            if span.occupied_count == 0 || span.pages.is_empty() {
                continue;
            }

            pages.push(span.pages);
        }

        for entry in &store.large.entries {
            let entry = entry.read();

            if !entry.is_live {
                continue;
            }

            pages.push(entry.pages);
        }

        pages
    }

    /// Return the mapped live bytes for the current shared managed state.
    pub(crate) fn live_mapped_bytes(&self, store: &SharedManagedState) -> u64 {
        let pages = self.live_pages(store);
        let cached_bytes = store.page_run_cache.cached_bytes(self.arena.page_bytes());

        self.arena
            .mapped_bytes_for_page_views(pages.iter())
            .saturating_add(cached_bytes)
    }

    /// Return the page-rounded mapped bytes for one shared managed-space entry.
    fn round_up_allocation_bytes(&self, byte_len: usize) -> u64 {
        let page_bytes = self.page_bytes() as u64;
        let byte_len = byte_len as u64;

        byte_len.div_ceil(page_bytes) * page_bytes
    }
}

/// Return the dense shared managed reference table index for one stable id.
fn shared_reference_index(reference_id: u32) -> HeapResult<usize> {
    let Some(index) = reference_id.checked_sub(1) else {
        return Err(HeapError::InvalidSharedManagedReferenceId {
            id: reference_id.into(),
        });
    };

    Ok(index as usize)
}

/// Return the visible byte length for one shared managed reference.
fn checked_remaining_byte_len(
    reference: SharedManagedReference,
    byte_len: usize,
) -> HeapResult<usize> {
    let byte_offset = reference.byte_offset();

    if byte_offset > byte_len {
        return Err(HeapError::InvalidSharedManagedReference { reference });
    }

    Ok(byte_len - byte_offset)
}

/// Validate one byte range inside one shared managed entry.
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

impl Drop for SharedManagedSpace {
    fn drop(&mut self) {
        let mut store = self.store.write();

        store.page_run_cache.flush(&self.arena);
    }
}
