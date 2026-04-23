use std::collections::BTreeMap;

use destack_mir::ReferenceMap;

use super::Promotion;
use crate::local::space::{
    GcKind, GcStats, HeapLocation, HeapPageOwner, HeapSpace, HeapStorage, HeapYoungId, LargeEntryId,
};
use crate::{
    HeapError, HeapReference, HeapResult, ScanSource, SharedHeapReference, TraceQueue,
    slot_reference_map, visit_heap_references_in_reader, visit_heap_references_in_reader_range,
    visit_shared_references_in_reader,
};

/// Collector queue for local heap references.
type HeapTraceQueue = TraceQueue<HeapReference>;

impl HeapSpace {
    /// Return the currently live heap references.
    pub fn live_references(&self) -> HeapResult<Vec<HeapReference>> {
        let mut references = Vec::new();

        for entry in &self.young.entries {
            if !entry.is_live {
                continue;
            }

            let entry_offset = self.young_entry_offset(entry);
            let base_address = self
                .allocator()
                .page_view_ptr(&self.young.pages, entry_offset)?
                as *mut u8 as usize;

            references.push(HeapReference::new(base_address));
        }

        for span in self.small.spans.iter() {
            for slot_index in 0..span.slot_count {
                if !span.occupied.contains(slot_index) {
                    continue;
                }

                let slot_offset = span.class.size_class.checked_mul(slot_index).ok_or(
                    HeapError::InvariantOverflow {
                        context: "heap slot base offset",
                    },
                )?;
                let base_address =
                    self.allocator().page_view_ptr(&span.pages, slot_offset)? as *mut u8 as usize;

                references.push(HeapReference::new(base_address));
            }
        }

        for entry in self.large.entries.iter() {
            if !entry.is_live {
                continue;
            }

            let base_address = self.allocator().page_view_ptr(&entry.pages, 0)? as *mut u8 as usize;
            references.push(HeapReference::new(base_address));
        }

        Ok(references)
    }

    /// Start one incremental local-to-shared edge scan.
    pub(crate) fn start_shared_edge_scan(&mut self) {
        self.is_scanning_shared_edges = true;
        self.shared_edge_cursor = 0;
        self.shared_edge_queue.clear();
        self.shared_edge_pending.clear();
    }

    /// Return whether the current local-to-shared edge scan is fully drained.
    pub(crate) fn shared_edge_scan_idle(&self) -> bool {
        !self.is_scanning_shared_edges
            || (self.shared_edge_cursor >= self.shared_edge_roots.len()
                && self.shared_edge_queue.is_empty())
    }

    /// Finish the current local-to-shared edge scan.
    pub(crate) fn finish_shared_edge_scan(&mut self) {
        self.is_scanning_shared_edges = false;
        self.shared_edge_cursor = 0;
        self.shared_edge_queue.clear();
        self.shared_edge_pending.clear();
    }

    /// Scan bounded local-to-shared edge work into the provided root buffer.
    pub(crate) fn scan_shared_edge_step(
        &mut self,
        roots: &mut Vec<SharedHeapReference>,
        work_items: usize,
    ) -> HeapResult<usize> {
        if !self.is_scanning_shared_edges || work_items == 0 {
            return Ok(0);
        }

        let mut work_done = 0usize;

        // drain queued rescans first
        while work_done < work_items {
            let Some(reference) = self.shared_edge_queue.pop() else {
                break;
            };

            self.clear_shared_edge_pending(reference)?;
            self.trace_shared_edges(reference, roots)?;
            work_done += 1;
        }

        // then continue the tracked shared-edge walk
        while work_done < work_items {
            let Some(reference) = self.next_shared_edge_root()? else {
                break;
            };

            self.trace_shared_edges(reference, roots)?;
            work_done += 1;
        }

        Ok(work_done)
    }

    /// Queue one local reference for one later shared-edge rescan.
    pub(crate) fn queue_shared_reference(&mut self, reference: HeapReference) -> HeapResult<()> {
        if !self.is_scanning_shared_edges || !self.reference_has_shared_roots(reference)? {
            return Ok(());
        }

        if self.shared_edge_pending.contains(&reference) {
            return Ok(());
        }

        self.shared_edge_pending.insert(reference);
        self.shared_edge_queue.push(reference);

        Ok(())
    }

    /// Return the next tracked local reference that may contain shared edges.
    fn next_shared_edge_root(&mut self) -> HeapResult<Option<HeapReference>> {
        while self.shared_edge_cursor < self.shared_edge_roots.len() {
            let index = self.shared_edge_cursor;
            self.shared_edge_cursor += 1;

            return Ok(self.shared_edge_roots.get(index).copied());
        }

        Ok(None)
    }

