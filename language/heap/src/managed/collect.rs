use std::mem;

use super::super::value::{ManagedReference, Value};
use super::{GcKind, GcPhase, GcStats, ManagedLocation, ManagedSpace, YOUNG_PROMOTION_AGE};

impl ManagedSpace {
    /// Begin one full GC cycle.
    pub fn begin_gc_cycle(&mut self, roots: impl IntoIterator<Item = Value>) {
        if self.gc_state.phase != GcPhase::Idle {
            return;
        }

        self.begin_collection(GcKind::Full);

        // explicit roots
        for root in roots {
            self.mark_value(root);
        }
    }

    /// Begin one young-generation GC cycle.
    pub fn begin_minor_gc_cycle(&mut self, roots: impl IntoIterator<Item = ManagedReference>) {
        if self.gc_state.phase != GcPhase::Idle {
            return;
        }

        let (dirty_spans, dirty_large_allocations) = self.begin_collection(GcKind::Minor);

        // explicit roots
        for root in roots {
            self.seed_minor_root(root);
        }

        // dirty mature regions
        self.scan_minor_dirty_spans(dirty_spans);
        self.scan_minor_dirty_large_allocations(dirty_large_allocations);
    }

    /// Advance one incremental GC step.
    pub fn gc_step(&mut self, budget: usize) {
        let mut remaining = budget;

        if remaining == 0 {
            return;
        }

        // drain the current mark queue within the requested budget
        if self.gc_state.phase == GcPhase::Mark {
            while remaining > 0 {
                let Some(handle) = self.mark_queue.pop() else {
                    self.gc_state.phase = GcPhase::Sweep;
                    break;
                };

                remaining -= 1;
                self.trace_pointer(handle);
            }
        }

        // complete sweep once marking is done
        if self.gc_state.phase == GcPhase::Sweep {
            self.finish_gc_cycle();
        }
    }

    /// Run one full GC cycle immediately.
    pub fn collect_handles(
        &mut self,
        roots: impl IntoIterator<Item = ManagedReference>,
    ) -> GcStats {
        self.begin_gc_cycle(roots.into_iter().map(Value::managed_reference));
        self.finish_gc_cycle();

        self.gc_state.last_stats.unwrap_or_default()
    }

    /// Run one young-generation collection immediately.
    pub fn collect_young_handles(
        &mut self,
        roots: impl IntoIterator<Item = ManagedReference>,
    ) -> GcStats {
        self.begin_minor_gc_cycle(roots);
        self.finish_gc_cycle();

        self.gc_state.last_stats.unwrap_or_default()
    }

    /// Complete one GC cycle immediately.
    pub fn finish_gc_cycle(&mut self) {
        if self.gc_state.phase == GcPhase::Idle {
            return;
        }

        // drain any remaining mark work before sweeping
        while let Some(handle) = self.mark_queue.pop() {
            self.trace_pointer(handle);
        }

        let kind = self.gc_state.kind.unwrap_or(GcKind::Full);
        let stats = match kind {
            GcKind::Full => self.finish_full_collection(),
            GcKind::Minor => self.finish_minor_collection(),
        };

        self.gc_state.finish_cycle(stats);
        self.mark_queue.clear();
        self.young.clear_marks();
        if let Some(mut from_space) = self.young_from.take() {
            from_space.reset(&mut self.page_arena);
        }
        self.dirty_spans = mem::take(&mut self.dirty_next_spans);
        self.dirty_large_allocations = mem::take(&mut self.dirty_next_large_allocations);
        self.mark_retained_bytes_dirty();
        self.refresh_retained_bytes();
    }

    /// Clear pending mark work without completing a collection cycle.
    pub(crate) fn clear_mark_queue(&mut self) {
        self.mark_queue.clear();
    }

    /// Mark one managed reference if it has not already been marked this cycle.
    pub fn mark_reference(&mut self, handle: ManagedReference) {
        match self.gc_state.kind {
            Some(GcKind::Full) => {
                let _ = self.mark_reference_full(handle);
            }
            Some(GcKind::Minor) => {
                let _ = self.mark_reference_minor(handle);
            }
            None => {}
        }
    }

    /// Mark one runtime value if it contains one managed reference.
    pub fn mark_value(&mut self, value: Value) {
        if let Some(handle) = value.as_managed_reference() {
            self.mark_reference(handle);
        }
    }

