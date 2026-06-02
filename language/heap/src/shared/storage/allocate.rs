use std::sync::Arc;

use destack_mir::TraceMap;
use parking_lot::RwLock;

use super::{
    AllocationCache, HeapPageMapEntry, HeapPlace, HeapState, HeapStorage, LargeBlock, LargeBlockId,
    ReservedSlot, SmallSizeClassCache, SmallSpan, SpanList,
};
use crate::allocator::{PageSpan, Slot};
use crate::{
    AllocationPlan, HeapAllocationError, HeapError, HeapRepresentationError, HeapResult, Payload,
    SharedHeapReference, SmallAllocationPlan, SmallSpanClass,
};

impl HeapStorage {
    /// Allocate one shared heap block.
    pub(crate) fn allocate(
        &self,
        cache: &mut AllocationCache,
        layout: &AllocationPlan<'_>,
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
        let storage = self.allocate_place(cache, layout, payload, should_keep_worker_cache)?;
        let reference = self.base_reference_for_place(storage)?;

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
        small: SmallAllocationPlan,
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
        let usage = size_class_cache.cursor.flush_usage(class.size_class);
        if usage.allocation_count() == 0 {
            return;
        }

        self.accounting
            .allocate_many(usage.allocation_count(), usage.allocated_bytes());
    }

