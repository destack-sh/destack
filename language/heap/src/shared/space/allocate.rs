use std::sync::Arc;

use destack_mir::TraceMap;
use parking_lot::RwLock;

use super::{
    SharedAllocationCache, SharedHeapPageMapEntry, SharedHeapPlace, SharedHeapSpace,
    SharedHeapState, SharedLargeAllocation, SharedLargeAllocationId, SharedSmallSpan,
    SmallAllocation, SmallBucket, SmallRun, SpanList,
};
use crate::allocator::{PageRun, SpanSlot};
use crate::{
    AllocationPlan, HeapError, HeapResult, Payload, SharedHeapReference, SmallAllocationPlan,
    SmallSpanClass,
};

impl SharedHeapSpace {
    /// Allocate one shared managed heap allocation.
    pub fn allocate(
        &self,
        cache: &mut SharedAllocationCache,
        layout: &AllocationPlan<'_>,
        payload: Payload<'_>,
        should_keep_worker_bucket: bool,
    ) -> HeapResult<SharedHeapReference> {
        // heap allocations must have a physical payload
        if layout.is_empty() {
            return Err(HeapError::ZeroSizeAllocation);
        }

        // caller-provided bytes must exactly initialize the layout
        if let Some(actual) = payload.byte_len()
            && actual != layout.byte_len
        {
            return Err(HeapError::InvalidAllocationBytes {
                expected: layout.byte_len,
                actual,
            });
        }

        // allocate the payload bytes
        let place = self.allocate_place(cache, layout, payload, should_keep_worker_bucket)?;
        let reference = self.base_reference_for_place(place)?;

        // publish initialized shared references to an active mark cycle
        let has_shared_reference = payload.byte_len().is_some() && layout.has_shared_reference;
        self.publish_shared_allocation(reference, has_shared_reference)?;

        Ok(reference)
    }

    /// Try to allocate one small payload from a worker-local bucket.
    #[inline(always)]
    pub(crate) fn try_allocate_worker_small(
        &self,
        cache: &mut SharedAllocationCache,
        layout: &AllocationPlan<'_>,
        payload: Payload<'_>,
    ) -> HeapResult<Option<SharedHeapReference>> {
        // worker buckets only handle small allocations
        let Some(small) = layout.class.small() else {
            return Ok(None);
        };
        let bucket_index = small.bucket_index;

        // inactive buckets have no local slots
        let run = &mut cache.runs[bucket_index];
        let bucket = &mut cache.small[bucket_index];
        if !bucket.is_active() {
            return Ok(None);
        }
        if bucket.class != small.class {
            return Ok(None);
        }

        // no-scan zeroed allocations are just a dense-run bump
        if layout.is_noscan
            && matches!(payload, Payload::Zeroed)
            && let Some(reference) = run.reserve_reference(small.class.size_class)
        {
            bucket.publish_run(run);
            if !bucket.has_available_slot(run) {
                bucket.finish(run);
                bucket.clear(run);
            }

            self.accounting.allocate(small.class.size_class);

            return Ok(Some(reference));
        }

        // no-scan byte allocations write directly into fresh dense-run slots
        if layout.is_noscan
            && let Payload::Bytes(bytes) = payload
            && let Some(reference) = run.reserve_reference(small.class.size_class)
        {
            self.write_mapped_bytes(reference.offset(), bytes);

            bucket.publish_run(run);
            if !bucket.has_available_slot(run) {
                bucket.finish(run);
                bucket.clear(run);
            }

            self.accounting.allocate(small.class.size_class);

            return Ok(Some(reference));
        }

        // remaining small allocations need span metadata updates
        let allocation = match payload {
            Payload::Zeroed if layout.is_noscan => {
                self.try_allocate_zeroed_noscan_slot(bucket, run)?
            }
            Payload::Bytes(bytes) if layout.is_noscan => {
                self.try_allocate_bytes_noscan_slot(bucket, run, bytes)?
            }
            _ => self.try_allocate_small_slot(bucket, run, layout.trace_map, payload)?,
        };
        let Some(allocation) = allocation else {
            // exhausted buckets leave worker ownership immediately
            bucket.finish(run);
            bucket.clear(run);

            return Ok(None);
        };

        // full buckets leave worker ownership immediately
        if !allocation.keep_bucket {
            bucket.finish(run);
            bucket.clear(run);
        }

        self.accounting.allocate(small.class.size_class);

        Ok(Some(allocation.reference))
    }