    // collection setup

    fn begin_collection(
        &mut self,
        kind: GcKind,
    ) -> (Vec<usize>, Vec<super::ManagedLargeAllocationId>) {
        self.gc_state.begin_cycle(kind);
        self.clear_mark_queue();
        self.dirty_next_spans.clear();
        self.dirty_next_large_allocations.clear();
        self.mark_retained_bytes_dirty();

        let dirty_spans = mem::take(&mut self.dirty_spans);
        let dirty_large_allocations = mem::take(&mut self.dirty_large_allocations);

        // clear queue flags before rebuilding dirty mature regions
        for span_index in dirty_spans.iter().copied() {
            let Some(span) = self.small.spans.get_mut(span_index) else {
                continue;
            };
            span.set_dirty_queued(false);
        }

        for large_allocation_id in dirty_large_allocations.iter().copied() {
            let Some(large_allocation) = self.large_allocation_mut(large_allocation_id) else {
                continue;
            };
            large_allocation.set_dirty_queued(false);
        }

        // clear mature marks before one full cycle
        if kind == GcKind::Full {
            for span in &mut self.small.spans {
                span.clear_marks();
            }

            for large_allocation in &mut self.large.large_allocations {
                large_allocation.clear_mark();
            }
        }

        // flip the active young generation into from-space
        let successor = self.young.successor();
        let from_space = mem::replace(&mut self.young, successor);
        self.young_from = Some(from_space);

        (dirty_spans, dirty_large_allocations)
    }

    fn seed_minor_root(&mut self, handle: ManagedReference) {
        let base = ManagedReference::new(handle.id());
        let Some(location) = self.location(base) else {
            return;
        };

        match location {
            ManagedLocation::Vacant => {}
            ManagedLocation::Young(_) => {
                let _ = self.mark_reference_minor(base);
            }
            ManagedLocation::Small(_) | ManagedLocation::Large(_) => {
                self.mark_queue.push(base);
            }
        }
    }

    // sweep

    fn finish_full_collection(&mut self) -> GcStats {
        let live_handles = self.live_handles.clone();
        let from_generation = self
            .young_from
            .as_ref()
            .map(|space| space.generation())
            .unwrap_or_default();
        let mut freed_allocations = 0usize;
        let mut freed_bytes = 0u64;

        // sweep only live handles from this cycle
        for reference_id in live_handles {
            let handle = ManagedReference::new(reference_id);
            let Some(location) = self.location(handle) else {
                continue;
            };

            // live current-young survivors stay as-is
            if let ManagedLocation::Young(young_id) = location
                && young_id.generation() != from_generation
            {
                continue;
            }

            let is_dead = match location {
                ManagedLocation::Vacant => false,
                ManagedLocation::Young(young_id) => {
                    young_id.generation() == from_generation && self.is_young_allocated(young_id)
                }
                ManagedLocation::Small(slot) => {
                    let Some(span) = self.small.spans.get(slot.span_index()) else {
                        continue;
                    };

                    span.is_occupied(slot.slot_index()) && !span.is_marked(slot.slot_index())
                }
                ManagedLocation::Large(large_allocation_id) => {
                    let Some(large_allocation) = self.large_allocation(large_allocation_id) else {
                        continue;
                    };

                    large_allocation.is_allocated() && !large_allocation.is_marked()
                }
            };

            if !is_dead {
                continue;
            }

            let Some(allocation_bytes) = self.byte_len(handle).map(|len| len as u64) else {
                continue;
            };
            self.free_storage(location);
            self.release_dead_handle(handle.id(), allocation_bytes);

            freed_allocations += 1;
            freed_bytes = freed_bytes.saturating_add(allocation_bytes);
        }

        GcStats {
            freed_allocations,
            live_allocations: self.allocated_count,
            freed_bytes,
            allocated_bytes: self.allocated_bytes,
            active_bytes: self.active_bytes(),
        }
    }