    /// Return whether one live local reference may contain shared heap roots.
    pub(crate) fn reference_has_shared_roots(&self, reference: HeapReference) -> HeapResult<bool> {
        let Some(location) = self.resolve_location(reference) else {
            return Ok(false);
        };
        let reference_map = self.location_reference_map(location.storage)?;

        Ok(reference_map.has_shared_reference())
    }

    /// Clear the queued bit for one local shared-edge rescan.
    fn clear_shared_edge_pending(&mut self, reference: HeapReference) -> HeapResult<()> {
        self.shared_edge_pending.remove(&reference);

        Ok(())
    }

    /// Trace shared heap roots from one local heap reference.
    fn trace_shared_edges(
        &mut self,
        reference: HeapReference,
        roots: &mut Vec<SharedHeapReference>,
    ) -> HeapResult<()> {
        let Some(location) = self.resolve_location(reference) else {
            return Ok(());
        };
        let reference_map = self
            .location_reference_map(location.storage)
            .map_err(|error| HeapError::HeapScanFailed {
                source: ScanSource::Reference(reference),
                error: Box::new(error),
            })?;

        if !reference_map.has_shared_reference() {
            return Ok(());
        }

        let mut first_reader_error = None;
        let result = visit_shared_references_in_reader(
            &reference_map,
            |start, buffer| match self.fill_location_bytes(location, start, buffer) {
                Ok(()) => true,
                Err(error) => {
                    first_reader_error.get_or_insert(error);
                    false
                }
            },
            |reference: SharedHeapReference| {
                if !reference.is_null() {
                    roots.push(reference);
                }
            },
        );

        if let Err(error) = result {
            if let Some(error) = first_reader_error {
                return Err(HeapError::HeapScanFailed {
                    source: ScanSource::Reference(reference),
                    error: Box::new(error),
                });
            }

            return Err(HeapError::HeapScanFailed {
                source: ScanSource::Reference(reference),
                error: Box::new(error),
            });
        }

        Ok(())
    }

    /// Perform one young-generation collection over explicit heap roots.
    pub fn collect_minor(&mut self, roots: &mut [HeapReference]) -> HeapResult<GcStats> {
        // reject overlapping collection work
        if self.is_collecting {
            return Err(HeapError::HeapCollectionActive);
        }

        // prepare reusable collection state
        let queue = std::mem::take(&mut self.trace_queue);
        let mut pending = queue;
        let mut promotions = Vec::new();
        let pinned = self.pins.references().collect::<Vec<_>>();

        // begin the new cycle
        self.clear_mark_bits();
        pending.clear();
        self.is_collecting = true;

        // trace every reachable young entry from roots and remembered mature writes
        let result =
            self.collect_minor_cycle(roots, pinned.iter().copied(), &mut pending, &mut promotions);

        self.trace_queue = pending;
        self.is_collecting = false;

        result
    }

    /// Perform one full heap collection over explicit heap roots.
    pub fn collect_full(&mut self, roots: &mut [HeapReference]) -> HeapResult<GcStats> {
        let pinned = self.pins.references().collect::<Vec<_>>();
        let _minor = self.collect_minor(roots)?;

        // reject overlapping collection work
        if self.is_collecting {
            return Err(HeapError::HeapCollectionActive);
        }

        // prepare reusable collection state
        let queue = std::mem::take(&mut self.trace_queue);
        let mut pending = queue;

        // begin the new cycle
        self.clear_mark_bits();
        pending.clear();
        self.is_collecting = true;

        // trace every reachable mature entry from the explicit roots
        let result = self.collect_full_cycle(roots, pinned.iter().copied(), &mut pending);

        self.trace_queue = pending;
        self.is_collecting = false;

        result
    }

    /// Perform one prepared minor collection cycle.
    fn collect_minor_cycle(
        &mut self,
        roots: &mut [HeapReference],
        pinned: impl Iterator<Item = HeapReference>,
        pending: &mut HeapTraceQueue,
        promotions: &mut Vec<Promotion>,
    ) -> HeapResult<GcStats> {
        self.mark_reachable_young_references(roots.iter().copied().chain(pinned), pending)?;
        let (freed_allocations, freed_bytes) = self.promote_or_free_young_references(promotions)?;

        // rewrite roots and traced mature payloads before resetting young space
        self.rewrite_promoted_references(roots, promotions)?;

        // reset the young space after promotion and sweeping
        self.reset_young_space()?;

        // finalize the completed minor-cycle statistics
        let stats = self.stats_after_collection(freed_allocations, freed_bytes);

        self.gc.record_cycle(GcKind::Minor, stats)?;

        Ok(stats)
    }