    /// Publish and retire every worker-local small run.
    pub(crate) fn flush_allocation_cache(&self, cache: &mut SharedAllocationCache) {
        let mut store = self.state.write();

        // publish each worker-owned bucket back to central state
        for (bucket_index, bucket) in cache.small.iter_mut().enumerate() {
            let run = &mut cache.runs[bucket_index];
            let Some(span) = bucket.span.clone() else {
                continue;
            };

            bucket.publish_run(run);

            // partial spans return to the central partial list
            if bucket.has_available_slot(run) {
                span.list.store(SpanList::Central);
                store
                    .small
                    .partial_spans
                    .entry(bucket.class)
                    .or_default()
                    .push(bucket.span_index as usize);
                bucket.clear(run);

                continue;
            }

            // full spans only need a list transition
            span.list.store(SpanList::Full);
            bucket.clear(run);
        }
    }

    /// Return the projected retained-byte delta for one shared heap layout.
    pub(crate) fn retained_byte_delta(
        &self,
        cache: &SharedAllocationCache,
        layout: &AllocationPlan<'_>,
    ) -> HeapResult<i64> {
        // heap allocations must have a physical payload
        if layout.is_empty() {
            return Err(HeapError::ZeroSizeAllocation);
        }

        // small allocations may reuse worker-local or central slots
        if let Some(small) = layout.class.small() {
            let bucket_index = small.bucket_index;
            let bucket = &cache.small[bucket_index];
            let run = &cache.runs[bucket_index];
            if bucket.span.as_ref().is_some_and(|span| {
                span.list.load() == SpanList::Worker && bucket.has_available_slot(run)
            }) {
                return Ok(0);
            }

            let store = self.state.read();
            if self.has_available_small_slot(&store, &small) {
                return Ok(0);
            }

            return Ok(small.class.span_bytes as i64);
        }

        // large allocations retain whole pages
        Ok(self.round_up_allocation_bytes(layout.byte_len) as i64)
    }

    /// Release one shared heap small slot.
    pub(crate) fn release_small_slot(
        &self,
        store: &mut SharedHeapState,
        slot: SpanSlot,
    ) -> HeapResult<()> {
        // resolve the owning span
        let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
            return Err(HeapError::MissingSpan {
                span_index: slot.span_index(),
            });
        };

