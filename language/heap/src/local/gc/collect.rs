use super::Promotion;
use crate::local::managed::{
    GcKind, GcStats, LargeEntryId, ManagedLocation, ManagedSpace, ManagedYoungId,
    checked_reference_id,
};
use crate::{
    HeapError, HeapResult, ManagedReference, MarkSet, ScanSource, ShapeId, SharedManagedReference,
    TraceQueue, sum_bytes, visit_managed_references_in_reader,
    visit_managed_references_in_reader_range, visit_shared_references_in_reader,
};

/// Collector marks for local managed references.
type ManagedMarkSet = MarkSet<ManagedReference>;

/// Collector queue for local managed references.
type ManagedTraceQueue = TraceQueue<ManagedReference>;

impl ManagedSpace {
    /// Return the currently live managed references.
    pub fn live_references(&self) -> HeapResult<Vec<ManagedReference>> {
        // compact the dense table back into stable live references
        let mut references = Vec::new();

        for (index, record) in self.references.iter().enumerate() {
            if record.is_vacant() {
                continue;
            }

            let reference_id = checked_reference_id(index as u64 + 1)?;
            references.push(ManagedReference::new(reference_id));
        }

        Ok(references)
    }

    /// Start one incremental local-to-shared root scan.
    pub(crate) fn start_shared_root_scan(&mut self) {
        self.is_scanning_shared_roots = true;
        self.shared_root_cursor = 0;
        self.shared_root_queue.clear();
        self.shared_root_pending.clear();
    }

    /// Return whether the current local-to-shared scan is fully drained.
    pub(crate) fn shared_root_scan_idle(&self) -> bool {
        !self.is_scanning_shared_roots
            || (self.shared_root_cursor >= self.references.len()
                && self.shared_root_queue.is_empty())
    }

    /// Finish the current local-to-shared root scan.
    pub(crate) fn finish_shared_root_scan(&mut self) {
        self.is_scanning_shared_roots = false;
        self.shared_root_cursor = 0;
        self.shared_root_queue.clear();
        self.shared_root_pending.clear();
    }

    /// Scan bounded local-to-shared root work into the provided root buffer.
    pub(crate) fn scan_shared_root_step(
        &mut self,
        roots: &mut Vec<SharedManagedReference>,
        work_items: usize,
    ) -> HeapResult<usize> {
        if !self.is_scanning_shared_roots || work_items == 0 {
            return Ok(0);
        }

        let mut work_done = 0usize;

        // drain queued rescans first
        while work_done < work_items {
            let Some(reference) = self.shared_root_queue.pop() else {
                break;
            };

            self.clear_shared_root_pending(reference)?;
            self.trace_shared_roots(reference, roots)?;
            work_done += 1;
        }

        // then continue the full-table walk
        while work_done < work_items {
            let Some(reference) = self.next_shared_root_reference()? else {
                break;
            };

            self.trace_shared_roots(reference, roots)?;
            work_done += 1;
        }

        Ok(work_done)
    }

    /// Queue one local reference for one later shared-root rescan.
    pub(crate) fn queue_shared_reference(&mut self, reference: ManagedReference) -> HeapResult<()> {
        if !self.is_scanning_shared_roots || !self.reference_has_shared_roots(reference)? {
            return Ok(());
        }

        let index = Self::reference_index(reference.id())?;
        if self.shared_root_pending.len() <= index {
            self.shared_root_pending.resize(index + 1, false);
        }

        if self.shared_root_pending[index] {
            return Ok(());
        }

        self.shared_root_pending[index] = true;
        self.shared_root_queue.push(reference);

        Ok(())
    }

    /// Return the next live local reference that may contain shared roots.
    fn next_shared_root_reference(&mut self) -> HeapResult<Option<ManagedReference>> {
        while self.shared_root_cursor < self.references.len() {
            let index = self.shared_root_cursor;
            self.shared_root_cursor += 1;

            let Some(record) = self.references.get(index).copied() else {
                continue;
            };
            if record.is_vacant() {
                continue;
            }

            let reference_id = checked_reference_id(index as u64 + 1)?;
            let reference = ManagedReference::new(reference_id);

            if !self.reference_has_shared_roots(reference)? {
                continue;
            }

            return Ok(Some(reference));
        }

        Ok(None)
    }