    /// Perform one prepared full collection cycle.
    fn collect_full_cycle(
        &mut self,
        roots: &mut [HeapReference],
        pinned: impl Iterator<Item = HeapReference>,
        pending: &mut HeapTraceQueue,
    ) -> HeapResult<GcStats> {
        self.mark_reachable_references(roots.iter().copied().chain(pinned), pending)?;
        let (freed_allocations, freed_bytes) = self.free_unreachable_references()?;

        // finalize the completed full-cycle statistics
        let stats = self.stats_after_collection(freed_allocations, freed_bytes);

        self.gc.record_cycle(GcKind::Full, stats)?;

        Ok(stats)
    }

    /// Rewrite roots and traced mature payloads through one completed promotion set.
    fn rewrite_promoted_references(
        &mut self,
        roots: &mut [HeapReference],
        promotions: &[Promotion],
    ) -> HeapResult<()> {
        if promotions.is_empty() {
            return Ok(());
        }

        let references = self.promotion_references(promotions)?;

        // rewrite the explicit root set first
        for reference in roots.iter_mut() {
            if let Some(next_reference) = references.get(reference).copied() {
                *reference = next_reference;
            }
        }

        // then rewrite every pinned reference
        self.pins.rewrite(&references)?;

        // then rewrite every mature payload that may still contain young references
        self.rewrite_live_heap_references(&references)?;

        // finally rebuild the auxiliary shared-edge tracking over the new stable refs
        self.rebuild_shared_edge_roots()?;
        self.shared_edge_cursor = 0;
        self.shared_edge_queue.clear();
        self.shared_edge_pending.clear();

        Ok(())
    }

    /// Return the completed old to new reference map for one promotion set.
    fn promotion_references(
        &self,
        promotions: &[Promotion],
    ) -> HeapResult<BTreeMap<HeapReference, HeapReference>> {
        let mut references = BTreeMap::new();

        for promotion in promotions {
            let next_reference = self.base_reference(promotion.target)?;
            references.insert(promotion.reference, next_reference);
        }

        Ok(references)
    }

    /// Rewrite every live mature payload through one completed promotion map.
    fn rewrite_live_heap_references(
        &mut self,
        references: &BTreeMap<HeapReference, HeapReference>,
    ) -> HeapResult<()> {
        let live_references = self.live_references()?;

        for reference in live_references {
            let Some(location) = self.resolve_location(reference) else {
                continue;
            };

            if matches!(location.storage, HeapStorage::Young(_)) {
                continue;
            }

            let reference_map = self.location_reference_map(location.storage)?;

            if !reference_map.has_local_reference() {
                continue;
            }

            self.rewrite_location_heap_references(location, &reference_map, references)?;
        }

        Ok(())
    }

