use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use destack_mir::ReferenceMap;
use parking_lot::RwLock;

use super::{
    SharedHeapLocation, SharedHeapPageMapEntry, SharedHeapPlace, SharedLargeAllocation,
    SharedLargeAllocationId, SharedSmallSpan, SpanList,
};
use crate::allocator::{AddressSpace, Allocator, PageRun, PageRunCache, SizeClassTable, SpanSlot};
use crate::shared::gc::{SharedGcPhase, SharedGcState};
use crate::{
    AllocationLayout, GcState, HeapError, HeapOptions, HeapResult, Payload, SharedHeapReference,
    SharedHeapSpaceUsage, SmallSpanClass, clear_slot_reference_bits, slot_reference_map,
    write_slot_reference_bits,
};

/// The first allocated shared heap large-allocation id.
const FIRST_SHARED_MANAGED_LARGE_ALLOCATION_ID: u64 = 1;

/// One shared heap small space.
#[derive(Debug)]
pub(crate) struct SharedSmallSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span size.
    pub(crate) span_bytes: usize,
    /// The live shared heap spans.
    pub(crate) spans: Vec<Arc<RwLock<SharedSmallSpan>>>,
    /// The reusable non-full spans per size and scan class.
    pub(crate) partial_spans: Vec<Vec<usize>>,
}

/// One shared heap large space.
#[derive(Debug)]
pub(crate) struct SharedLargeSpace {
    /// The configured page size for allocations in large space.
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
    /// The shared front-end cache of reusable page runs.
    pub(crate) page_run_cache: PageRunCache,
    /// The shared heap small space.
    pub(crate) small: SharedSmallSpace,
    /// The shared heap large space.
    pub(crate) large: SharedLargeSpace,
    /// The owning shared heap metadata for each visible allocator page.
    pub(crate) page_map: Vec<Option<SharedHeapPageMapEntry>>,
    /// The next unused byte offset in shared heap space.
    pub(crate) next_offset: usize,
    /// The fixed live byte mapping for shared heap space.
    pub(crate) mapping: AddressSpace,
    /// The live shared heap collector state.
    pub(crate) gc: GcState,
}

/// One shared heap allocation front end.
#[derive(Debug)]
pub struct SharedAllocator {
    /// The immutable size-class table for this allocator front end.
    size_classes: SizeClassTable,
    /// The immutable shared span byte width.
    span_bytes: usize,
    /// The immutable allocator page byte width.
    page_bytes: usize,
    /// The current cached span per small-span bucket.
    cached_spans: Vec<Option<CachedSmallSpan>>,
}

/// One cached shared small span.
#[derive(Debug)]
struct CachedSmallSpan {
    /// The shared span table index.
    span_index: usize,
    /// The shared span itself.
    span: Arc<RwLock<SharedSmallSpan>>,
}

/// Atomic shared heap-space usage.
#[derive(Debug)]
pub(crate) struct SharedUsage {
    /// The number of live allocations.
    allocation_count: AtomicUsize,
    /// The number of live allocated bytes.
    allocated_bytes: AtomicU64,
}

/// One shared heap space store over a shared allocator.
#[derive(Debug)]
pub struct SharedHeapSpace {
    /// The shared heap-space allocator for every allocation.
    pub(crate) allocator: Arc<Allocator>,
    /// The shared heap allocator state.
    pub(crate) state: RwLock<SharedHeapState>,
    /// The exact live shared heap-space usage.
    pub(crate) usage: SharedUsage,
    /// The active shared collection state.
    pub(crate) gc: SharedGcState,
}

impl SharedAllocator {
    /// Create one empty shared allocator front end.
    fn new(size_classes: SizeClassTable, span_bytes: usize, page_bytes: usize) -> Self {
        let bucket_count = SmallSpanClass::bucket_count(&size_classes);
        let mut cached_spans = Vec::with_capacity(bucket_count);
        cached_spans.resize_with(bucket_count, || None);

        Self {
            size_classes,
            span_bytes,
            page_bytes,
            cached_spans,
        }
    }