        // release the slot and maybe detach an empty span
        let pages = {
            let slot_index = slot.slot_index();
            let was_full = span.occupied_count() == span.slot_count;

            if !span.release_slot(slot_index) {
                return Err(HeapError::MissingSmallSlot {
                    span_index: slot.span_index(),
                    slot_index,
                });
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

        // empty spans return their page run to the cache
        if let Some((first_offset, pages)) = pages {
            self.unmap_page_run(store, first_offset, &pages);
            store
                .page_run_cache
                .release_page_run(&self.allocator, pages)?;
            self.accounting.release_pages(pages, self.page_bytes());
        }

        Ok(())
    }

    /// Allocate one shared heap place for the given payload.
    fn allocate_place(
        &self,
        cache: &mut SharedAllocationCache,
        layout: &AllocationPlan<'_>,
        payload: Payload<'_>,
        should_keep_worker_bucket: bool,
    ) -> HeapResult<SharedHeapPlace> {
        // small allocations use worker buckets and size-class spans
        if let Some(small) = layout.class.small() {
            let bucket_index = small.bucket_index;
            let class = small.class;
            let slot = self.allocate_small(
                cache,
                bucket_index,
                &class,
                layout.trace_map,
                payload,
                should_keep_worker_bucket,
            )?;

            self.accounting.allocate(class.size_class);

            return Ok(SharedHeapPlace::Small(slot));
        }

        // large allocations reserve whole page runs
        let pages = self.allocate_large_pages(layout.byte_len)?;

        let mut store = self.state.write();
        let allocation_id = self.insert_large_allocation(
            &mut store,
            layout.byte_len,
            layout.alignment,
            pages,
            layout.trace_map.clone(),
        )?;
        let Some(allocation) = store.large.allocations.get(allocation_id.index()?).cloned() else {
            return Err(HeapError::MissingLargeAllocation {
                allocation_id: allocation_id.id(),
            });
        };
        let first_offset = allocation.read().first_offset;

        self.initialize_mapped_payload(first_offset, layout.byte_len, payload);
        self.accounting.allocate(layout.byte_len);
        self.accounting.retain_pages(pages, self.page_bytes());

        Ok(SharedHeapPlace::Large(allocation_id))
    }

    /// Allocate large pages outside the shared state lock.
    fn allocate_large_pages(&self, byte_len: usize) -> HeapResult<PageRun> {
        // page-run cache owns allocator interaction
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
        small: &SmallAllocationPlan,
    ) -> bool {
        store
            .small
            .partial_spans
            .get(&small.class)
            .is_some_and(|spans| !spans.is_empty())
    }

    /// Allocate or reuse one non-full shared heap span for the given size class.
    fn allocate_small_span(
        &self,
        store: &mut SharedHeapState,
        class: &SmallSpanClass,
    ) -> HeapResult<(usize, bool)> {
        // first reuse a central partial span
        while let Some(span_index) = store.small.partial_spans.entry(*class).or_default().pop() {
            let Some(span) = store.small.spans.get(span_index).cloned() else {
                return Err(HeapError::MissingSpan { span_index });
            };

            // skip stale full-span entries
            if span.occupied_count() < span.slot_count {
                if span.list.load() != SpanList::Central {
                    continue;
                }

                // rematerialize spans whose pages were released
                if span.occupied_count() == 0 && span.pages_empty() {
                    let pages = store
                        .page_run_cache
                        .allocate_pages(&self.allocator, class.span_bytes)?;
                    let first_offset = span.first_offset;

                    // materialize the full span before worker-local runs use it
                    self.mapping.materialize(first_offset, class.span_bytes)?;

                    self.map_page_run(store, first_offset, &pages, |logical_page_index| {
                        SharedHeapPageMapEntry::Small {
                            span_index,
                            logical_page_index,
                        }
                    });

                    span.set_pages(pages);
                    self.accounting.retain_pages(pages, self.page_bytes());
                }

                span.list.store(SpanList::Worker);
                span.reset_free_cursor();

                return Ok((span_index, false));
            }
        }

        // otherwise map a new span for this size class
        let slot_count = (class.span_bytes / class.size_class).max(1);
        let pages = store
            .page_run_cache
            .allocate_pages(&self.allocator, class.span_bytes)?;
        let first_offset = self.reserve_space_range(store, class.span_bytes)?;

        // materialize the full span before worker-local runs use it
        self.mapping.materialize(first_offset, class.span_bytes)?;

        let span = SharedSmallSpan::new(first_offset, *class, slot_count, pages, SpanList::Worker);
        let span_index = store.small.spans.len();
        self.map_page_run(store, first_offset, &pages, |logical_page_index| {
            SharedHeapPageMapEntry::Small {
                span_index,
                logical_page_index,
            }
        });

        store.small.spans.push(Arc::new(span));
        self.accounting.retain_pages(pages, self.page_bytes());

        Ok((span_index, true))
    }

    /// Install one small span into one worker-local bucket.
    fn install_small_bucket(
        &self,
        store: &SharedHeapState,
        bucket: &mut SmallBucket,
        run: &mut SmallRun,
        span_index: usize,
        use_dense_run: bool,
    ) -> HeapResult<()> {
        // resolve the selected shared small span
        let Some(span) = store.small.spans.get(span_index).cloned() else {
            return Err(HeapError::MissingSpan { span_index });
        };

        // install the span in the worker-local bucket
        bucket.install(run, span_index, span, use_dense_run);

        Ok(())
    }

    /// Allocate one shared heap small slot from one explicit initialization source.
    fn allocate_small(
        &self,
        cache: &mut SharedAllocationCache,
        bucket_index: usize,
        class: &SmallSpanClass,
        trace_map: &TraceMap,
        payload: Payload<'_>,
        should_keep_worker_bucket: bool,
    ) -> HeapResult<SpanSlot> {
        // reuse the worker-owned span when it still has a slot
        if cache.small[bucket_index].is_active() {
            if cache.small[bucket_index].class != *class {
                self.release_cache_bucket(cache, bucket_index);
            }
        }

        // reuse the worker-owned span when it still has a matching slot
        if cache.small[bucket_index].is_active() {
            let mut should_release_bucket = false;
            let allocated_slot = {
                let run = &mut cache.runs[bucket_index];
                let bucket = &mut cache.small[bucket_index];
                let allocation = self.try_allocate_small_slot(bucket, run, trace_map, payload)?;

                if let Some(allocation) = allocation {
                    let slot = allocation.slot;

                    // exhausted buckets leave worker ownership immediately
                    if !allocation.keep_bucket {
                        bucket.finish(run);
                        bucket.clear(run);
                    }
                    // worker-run allocations must still publish occupied metadata
                    else if should_keep_worker_bucket {
                        bucket.publish_run(run);
                    }
                    // active marking cannot leave free slots hidden in the worker
                    else {
                        should_release_bucket = true;
                    }

                    Some(slot)
                } else {
                    bucket.finish(run);
                    bucket.clear(run);

                    None
                }
            };

            // centralize a bucket whose free slots must remain visible
            if should_release_bucket {
                self.release_cache_bucket(cache, bucket_index);
            }

            // return the slot if the active bucket produced one
            if let Some(slot) = allocated_slot {
                return Ok(slot);
            }
        }

        // acquire a central span or map a new one for this size class
        {
            let mut store = self.state.write();
            let (span_index, use_dense_run) = self.allocate_small_span(&mut store, class)?;
            let run = &mut cache.runs[bucket_index];
            let bucket = &mut cache.small[bucket_index];

            self.install_small_bucket(&store, bucket, run, span_index, use_dense_run)?;
        }

        // initialize one slot from the newly installed worker bucket
        let run = &mut cache.runs[bucket_index];
        let bucket = &mut cache.small[bucket_index];
        let allocation = match payload {
            Payload::Zeroed if !trace_map.has_reference() => {
                self.try_allocate_zeroed_noscan_slot(bucket, run)?
            }
            Payload::Bytes(bytes) if !trace_map.has_reference() => {
                self.try_allocate_bytes_noscan_slot(bucket, run, bytes)?
            }
            _ => self.try_allocate_small_slot(bucket, run, trace_map, payload)?,
        };
        let Some(allocation) = allocation else {
            return Err(HeapError::MissingSpan {
                span_index: bucket.span_index as usize,
            });
        };

        let slot = allocation.slot;
        let should_release_bucket = allocation.keep_bucket && !should_keep_worker_bucket;

        // publish or retire the newly installed bucket
        if allocation.keep_bucket && should_keep_worker_bucket {
            bucket.publish_run(run);
        } else if !allocation.keep_bucket {
            bucket.finish(run);
            bucket.clear(run);
        }

        // return published buckets to the central partial list
        if should_release_bucket {
            self.release_cache_bucket(cache, bucket_index);
        }

        Ok(slot)
    }

    /// Return one worker-local bucket to the central list.
    fn release_cache_bucket(&self, cache: &mut SharedAllocationCache, bucket_index: usize) {
        // take the worker-owned span
        let mut store = self.state.write();
        let run = &mut cache.runs[bucket_index];
        let bucket = &mut cache.small[bucket_index];
        let Some(span) = bucket.span.clone() else {
            return;
        };

        // publish occupied slots before list transition
        bucket.publish_run(run);

        // keep reusable spans on the central partial list
        if bucket.has_available_slot(run) {
            span.list.store(SpanList::Central);
            store
                .small
                .partial_spans
                .entry(bucket.class)
                .or_default()
                .push(bucket.span_index as usize);
        } else {
            span.list.store(SpanList::Full);
        }

        bucket.clear(run);
    }

    /// Try to allocate one zeroed no-scan slot from one worker-local bucket.
    #[inline(always)]
    fn try_allocate_zeroed_noscan_slot(
        &self,
        bucket: &mut SmallBucket,
        run: &mut SmallRun,
    ) -> HeapResult<Option<SmallAllocation>> {
        let Some(slot) = bucket.reserve_slot(run) else {
            bucket.finish(run);

            return Ok(None);
        };
        let slot_index = slot.slot_index;
        let span = bucket.span.as_ref().cloned();
        let span_slot = bucket.span_slot(slot_index);
        let reference = bucket.reference_for_slot(slot_index);

        // clear only slots that previously held arbitrary bytes
        if let Some(span) = &span
            && !slot.is_dense
            && span.take_needs_zero(slot_index)
        {
            self.clear_mapped_bytes(reference.offset(), bucket.class.size_class);
        }

        // publish initialized slots before returning the reference
        if slot.is_dense {
            bucket.publish_run(run);
        }
        if let Some(span) = &span
            && !slot.is_dense
        {
            span.publish_slot(slot_index);
        }

        // decide whether the worker keeps this bucket
        let keep_bucket = bucket.has_available_slot(run);
        if !keep_bucket {
            bucket.finish(run);
        }

        Ok(Some(SmallAllocation {
            slot: span_slot,
            reference,
            keep_bucket,
        }))
    }

    /// Try to allocate one byte-initialized no-scan slot from one worker-local bucket.
    #[inline(always)]
    fn try_allocate_bytes_noscan_slot(
        &self,
        bucket: &mut SmallBucket,
        run: &mut SmallRun,
        bytes: &[u8],
    ) -> HeapResult<Option<SmallAllocation>> {
        let Some(slot) = bucket.reserve_slot(run) else {
            bucket.finish(run);

            return Ok(None);
        };
        let slot_index = slot.slot_index;
        let span = bucket.span.as_ref().cloned();
        let span_slot = bucket.span_slot(slot_index);
        let reference = bucket.reference_for_slot(slot_index);

        // clear stale tail bytes before copying short payloads
        if let Some(span) = &span
            && bytes.len() < bucket.class.size_class
            && span.take_needs_zero(slot_index)
        {
            self.clear_mapped_bytes(reference.offset(), bucket.class.size_class);
        }

        // copy the payload before publishing the initialized slot
        self.write_mapped_bytes(reference.offset(), bytes);
        if let Some(span) = &span {
            span.clear_needs_zero(slot_index);
        }

        // publish initialized slots before returning the reference
        if slot.is_dense {
            bucket.publish_run(run);
        }
        if let Some(span) = &span
            && !slot.is_dense
        {
            span.publish_slot(slot_index);
        }

        // decide whether the worker keeps this bucket
        let keep_bucket = bucket.has_available_slot(run);
        if !keep_bucket {
            bucket.finish(run);
        }

        Ok(Some(SmallAllocation {
            slot: span_slot,
            reference,
            keep_bucket,
        }))
    }

    /// Try to allocate one slot from one worker-local shared small bucket.
    #[inline(always)]
    fn try_allocate_small_slot(
        &self,
        bucket: &mut SmallBucket,
        run: &mut SmallRun,
        trace_map: &TraceMap,
        payload: Payload<'_>,
    ) -> HeapResult<Option<SmallAllocation>> {
        let Some(slot) = bucket.reserve_slot(run) else {
            bucket.finish(run);

            return Ok(None);
        };
        let slot_index = slot.slot_index;
        let span = bucket.span.as_ref().cloned();
        let span_slot = bucket.span_slot(slot_index);
        let reference = bucket.reference_for_slot(slot_index);
        let mapping_offset = reference.offset();

        match payload {
            Payload::Bytes(bytes) if bytes.len() < bucket.class.size_class => {
                self.clear_mapped_bytes(mapping_offset, bucket.class.size_class);
                self.write_mapped_bytes(mapping_offset, bytes);
            }
            Payload::Bytes(bytes) => self.write_mapped_bytes(mapping_offset, bytes),
            Payload::Zeroed => {
                if let Some(span) = &span
                    && span.take_needs_zero(slot_index)
                {
                    self.clear_mapped_bytes(mapping_offset, bucket.class.size_class);
                }
            }
        }

        if let Some(span) = &span {
            if matches!(payload, Payload::Bytes(_)) {
                span.clear_needs_zero(slot_index);
            }

            span.write_reference_bits(slot_index, trace_map);
        }

        // publish initialized slots before returning the reference
        if slot.is_dense {
            bucket.publish_run(run);
        }
        if let Some(span) = &span
            && !slot.is_dense
        {
            span.publish_slot(slot_index);
        }

        // decide whether the worker keeps this bucket
        let keep_bucket = bucket.has_available_slot(run);
        if !keep_bucket {
            bucket.finish(run);
        }

        Ok(Some(SmallAllocation {
            slot: span_slot,
            reference,
            keep_bucket,
        }))
    }

    /// Insert one shared large allocation record.
    fn insert_large_allocation(
        &self,
        store: &mut SharedHeapState,
        byte_len: usize,
        alignment: usize,
        pages: PageRun,
        trace_map: TraceMap,
    ) -> HeapResult<SharedLargeAllocationId> {
        // reuse retired large-allocation ids before growing the table
        let (allocation_id, reused_allocation_id) =
            if let Some(allocation_id) = store.large.free_large_allocation_ids.pop() {
                (allocation_id, true)
            } else {
                let allocation_id = store.large.next_unused_large_allocation_id;
                let next_allocation_id = store.large.next_unused_large_allocation_id + 1;

                store.large.next_unused_large_allocation_id = next_allocation_id;
                (allocation_id, false)
            };

        // zero is reserved for null references
        if allocation_id == 0 {
            if reused_allocation_id {
                store.large.free_large_allocation_ids.push(allocation_id);
            }

            store
                .page_run_cache
                .release_page_run(&self.allocator, pages)?;

            return Err(HeapError::InvalidLargeAllocationId { id: allocation_id });
        }

        // reused ids must address an existing table slot
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

        let first_offset = match self.reserve_space_range_aligned(
            store,
            pages.len() * self.allocator.page_bytes(),
            alignment,
        ) {
            Ok(first_offset) => first_offset,
            Err(error) => {
                store
                    .page_run_cache
                    .release_page_run(&self.allocator, pages)?;

                return Err(error);
            }
        };

        // materialize the full large range before publishing it
        if let Err(error) = self
            .mapping
            .materialize(first_offset, pages.len() * self.allocator.page_bytes())
        {
            store
                .page_run_cache
                .release_page_run(&self.allocator, pages)?;

            return Err(error.into());
        }

        self.map_page_run(store, first_offset, &pages, |logical_page_index| {
            SharedHeapPageMapEntry::Large {
                allocation_id,
                logical_page_index,
            }
        });

        let allocation = SharedLargeAllocation {
            is_live: true,
            first_offset,
            byte_len,
            pages,
            trace_map,
            mark_epoch: 0,
        };
        let allocation = Arc::new(RwLock::new(allocation));

        // insert or replace the allocation record
        if index == store.large.allocations.len() {
            store.large.allocations.push(allocation);
        } else {
            store.large.allocations[index] = allocation;
        }

        Ok(allocation_id)
    }

    /// Return the page-rounded retained bytes for one shared heap-space allocation.
    fn round_up_allocation_bytes(&self, byte_len: usize) -> u64 {
        let page_bytes = self.page_bytes() as u64;
        let byte_len = byte_len as u64;

        byte_len.div_ceil(page_bytes) * page_bytes
    }

    /// Reserve one logical shared heap-space byte range.
    fn reserve_space_range(
        &self,
        store: &mut SharedHeapState,
        byte_len: usize,
    ) -> HeapResult<usize> {
        self.reserve_space_range_aligned(store, byte_len, self.allocator.page_bytes())
    }

    /// Reserve one logical shared heap-space byte range with the given alignment.
    fn reserve_space_range_aligned(
        &self,
        store: &mut SharedHeapState,
        byte_len: usize,
        alignment: usize,
    ) -> HeapResult<usize> {
        debug_assert!(store.next_offset <= self.mapping.byte_len());

        // align shared ranges to page boundaries or stricter layout alignment
        let alignment = alignment.max(self.allocator.page_bytes());
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
        }
    }

    /// Write bytes into one mapped payload range.
    #[inline(always)]
    fn write_mapped_bytes(&self, offset: usize, bytes: &[u8]) {
        // SAFETY: allocation paths materialize the destination before publishing it
        unsafe {
            self.mapping.write_mapped_bytes(offset, bytes);
        }
    }

    /// Clear one mapped payload range.
    #[inline(always)]
    fn clear_mapped_bytes(&self, offset: usize, byte_len: usize) {
        let address = self.mapping.base_address() + offset;

        // SAFETY: allocation paths materialize the destination before publishing it
        unsafe {
            std::ptr::write_bytes(address as *mut u8, 0, byte_len);
        }
    }
}

/// Return the offset rounded up to one allocation boundary.
fn align_up(byte_len: usize, alignment_bytes: usize) -> usize {
    let alignment_bytes = alignment_bytes.max(1);

    byte_len.div_ceil(alignment_bytes) * alignment_bytes
}
