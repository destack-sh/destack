use std::sync::Arc;

use parking_lot::RwLock;
use tspp_mir::TraceMap;

use super::{
    AllocationCache, HeapPlace, HeapState, HeapStorage, LargeBlock, LargeBlockId, PageOwner,
    ReservedSlot, SmallSizeClassCache, SmallSpan, SpanList,
};
use crate::{
    Allocation, DropPlan, HeapAllocationError, HeapError, HeapRepresentationError, HeapResult,
    Payload, SharedHeapReference, Slot, SmallAllocationClass, SmallSpanClass,
};
use tspp_memory::MemoryRange;

impl HeapStorage {
    /// Allocate one shared heap block.
    pub(crate) fn allocate(
        &self,
        cache: &mut AllocationCache,
        layout: &Allocation<'_>,
        payload: Payload<'_>,
        should_keep_worker_cache: bool,
    ) -> HeapResult<SharedHeapReference> {
        // heap blocks must have a physical payload
        if layout.is_empty() {
            return Err(HeapError::invalid_allocation(HeapAllocationError::ZeroSize));
        }

        // caller-provided bytes must exactly initialize the layout
        if let Some(actual) = payload.byte_len()
            && actual != layout.byte_len
        {
            return Err(HeapError::invalid_allocation(
                HeapAllocationError::ByteLengthMismatch {
                    expected: layout.byte_len,
                    actual,
                },
            ));
        }

        // allocate the payload bytes
        let place = self.allocate_place(cache, layout, payload, should_keep_worker_cache)?;
        let reference = self.base_reference(place)?;

        // retain the shared allocations the payload references
        if let Payload::Bytes(bytes) = payload {
            self.retain_payload(cache, layout.trace_map, bytes)?;
        }

        // publish initialized shared references to an active mark cycle
        let has_shared_reference = payload.byte_len().is_some() && layout.has_shared_reference;
        self.publish_shared_allocation(reference, has_shared_reference)?;

        Ok(reference)
    }

    /// Reserve one payload from a worker-local small cache.
    #[inline(always)]
    pub(crate) fn reserve_small_from_cache(
        &self,
        cache: &mut AllocationCache,
        small: SmallAllocationClass,
    ) -> Option<SharedHeapReference> {
        let cache_index = small.cache_index();
        if cache.small.len() <= cache_index {
            return None;
        }

        let size_class_cache = &mut cache.small[cache_index];
        if !size_class_cache.is_active() || size_class_cache.class() != Some(small.class) {
            return None;
        }

        let reference = size_class_cache
            .cursor
            .reserve_reference(small.slot_bytes())?;
        if !size_class_cache.has_available_slot() {
            self.flush_size_class_cache(size_class_cache);
            size_class_cache.finish();
            size_class_cache.clear();
        }

        Some(reference)
    }

    /// Publish and retire every worker-local small cache.
    pub(crate) fn flush_allocation_cache(&self, cache: &mut AllocationCache) {
        let mut store = self.state.write();

        // publish each worker-owned cache back to central state
        for size_class_cache in &mut cache.small {
            let Some(span) = size_class_cache.span.clone() else {
                continue;
            };

            self.flush_size_class_cache(size_class_cache);
            size_class_cache.publish_cursor();

            // partial spans return to the central partial list
            if size_class_cache.has_available_slot() {
                span.list.store(SpanList::Central);
                if let Some(class) = size_class_cache.class() {
                    store
                        .small
                        .partial_spans
                        .entry(class)
                        .or_default()
                        .push(size_class_cache.span_index as usize);
                }
                size_class_cache.clear();

                continue;
            }

            // full spans only need a list transition
            span.list.store(SpanList::Full);
            size_class_cache.clear();
        }
    }

    /// Publish one worker-local cache into shared accounting.
    #[inline(always)]
    fn flush_size_class_cache(&self, size_class_cache: &mut SmallSizeClassCache) {
        let Some(class) = size_class_cache.class() else {
            return;
        };
        let usage = size_class_cache.cursor.flush_usage(class);
        if usage.allocation_count() == 0 {
            return;
        }

        self.accounting
            .allocate_many(usage.allocation_count(), usage.allocated_bytes());
    }