    /// Return whether one live local reference may contain shared managed roots.
    fn reference_has_shared_roots(&self, reference: ManagedReference) -> HeapResult<bool> {
        let Some(record) = self.reference(reference).copied() else {
            return Ok(false);
        };
        let Some(location) = record.location() else {
            return Ok(false);
        };
        let Some(shape_id) = self.location_shape_id(location) else {
            return Ok(false);
        };
        let Some(shape) = self.shape_table.shape(shape_id) else {
            return Err(HeapError::InvalidShapeId {
                index: shape_id.index(),
            });
        };

        Ok(shape.scan.has_shared_reference())
    }

    /// Clear the queued bit for one local shared-root rescan.
    fn clear_shared_root_pending(&mut self, reference: ManagedReference) -> HeapResult<()> {
        let index = Self::reference_index(reference.id())?;

        if let Some(is_pending) = self.shared_root_pending.get_mut(index) {
            *is_pending = false;
        }

        Ok(())
    }

    /// Trace shared managed roots from one local managed reference.
    fn trace_shared_roots(
        &mut self,
        reference: ManagedReference,
        roots: &mut Vec<SharedManagedReference>,
    ) -> HeapResult<()> {
        let Some(record) = self.reference(reference).copied() else {
            return Ok(());
        };
        let Some(location) = record.location() else {
            return Ok(());
        };
        let Some(shape_id) = self.location_shape_id(location) else {
            return Ok(());
        };
        let Some(shape) = self.shape_table.shape(shape_id) else {
            return Err(HeapError::ManagedScanFailed {
                source: ScanSource::Reference(reference),
                error: Box::new(HeapError::InvalidShapeId {
                    index: shape_id.index(),
                }),
            });
        };

        if !shape.scan.has_shared_reference() {
            return Ok(());
        }

        let mut first_reader_error = None;
        let result = visit_shared_references_in_reader(
            &shape.scan,
            SharedManagedReference::BYTE_LEN,
            |start, buffer| match self.fill_location_bytes(location, start, buffer) {
                Ok(()) => true,
                Err(error) => {
                    first_reader_error.get_or_insert(error);
                    false
                }
            },
            |reference: SharedManagedReference| {
                if !reference.is_null() {
                    roots.push(reference);
                }
            },
        );

        if let Err(error) = result {
            if let Some(error) = first_reader_error {
                return Err(HeapError::ManagedScanFailed {
                    source: ScanSource::Reference(reference),
                    error: Box::new(error),
                });
            }

            return Err(HeapError::ManagedScanFailed {
                source: ScanSource::Reference(reference),
                error: Box::new(error),
            });
        }

        Ok(())
    }

    /// Perform one young-generation collection over explicit managed roots.
    pub fn collect_minor(
        &mut self,
        roots: impl IntoIterator<Item = ManagedReference>,
    ) -> HeapResult<GcStats> {
        if self.is_collecting {
            return Err(HeapError::ManagedCollectionActive);
        }

        if self.pins.is_active() {
            return Err(HeapError::ManagedCollectionPinsActive);
        }

        let marks = std::mem::take(&mut self.marks);
        let queue = std::mem::take(&mut self.trace_queue);
        let mut marks = marks;
        let mut pending = queue;
        let mut promotions = Vec::new();
        marks.start_cycle();
        pending.clear();
        self.is_collecting = true;

        // trace every reachable young entry from roots and remembered mature writes
        let result = (|| {
            self.mark_reachable_young_references(roots, &mut marks, &mut pending)?;
            let (freed_allocations, freed_bytes) =
                self.promote_or_free_young_references(&marks, &mut promotions)?;

            // reset the young space after promotion and sweeping
            self.reset_young_space()?;

            // finalize the completed minor-cycle statistics
            let stats = self.stats_after_collection(freed_allocations, freed_bytes);

            self.gc_state.record_cycle(GcKind::Minor, stats)?;

            Ok(stats)
        })();

        self.marks = marks;
        self.trace_queue = pending;
        self.is_collecting = false;

        result
    }