    /// Publish worker-local accounting for the cache that owns one reference.
    pub(crate) fn flush_cache_for_reference(
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
            let span_size_bytes = class.size_class * size_class_cache.slot_count;
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
        layout: &AllocationPlan<'_>,
    ) -> HeapResult<i64> {
        // heap blocks must have a physical payload
        if layout.is_empty() {
            return Err(HeapError::invalid_allocation(HeapAllocationError::ZeroSize));
        }

        // small blocks may reuse worker-local or central slots
        if let Some(small) = layout.class.small() {
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

            return Ok(small.class.span_size_bytes as i64);
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

        // release the slot and maybe detach an empty span
        let pages = {
            let slot_index = slot.slot_index();
            let was_full = span.occupied_count() == span.slot_count;

            if !span.release_slot(slot_index) {
                return Err(HeapError::internal("missing small slot"));
            }

            if span.occupied_count() == 0 && span.list.load() != SpanList::Worker {
                span.reset_free_cursor_to_start();
                span.list.store(SpanList::Released);

                let first_offset = span.first_offset;
                let pages = span.take_pages();

                Some((first_offset, pages))
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

                None
            }
        };

        // empty spans return their page span to the cache
        if let Some((first_offset, pages)) = pages {
            self.unmap_page_span(store, first_offset, &pages);
            store
                .page_span_cache
                .release_page_span(&self.allocator, pages)?;
            self.accounting.release_pages(pages, self.page_size_bytes());
        }

        Ok(())
    }

    /// Allocate one shared heap storage for the given payload.
    fn allocate_place(
        &self,
        cache: &mut AllocationCache,
        layout: &AllocationPlan<'_>,
        payload: Payload<'_>,
        should_keep_worker_cache: bool,
    ) -> HeapResult<HeapPlace> {
        // small blocks use worker-local caches and size-class spans
        if let Some(small) = layout.class.small() {
            let cache_index = small.cache_index();
            let class = small.class;
            cache.ensure_small(small);
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
        let pages = self.allocate_large_pages(layout.byte_len)?;

        let mut store = self.state.write();
        let block_id = self.insert_large_block(
            &mut store,
            layout.byte_len,
            layout.alignment,
            pages,
            layout.trace_map.clone(),
        )?;
        let Some(block) = store.large.blocks.get(block_id.index()?).cloned() else {
            return Err(HeapError::internal("missing large block"));
        };
        let first_offset = block.read().first_offset;

        self.initialize_mapped_payload(first_offset, layout.byte_len, payload);
        self.accounting.allocate(layout.byte_len);
        self.accounting.retain_pages(pages, self.page_size_bytes());

        Ok(HeapPlace::LargeBlock(block_id))
    }

    /// Allocate large pages outside the shared state lock.
    fn allocate_large_pages(&self, byte_len: usize) -> HeapResult<PageSpan> {
        // page-cursor cache owns allocator interaction
        let pages = {
            let mut store = self.state.write();

            store
                .page_span_cache
                .allocate_pages(&self.allocator, byte_len)?
        };

        Ok(pages)
    }

    /// Return whether one size class still has one live reusable slot.
    fn has_available_small_slot(&self, store: &HeapState, small: &SmallAllocationPlan) -> bool {
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
    ) -> HeapResult<(usize, bool)> {
        // first reuse a central partial span
        while let Some(span_index) = store.small.partial_spans.entry(*class).or_default().pop() {
            let Some(span) = store.small.spans.get(span_index).cloned() else {
                return Err(HeapError::internal("missing span"));
            };

            // skip stale full-span entries
            if span.occupied_count() < span.slot_count {
                if span.list.load() != SpanList::Central {
                    continue;
                }

                // rematerialize spans whose pages were released
                if span.occupied_count() == 0 && span.pages_empty() {
                    let pages = store
                        .page_span_cache
                        .allocate_pages(&self.allocator, class.span_size_bytes)?;
                    let first_offset = span.first_offset;

                    // materialize the full span before worker-local spans use it
                    self.mapping
                        .materialize(first_offset, class.span_size_bytes)?;

                    self.map_page_span(store, first_offset, &pages, |logical_page_index| {
                        HeapPageMapEntry::SmallSpan {
                            span_index,
                            logical_page_index,
                        }
                    });

                    span.set_pages(pages);
                    self.accounting.retain_pages(pages, self.page_size_bytes());
                }

                span.list.store(SpanList::Worker);
                span.reset_free_cursor();

                return Ok((span_index, false));
            }
        }

        // otherwise map a new span for this size class
        let slot_count = (class.span_size_bytes / class.size_class).max(1);
        let pages = store
            .page_span_cache
            .allocate_pages(&self.allocator, class.span_size_bytes)?;
        let first_offset = self.reserve_address_range(store, class.span_size_bytes)?;

        // materialize the full span before worker-local spans use it
        self.mapping
            .materialize(first_offset, class.span_size_bytes)?;
        self.clear_mapped_bytes(first_offset, class.span_size_bytes);

        let span = SmallSpan::new(first_offset, *class, slot_count, pages, SpanList::Worker);
        let span_index = store.small.spans.len();
        self.map_page_span(store, first_offset, &pages, |logical_page_index| {
            HeapPageMapEntry::SmallSpan {
                span_index,
                logical_page_index,
            }
        });

        store.small.spans.push(Arc::new(span));
        self.accounting.retain_pages(pages, self.page_size_bytes());

        Ok((span_index, true))
    }

    /// Install one small span into one worker-local cache.
    fn install_small_cache(
        &self,
        store: &HeapState,
        size_class_cache: &mut SmallSizeClassCache,
        span_index: usize,
        use_dense_cursor: bool,
    ) -> HeapResult<()> {
        // resolve the selected shared small span
        let Some(span) = store.small.spans.get(span_index).cloned() else {
            return Err(HeapError::internal("missing span"));
        };

        // install the span in the worker-local cache
        size_class_cache.install(span_index, span, use_dense_cursor);

        Ok(())
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
                    if !block.keep_cache {
                        self.flush_reserved_slot(size_class_cache, block);
                        size_class_cache.finish();
                        size_class_cache.clear();
                    }
                    // kept non-dense slots have no cursor flush to publish them
                    else if !block.is_dense {
                        let Some(class) = size_class_cache.class() else {
                            return Err(HeapError::internal("missing cache class"));
                        };

                        self.accounting.allocate(class.size_class);
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
            let (span_index, use_dense_cursor) = self.allocate_small_span(&mut store, class)?;
            let size_class_cache = &mut cache.small[cache_index];

            self.install_small_cache(&store, size_class_cache, span_index, use_dense_cursor)?;
        }

        // initialize one slot from the newly installed worker cache
        let size_class_cache = &mut cache.small[cache_index];
        let block = self.reserve_small_slot(size_class_cache, trace_map, payload)?;
        let Some(block) = block else {
            return Err(HeapError::internal("missing span"));
        };

        let slot = block.slot;
        let should_release_cache = block.keep_cache && !should_keep_worker_cache;

        // publish or retire the newly installed cache
        if !block.keep_cache {
            self.flush_reserved_slot(size_class_cache, block);
            size_class_cache.finish();
            size_class_cache.clear();
        } else if !block.is_dense {
            let Some(class) = size_class_cache.class() else {
                return Err(HeapError::internal("missing cache class"));
            };

            self.accounting.allocate(class.size_class);
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

            self.accounting.allocate(class.size_class);
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
        // no-scan payloads avoid reference metadata writes
        if !trace_map.has_reference() {
            match payload {
                Payload::Zeroed | Payload::Uninit => {
                    return self.reserve_zeroed_noscan_slot(size_class_cache);
                }
                Payload::Bytes(bytes) => {
                    return self.reserve_bytes_noscan_slot(size_class_cache, bytes);
                }
            }
        }

        self.reserve_traced_slot(size_class_cache, trace_map, payload)
    }

    /// Reserve one zeroed no-scan slot from one worker-local cache.
    #[inline(always)]
    fn reserve_zeroed_noscan_slot(
        &self,
        size_class_cache: &mut SmallSizeClassCache,
    ) -> HeapResult<Option<ReservedSlot>> {
        let Some(slot) = size_class_cache.reserve_slot() else {
            size_class_cache.finish();

            return Ok(None);
        };
        let slot_index = slot.slot_index;
        let span = size_class_cache.span.as_ref().cloned();
        let span_slot = size_class_cache.span_slot(slot_index);
        let Some(class) = size_class_cache.class() else {
            return Err(HeapError::internal("missing cache class"));
        };
        let Some(reference) = size_class_cache.reference_for_slot(slot_index) else {
            return Err(HeapError::internal("missing cache class"));
        };

        // clear only slots that previously held arbitrary bytes
        if let Some(span) = &span
            && !slot.is_dense
            && span.take_needs_zero(slot_index)
        {
            self.clear_mapped_bytes(reference.offset(), class.size_class);
        }

        // publish reused slots immediately
        if let Some(span) = &span
            && !slot.is_dense
        {
            span.publish_slot(slot_index);
        }

        // decide whether the worker keeps this cache
        let keep_cache = size_class_cache.has_available_slot();

        Ok(Some(ReservedSlot {
            slot: span_slot,
            is_dense: slot.is_dense,
            keep_cache,
        }))
    }

    /// Reserve one byte-initialized no-scan slot from one worker-local cache.
    #[inline(always)]
    fn reserve_bytes_noscan_slot(
        &self,
        size_class_cache: &mut SmallSizeClassCache,
        bytes: &[u8],
    ) -> HeapResult<Option<ReservedSlot>> {
        let Some(slot) = size_class_cache.reserve_slot() else {
            size_class_cache.finish();

            return Ok(None);
        };
        let slot_index = slot.slot_index;
        let span = size_class_cache.span.as_ref().cloned();
        let span_slot = size_class_cache.span_slot(slot_index);
        let Some(class) = size_class_cache.class() else {
            return Err(HeapError::internal("missing cache class"));
        };
        let Some(reference) = size_class_cache.reference_for_slot(slot_index) else {
            return Err(HeapError::internal("missing cache class"));
        };

        // clear stale tail bytes before copying short payloads
        if let Some(span) = &span
            && bytes.len() < class.size_class
            && span.take_needs_zero(slot_index)
        {
            self.clear_mapped_bytes(reference.offset(), class.size_class);
        }

        // copy the payload before publishing the initialized slot
        self.write_mapped_bytes(reference.offset(), bytes);
        if let Some(span) = &span {
            span.clear_needs_zero(slot_index);
        }

        // publish reused slots immediately
        if let Some(span) = &span
            && !slot.is_dense
        {
            span.publish_slot(slot_index);
        }

        // decide whether the worker keeps this cache
        let keep_cache = size_class_cache.has_available_slot();

        Ok(Some(ReservedSlot {
            slot: span_slot,
            is_dense: slot.is_dense,
            keep_cache,
        }))
    }

    /// Reserve one traced slot from one worker-local shared small cache.
    #[inline(always)]
    fn reserve_traced_slot(
        &self,
        size_class_cache: &mut SmallSizeClassCache,
        trace_map: &TraceMap,
        payload: Payload<'_>,
    ) -> HeapResult<Option<ReservedSlot>> {
        let Some(slot) = size_class_cache.reserve_slot() else {
            size_class_cache.finish();

            return Ok(None);
        };
        let slot_index = slot.slot_index;
        let span = size_class_cache.span.as_ref().cloned();
        let span_slot = size_class_cache.span_slot(slot_index);
        let Some(class) = size_class_cache.class() else {
            return Err(HeapError::internal("missing cache class"));
        };
        let Some(reference) = size_class_cache.reference_for_slot(slot_index) else {
            return Err(HeapError::internal("missing cache class"));
        };
        let mapping_offset = reference.offset();

        match payload {
            Payload::Bytes(bytes) if bytes.len() < class.size_class => {
                self.clear_mapped_bytes(mapping_offset, class.size_class);
                self.write_mapped_bytes(mapping_offset, bytes);
            }
            Payload::Bytes(bytes) => self.write_mapped_bytes(mapping_offset, bytes),
            Payload::Zeroed => {
                if let Some(span) = &span
                    && span.take_needs_zero(slot_index)
                {
                    self.clear_mapped_bytes(mapping_offset, class.size_class);
                }
            }
            Payload::Uninit => {}
        }

        if let Some(span) = &span {
            if matches!(payload, Payload::Bytes(_)) {
                span.clear_needs_zero(slot_index);
            }

            span.write_reference_bits(slot_index, trace_map);
        }

        // publish reused slots immediately
        if let Some(span) = &span
            && !slot.is_dense
        {
            span.publish_slot(slot_index);
        }

        // decide whether the worker keeps this cache
        let keep_cache = size_class_cache.has_available_slot();

        Ok(Some(ReservedSlot {
            slot: span_slot,
            is_dense: slot.is_dense,
            keep_cache,
        }))
    }

    /// Insert one shared large block record.
    fn insert_large_block(
        &self,
        store: &mut HeapState,
        byte_len: usize,
        alignment: usize,
        pages: PageSpan,
        trace_map: TraceMap,
    ) -> HeapResult<LargeBlockId> {
        // reuse retired large-block ids before growing the table
        let (block_id, reused_block_id) =
            if let Some(block_id) = store.large.free_large_block_ids.pop() {
                (block_id, true)
            } else {
                let block_id = store.large.next_unused_large_block_id;
                let next_block_id = store.large.next_unused_large_block_id + 1;

                store.large.next_unused_large_block_id = next_block_id;
                (block_id, false)
            };

        // zero is reserved for null references
        if block_id == 0 {
            if reused_block_id {
                store.large.free_large_block_ids.push(block_id);
            }

            store
                .page_span_cache
                .release_page_span(&self.allocator, pages)?;

            return Err(HeapError::representation(
                HeapRepresentationError::InvalidLargeBlockId { id: block_id },
            ));
        }

        // reused ids must address an existing table slot
        let block_id = LargeBlockId::new(block_id);
        let index = block_id.index()?;
        if index > store.large.blocks.len() {
            if reused_block_id {
                store.large.free_large_block_ids.push(block_id.id());
            }

            store
                .page_span_cache
                .release_page_span(&self.allocator, pages)?;

            return Err(HeapError::representation(
                HeapRepresentationError::InvalidLargeBlockId { id: block_id.id() },
            ));
        }

        let first_offset = match self.reserve_address_range_aligned(
            store,
            pages.len() * self.allocator.page_size_bytes(),
            alignment,
        ) {
            Ok(first_offset) => first_offset,
            Err(error) => {
                store
                    .page_span_cache
                    .release_page_span(&self.allocator, pages)?;

                return Err(error);
            }
        };

        // materialize the full large range before publishing it
        if let Err(error) = self
            .mapping
            .materialize(first_offset, pages.len() * self.allocator.page_size_bytes())
        {
            store
                .page_span_cache
                .release_page_span(&self.allocator, pages)?;

            return Err(error.into());
        }

        self.map_page_span(store, first_offset, &pages, |logical_page_index| {
            HeapPageMapEntry::LargeBlock {
                block_id,
                logical_page_index,
            }
        });

        let block = LargeBlock {
            is_live: true,
            first_offset,
            byte_len,
            pages,
            trace_map,
            mark_epoch: 0,
        };
        let block = Arc::new(RwLock::new(block));

        // insert or replace the block record
        if index == store.large.blocks.len() {
            store.large.blocks.push(block);
        } else {
            store.large.blocks[index] = block;
        }

        Ok(block_id)
    }

    /// Return the page-rounded retained bytes for one shared heap storage block.
    fn round_up_allocation_bytes(&self, byte_len: usize) -> u64 {
        let page_size_bytes = self.page_size_bytes() as u64;
        let byte_len = byte_len as u64;

        byte_len.div_ceil(page_size_bytes) * page_size_bytes
    }

    /// Reserve one logical shared heap storage byte range.
    fn reserve_address_range(&self, store: &mut HeapState, byte_len: usize) -> HeapResult<usize> {
        self.reserve_address_range_aligned(store, byte_len, self.allocator.page_size_bytes())
    }

    /// Reserve one logical shared heap storage byte range with the given alignment.
    fn reserve_address_range_aligned(
        &self,
        store: &mut HeapState,
        byte_len: usize,
        alignment: usize,
    ) -> HeapResult<usize> {
        debug_assert!(store.next_offset <= self.mapping.byte_len());

        // align shared ranges to page boundaries or stricter layout alignment
        let alignment = alignment.max(self.allocator.page_size_bytes());
        let first_offset = align_up(store.next_offset, alignment);
        let next_offset = first_offset + byte_len;

        // reject ranges outside the reserved shared address space
        if next_offset > self.mapping.byte_len() {
            return Err(HeapError::InvalidByteRange {
                start: first_offset,
                len: byte_len,
                capacity: self.mapping.byte_len(),
            });
        }

        store.next_offset = next_offset;

        Ok(first_offset)
    }

    /// Initialize one mapped payload range.
    #[inline(always)]
    fn initialize_mapped_payload(
        &self,
        offset: usize,
        clear_byte_len: usize,
        payload: Payload<'_>,
    ) {
        match payload {
            Payload::Bytes(bytes) if bytes.len() < clear_byte_len => {
                self.clear_mapped_bytes(offset, clear_byte_len);
                self.write_mapped_bytes(offset, bytes);
            }
            Payload::Bytes(bytes) => self.write_mapped_bytes(offset, bytes),
            Payload::Zeroed => self.clear_mapped_bytes(offset, clear_byte_len),
            Payload::Uninit => {}
        }
    }

    /// Write bytes into one mapped payload range.
    #[inline(always)]
    fn write_mapped_bytes(&self, offset: usize, bytes: &[u8]) {
        // SAFETY: block paths materialize the destination before publishing it
        unsafe {
            self.mapping.write_mapped_bytes(offset, bytes);
        }
    }

    /// Clear one mapped payload range.
    #[inline(always)]
    fn clear_mapped_bytes(&self, offset: usize, byte_len: usize) {
        let address = self.mapping.base_address() + offset;

        // SAFETY: block paths materialize the destination before publishing it
        unsafe {
            std::ptr::write_bytes(address as *mut u8, 0, byte_len);
        }
    }
}

/// Return the offset rounded up to one block boundary.
fn align_up(byte_len: usize, alignment_bytes: usize) -> usize {
    let alignment_bytes = alignment_bytes.max(1);

    byte_len.div_ceil(alignment_bytes) * alignment_bytes
}