    /// Return one small-span class for the given payload when it fits.
    fn small_span_class(
        &self,
        byte_len: usize,
        reference_map: &ReferenceMap,
    ) -> Option<SmallSpanClass> {
        let class_index = self.size_classes.class_index_for(byte_len)?;
        let size_class = self.size_classes.classes[class_index];

        Some(SmallSpanClass {
            size_class: size_class.bytes,
            span_bytes: size_class.span_bytes(self.page_bytes, self.span_bytes),
            is_noscan: !reference_map.has_reference(),
        })
    }

    /// Return the bucket index for one small-span class.
    fn bucket_index(&self, class: &SmallSpanClass) -> HeapResult<usize> {
        class.bucket_index(&self.size_classes)
    }
}

impl SharedUsage {
    /// Create atomic shared usage from exact counters.
    pub(crate) fn new(allocation_count: usize, allocated_bytes: u64) -> Self {
        Self {
            allocation_count: AtomicUsize::new(allocation_count),
            allocated_bytes: AtomicU64::new(allocated_bytes),
        }
    }

    /// Return the number of live allocations.
    pub(crate) fn allocation_count(&self) -> usize {
        self.allocation_count.load(Ordering::Acquire)
    }

    /// Return the number of live allocated bytes.
    pub(crate) fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes.load(Ordering::Acquire)
    }

    /// Charge one allocation.
    pub(crate) fn allocate(&self, byte_len: usize) {
        let added_bytes = byte_len as u64;
        let previous_count = self.allocation_count.fetch_add(1, Ordering::AcqRel);
        let previous_bytes = self
            .allocated_bytes
            .fetch_add(added_bytes, Ordering::AcqRel);

        debug_assert!(previous_count < usize::MAX);
        debug_assert!(u64::MAX - previous_bytes >= added_bytes);
    }

    /// Release one allocation.
    pub(crate) fn free(&self, freed_bytes: u64) {
        let previous_bytes = self
            .allocated_bytes
            .fetch_sub(freed_bytes, Ordering::AcqRel);
        let previous_count = self.allocation_count.fetch_sub(1, Ordering::AcqRel);

        debug_assert!(previous_bytes >= freed_bytes);
        debug_assert!(previous_count > 0);
    }
}

impl SharedHeapSpace {
    /// Create a new empty shared heap-space store over one shared allocator.
    pub fn with_allocator(allocator: Arc<Allocator>) -> HeapResult<Self> {
        let options = HeapOptions {
            page_bytes: allocator.page_bytes(),
            allocator_chunk_bytes: allocator.chunk_bytes(),
            ..HeapOptions::shared()
        };

        Self::with_options(allocator, &options)
    }