    /// Perform one full managed collection over explicit managed roots.
    pub fn collect_full(
        &mut self,
        roots: impl IntoIterator<Item = ManagedReference>,
    ) -> HeapResult<GcStats> {
        let roots = roots.into_iter().collect::<Vec<_>>();
        let _minor = self.collect_minor(roots.iter().copied())?;
        if self.is_collecting {
            return Err(HeapError::ManagedCollectionActive);
        }
        if self.pins.is_active() {
            return Err(HeapError::ManagedCollectionPinsActive);
        }

        let marks = std::mem::take(&mut self.marks);
        let queue = std::mem::take(&mut self.trace_queue);
        let mut marks = marks;
        let mut pending = queue;
        marks.start_cycle();
        pending.clear();
        self.is_collecting = true;

        // trace every reachable mature entry from the explicit roots
        let result = (|| {
            self.mark_reachable_references(roots.iter().copied(), &mut marks, &mut pending)?;
            let (freed_allocations, freed_bytes) = self.free_unreachable_references(&marks)?;

            // finalize the completed full-cycle statistics
            let stats = self.stats_after_collection(freed_allocations, freed_bytes);

            self.gc_state.record_cycle(GcKind::Full, stats)?;

            Ok(stats)
        })();

        self.marks = marks;
        self.trace_queue = pending;
        self.is_collecting = false;

        result
    }

    /// Stage one live young entry relocation into mature space.
    fn stage_young_promotion(
        &mut self,
        reference: ManagedReference,
        young_id: ManagedYoungId,
        promotions: &mut Vec<Promotion>,
    ) -> HeapResult<()> {
        let Some(entry) = self.young_entry(young_id).cloned() else {
            return Err(HeapError::ManagedPromotionFailed {
                reference,
                error: Box::new(HeapError::MissingYoungEntry {
                    generation: young_id.generation(),
                    entry_index: young_id.index(),
                }),
            });
        };

        // resolve the shared source range once before relocating the entry
        let young_offset = self.young_entry_offset(&entry);
        let shape_id = entry.shape_id;
        let young_pages = self.young.pages;

        // prefer one mature small slot when the payload fits one size class
        let location = if self
            .small
            .size_classes
            .class_index_for(entry.byte_len)
            .is_some()
        {
            let slot = self
                .allocate_small_from_page_view(
                    &young_pages,
                    young_offset,
                    entry.byte_len,
                    shape_id,
                    false,
                )
                .map_err(|error| HeapError::ManagedPromotionFailed {
                    reference,
                    error: Box::new(error),
                })?;
            let Some(slot) = slot else {
                return Err(HeapError::ManagedPromotionUnavailableSmallSlot {
                    reference,
                    byte_len: entry.byte_len,
                });
            };

            ManagedLocation::Small(slot)
        }
        // otherwise copy the payload directly into one mature large entry
        else {
            let mut pages = self
                .allocate_page_view_zeroed(entry.byte_len)
                .map_err(|error| HeapError::ManagedPromotionFailed {
                    reference,
                    error: Box::new(error),
                })?;

            let arena = self.arena().clone();

            // copy the young payload into the new large entry
            if let Err(error) = arena.copy_bytes_between_page_views(
                &young_pages,
                young_offset,
                &mut pages,
                0,
                entry.byte_len,
            ) {
                self.release_page_view(pages).map_err(|error| {
                    HeapError::ManagedPromotionFailed {
                        reference,
                        error: Box::new(error),
                    }
                })?;

                return Err(HeapError::ManagedPromotionFailed {
                    reference,
                    error: Box::new(error),
                });
            }

            let entry_id = self
                .store_large_entry(entry.byte_len, pages, shape_id, false)
                .map_err(|error| HeapError::ManagedPromotionFailed {
                    reference,
                    error: Box::new(error),
                })?;

            ManagedLocation::Large(entry_id)
        };

        promotions.push(Promotion {
            reference,
            source: ManagedLocation::Young(young_id),
            target: location,
        });

        Ok(())
    }

    /// Commit every staged young relocation to the reference table.
    fn commit_young_promotions(&mut self, promotions: &[Promotion]) -> HeapResult<()> {
        for promotion in promotions {
            let reference = promotion.reference;
            let Some(record) = self.reference(reference) else {
                return Err(HeapError::InvalidManagedReference { reference });
            };
            if record.location() != Some(promotion.source) {
                return Err(HeapError::InvalidManagedReference { reference });
            }
        }

        for promotion in promotions {
            let reference = promotion.reference;
            let Some(record) = self.reference_mut(reference) else {
                return Err(HeapError::InvalidManagedReference { reference });
            };
            record.set_location(promotion.target);
        }

        Ok(())
    }