    fn finish_minor_collection(&mut self) -> GcStats {
        let live_handles = self.live_handles.clone();
        let from_generation = self
            .young_from
            .as_ref()
            .map(|space| space.generation())
            .unwrap_or_default();
        let mut freed_allocations = 0usize;
        let mut freed_bytes = 0u64;

        // free dead from-space handles while keeping mature allocations untouched
        for reference_id in live_handles {
            let handle = ManagedReference::new(reference_id);
            let Some(ManagedLocation::Young(young_id)) = self.location(handle) else {
                continue;
            };

            if young_id.generation() != from_generation || !self.is_young_allocated(young_id) {
                continue;
            }

            let Some(allocation_bytes) = self.byte_len(handle).map(|len| len as u64) else {
                continue;
            };
            self.free_storage(ManagedLocation::Young(young_id));
            self.release_dead_handle(handle.id(), allocation_bytes);

            freed_allocations += 1;
            freed_bytes = freed_bytes.saturating_add(allocation_bytes);
        }

        GcStats {
            freed_allocations,
            live_allocations: self.allocated_count,
            freed_bytes,
            allocated_bytes: self.allocated_bytes,
            active_bytes: self.active_bytes(),
        }
    }

    fn release_dead_handle(&mut self, handle_id: u64, allocation_bytes: u64) {
        self.remove_live_handle(handle_id);
        let next_free = self.free_handle_head;
        let Some(entry) = self.handle_entry_mut(handle_id) else {
            return;
        };
        *entry = super::ManagedHandleEntry::Free { next_free };
        self.free_handle_head = handle_id;
        self.allocated_count = self.allocated_count.saturating_sub(1);
        self.allocated_bytes = self.allocated_bytes.saturating_sub(allocation_bytes);
    }

    // mark and evacuation

    fn mark_reference_full(&mut self, handle: ManagedReference) -> bool {
        let base = ManagedReference::new(handle.id());
        let Some(location) = self.location(base) else {
            return false;
        };

        match location {
            ManagedLocation::Vacant => false,
            ManagedLocation::Young(_) => self.evacuate_young_reference(base),
            ManagedLocation::Small(slot) => {
                let Some(span) = self.small.spans.get_mut(slot.span_index()) else {
                    return false;
                };

                if !span.is_occupied(slot.slot_index()) || span.is_marked(slot.slot_index()) {
                    return false;
                }

                span.mark(slot.slot_index());
                self.mark_queue.push(base);
                false
            }
            ManagedLocation::Large(large_allocation_id) => {
                let Some(large_allocation) = self.large_allocation_mut(large_allocation_id) else {
                    return false;
                };

                if !large_allocation.is_allocated() || large_allocation.is_marked() {
                    return false;
                }

                large_allocation.mark();
                self.mark_queue.push(base);
                false
            }
        }
    }

    fn mark_reference_minor(&mut self, handle: ManagedReference) -> bool {
        let base = ManagedReference::new(handle.id());
        let Some(location) = self.location(base) else {
            return false;
        };

        match location {
            ManagedLocation::Young(_) => self.evacuate_young_reference(base),
            ManagedLocation::Vacant | ManagedLocation::Small(_) | ManagedLocation::Large(_) => {
                false
            }
        }
    }

    fn evacuate_young_reference(&mut self, handle: ManagedReference) -> bool {
        let base = ManagedReference::new(handle.id());
        let Some(ManagedLocation::Young(young_id)) = self.location(base) else {
            return false;
        };

        // already evacuated into the current young generation
        if self.young.is_allocated(young_id) {
            return true;
        }

        let Some(byte_len) = self.young_byte_len(young_id) else {
            return false;
        };
        let Some(trace_id) = self.young_trace_id(young_id) else {
            return false;
        };
        let layout_id = self.young_layout_id(young_id);
        let age = self.young_age(young_id).unwrap_or(0).saturating_add(1);
        let Some(bytes) = self.young_bytes(young_id).map(ToOwned::to_owned) else {
            return false;
        };

        let stays_small = self.small.size_classes.class_index_for(byte_len).is_some();
        let should_stay_young = stays_small && age < YOUNG_PROMOTION_AGE;

        if should_stay_young
            && let Some(survivor_id) =
                self.young
                    .allocate_survivor(&bytes, trace_id, layout_id, age, &mut self.page_arena)
        {
            let Some(entry) = self.handle_mut(base) else {
                return false;
            };
            entry.location = ManagedLocation::Young(survivor_id);
            self.mark_queue.push(base);

            return true;
        }

        let location = self.allocate_mature_location(&bytes, trace_id, layout_id);

        // mature allocations created during a full cycle count as marked immediately
        if self.gc_state.kind == Some(GcKind::Full) {
            match location {
                ManagedLocation::Small(slot) => {
                    if let Some(span) = self.small.spans.get_mut(slot.span_index()) {
                        span.mark(slot.slot_index());
                    }
                }
                ManagedLocation::Large(large_allocation_id) => {
                    if let Some(large_allocation) = self.large_allocation_mut(large_allocation_id) {
                        large_allocation.mark();
                    }
                }
                ManagedLocation::Vacant | ManagedLocation::Young(_) => {}
            }
        }

        let Some(entry) = self.handle_mut(base) else {
            return false;
        };
        entry.location = location;
        self.mark_queue.push(base);

        false
    }