    /// Rewrite one traced heap payload through one completed promotion map.
    fn rewrite_location_heap_references(
        &mut self,
        location: HeapLocation,
        reference_map: &ReferenceMap,
        references: &BTreeMap<HeapReference, HeapReference>,
    ) -> HeapResult<()> {
        match reference_map {
            ReferenceMap::None => {}
            ReferenceMap::Reference { local_offsets, .. } => {
                for offset in local_offsets.iter().copied() {
                    self.rewrite_location_heap_reference_word(
                        location,
                        offset as usize,
                        references,
                    )?;
                }
            }
            ReferenceMap::RepeatedReference {
                count,
                stride,
                local_offsets,
                ..
            } => {
                for index in 0..*count as usize {
                    let element_start = index.checked_mul(*stride as usize).ok_or(
                        HeapError::InvariantOverflow {
                            context: "repeated local reference rewrite",
                        },
                    )?;

                    for offset in local_offsets.iter().copied() {
                        let start = element_start.checked_add(offset as usize).ok_or(
                            HeapError::InvariantOverflow {
                                context: "repeated local reference rewrite",
                            },
                        )?;

                        self.rewrite_location_heap_reference_word(location, start, references)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Rewrite one direct heap-reference word inside one traced payload.
    fn rewrite_location_heap_reference_word(
        &mut self,
        location: HeapLocation,
        start: usize,
        references: &BTreeMap<HeapReference, HeapReference>,
    ) -> HeapResult<()> {
        let mut window = [0u8; HeapReference::BYTE_LEN];

        self.fill_location_bytes(location, start, &mut window)?;

        let reference = HeapReference::from_bits(usize::from_le_bytes(window));
        let Some(next_reference) = references.get(&reference).copied() else {
            return Ok(());
        };
        let next_bytes = next_reference.bits().to_le_bytes();

        self.write_location_bytes(location, start, &next_bytes)
    }

    /// Promote one young reference into mature space.
    pub(crate) fn promote_reference(
        &mut self,
        reference: HeapReference,
    ) -> HeapResult<HeapReference> {
        // resolve the current live location
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        // mature references already have stable addresses
        let HeapStorage::Young(young_id) = location.storage else {
            return Ok(reference);
        };

        // stage the young to mature relocation
        let mut promotions = Vec::with_capacity(1);

        self.stage_young_promotion(reference, young_id, &mut promotions)?;

        // publish the mature location only after staging succeeds
        if let Err(error) = self.commit_young_promotions(&promotions) {
            self.discard_young_promotions(&promotions)?;

            return Err(error);
        }

        // retire the old nursery source after the promoted reference points at mature storage
        let Some(entry) = self.young_entry_mut(young_id) else {
            return Err(HeapError::MissingYoungEntry {
                generation: young_id.generation(),
                entry_index: young_id.index(),
            });
        };
        entry.is_live = false;

        // remember the new mature location conservatively
        let Some(promotion) = promotions.first().copied() else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        let promoted_reference = self.base_reference(promotion.target)?;

        self.write_barrier_location(
            HeapLocation {
                storage: promotion.target,
                base: promoted_reference,
                byte_offset: 0,
                byte_len: location.byte_len,
            },
            0,
            location.byte_len,
        )?;

        // keep active shared-edge scans aware of the now-mature entry
        if self.is_scanning_shared_edges
            && self
                .location_reference_map(promotion.target)?
                .has_shared_reference()
        {
            self.queue_shared_reference(promoted_reference)?;
        }

        Ok(promoted_reference)
    }

    /// Stage one live young entry relocation into mature space.
    fn stage_young_promotion(
        &mut self,
        reference: HeapReference,
        young_id: HeapYoungId,
        promotions: &mut Vec<Promotion>,
    ) -> HeapResult<()> {
        let Some(entry) = self.young_entry(young_id).cloned() else {
            return Err(HeapError::HeapPromotionFailed {
                reference,
                error: Box::new(HeapError::MissingYoungEntry {
                    generation: young_id.generation(),
                    entry_index: young_id.index(),
                }),
            });
        };

        // resolve the shared source range once before relocating the entry
        let young_offset = self.young_entry_offset(&entry);
        let young_pages = self.young.pages.clone();

        // prefer one mature small slot when the payload fits one size class
        let location = if self
            .small
            .size_classes
            .class_index_for(entry.byte_len)
            .is_some()
        {
            let slot = self
                .allocate_small_from_page_view(&young_pages, young_offset, entry.layout_id, false)
                .map_err(|error| HeapError::HeapPromotionFailed {
                    reference,
                    error: Box::new(error),
                })?;
            let Some(slot) = slot else {
                return Err(HeapError::HeapPromotionUnavailableSmallSlot {
                    reference,
                    byte_len: entry.byte_len,
                });
            };

            HeapStorage::Small(slot)
        }
        // otherwise copy the payload directly into one mature large entry
        else {
            let mut pages = self
                .allocate_page_view_zeroed(entry.byte_len)
                .map_err(|error| HeapError::HeapPromotionFailed {
                    reference,
                    error: Box::new(error),
                })?;

            let allocator = self.allocator().clone();

            // copy the young payload into the new large entry
            if let Err(error) = allocator.copy_bytes_between_page_views(
                &young_pages,
                young_offset,
                &mut pages,
                0,
                entry.byte_len,
            ) {
                self.release_page_view(pages)
                    .map_err(|error| HeapError::HeapPromotionFailed {
                        reference,
                        error: Box::new(error),
                    })?;

                return Err(HeapError::HeapPromotionFailed {
                    reference,
                    error: Box::new(error),
                });
            }

            let entry_id = self
                .store_large_entry(entry.byte_len, pages, entry.layout_id, false)
                .map_err(|error| HeapError::HeapPromotionFailed {
                    reference,
                    error: Box::new(error),
                })?;

            HeapStorage::Large(entry_id)
        };

        promotions.push(Promotion {
            reference,
            source: HeapStorage::Young(young_id),
            target: location,
        });

        Ok(())
    }

    /// Verify every staged young relocation still refers to the staged source.
    fn commit_young_promotions(&mut self, promotions: &[Promotion]) -> HeapResult<()> {
        for promotion in promotions {
            let reference = promotion.reference;
            let Some(location) = self.resolve_location(reference) else {
                return Err(HeapError::InvalidHeapReference { reference });
            };
            if location.storage != promotion.source {
                return Err(HeapError::InvalidHeapReference { reference });
            }
        }

        Ok(())
    }

    /// Discard every staged mature relocation target.
    fn discard_young_promotions(&mut self, promotions: &[Promotion]) -> HeapResult<()> {
        for promotion in promotions.iter().rev() {
            let reference = promotion.reference;
            let result = match promotion.target {
                HeapStorage::Young(young_id) => Err(HeapError::MissingYoungEntry {
                    generation: young_id.generation(),
                    entry_index: young_id.index(),
                }),
                HeapStorage::Small(slot) => self.release_small_slot(slot),
                HeapStorage::Large(entry_id) => {
                    let Some(entry) = self.large_entry_mut(entry_id) else {
                        return Err(HeapError::MissingLargeEntry {
                            entry_id: entry_id.id(),
                        });
                    };

                    if !entry.is_live {
                        return Err(HeapError::MissingLargeEntry {
                            entry_id: entry_id.id(),
                        });
                    }

                    let pages = entry.pages.clone();
                    entry.retire();
                    self.large.free_large_entry_ids.push(entry_id.id());

                    // release the unpublished target pages after discarding the slot
                    self.release_page_view(pages)?;

                    Ok(())
                }
            };

            result.map_err(|error| HeapError::HeapPromotionFailed {
                reference,
                error: Box::new(error),
            })?;
        }

        Ok(())
    }

    /// Promote or free every young reference seen during one minor collection.
    fn promote_or_free_young_references(
        &mut self,
        promotions: &mut Vec<Promotion>,
    ) -> Result<(usize, u64), HeapError> {
        let mut freed_allocations = 0usize;
        let mut freed_bytes = 0u64;

        // walk every live local allocation directly
        for reference in self.live_references()? {
            let Some(location) = self.resolve_location(reference) else {
                continue;
            };
            let HeapStorage::Young(young_id) = location.storage else {
                continue;
            };

            if self.is_marked_storage(location.storage) {
                if let Err(error) = self.stage_young_promotion(reference, young_id, promotions) {
                    self.discard_young_promotions(promotions)?;

                    return Err(error);
                }

                continue;
            }

            // otherwise free unreachable young entries
            let did_free = match self.free(reference) {
                Ok(did_free) => did_free,
                Err(error) => {
                    self.discard_young_promotions(promotions)?;

                    return Err(HeapError::HeapFreeFailed {
                        reference,
                        error: Box::new(error),
                    });
                }
            };

            if did_free {
                freed_allocations =
                    freed_allocations
                        .checked_add(1)
                        .ok_or(HeapError::InvariantOverflow {
                            context: "young gc freed allocation count",
                        })?;
                freed_bytes = freed_bytes.checked_add(location.byte_len as u64).ok_or(
                    HeapError::InvariantOverflow {
                        context: "young gc freed bytes",
                    },
                )?;
            }
        }

        // rewrite references only after every target has been staged
        if let Err(error) = self.commit_young_promotions(promotions) {
            self.discard_young_promotions(promotions)?;

            return Err(error);
        }

        Ok((freed_allocations, freed_bytes))
    }

    /// Free every unreachable heap reference during one full collection.
    fn free_unreachable_references(&mut self) -> Result<(usize, u64), HeapError> {
        let mut freed_allocations = 0usize;
        let mut freed_bytes = 0u64;

        // walk every live local allocation directly
        for reference in self.live_references()? {
            let Some(location) = self.resolve_location(reference) else {
                continue;
            };

            // keep reachable references intact
            if self.is_marked_storage(location.storage) {
                continue;
            }

            // free unreachable references and charge the reclaimed bytes
            if self
                .free(reference)
                .map_err(|error| HeapError::HeapFreeFailed {
                    reference,
                    error: Box::new(error),
                })?
            {
                freed_allocations =
                    freed_allocations
                        .checked_add(1)
                        .ok_or(HeapError::InvariantOverflow {
                            context: "full gc freed allocation count",
                        })?;
                freed_bytes = freed_bytes.checked_add(location.byte_len as u64).ok_or(
                    HeapError::InvariantOverflow {
                        context: "full gc freed bytes",
                    },
                )?;
            }
        }

        Ok((freed_allocations, freed_bytes))
    }

    /// Reset the young space after one collection cycle.
    fn reset_young_space(&mut self) -> HeapResult<()> {
        let allocator = self.allocator().clone();
        let next_pages = self
            .page_run_cache
            .allocate_zeroed(&allocator, self.young.capacity_bytes)
            .map_err(|error| HeapError::HeapYoungResetFailed {
                error: Box::new(error),
            })?;
        let previous_pages = self.young.pages.clone();

        // remove the old nursery ownership before its pages reenter the allocator cache
        self.unmap_page_view(&previous_pages)?;

        // release the old nursery pages once the page-owner table is clean
        if let Err(error) = self
            .page_run_cache
            .release_page_view(&allocator, previous_pages.clone())
        {
            self.map_page_view(&previous_pages, |logical_page_index| HeapPageOwner::Young {
                logical_page_index,
            })?;
            self.page_run_cache
                .release_page_view(&allocator, next_pages)?;

            return Err(HeapError::HeapYoungResetFailed {
                error: Box::new(error),
            });
        }

        // publish the fresh nursery state
        self.young.generation =
            self.young
                .generation
                .checked_add(1)
                .ok_or(HeapError::InvariantOverflow {
                    context: "heap young generation",
                })?;
        self.young.next_offset = 0;
        self.young.pages = next_pages;
        self.young.entries.clear();

        let next_pages = self.young.pages.clone();

        self.map_page_view(&next_pages, |logical_page_index| HeapPageOwner::Young {
            logical_page_index,
        })
    }

    /// Mark every reachable young reference.
    fn mark_reachable_young_references(
        &mut self,
        roots: impl IntoIterator<Item = HeapReference>,
        pending: &mut HeapTraceQueue,
    ) -> HeapResult<()> {
        // seed the work queue from explicit young roots
        for reference in roots {
            self.enqueue_young_reference(reference, pending)?;
        }

        // seed the work queue from remembered mature writes
        self.enqueue_dirty_young_references(pending)?;

        // drain the young-object queue and trace each reachable young payload once
        while let Some(reference) = pending.pop() {
            let Some(location) = self.resolve_location(reference) else {
                return Err(HeapError::InvalidHeapReference { reference });
            };

            // skip references that are already reached
            if !self.mark_storage(location.storage) {
                continue;
            }
            if !matches!(location.storage, HeapStorage::Young(_)) {
                continue;
            }

            let reference_map = self
                .location_reference_map(location.storage)
                .map_err(|error| HeapError::HeapScanFailed {
                    source: ScanSource::Reference(reference),
                    error: Box::new(error),
                })?;

            let mut first_reader_error = None;
            let mut first_edge_error = None;

            // enqueue every non-null young edge discovered in this payload
            let trace_result = visit_heap_references_in_reader(
                &reference_map,
                |start, buffer| match self.read_bytes_into(reference, start, buffer) {
                    Ok(()) => true,
                    Err(error) => {
                        first_reader_error.get_or_insert(error);
                        false
                    }
                },
                |reference: HeapReference| {
                    if first_edge_error.is_some() {
                        return;
                    }

                    if let Err(error) = self.enqueue_young_reference(reference, pending) {
                        first_edge_error = Some(error);
                    }
                },
            );

            if let Err(error) = trace_result {
                if let Some(error) = first_reader_error {
                    return Err(HeapError::HeapScanFailed {
                        source: ScanSource::Reference(reference),
                        error: Box::new(error),
                    });
                }

                return Err(HeapError::HeapScanFailed {
                    source: ScanSource::Reference(reference),
                    error: Box::new(error),
                });
            }

            if let Some(error) = first_edge_error {
                return Err(error);
            }
        }

        Ok(())
    }

    /// Mark every reachable heap reference.
    fn mark_reachable_references(
        &mut self,
        roots: impl IntoIterator<Item = HeapReference>,
        pending: &mut HeapTraceQueue,
    ) -> HeapResult<()> {
        // seed the work queue from the explicit roots
        pending.extend(roots);

        // drain the explicit root queue and trace each reachable payload once
        while let Some(reference) = pending.pop() {
            let Some(location) = self.resolve_location(reference) else {
                return Err(HeapError::InvalidHeapReference { reference });
            };

            // skip references that are already reached
            if !self.mark_storage(location.storage) {
                continue;
            }

            let reference_map = self
                .location_reference_map(location.storage)
                .map_err(|error| HeapError::HeapScanFailed {
                    source: ScanSource::Reference(reference),
                    error: Box::new(error),
                })?;

            let mut first_reader_error = None;
            let mut first_edge_error = None;

            // enqueue every non-null edge discovered in this payload
            let trace_result = visit_heap_references_in_reader(
                &reference_map,
                |start, buffer| match self.read_bytes_into(reference, start, buffer) {
                    Ok(()) => true,
                    Err(error) => {
                        first_reader_error.get_or_insert(error);
                        false
                    }
                },
                |reference: HeapReference| {
                    if first_edge_error.is_some() {
                        return;
                    }

                    if !reference.is_null() && self.resolve_location(reference).is_none() {
                        first_edge_error = Some(HeapError::InvalidHeapReference { reference });
                        return;
                    }

                    pending.push(reference);
                },
            );

            if let Err(error) = trace_result {
                if let Some(error) = first_reader_error {
                    return Err(HeapError::HeapScanFailed {
                        source: ScanSource::Reference(reference),
                        error: Box::new(error),
                    });
                }

                return Err(HeapError::HeapScanFailed {
                    source: ScanSource::Reference(reference),
                    error: Box::new(error),
                });
            }

            if let Some(error) = first_edge_error {
                return Err(error);
            }
        }

        Ok(())
    }

    /// Queue one young reference when it currently points into the young space.
    fn enqueue_young_reference(
        &self,
        reference: HeapReference,
        pending: &mut HeapTraceQueue,
    ) -> HeapResult<()> {
        if reference.is_null() {
            return Ok(());
        }

        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        if matches!(location.storage, HeapStorage::Young(_)) {
            pending.push(reference);
        }

        Ok(())
    }

    /// Clear every collector mark bit in the live heap.
    fn clear_mark_bits(&mut self) {
        for entry in &mut self.young.entries {
            entry.is_marked = false;
        }

        for span_index in 0..self.small.spans.len() {
            let Some(span) = self.small.spans.get_mut(span_index) else {
                panic!("marked small span should exist: {span_index}")
            };

            span.clear_marks();
        }

        for entry_index in 0..self.large.entries.len() {
            let Some(entry) = self.large.entries.get_mut(entry_index) else {
                panic!("marked large entry should exist: {entry_index}")
            };

            entry.is_marked = false;
        }
    }

    /// Return whether one heap storage location is marked in the active cycle.
    fn is_marked_storage(&self, storage: HeapStorage) -> bool {
        match storage {
            HeapStorage::Young(young_id) => {
                if young_id.generation() != self.young.generation {
                    return false;
                }

                let Some(entry) = self.young_entry(young_id) else {
                    panic!(
                        "marked young entry should exist: generation={}, entry_index={}",
                        young_id.generation(),
                        young_id.index()
                    )
                };

                entry.is_marked
            }
            HeapStorage::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    panic!("marked small span should exist: {}", slot.span_index())
                };

                span.marked.contains(slot.slot_index())
            }
            HeapStorage::Large(entry_id) => {
                let Some(entry) = self.large_entry(entry_id) else {
                    panic!("marked large entry should exist: {}", entry_id.id())
                };

                entry.is_marked
            }
        }
    }

    /// Mark one heap storage location and return whether this was the first mark.
    fn mark_storage(&mut self, storage: HeapStorage) -> bool {
        if self.is_marked_storage(storage) {
            return false;
        }

        match storage {
            HeapStorage::Young(young_id) => {
                if young_id.generation() != self.young.generation {
                    panic!(
                        "young mark should stay in the active generation: marked={}, active={}",
                        young_id.generation(),
                        self.young.generation
                    )
                }

                let Some(entry) = self.young_entry_mut(young_id) else {
                    panic!(
                        "marked young entry should exist: generation={}, entry_index={}",
                        young_id.generation(),
                        young_id.index()
                    )
                };

                entry.is_marked = true;
            }
            HeapStorage::Small(slot) => {
                let Some(span) = self.span_mut(slot.span_index()) else {
                    panic!("marked small span should exist: {}", slot.span_index())
                };

                span.marked.set(slot.slot_index());
            }
            HeapStorage::Large(entry_id) => {
                let Some(entry) = self.large_entry_mut(entry_id) else {
                    panic!("marked large entry should exist: {}", entry_id.id())
                };

                entry.is_marked = true;
            }
        }

        true
    }

    /// Queue every young reference discovered from remembered mature writes.
    fn enqueue_dirty_young_references(&mut self, pending: &mut HeapTraceQueue) -> HeapResult<()> {
        let dirty_spans = self.dirty_spans.clone();
        let dirty_large_entries = self.dirty_large_entries.clone();

        // scan each queued dirty span
        for span_index in dirty_spans {
            self.enqueue_dirty_span_references(span_index, pending)?;
        }

        // scan each queued dirty large entry
        for entry_id in dirty_large_entries {
            self.enqueue_dirty_large_entry_references(entry_id, pending)?;
        }

        self.dirty_spans.clear();
        self.dirty_large_entries.clear();

        Ok(())
    }

    /// Queue every young reference discovered from one dirty span.
    fn enqueue_dirty_span_references(
        &mut self,
        span_index: usize,
        pending: &mut HeapTraceQueue,
    ) -> HeapResult<()> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::HeapScanFailed {
                source: ScanSource::Span(span_index),
                error: Box::new(HeapError::MissingSpan { span_index }),
            });
        };
        let occupied = span.occupied.clone();
        let slot_count = span.slot_count;
        let size_class = span.class.size_class;
        let byte_lens = span.byte_lens.clone();
        let local_reference_bits = span.local_reference_bits.clone();
        let shared_reference_bits = span.shared_reference_bits.clone();
        let pages = span.pages.clone();
        let dirty_cards = span.dirty_cards.clone();

        let mut first_error = None;

        // scan each dirty card window against each occupied slot
        dirty_cards.visit_dirty_ranges(|card_start, card_len| {
            if first_error.is_some() {
                return;
            }

            let card_end = card_start.saturating_add(card_len);

            for slot_index in 0..slot_count {
                if !occupied.contains(slot_index) {
                    continue;
                }

                let Some(&byte_len) = byte_lens.get(slot_index) else {
                    first_error = Some(HeapError::HeapScanFailed {
                        source: ScanSource::Span(span_index),
                        error: Box::new(HeapError::MissingSmallSlot {
                            span_index,
                            slot_index,
                        }),
                    });
                    return;
                };

                let reference_map = slot_reference_map(
                    &local_reference_bits,
                    &shared_reference_bits,
                    slot_index,
                    size_class,
                    byte_len,
                );
                if !reference_map.has_local_reference() {
                    continue;
                }

                let slot_start = size_class.saturating_mul(slot_index);
                let slot_end = slot_start.saturating_add(size_class);
                let overlap_start = card_start.max(slot_start);
                let overlap_end = card_end.min(slot_end);

                if overlap_start >= overlap_end {
                    continue;
                }

                let local_start = overlap_start.saturating_sub(slot_start);
                let local_len = overlap_end.saturating_sub(overlap_start);
                let mut first_reader_error = None;
                let result = visit_heap_references_in_reader_range(
                    &reference_map,
                    local_start,
                    local_len,
                    |start, buffer| match self.allocator().fill_bytes_from(
                        &pages,
                        slot_start.saturating_add(start),
                        buffer,
                    ) {
                        Ok(()) => true,
                        Err(error) => {
                            first_reader_error.get_or_insert(error);
                            false
                        }
                    },
                    |reference| {
                        if first_error.is_some() {
                            return;
                        }

                        if let Err(error) = self.enqueue_young_reference(reference, pending) {
                            first_error = Some(error);
                        }
                    },
                );

                if let Err(error) = result {
                    if let Some(error) = first_reader_error {
                        first_error = Some(HeapError::HeapScanFailed {
                            source: ScanSource::Span(span_index),
                            error: Box::new(error),
                        });
                        return;
                    }

                    first_error = Some(HeapError::HeapScanFailed {
                        source: ScanSource::Span(span_index),
                        error: Box::new(error),
                    });
                    return;
                }
            }
        });

        if let Some(error) = first_error {
            return Err(error);
        }

        // clear only after the scan succeeds
        if let Some(span) = self.span_mut(span_index) {
            span.dirty_cards.clear();
            span.is_dirty_queued = false;
        }

        Ok(())
    }