    /// Discard every staged mature relocation target.
    fn discard_young_promotions(&mut self, promotions: &[Promotion]) -> HeapResult<()> {
        for promotion in promotions.iter().rev() {
            let reference = promotion.reference;
            let result = match promotion.target {
                ManagedLocation::Young(young_id) => Err(HeapError::MissingYoungEntry {
                    generation: young_id.generation(),
                    entry_index: young_id.index(),
                }),
                ManagedLocation::Small(slot) => self.release_small_slot(slot),
                ManagedLocation::Large(entry_id) => {
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

                    let pages = entry.pages;
                    entry.retire();
                    self.large.free_large_entry_ids.push(entry_id.id());

                    // release the unpublished target pages after discarding the slot
                    self.release_page_view(pages)?;

                    Ok(())
                }
            };

            result.map_err(|error| HeapError::ManagedPromotionFailed {
                reference,
                error: Box::new(error),
            })?;
        }

        Ok(())
    }

    /// Promote or free every young reference seen during one minor collection.
    fn promote_or_free_young_references(
        &mut self,
        marks: &ManagedMarkSet,
        promotions: &mut Vec<Promotion>,
    ) -> Result<(usize, u64), HeapError> {
        let mut freed_allocations = 0usize;
        let mut freed_bytes = 0u64;

        // walk every stable reference slot directly
        for reference_index in 0..self.references.len() {
            let reference_id = checked_reference_id(reference_index as u64 + 1)?;
            let reference = ManagedReference::new(reference_id);
            let Some(record) = self.reference(reference).copied() else {
                continue;
            };
            let Some(location) = record.location() else {
                continue;
            };

            // skip mature references during the young pass
            let ManagedLocation::Young(young_id) = location else {
                continue;
            };

            if marks.contains(reference) {
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

                    return Err(HeapError::ManagedFreeFailed {
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
                freed_bytes = sum_bytes(freed_bytes, record.byte_len() as u64)?;
            }
        }

        // rewrite references only after every target has been staged
        if let Err(error) = self.commit_young_promotions(promotions) {
            self.discard_young_promotions(promotions)?;

            return Err(error);
        }

        Ok((freed_allocations, freed_bytes))
    }

    /// Free every unreachable managed reference during one full collection.
    fn free_unreachable_references(
        &mut self,
        reachable: &ManagedMarkSet,
    ) -> Result<(usize, u64), HeapError> {
        let mut freed_allocations = 0usize;
        let mut freed_bytes = 0u64;

        // walk every stable reference slot directly
        for reference_index in 0..self.references.len() {
            let reference_id = checked_reference_id(reference_index as u64 + 1)?;
            let reference = ManagedReference::new(reference_id);

            // keep reachable references intact
            if reachable.contains(reference) {
                continue;
            }

            let Some(record) = self.reference(reference).copied() else {
                continue;
            };

            // free unreachable references and charge the reclaimed bytes
            if self
                .free(reference)
                .map_err(|error| HeapError::ManagedFreeFailed {
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
                freed_bytes = sum_bytes(freed_bytes, record.byte_len() as u64)?;
            }
        }

        Ok((freed_allocations, freed_bytes))
    }

    /// Reset the young space after one collection cycle.
    fn reset_young_space(&mut self) -> HeapResult<()> {
        let arena = self.arena().clone();
        self.young
            .reset(&arena, &mut self.page_run_cache)
            .map_err(|error| HeapError::ManagedYoungResetFailed {
                error: Box::new(error),
            })
    }

    /// Walk young entries and return every reachable young reference id.
    fn mark_reachable_young_references(
        &mut self,
        roots: impl IntoIterator<Item = ManagedReference>,
        marks: &mut ManagedMarkSet,
        pending: &mut ManagedTraceQueue,
    ) -> HeapResult<()> {
        // seed the work queue from explicit young roots
        for reference in roots {
            self.enqueue_young_reference(reference, pending)?;
        }

        // seed the work queue from remembered mature writes
        self.enqueue_dirty_young_references(pending)?;

        // drain the young-object queue and trace each reachable young payload once
        while let Some(reference) = pending.pop() {
            // skip references that are already reached
            if !marks.mark(reference) {
                continue;
            }

            let Some(record) = self.reference(reference).copied() else {
                return Err(HeapError::InvalidManagedReference { reference });
            };
            let location = record
                .location()
                .ok_or(HeapError::InvalidManagedReference { reference })?;
            if !matches!(location, ManagedLocation::Young(_)) {
                continue;
            }

            let shape_id = self.shape_id(reference, location)?;
            let Some(shape) = self.shape_table.shape(shape_id) else {
                return Err(HeapError::ManagedScanFailed {
                    source: ScanSource::Reference(reference),
                    error: Box::new(HeapError::InvalidShapeId {
                        index: shape_id.index(),
                    }),
                });
            };

            let mut first_reader_error = None;
            let mut first_edge_error = None;

            // enqueue every non-null young edge discovered in this payload
            let trace_result = visit_managed_references_in_reader(
                &shape.scan,
                self.managed_reference_bytes,
                |start, buffer| match self.read_bytes_into(reference, start, buffer) {
                    Ok(()) => true,
                    Err(error) => {
                        first_reader_error.get_or_insert(error);
                        false
                    }
                },
                |reference: ManagedReference| {
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
                    return Err(HeapError::ManagedScanFailed {
                        source: ScanSource::Reference(reference),
                        error: Box::new(error),
                    });
                }

                return Err(HeapError::ManagedScanFailed {
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

    /// Walk one managed entry and return every reachable managed reference id.
    fn mark_reachable_references(
        &self,
        roots: impl IntoIterator<Item = ManagedReference>,
        marks: &mut ManagedMarkSet,
        pending: &mut ManagedTraceQueue,
    ) -> HeapResult<()> {
        // seed the work queue from the explicit roots
        pending.extend(roots);

        // drain the explicit root queue and trace each reachable payload once
        while let Some(reference) = pending.pop() {
            // skip references that are already reached
            if !marks.mark(reference) {
                continue;
            }

            let Some(record) = self.reference(reference).copied() else {
                return Err(HeapError::InvalidManagedReference { reference });
            };
            let location = record
                .location()
                .ok_or(HeapError::InvalidManagedReference { reference })?;

            let shape_id = self.shape_id(reference, location)?;
            let Some(shape) = self.shape_table.shape(shape_id) else {
                return Err(HeapError::ManagedScanFailed {
                    source: ScanSource::Reference(reference),
                    error: Box::new(HeapError::InvalidShapeId {
                        index: shape_id.index(),
                    }),
                });
            };

            let mut first_reader_error = None;
            let mut first_edge_error = None;

            // enqueue every non-null edge discovered in this payload
            let trace_result = visit_managed_references_in_reader(
                &shape.scan,
                self.managed_reference_bytes,
                |start, buffer| match self.read_bytes_into(reference, start, buffer) {
                    Ok(()) => true,
                    Err(error) => {
                        first_reader_error.get_or_insert(error);
                        false
                    }
                },
                |reference: ManagedReference| {
                    if first_edge_error.is_some() {
                        return;
                    }

                    if !reference.is_null() && self.reference(reference).is_none() {
                        first_edge_error = Some(HeapError::InvalidManagedReference { reference });
                        return;
                    }

                    pending.push(reference);
                },
            );

            if let Err(error) = trace_result {
                if let Some(error) = first_reader_error {
                    return Err(HeapError::ManagedScanFailed {
                        source: ScanSource::Reference(reference),
                        error: Box::new(error),
                    });
                }

                return Err(HeapError::ManagedScanFailed {
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
        reference: ManagedReference,
        pending: &mut ManagedTraceQueue,
    ) -> HeapResult<()> {
        if reference.is_null() {
            return Ok(());
        }

        let record = self
            .reference(reference)
            .ok_or(HeapError::InvalidManagedReference { reference })?;
        let location = record
            .location()
            .ok_or(HeapError::InvalidManagedReference { reference })?;

        if matches!(location, ManagedLocation::Young(_)) {
            pending.push(reference);
        }

        Ok(())
    }

    /// Return the shape id for one traceable managed location.
    fn shape_id(
        &self,
        reference: ManagedReference,
        location: ManagedLocation,
    ) -> HeapResult<ShapeId> {
        let Some(shape_id) = self.location_shape_id(location) else {
            let error = match location {
                ManagedLocation::Young(young_id) => HeapError::MissingYoungEntry {
                    generation: young_id.generation(),
                    entry_index: young_id.index(),
                },
                ManagedLocation::Small(slot) => HeapError::MissingSmallSlot {
                    span_index: slot.span_index(),
                    slot_index: slot.slot_index(),
                },
                ManagedLocation::Large(entry_id) => HeapError::MissingLargeEntry {
                    entry_id: entry_id.id(),
                },
            };

            return Err(HeapError::ManagedScanFailed {
                source: ScanSource::Reference(reference),
                error: Box::new(error),
            });
        };

        Ok(shape_id)
    }

    /// Queue every young reference discovered from remembered mature writes.
    fn enqueue_dirty_young_references(
        &mut self,
        pending: &mut ManagedTraceQueue,
    ) -> HeapResult<()> {
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
        pending: &mut ManagedTraceQueue,
    ) -> HeapResult<()> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::ManagedScanFailed {
                source: ScanSource::Span(span_index),
                error: Box::new(HeapError::MissingSpan { span_index }),
            });
        };
        let occupied = span.occupied.clone();
        let shape_ids = span.shape_ids.clone();
        let slot_count = span.slot_count;
        let size_class = span.size_class;
        let pages = span.pages;
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

                let slot_start = size_class.saturating_mul(slot_index);
                let slot_end = slot_start.saturating_add(size_class);
                let overlap_start = card_start.max(slot_start);
                let overlap_end = card_end.min(slot_end);

                if overlap_start >= overlap_end {
                    continue;
                }

                let Some(shape_id) = shape_ids.get(slot_index).copied().flatten() else {
                    first_error = Some(HeapError::ManagedScanFailed {
                        source: ScanSource::Span(span_index),
                        error: Box::new(HeapError::MissingSmallSlot {
                            span_index,
                            slot_index,
                        }),
                    });
                    return;
                };
                let Some(shape) = self.shape_table.shape(shape_id) else {
                    first_error = Some(HeapError::ManagedScanFailed {
                        source: ScanSource::Span(span_index),
                        error: Box::new(HeapError::InvalidShapeId {
                            index: shape_id.index(),
                        }),
                    });
                    return;
                };

                let local_start = overlap_start.saturating_sub(slot_start);
                let local_len = overlap_end.saturating_sub(overlap_start);
                let mut first_reader_error = None;
                let result = visit_managed_references_in_reader_range(
                    &shape.scan,
                    local_start,
                    local_len,
                    self.managed_reference_bytes,
                    |start, buffer| match self.arena().fill_bytes_from(
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
                        first_error = Some(HeapError::ManagedScanFailed {
                            source: ScanSource::Span(span_index),
                            error: Box::new(error),
                        });
                        return;
                    }

                    first_error = Some(HeapError::ManagedScanFailed {
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
        pending: &mut ManagedTraceQueue,
    ) -> HeapResult<()> {
        let Some(entry) = self.large_entry(entry_id) else {
            return Err(HeapError::ManagedScanFailed {
                source: ScanSource::LargeEntry(entry_id.id()),
                error: Box::new(HeapError::MissingLargeEntry {
                    entry_id: entry_id.id(),
                }),
            });
        };
        let shape_id = entry.shape_id;
        let pages = entry.pages;
        let dirty_cards = entry.dirty_cards.clone();

        let Some(shape) = self.shape_table.shape(shape_id) else {
            return Err(HeapError::ManagedScanFailed {
                source: ScanSource::LargeEntry(entry_id.id()),
                error: Box::new(HeapError::InvalidShapeId {
                    index: shape_id.index(),
                }),
            });
        };

        let mut first_error = None;

        // scan each dirty card window directly against the large entry
        dirty_cards.visit_dirty_ranges(|card_start, card_len| {
            if first_error.is_some() {
                return;
            }

            let mut first_reader_error = None;
            let result = visit_managed_references_in_reader_range(
                &shape.scan,
                card_start,
                card_len,
                self.managed_reference_bytes,
                |start, buffer| match self.arena().fill_bytes_from(&pages, start, buffer) {
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
                    first_error = Some(HeapError::ManagedScanFailed {
                        source: ScanSource::LargeEntry(entry_id.id()),
                        error: Box::new(error),
                    });
                    return;
                }

                first_error = Some(HeapError::ManagedScanFailed {
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