    // tracing

    fn trace_pointer(&mut self, handle: ManagedReference) {
        let Some(kind) = self.gc_state.kind else {
            return;
        };
        let base = ManagedReference::new(handle.id());
        let is_mature_parent = matches!(
            self.location(base),
            Some(ManagedLocation::Small(_) | ManagedLocation::Large(_))
        );
        let mut references = std::mem::take(&mut self.trace_scratch);
        references.clear();

        self.collect_outgoing_references(handle, &mut references);

        let mut keeps_young_edges = false;

        for reference in references.iter().copied() {
            let reaches_young = match kind {
                GcKind::Full => self.mark_reference_full(reference),
                GcKind::Minor => self.mark_reference_minor(reference),
            };

            keeps_young_edges |= is_mature_parent && reaches_young;
        }

        if is_mature_parent && keeps_young_edges {
            self.mark_mature_allocation_dirty(base);
        }

        self.trace_scratch = references;
    }

    fn collect_outgoing_references(
        &self,
        handle: ManagedReference,
        references: &mut Vec<ManagedReference>,
    ) {
        let base = ManagedReference::new(handle.id());
        let Some(byte_len) = self.handle(base).map(|handle| handle.byte_len) else {
            return;
        };
        let Some(location) = self.location(base) else {
            return;
        };

        let Some(reference_map) = self.reference_map(handle) else {
            return;
        };

        // young and small storage are contiguous, so trace directly from the slot bytes
        match location {
            ManagedLocation::Young(young_id) => {
                let Some(bytes) = self.young_bytes(young_id) else {
                    return;
                };

                reference_map.for_each_reference(
                    bytes,
                    self.managed_reference_bytes,
                    |reference| {
                        references.push(reference);
                    },
                );
            }
            ManagedLocation::Small(slot) => {
                let Some(span) = self.small.spans.get(slot.span_index()) else {
                    return;
                };
                let Some(bytes) = span.bytes(&self.page_arena, slot.slot_index(), byte_len) else {
                    return;
                };

                reference_map.for_each_reference(
                    &bytes,
                    self.managed_reference_bytes,
                    |reference| {
                        references.push(reference);
                    },
                );
            }
            // large-allocation storage may be chunked, so read only the traced windows
            ManagedLocation::Large(large_allocation_id) => {
                let Some(large_allocation) = self.large_allocation(large_allocation_id) else {
                    return;
                };

                reference_map.for_each_reference_in_reader(
                    self.managed_reference_bytes,
                    |start, dest| large_allocation.read_window(&self.page_arena, start, dest),
                    |reference| references.push(reference),
                );
            }
            ManagedLocation::Vacant => {}
        }
    }

    fn scan_minor_dirty_spans(&mut self, dirty_spans: Vec<usize>) {
        for span_index in dirty_spans {
            let mut card_index = 0usize;

            loop {
                let next_card = self
                    .small
                    .spans
                    .get(span_index)
                    .and_then(|span| span.first_dirty_card_from(card_index));
                let Some(card_index_found) = next_card else {
                    break;
                };

                let keeps_young = self.scan_minor_dirty_span_card(span_index, card_index_found);
                let Some(span) = self.small.spans.get_mut(span_index) else {
                    break;
                };

                span.clear_dirty_card(card_index_found);
                if keeps_young {
                    span.mark_dirty_card(card_index_found);
                }

                card_index = card_index_found.saturating_add(1);
            }

            let should_requeue = self
                .small
                .spans
                .get(span_index)
                .map(|span| span.has_dirty_cards())
                .unwrap_or(false);
            if should_requeue {
                self.enqueue_dirty_span(span_index);
            }
        }
    }