    /// Create a new empty shared heap-space store over one shared allocator and options.
    pub fn with_options(allocator: Arc<Allocator>, options: &HeapOptions) -> HeapResult<Self> {
        options.validate_shared()?;
        options.validate_allocator(&allocator)?;

        let mapping = AddressSpace::reserve(options.heap_space_bytes, options.page_bytes)?;
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
                next_unused_large_allocation_id: FIRST_SHARED_MANAGED_LARGE_ALLOCATION_ID,
            },
            page_map: Vec::new(),
            next_offset: allocator.page_bytes(),
            mapping,
            gc: GcState::default(),
        };

        Ok(Self {
            allocator,
            state: RwLock::new(store),
            usage: SharedUsage::new(0, 0),
            gc: SharedGcState::default(),
        })
    }

    /// Return the configured shared page size.
    pub fn page_bytes(&self) -> usize {
        self.allocator.page_bytes()
    }

    /// Return the exact retained shared heap allocator-page bytes.
    pub fn retained_bytes(&self) -> u64 {
        let store = self.state.read();

        self.live_retained_bytes(&store)
    }

    /// Return the exact usage for this live shared heap-space store.
    pub fn usage(&self) -> SharedHeapSpaceUsage {
        let store = self.state.read();
        let retained_bytes = self.live_retained_bytes(&store);

        drop(store);

        let allocation_count = self.usage.allocation_count();
        let allocated_bytes = self.usage.allocated_bytes();

        SharedHeapSpaceUsage {
            allocation_count,
            allocated_bytes,
            retained_bytes,
        }
    }

    /// Return the number of live shared heap allocations.
    pub fn allocation_count(&self) -> usize {
        self.usage.allocation_count()
    }

    /// Return the number of live shared heap bytes.
    pub fn allocated_bytes(&self) -> u64 {
        self.usage.allocated_bytes()
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

    /// Return whether one shared heap reference currently refers to one live allocation.
    pub fn is_live(&self, reference: SharedHeapReference) -> bool {
        self.resolve_location(reference).is_some()
    }

    /// Create one shared heap allocator front end.
    pub fn allocator(&self) -> SharedAllocator {
        let store = self.state.read();

        SharedAllocator::new(
            store.small.size_classes.clone(),
            store.small.span_bytes,
            self.allocator.page_bytes(),
        )
    }

    /// Allocate one shared managed heap allocation.
    pub fn allocate(
        &self,
        allocator: &mut SharedAllocator,
        layout: AllocationLayout<'_>,
        payload: Payload<'_>,
    ) -> HeapResult<SharedHeapReference> {
        if layout.byte_len == 0 {
            return Err(HeapError::ZeroSizeAllocation);
        }

        if let Some(actual) = payload.byte_len()
            && actual != layout.byte_len
        {
            return Err(HeapError::InvalidAllocationBytes {
                expected: layout.byte_len,
                actual,
            });
        }

        let (place, charged_bytes) = self.allocate_place(allocator, layout, payload)?;
        let reference = self.base_reference_for_place(place)?;

        self.usage.allocate(charged_bytes);

        let has_shared_reference =
            payload.byte_len().is_some() && layout.reference_map.has_shared_reference();
        self.publish_shared_allocation(reference, has_shared_reference)?;

        Ok(reference)
    }

    /// Return the projected retained-byte delta for one shared heap allocation.
    pub(crate) fn retained_byte_delta(
        &self,
        allocator: &SharedAllocator,
        layout: AllocationLayout<'_>,
    ) -> HeapResult<i64> {
        if layout.byte_len == 0 {
            return Err(HeapError::ZeroSizeAllocation);
        }

        if let Some(class) = allocator.small_span_class(layout.byte_len, layout.reference_map) {
            let bucket_index = allocator.bucket_index(&class)?;
            if allocator
                .cached_spans
                .get(bucket_index)
                .and_then(Option::as_ref)
                .is_some_and(|cached| {
                    let span = cached.span.read();

                    span.list == SpanList::Cached && span.occupied_count < span.slot_count
                })
            {
                return Ok(0);
            }

            let store = self.state.read();
            if self.has_available_small_slot(&store, &class)? {
                return Ok(0);
            }

            return Ok(class.span_bytes as i64);
        }

        Ok(self.round_up_allocation_bytes(layout.byte_len) as i64)
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

    /// Fill one caller-provided buffer from one shared heap allocation at one offset.
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

    /// Return one checked address for a shared heap byte range.
    pub fn address(
        &self,
        reference: SharedHeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*mut u8> {
        let (location, byte_offset) = self.checked_location_range(reference, start, byte_len)?;
        let mapping_offset = self.location_mapping_offset(location, byte_offset)?;
        let store = self.state.read();

        store.mapping.address(mapping_offset, byte_len)
    }

    /// Return one checked writable address for a shared heap byte range.
    pub fn address_mut(
        &self,
        reference: SharedHeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*mut u8> {
        let (location, byte_offset) = self.checked_location_range(reference, start, byte_len)?;
        let mapping_offset = self.location_mapping_offset(location, byte_offset)?;
        let store = self.state.read();

        store.mapping.address(mapping_offset, byte_len)
    }

    /// Return the reference map for one shared heap reference.
    pub fn scan(&self, reference: SharedHeapReference) -> HeapResult<ReferenceMap> {
        let (location, _) = self.checked_location_range(reference, 0, 0)?;

        self.place_reference_map(location.place)
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

    /// Record one shared heap write barrier before one byte store.
    pub fn write_barrier_bytes(
        &self,
        reference: SharedHeapReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let (_, byte_offset) = self.checked_location_range(reference, start, bytes.len())?;

        self.write_shared_barrier_bytes(reference, byte_offset, bytes)
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
            allocation_byte_offset(location.byte_offset, start, byte_len, location.byte_len)?;

        Ok((location, byte_offset))
    }

    /// Return one shared heap large allocation by allocation id.
    pub(crate) fn large_allocation_ref(
        &self,
        allocation_id: SharedLargeAllocationId,
    ) -> Option<Arc<RwLock<SharedLargeAllocation>>> {
        let index = allocation_id.index().ok()?;
        let store = self.state.read();

        store.large.allocations.get(index).cloned()
    }

    /// Allocate one shared heap place for the given payload.
    fn allocate_place(
        &self,
        allocator: &mut SharedAllocator,
        layout: AllocationLayout<'_>,
        payload: Payload<'_>,
    ) -> HeapResult<(SharedHeapPlace, usize)> {
        if let Some(class) = allocator.small_span_class(layout.byte_len, layout.reference_map) {
            let slot = self.allocate_small(allocator, &class, layout.reference_map, payload)?;

            return Ok((SharedHeapPlace::Small(slot), class.size_class));
        }

        let pages = self.allocate_large_pages(layout.byte_len)?;

        let mut store = self.state.write();
        let allocation_id = self.insert_large_allocation(
            &mut store,
            layout.byte_len,
            pages,
            layout.reference_map.clone(),
        )?;
        let Some(allocation) = store.large.allocations.get(allocation_id.index()?).cloned() else {
            return Err(HeapError::MissingLargeAllocation {
                allocation_id: allocation_id.id(),
            });
        };
        let first_offset = allocation.read().first_offset;

        // initialize bytes before returning the allocation reference
        let initialize = match payload {
            Payload::Bytes(bytes) => store.mapping.write(first_offset, bytes),
            Payload::Zeroed => store.mapping.zero(first_offset, layout.byte_len),
        };
        if let Err(error) = initialize {
            allocation.write().retire();
            store
                .large
                .free_large_allocation_ids
                .push(allocation_id.id());
            self.unmap_page_run(&mut store, first_offset, &pages);
            store
                .page_run_cache
                .release_page_run(&self.allocator, pages)?;

            return Err(error);
        }

        Ok((SharedHeapPlace::Large(allocation_id), layout.byte_len))
    }

    /// Allocate large pages outside the shared state lock.
    fn allocate_large_pages(&self, byte_len: usize) -> HeapResult<PageRun> {
        let pages = {
            let mut store = self.state.write();

            store
                .page_run_cache
                .allocate_pages(&self.allocator, byte_len)?
        };

        Ok(pages)
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
            .partial_spans
            .get(bucket_index)
            .is_some_and(|spans| !spans.is_empty()))
    }

    /// Allocate or reuse one non-full shared heap span for the given size class.
    fn allocate_small_span(
        &self,
        store: &mut SharedHeapState,
        class: &SmallSpanClass,
    ) -> HeapResult<usize> {
        let bucket_index = class.bucket_index(&store.small.size_classes)?;

        while let Some(span_index) = store.small.partial_spans[bucket_index].pop() {
            let Some(span) = store.small.spans.get(span_index).cloned() else {
                return Err(HeapError::MissingSpan { span_index });
            };
            let mut span = span.write();

            if span.occupied_count < span.slot_count {
                if span.list != SpanList::Central {
                    continue;
                }

                if span.occupied_count == 0 && span.pages.is_empty() {
                    let pages = store
                        .page_run_cache
                        .allocate_pages(&self.allocator, class.span_bytes)?;
                    let first_offset = span.first_offset;
                    self.map_page_run(store, first_offset, &pages, |logical_page_index| {
                        SharedHeapPageMapEntry::Small {
                            span_index,
                            logical_page_index,
                        }
                    });

                    span.pages = pages;
                }

                span.list = SpanList::Cached;

                return Ok(span_index);
            }
        }

        let slot_count = (class.span_bytes / class.size_class).max(1);
        let scan_word_count = class.size_class.div_ceil(std::mem::size_of::<usize>());
        let pages = store
            .page_run_cache
            .allocate_pages(&self.allocator, class.span_bytes)?;
        let first_offset = self.reserve_space_range(store, class.span_bytes)?;
        let span = SharedSmallSpan {
            first_offset,
            class: class.clone(),
            slot_count,
            occupied_count: 0,
            free_cursor: 0,
            occupied: crate::Bitmap::with_capacity(slot_count),
            local_reference_bits: crate::Bitmap::with_capacity(slot_count * scan_word_count),
            shared_reference_bits: crate::Bitmap::with_capacity(slot_count * scan_word_count),
            marked: crate::Bitmap::with_capacity(slot_count),
            scanned: crate::Bitmap::with_capacity(slot_count),
            is_queued_for_scan: false,
            list: SpanList::Cached,
            pages,
        };
        let span_index = store.small.spans.len();
        self.map_page_run(store, first_offset, &pages, |logical_page_index| {
            SharedHeapPageMapEntry::Small {
                span_index,
                logical_page_index,
            }
        });

        store.small.spans.push(Arc::new(RwLock::new(span)));

        Ok(span_index)
    }

    /// Allocate one shared heap small slot from one explicit initialization source.
    fn allocate_small(
        &self,
        allocator: &mut SharedAllocator,
        class: &SmallSpanClass,
        reference_map: &ReferenceMap,
        payload: Payload<'_>,
    ) -> HeapResult<SpanSlot> {
        let bucket_index = allocator.bucket_index(class)?;
        let span_bytes = class.span_bytes;

        if let Some(cached) = allocator.cached_spans[bucket_index].as_ref() {
            if let Some((slot, keep_cached)) = self.try_allocate_cached_small_slot(
                cached,
                class,
                span_bytes,
                reference_map,
                payload,
            )? {
                if !keep_cached {
                    allocator.cached_spans[bucket_index] = None;
                }

                return Ok(slot);
            }

            allocator.cached_spans[bucket_index] = None;
        }

        let cached = {
            let mut store = self.state.write();
            let span_index = self.allocate_small_span(&mut store, class)?;
            let Some(span) = store.small.spans.get(span_index).cloned() else {
                return Err(HeapError::MissingSpan { span_index });
            };

            CachedSmallSpan { span_index, span }
        };
        let Some((slot, keep_cached)) = self.try_allocate_cached_small_slot(
            &cached,
            class,
            span_bytes,
            reference_map,
            payload,
        )?
        else {
            return Err(HeapError::MissingSpan {
                span_index: cached.span_index,
            });
        };
        if keep_cached {
            allocator.cached_spans[bucket_index] = Some(cached);
        } else {
            allocator.cached_spans[bucket_index] = None;
        }

        Ok(slot)
    }

    /// Try to allocate one slot from one allocator-cached shared small span.
    fn try_allocate_cached_small_slot(
        &self,
        cached: &CachedSmallSpan,
        class: &SmallSpanClass,
        span_bytes: usize,
        reference_map: &ReferenceMap,
        payload: Payload<'_>,
    ) -> HeapResult<Option<(SpanSlot, bool)>> {
        let store = self.state.read();
        let mut span = cached.span.write();

        if span.list != SpanList::Cached
            || span.class != *class
            || span.occupied_count >= span.slot_count
            || span.pages.is_empty()
        {
            return Ok(None);
        }

        let slot_index = span.free_cursor;
        let slot = SpanSlot::new(cached.span_index, slot_index)?;
        let slot_offset = small_slot_offset(span.class.size_class, slot_index);
        let write_offset = checked_place_offset(slot_offset, 0, span_bytes)?;
        let first_offset = span.first_offset;
        let mapping_offset = first_offset + write_offset;

        // clear the full slot before publishing caller bytes
        match payload {
            Payload::Bytes(bytes) if bytes.len() < class.size_class => {
                store.mapping.zero(mapping_offset, class.size_class)?;
                store.mapping.write(mapping_offset, bytes)?;
            }
            Payload::Bytes(bytes) => store.mapping.write(mapping_offset, bytes)?,
            Payload::Zeroed => store.mapping.zero(mapping_offset, class.size_class)?,
        }

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
            )?;
        }

        span.occupied.set(slot_index);
        span.marked.clear(slot_index);
        span.scanned.clear(slot_index);
        span.occupied_count += 1;
        span.free_cursor = find_free_cursor(&span.occupied, slot_index + 1, span.slot_count);

        let keep_cached = span.occupied_count < span.slot_count;
        if !keep_cached {
            span.list = SpanList::Full;
        }

        Ok(Some((slot, keep_cached)))
    }

    /// Insert one shared large allocation record.
    fn insert_large_allocation(
        &self,
        store: &mut SharedHeapState,
        len: usize,
        pages: PageRun,
        reference_map: ReferenceMap,
    ) -> HeapResult<SharedLargeAllocationId> {
        let (allocation_id, reused_allocation_id) =
            if let Some(allocation_id) = store.large.free_large_allocation_ids.pop() {
                (allocation_id, true)
            } else {
                let allocation_id = store.large.next_unused_large_allocation_id;
                let next_allocation_id = store.large.next_unused_large_allocation_id + 1;

                store.large.next_unused_large_allocation_id = next_allocation_id;
                (allocation_id, false)
            };

        if allocation_id == 0 {
            if reused_allocation_id {
                store.large.free_large_allocation_ids.push(allocation_id);
            }

            store
                .page_run_cache
                .release_page_run(&self.allocator, pages)?;

            return Err(HeapError::InvalidLargeAllocationId { id: allocation_id });
        }

        let allocation_id = SharedLargeAllocationId::new(allocation_id);
        let index = allocation_id.index()?;
        if index > store.large.allocations.len() {
            if reused_allocation_id {
                store
                    .large
                    .free_large_allocation_ids
                    .push(allocation_id.id());
            }

            store
                .page_run_cache
                .release_page_run(&self.allocator, pages)?;

            return Err(HeapError::InvalidLargeAllocationId {
                id: allocation_id.id(),
            });
        }

        let first_offset =
            self.reserve_space_range(store, pages.len() * self.allocator.page_bytes())?;

        self.map_page_run(store, first_offset, &pages, |logical_page_index| {
            SharedHeapPageMapEntry::Large {
                allocation_id,
                logical_page_index,
            }
        });

        let allocation = SharedLargeAllocation {
            is_live: true,
            first_offset,
            len,
            pages,
            reference_map,
            is_marked: false,
        };
        let allocation = Arc::new(RwLock::new(allocation));

        if index == store.large.allocations.len() {
            store.large.allocations.push(allocation);
        } else {
            store.large.allocations[index] = allocation;
        }

        Ok(allocation_id)
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
            span.scanned.clear(slot_index);
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
                span.list = SpanList::Released;

                let first_offset = span.first_offset;
                let pages = span.pages;
                span.pages = PageRun::empty();

                Some((first_offset, pages))
            } else {
                let should_requeue = was_full && span.occupied_count < span.slot_count;

                if should_requeue {
                    let bucket_index = span.class.bucket_index(&store.small.size_classes)?;
                    span.list = SpanList::Central;
                    store.small.partial_spans[bucket_index].push(slot.span_index());
                }

                None
            }
        };

        if let Some((first_offset, pages)) = pages {
            self.unmap_page_run(store, first_offset, &pages);
            store
                .page_run_cache
                .release_page_run(&self.allocator, pages)?;
        }

        Ok(())
    }

    /// Return the reference map for one shared heap location.
    fn place_reference_map(&self, place: SharedHeapPlace) -> HeapResult<ReferenceMap> {
        match place {
            SharedHeapPlace::Small(slot) => {
                self.small_slot_reference_map(slot.span_index(), slot.slot_index())
            }
            SharedHeapPlace::Large(allocation_id) => {
                let reference_map = self
                    .state
                    .read()
                    .large
                    .allocations
                    .get(allocation_id.index()?)
                    .cloned()
                    .ok_or(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    })?
                    .read()
                    .reference_map
                    .clone();

                Ok(reference_map)
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
        match location.place {
            SharedHeapPlace::Small(slot) => {
                let store = self.state.read();
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span = span.read();

                let slot_offset = small_slot_offset(span.class.size_class, slot.slot_index());
                let span_bytes = span.pages.len() * self.allocator.page_bytes();
                let read_offset = checked_place_offset(slot_offset, byte_offset, span_bytes)?;

                store
                    .mapping
                    .bytes(span.first_offset + read_offset, byte_len)
            }
            SharedHeapPlace::Large(allocation_id) => {
                let store = self.state.read();
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

                store
                    .mapping
                    .bytes(allocation.first_offset + byte_offset, byte_len)
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
        match location.place {
            SharedHeapPlace::Small(slot) => {
                let store = self.state.read();
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span = span.read();

                let slot_offset = small_slot_offset(span.class.size_class, slot.slot_index());
                let span_bytes = span.pages.len() * self.allocator.page_bytes();
                let read_offset = checked_place_offset(slot_offset, byte_offset, span_bytes)?;

                store.mapping.read(span.first_offset + read_offset, target)
            }
            SharedHeapPlace::Large(allocation_id) => {
                let store = self.state.read();
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

                store
                    .mapping
                    .read(allocation.first_offset + byte_offset, target)
            }
        }
    }

    /// Return the mapping offset for one shared heap location.
    fn location_mapping_offset(
        &self,
        location: SharedHeapLocation,
        byte_offset: usize,
    ) -> HeapResult<usize> {
        match location.place {
            SharedHeapPlace::Small(slot) => {
                let store = self.state.read();
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span = span.read();
                let slot_offset = small_slot_offset(span.class.size_class, slot.slot_index());
                let span_bytes = span.pages.len() * self.allocator.page_bytes();
                let read_offset = checked_place_offset(slot_offset, byte_offset, span_bytes)?;

                Ok(span.first_offset + read_offset)
            }
            SharedHeapPlace::Large(allocation_id) => {
                let Some(allocation) = self.large_allocation_ref(allocation_id) else {
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

                Ok(allocation.first_offset + byte_offset)
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
        match location.place {
            SharedHeapPlace::Small(slot) => {
                let store = self.state.read();
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                let span = span.read();
                let slot_offset = small_slot_offset(span.class.size_class, slot.slot_index());
                let span_bytes = span.pages.len() * self.allocator.page_bytes();
                let write_offset = checked_place_offset(slot_offset, byte_offset, span_bytes)?;
                let mapping_offset = span.first_offset + write_offset;
                drop(span);

                // write the live mapping
                store.mapping.write(mapping_offset, bytes)?;

                Ok(())
            }
            SharedHeapPlace::Large(allocation_id) => {
                let Some(allocation) = self.large_allocation_ref(allocation_id) else {
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

                let mapping_offset = allocation.first_offset + byte_offset;
                drop(allocation);

                let store = self.state.read();

                // write the live mapping
                store.mapping.write(mapping_offset, bytes)?;

                Ok(())
            }
        }
    }

    /// Return the current live heap page runs.
    fn live_page_runs(&self, store: &SharedHeapState) -> Vec<PageRun> {
        let mut pages = Vec::with_capacity(store.small.spans.len() + store.large.allocations.len());

        for span in &store.small.spans {
            let span = span.read();

            if span.occupied_count == 0 || span.pages.is_empty() {
                continue;
            }

            pages.push(span.pages);
        }

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
        let pages = self.live_page_runs(store);
        let cached_bytes = store
            .page_run_cache
            .cached_bytes(self.allocator.page_bytes());

        self.allocator.retained_bytes_for_page_runs(pages.iter()) + cached_bytes
    }

    /// Release allocator roots owned by this shared heap space.
    fn close(&mut self) -> HeapResult<()> {
        let mut store = self.state.write();
        let page_runs = self.live_page_runs(&store);

        for page_run in page_runs {
            store
                .page_run_cache
                .release_page_run(&self.allocator, page_run)?;
        }

        store.page_run_cache.flush(&self.allocator)
    }

    /// Return the page-rounded retained bytes for one shared heap-space allocation.
    fn round_up_allocation_bytes(&self, byte_len: usize) -> u64 {
        let page_bytes = self.page_bytes() as u64;
        let byte_len = byte_len as u64;

        byte_len.div_ceil(page_bytes) * page_bytes
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

                let slot_offset = span.class.size_class * slot_index;
                let base_offset = span.first_offset + slot_offset;

                references.push(SharedHeapReference::new(base_offset));
            }
        }

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
    fn base_reference_for_place(&self, place: SharedHeapPlace) -> HeapResult<SharedHeapReference> {
        let store = self.state.read();

        self.base_reference(&store, place)
    }

    /// Return the base reference for one shared heap place in the current state.
    fn base_reference(
        &self,
        store: &SharedHeapState,
        place: SharedHeapPlace,
    ) -> HeapResult<SharedHeapReference> {
        let base_offset = match place {
            SharedHeapPlace::Small(slot) => {
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span = span.read();
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

    /// Reserve one logical shared heap-space byte range.
    fn reserve_space_range(
        &self,
        store: &mut SharedHeapState,
        byte_len: usize,
    ) -> HeapResult<usize> {
        debug_assert!(store.next_offset <= store.mapping.byte_len());

        let first_offset = align_up(store.next_offset, self.allocator.page_bytes());
        let next_offset = first_offset + byte_len;
        if next_offset > store.mapping.byte_len() {
            return Err(HeapError::InvalidByteRange {
                start: first_offset,
                len: byte_len,
                capacity: store.mapping.byte_len(),
            });
        }

        store.next_offset = next_offset;

        Ok(first_offset)
    }
}

/// Return the offset rounded up to one allocation boundary.
fn align_up(byte_len: usize, alignment_bytes: usize) -> usize {
    let alignment_bytes = alignment_bytes.max(1);

    byte_len.div_ceil(alignment_bytes) * alignment_bytes
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

/// Return one allocation-local byte offset for one visible range.
fn allocation_byte_offset(
    base_offset: usize,
    start: usize,
    len: usize,
    capacity: usize,
) -> HeapResult<usize> {
    debug_assert!(base_offset <= capacity);

    let remaining = capacity - base_offset;
    if start > remaining {
        return Err(HeapError::InvalidByteRange {
            start,
            len,
            capacity,
        });
    }

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

/// Return one nested place offset inside one bounded page run.
pub(crate) fn checked_place_offset(
    base: usize,
    byte_offset: usize,
    capacity: usize,
) -> HeapResult<usize> {
    if byte_offset > capacity {
        return Err(HeapError::InvalidByteRange {
            start: byte_offset,
            len: 0,
            capacity,
        });
    }

    let offset = base + byte_offset;

    Ok(offset)
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
        let _ = self.close();
    }
}