    /// Queue every young reference discovered from one dirty large entry.
    fn enqueue_dirty_large_entry_references(
        &mut self,
        entry_id: LargeEntryId,
        pending: &mut HeapTraceQueue,
    ) -> HeapResult<()> {
        let Some(entry) = self.large_entry(entry_id) else {
            return Err(HeapError::HeapScanFailed {
                source: ScanSource::LargeEntry(entry_id.id()),
                error: Box::new(HeapError::MissingLargeEntry {
                    entry_id: entry_id.id(),
                }),
            });
        };
        let pages = entry.pages.clone();
        let dirty_cards = entry.dirty_cards.clone();
        let reference_map = self.reference_map(entry.layout_id)?.clone();

        let mut first_error = None;

        // scan each dirty card window directly against the large entry
        dirty_cards.visit_dirty_ranges(|card_start, card_len| {
            if first_error.is_some() {
                return;
            }

            let mut first_reader_error = None;
            let result = visit_heap_references_in_reader_range(
                &reference_map,
                card_start,
                card_len,
                |start, buffer| match self.allocator().fill_bytes_from(&pages, start, buffer) {
                    Ok(()) => true,
                    Err(error) => {
                        first_reader_error.get_or_insert(error);
                        false
                    }
                },
                |reference| {
                    if first_error.is_some() {
                        return;
                    }

                    if let Err(error) = self.enqueue_young_reference(reference, pending) {
                        first_error = Some(error);
                    }
                },
            );

            if let Err(error) = result {
                if let Some(error) = first_reader_error {
                    first_error = Some(HeapError::HeapScanFailed {
                        source: ScanSource::LargeEntry(entry_id.id()),
                        error: Box::new(error),
                    });
                    return;
                }

                first_error = Some(HeapError::HeapScanFailed {
                    source: ScanSource::LargeEntry(entry_id.id()),
                    error: Box::new(error),
                });
            }
        });

        if let Some(error) = first_error {
            return Err(error);
        }

        // clear only after the scan succeeds
        if let Some(entry) = self.large_entry_mut(entry_id) {
            entry.dirty_cards.clear();
            entry.is_dirty_queued = false;
        }

        Ok(())
    }

    /// Build one GC statistics snapshot after one collection pass.
    fn stats_after_collection(&self, freed_allocations: usize, freed_bytes: u64) -> GcStats {
        GcStats {
            freed_allocations,
            live_allocations: self.usage.allocation_count(),
            freed_bytes,
            allocated_bytes: self.usage.allocated_bytes(),
            active_bytes: self.active_bytes(),
        }
    }
}