    fn scan_minor_dirty_large_allocations(
        &mut self,
        dirty_large_allocations: Vec<super::ManagedLargeAllocationId>,
    ) {
        for large_allocation_id in dirty_large_allocations {
            let mut card_index = 0usize;

            loop {
                let next_card =
                    self.large_allocation(large_allocation_id)
                        .and_then(|large_allocation| {
                            large_allocation.first_dirty_card_from(card_index)
                        });
                let Some(card_index_found) = next_card else {
                    break;
                };

                let keeps_young = self
                    .scan_minor_dirty_large_allocation_card(large_allocation_id, card_index_found);
                let Some(large_allocation) = self.large_allocation_mut(large_allocation_id) else {
                    break;
                };

                large_allocation.clear_dirty_card(card_index_found);
                if keeps_young {
                    large_allocation.mark_dirty_card(card_index_found);
                }

                card_index = card_index_found.saturating_add(1);
            }

            let should_requeue = self
                .large_allocation(large_allocation_id)
                .map(|large_allocation| large_allocation.has_dirty_cards())
                .unwrap_or(false);
            if should_requeue {
                self.enqueue_dirty_large_allocation(large_allocation_id);
            }
        }
    }

    fn scan_minor_dirty_span_card(&mut self, span_index: usize, card_index: usize) -> bool {
        let Some((card_start, card_len, slot_count, size_class)) =
            self.small.spans.get(span_index).map(|span| {
                (
                    span.dirty_card_start(card_index),
                    span.dirty_card_len(card_index),
                    span.slot_count(),
                    span.size_class(),
                )
            })
        else {
            return false;
        };

        let card_end = card_start.saturating_add(card_len);
        let first_slot = card_start / size_class;
        let last_slot = (card_end.saturating_sub(1)) / size_class;
        let mut keeps_young = false;

        for slot_index in first_slot..=last_slot.min(slot_count.saturating_sub(1)) {
            let Some(span) = self.small.spans.get(span_index) else {
                return keeps_young;
            };
            if !span.is_occupied(slot_index) {
                continue;
            }

            let slot_start = slot_index.saturating_mul(size_class);
            let overlap_start = card_start.max(slot_start).saturating_sub(slot_start);
            let overlap_end = card_end
                .min(slot_start + size_class)
                .saturating_sub(slot_start);
            let overlap_len = overlap_end.saturating_sub(overlap_start);
            let Some(reference_map) = span
                .trace_id(slot_index)
                .and_then(|trace_id| self.reference_map_table.get(trace_id))
                .cloned()
            else {
                continue;
            };

            if !reference_map.touches_managed_range(
                overlap_start,
                overlap_len,
                self.managed_reference_bytes,
            ) {
                continue;
            }

            let mut references = Vec::new();

            reference_map.for_each_reference_in_reader_range(
                overlap_start,
                overlap_len,
                self.managed_reference_bytes,
                |start, dest| {
                    let Some(span) = self.small.spans.get(span_index) else {
                        return false;
                    };

                    span.read_window(&self.page_arena, slot_start + start, dest)
                },
                |reference| references.push(reference),
            );

            for reference in references {
                keeps_young |= self.mark_reference_minor(reference);
            }
        }

        keeps_young
    }

    fn scan_minor_dirty_large_allocation_card(
        &mut self,
        large_allocation_id: super::ManagedLargeAllocationId,
        card_index: usize,
    ) -> bool {
        let Some((card_start, card_len, trace_id)) = self
            .large_allocation(large_allocation_id)
            .map(|large_allocation| {
                (
                    large_allocation.dirty_card_start(card_index),
                    large_allocation.dirty_card_len(card_index),
                    large_allocation.trace_id(),
                )
            })
        else {
            return false;
        };
        let Some(reference_map) = self.reference_map_table.get(trace_id).cloned() else {
            return false;
        };
        let mut keeps_young = false;

        let mut references = Vec::new();

        reference_map.for_each_reference_in_reader_range(
            card_start,
            card_len,
            self.managed_reference_bytes,
            |start, dest| {
                let Some(large_allocation) = self.large_allocation(large_allocation_id) else {
                    return false;
                };

                large_allocation.read_window(&self.page_arena, start, dest)
            },
            |reference| references.push(reference),
        );

        for reference in references {
            keeps_young |= self.mark_reference_minor(reference);
        }

        keeps_young
    }
}