    /// Publish worker-local accounting for the cache that owns one reference.
    pub(crate) fn flush_reference_cache(
        &self,
        cache: &mut AllocationCache,
        reference: SharedHeapReference,
    ) {
        let offset = reference.offset();

        // settle only the owning cache
        for size_class_cache in &mut cache.small {
            if !size_class_cache.is_active() {
                continue;
            }

            let Some(class) = size_class_cache.class() else {
                continue;
            };
            let span_size_bytes = class.size_class() * size_class_cache.slot_count;
            let span_end = size_class_cache.first_offset + span_size_bytes;
            if offset < size_class_cache.first_offset || offset >= span_end {
                continue;
            }

            self.flush_size_class_cache(size_class_cache);
            size_class_cache.publish_cursor();

            return;
        }
    }

    /// Return the projected retained-byte delta for one shared heap layout.
    pub(crate) fn retained_byte_delta(
        &self,
        cache: &AllocationCache,
        layout: &Allocation<'_>,
    ) -> HeapResult<i64> {
        // heap blocks must have a physical payload
        if layout.is_empty() {
            return Err(HeapError::invalid_allocation(HeapAllocationError::ZeroSize));
        }

        // small blocks may reuse worker-local or central slots
        if let Some(small) = layout.class.as_small() {
            let cache_index = small.cache_index();
            if let Some(size_class_cache) = cache.small.get(cache_index)
                && size_class_cache.span.as_ref().is_some_and(|span| {
                    span.list.load() == SpanList::Worker && size_class_cache.has_available_slot()
                })
            {
                return Ok(0);
            }

            let store = self.state.read();
            if self.has_available_small_slot(&store, &small) {
                return Ok(0);
            }

            return Ok(small.class.span_size_bytes() as i64);
        }

        // large blocks retain whole pages
        Ok(self.round_up_allocation_bytes(layout.byte_len) as i64)
    }

    /// Release one shared heap small slot.
    pub(crate) fn release_small_slot(&self, store: &mut HeapState, slot: Slot) -> HeapResult<()> {
        // resolve the owning span
        let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
            return Err(HeapError::internal("missing span"));
        };

        // release the slot and requeue reusable span capacity
        {
            let slot_index = slot.slot_index();
            let was_full = span.occupied_count() == span.slot_count;

            if !span.release_slot(slot_index) {
                return Err(HeapError::internal("missing small slot"));
            }

            let list = span.list.load();
            if span.occupied_count() == 0 && list != SpanList::Worker {
                span.reset_free_cursor_to_start();
                span.list.store(SpanList::Central);

                // full spans are not already present in the central list
                if list == SpanList::Full {
                    store
                        .small
                        .partial_spans
                        .entry(span.class)
                        .or_default()
                        .push(slot.span_index());
                }
            } else {
                // full spans become partial after releasing one slot
                let should_requeue = was_full
                    && span.occupied_count() < span.slot_count
                    && span.list.load() == SpanList::Full;

                if should_requeue {
                    span.list.store(SpanList::Central);
                    store
                        .small
                        .partial_spans
                        .entry(span.class)
                        .or_default()
                        .push(slot.span_index());
                }
            }
        }

        Ok(())
    }

    /// Allocate one shared heap storage for the given payload.
    fn allocate_place(
        &self,
        cache: &mut AllocationCache,
        layout: &Allocation<'_>,
        payload: Payload<'_>,
        should_keep_worker_cache: bool,
    ) -> HeapResult<HeapPlace> {
        // small blocks use worker-local caches and size-class spans
        if let Some(small) = layout.class.as_small() {
            let cache_index = small.cache_index();
            let class = small.class;

            // publish and replace a cache that belongs to another logical class
            let size_class_cache = cache.ensure_small(cache_index);
            if size_class_cache.class != Some(class) {
                self.flush_size_class_cache(size_class_cache);
                *size_class_cache = SmallSizeClassCache::inactive();
                size_class_cache.class = Some(class);
            }

            let slot = self.allocate_small(
                cache,
                cache_index,
                &class,
                layout.trace_map,
                payload,
                should_keep_worker_cache,
            )?;

            return Ok(HeapPlace::SmallSlot(slot));
        }

        // large blocks reserve whole page spans
        let pages = self.allocate_large_pages(layout.byte_len, layout.alignment)?;

        let mut store = self.state.write();
        let block_id = self.insert_large_block(
            &mut store,
            layout.byte_len,
            pages,
            layout.trace_map.clone(),
            layout.drop,
        )?;
        let Some(block) = store
            .large
            .blocks
            .get(block_id.index()?)
            .and_then(Option::as_ref)
            .cloned()
        else {
            return Err(HeapError::internal("missing large block"));
        };
        let first_offset = block.read().first_offset;

        payload.initialize_mapped(&self.memory, first_offset, layout.byte_len);
        self.accounting.allocate(layout.byte_len);
        self.accounting.retain_pages(pages);

        Ok(HeapPlace::LargeBlock(block_id))
    }

    /// Allocate large pages outside the shared state lock.
    fn allocate_large_pages(&self, byte_len: usize, alignment: usize) -> HeapResult<MemoryRange> {
        let page_size_bytes = self.page_size_bytes();
        let byte_len = byte_len.next_multiple_of(page_size_bytes);
        let alignment = alignment.max(page_size_bytes);
        let pages = self.memory.allocate(byte_len, alignment)?;

        Ok(pages)
    }

    /// Return whether one size class still has one live reusable slot.
    fn has_available_small_slot(&self, store: &HeapState, small: &SmallAllocationClass) -> bool {
        store
            .small
            .partial_spans
            .get(&small.class)
            .is_some_and(|spans| !spans.is_empty())
    }

    /// Allocate or reuse one non-full shared heap span for the given size class.
    fn allocate_small_span(
        &self,
        store: &mut HeapState,
        class: &SmallSpanClass,
    ) -> HeapResult<SmallSpanAllocation> {
        // first reuse a central partial span
        while let Some(span_index) = store.small.partial_spans.get_mut(class).and_then(Vec::pop) {
            let Some(span) = store.small.spans.get(span_index).cloned() else {
                return Err(HeapError::internal("missing span"));
            };

            // skip stale full-span entries
            if span.occupied_count() < span.slot_count {
                if span.list.load() != SpanList::Central {
                    continue;
                }

                span.list.store(SpanList::Worker);
                span.reset_free_cursor();

                return Ok(SmallSpanAllocation {
                    span_index,
                    is_dense: false,
                });
            }
        }

        // otherwise map a new span for this size class
        let slot_count = (class.span_size_bytes() / class.size_class()).max(1);
        let pages = self
            .memory
            .allocate(class.span_size_bytes(), self.page_size_bytes())?;
        let first_offset = pages.offset;

        // materialize the full span before worker-local spans use it
        self.memory
            .materialize(first_offset, class.span_size_bytes())?;
        // SAFETY: the span range was materialized above
        unsafe {
            self.memory
                .zero_mapped_bytes(first_offset, class.span_size_bytes());
        }

        let span = SmallSpan::new(first_offset, *class, slot_count, pages, SpanList::Worker);
        let span_index = store.small.spans.len();
        self.map_page_span(store, &pages, |logical_page_index| PageOwner::SmallSpan {
            span_index,
            logical_page_index,
        });

        store.small.spans.push(Arc::new(span));
        self.accounting.retain_pages(pages);

        Ok(SmallSpanAllocation {
            span_index,
            is_dense: true,
        })
    }

    /// Allocate one shared heap small slot from one explicit initialization source.
    fn allocate_small(
        &self,
        cache: &mut AllocationCache,
        cache_index: usize,
        class: &SmallSpanClass,
        trace_map: &TraceMap,
        payload: Payload<'_>,
        should_keep_worker_cache: bool,
    ) -> HeapResult<Slot> {
        debug_assert_eq!(cache.small[cache_index].class(), Some(*class));

        // reuse the worker-owned span when it still has a matching slot
        if cache.small[cache_index].is_active() {
            let mut should_release_cache = false;
            let allocated_slot = {
                let size_class_cache = &mut cache.small[cache_index];
                let block = self.reserve_small_slot(size_class_cache, trace_map, payload)?;

                if let Some(block) = block {
                    let slot = block.slot;

                    // exhausted caches leave worker ownership immediately
                    if !block.should_keep_cache {
                        self.flush_reserved_slot(size_class_cache, block);
                        size_class_cache.finish();
                        size_class_cache.clear();
                    }
                    // kept non-dense slots have no cursor flush to publish them
                    else if !block.is_dense {
                        let Some(class) = size_class_cache.class() else {
                            return Err(HeapError::internal("missing cache class"));
                        };

                        self.accounting.allocate(class.size_class());
                    }
                    // active marking cannot leave free slots hidden in the worker
                    else if !should_keep_worker_cache {
                        should_release_cache = true;
                    }

                    Some(slot)
                } else {
                    size_class_cache.finish();
                    size_class_cache.clear();

                    None
                }
            };

            // centralize a cache whose free slots must remain visible
            if should_release_cache {
                self.release_small_cache(cache, cache_index);
            }

            // return the slot if the active cache produced one
            if let Some(slot) = allocated_slot {
                return Ok(slot);
            }
        }

        // acquire a central span or map a new one for this size class
        {
            let mut store = self.state.write();
            let allocation = self.allocate_small_span(&mut store, class)?;
            let size_class_cache = &mut cache.small[cache_index];
            let Some(span) = store.small.spans.get(allocation.span_index).cloned() else {
                return Err(HeapError::internal("missing span"));
            };

            size_class_cache.install(allocation.span_index, span, allocation.is_dense);
        }

        // initialize one slot from the newly installed worker cache
        let size_class_cache = &mut cache.small[cache_index];
        let block = self.reserve_small_slot(size_class_cache, trace_map, payload)?;
        let Some(block) = block else {
            return Err(HeapError::internal("missing span"));
        };

        let slot = block.slot;
        let should_release_cache = block.should_keep_cache && !should_keep_worker_cache;

        // publish or retire the newly installed cache
        if !block.should_keep_cache {
            self.flush_reserved_slot(size_class_cache, block);
            size_class_cache.finish();
            size_class_cache.clear();
        } else if !block.is_dense {
            let Some(class) = size_class_cache.class() else {
                return Err(HeapError::internal("missing cache class"));
            };

            self.accounting.allocate(class.size_class());
        }

        // return published caches to the central partial list
        if should_release_cache {
            self.release_small_cache(cache, cache_index);
        }

        Ok(slot)
    }

    /// Return one worker-local cache to the central list.
    fn release_small_cache(&self, cache: &mut AllocationCache, cache_index: usize) {
        // take the worker-owned span
        let mut store = self.state.write();
        let size_class_cache = &mut cache.small[cache_index];
        let Some(span) = size_class_cache.span.clone() else {
            return;
        };

        // publish occupied slots before list transition
        self.flush_size_class_cache(size_class_cache);
        size_class_cache.publish_cursor();

        // keep reusable spans on the central partial list
        if size_class_cache.has_available_slot() {
            span.list.store(SpanList::Central);
            if let Some(class) = size_class_cache.class() {
                store
                    .small
                    .partial_spans
                    .entry(class)
                    .or_default()
                    .push(size_class_cache.span_index as usize);
            }
        } else {
            span.list.store(SpanList::Full);
        }

        size_class_cache.clear();
    }

    /// Publish accounting for one allocated worker-local slot.
    #[inline(always)]
    fn flush_reserved_slot(&self, size_class_cache: &mut SmallSizeClassCache, block: ReservedSlot) {
        if block.is_dense {
            self.flush_size_class_cache(size_class_cache);
        } else {
            let Some(class) = size_class_cache.class() else {
                return;
            };

            self.accounting.allocate(class.size_class());
        }
    }

    /// Reserve one small slot from one worker-local cache.
    #[inline(always)]
    fn reserve_small_slot(
        &self,
        size_class_cache: &mut SmallSizeClassCache,
        trace_map: &TraceMap,
        payload: Payload<'_>,
    ) -> HeapResult<Option<ReservedSlot>> {
        // claim one slot or retire the exhausted cache
        let Some(slot) = size_class_cache.reserve_slot() else {
            size_class_cache.finish();

            return Ok(None);
        };

        // resolve the claimed slot inside the worker-owned span
        let slot_index = slot.slot_index;
        let span = size_class_cache.span.as_ref().cloned();
        let span_slot = size_class_cache.span_slot(slot_index);
        let Some(class) = size_class_cache.class() else {
            return Err(HeapError::internal("missing cache class"));
        };
        let Some(reference) = size_class_cache.reference_for_slot(slot_index) else {
            return Err(HeapError::internal("missing cache class"));
        };
        let byte_offset = reference.offset();

        // dense slots come bulk-zeroed, a clear needs-zero bit means the free slot holds zeroes
        let needs_zero = !slot.is_dense
            && span
                .as_ref()
                .is_some_and(|span| span.take_needs_zero(slot_index));

        // initialize the slot payload
        // SAFETY: worker spans are materialized before they enter caches
        unsafe {
            match payload {
                // clear stale tail bytes before copying short payloads
                Payload::Bytes(bytes) if bytes.len() < class.size_class() => {
                    if needs_zero {
                        self.memory
                            .zero_mapped_bytes(byte_offset, class.size_class());
                    }

                    self.memory.write_mapped_bytes(byte_offset, bytes);
                }
                // full-width payloads overwrite the slot completely
                Payload::Bytes(bytes) => self.memory.write_mapped_bytes(byte_offset, bytes),
                // clear only slots that previously held arbitrary bytes
                Payload::Zeroed => {
                    if needs_zero {
                        self.memory
                            .zero_mapped_bytes(byte_offset, class.size_class());
                    }
                }
                // no-scan slots keep zeroed reuse semantics even when uninitialized
                Payload::Uninit if !trace_map.has_heap_reference() => {
                    if needs_zero {
                        self.memory
                            .zero_mapped_bytes(byte_offset, class.size_class());
                    }
                }
                // traced uninitialized slots are owned by the caller until written
                Payload::Uninit => {}
            }
        }

        // publish slot metadata before scanners can see the slot
        if let Some(span) = &span {
            // byte payloads leave arbitrary slot contents behind on release
            if matches!(payload, Payload::Bytes(_)) {
                span.clear_needs_zero(slot_index);
            }

            span.write_reference_bits(slot_index, trace_map);

            // publish reused slots immediately
            if !slot.is_dense {
                span.publish_slot(slot_index);
            }
        }

        // decide whether the worker keeps this cache
        let should_keep_cache = size_class_cache.has_available_slot();

        Ok(Some(ReservedSlot {
            slot: span_slot,
            is_dense: slot.is_dense,
            should_keep_cache,
        }))
    }

    /// Insert one shared large block record.
    fn insert_large_block(
        &self,
        store: &mut HeapState,
        byte_len: usize,
        pages: MemoryRange,
        trace_map: TraceMap,
        drop: Option<DropPlan>,
    ) -> HeapResult<LargeBlockId> {
        // reuse retired large-block ids before growing the table
        let reused_block_id = store.large.free_large_block_ids.pop();
        let block_id = match reused_block_id {
            Some(block_id) => block_id,
            None => store.large.next_unused_large_block_id,
        };

        // zero is reserved for null references
        if block_id == 0 {
            if reused_block_id.is_some() {
                store.large.free_large_block_ids.push(block_id);
            }

            self.memory.release(pages)?;

            return Err(HeapError::representation(
                HeapRepresentationError::InvalidLargeBlockId { id: block_id },
            ));
        }

        // reused ids must address an existing table slot
        let block_id = LargeBlockId::new(block_id);
        let index = block_id.index()?;
        if index > store.large.blocks.len() {
            if reused_block_id.is_some() {
                store.large.free_large_block_ids.push(block_id.id());
            }

            self.memory.release(pages)?;

            return Err(HeapError::representation(
                HeapRepresentationError::InvalidLargeBlockId { id: block_id.id() },
            ));
        }

        let first_offset = pages.offset;

        // materialize the full large range before publishing it
        if let Err(error) = self.memory.materialize(first_offset, pages.byte_len) {
            if reused_block_id.is_some() {
                store.large.free_large_block_ids.push(block_id.id());
            }

            self.memory.release(pages)?;

            return Err(error.into());
        }

        self.map_page_span(store, &pages, |logical_page_index| PageOwner::LargeBlock {
            block_id,
            logical_page_index,
        });

        let block = LargeBlock {
            first_offset,
            byte_len,
            pages,
            trace_map: Arc::new(trace_map),
            drop,
            retained: false,
            empty: false,
            mark_epoch: 0,
        };
        let block = Arc::new(RwLock::new(block));

        // insert or replace the block record
        if index == store.large.blocks.len() {
            store.large.blocks.push(Some(block));
        } else {
            store.large.blocks[index] = Some(block);
        }
        if reused_block_id.is_none() {
            store.large.next_unused_large_block_id += 1;
        }

        Ok(block_id)
    }

    /// Return the page-rounded retained bytes for one shared heap storage block.
    fn round_up_allocation_bytes(&self, byte_len: usize) -> u64 {
        let page_size_bytes = self.page_size_bytes() as u64;
        let byte_len = byte_len as u64;

        byte_len.div_ceil(page_size_bytes) * page_size_bytes
    }
}

/// One shared small span selected for worker-local allocation.
#[derive(Debug, Clone, Copy)]
struct SmallSpanAllocation {
    /// The selected small span index.
    span_index: usize,
    /// Whether the worker cache can use dense cursor allocation.
    is_dense: bool,
}
