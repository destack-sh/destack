use super::super::value::{ManagedReference, Value};
use super::heap::FIRST_ALLOCATED_REFERENCE_ID;
use super::{GcPhase, GcStats, ManagedLocation, ManagedSpace};

impl ManagedSpace {
    /// Begin one GC cycle.
    pub fn begin_gc_cycle(&mut self, roots: impl IntoIterator<Item = Value>) {
        if self.gc_state.phase != GcPhase::Idle {
            return;
        }

        self.gc_state.begin_cycle();
        self.clear_mark_queue();

        // clear run and extent marks before seeding new roots
        for run in &mut self.runs {
            run.clear_marks();
        }

        for extent in &mut self.extents {
            extent.clear_mark();
        }

        // seed the mark queue from explicit roots
        for root in roots {
            self.mark_value(root);
        }
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

    /// Complete one GC cycle immediately.
    pub fn finish_gc_cycle(&mut self) {
        if self.gc_state.phase == GcPhase::Idle {
            return;
        }

        // drain any remaining mark work before sweeping
        while let Some(handle) = self.mark_queue.pop() {
            self.trace_pointer(handle);
        }

        let mut freed_allocations = 0usize;
        let mut freed_bytes = 0u64;
        let previous_next_unused_id = self.next_unused_id;

        // sweep stable handles in id order so reuse stays predictable
        for reference_id in FIRST_ALLOCATED_REFERENCE_ID..previous_next_unused_id {
            let handle = ManagedReference::new(reference_id);
            let Some(location) = self.location(handle) else {
                continue;
            };

            let Some(is_allocated) = self.is_location_allocated(location) else {
                continue;
            };
            if !is_allocated {
                continue;
            }

            let Some(is_marked) = self.is_location_marked(location) else {
                continue;
            };
            if is_marked {
                continue;
            }

            let Some(allocation_bytes) = self.byte_len(handle).map(|len| len as u64) else {
                continue;
            };
            self.free_storage(location);
            let Some(location_slot) = self.location_entry_mut(reference_id) else {
                continue;
            };
            *location_slot = ManagedLocation::Vacant;
            self.free_ids.push(reference_id);
            self.allocated_count = self.allocated_count.saturating_sub(1);
            self.allocated_bytes = self.allocated_bytes.saturating_sub(allocation_bytes);

            freed_allocations += 1;
            freed_bytes = freed_bytes.saturating_add(allocation_bytes);
        }

        let stats = GcStats {
            freed_allocations,
            live_allocations: self.allocated_count,
            freed_bytes,
            live_bytes: self.allocated_bytes,
            heap_bytes: self.retained_bytes,
        };

        self.gc_state.finish_cycle(stats);
        self.mark_queue.clear();
        self.recompute_retained_bytes();
    }

    /// Clear pending mark work without completing a collection cycle.
    pub(crate) fn clear_mark_queue(&mut self) {
        self.mark_queue.clear();
    }

    /// Mark one managed reference if it has not already been marked this cycle.
    pub fn mark_reference(&mut self, handle: ManagedReference) {
        let reference_id = handle.id();

        if reference_id == 0 || reference_id >= self.next_unused_id {
            return;
        }

        let Some(location) = self.location(handle) else {
            return;
        };

        let Some(is_allocated) = self.is_location_allocated(location) else {
            return;
        };
        if !is_allocated {
            return;
        }

        let Some(is_marked) = self.is_location_marked(location) else {
            return;
        };
        if is_marked {
            return;
        }

        let is_marked = self.mark_location(location);
        if !is_marked {
            return;
        }

        self.mark_queue.push(handle);
    }

    /// Mark one runtime value if it contains one managed reference.
    pub fn mark_value(&mut self, value: Value) {
        if let Some(handle) = value.as_managed_reference() {
            self.mark_reference(handle);
        }
    }

    /// Return whether one managed location is currently allocated.
    fn is_location_allocated(&self, location: ManagedLocation) -> Option<bool> {
        match location {
            ManagedLocation::Vacant => Some(false),
            ManagedLocation::Run(slot) => {
                let run = self.runs.get(slot.run_index())?;

                Some(run.is_occupied(slot.slot_index()))
            }
            ManagedLocation::Extent(extent_id) => Some(self.extent(extent_id)?.is_allocated()),
        }
    }

    /// Return whether one managed location is marked in the current cycle.
    fn is_location_marked(&self, location: ManagedLocation) -> Option<bool> {
        match location {
            ManagedLocation::Vacant => Some(false),
            ManagedLocation::Run(slot) => {
                let run = self.runs.get(slot.run_index())?;

                Some(run.is_marked(slot.slot_index()))
            }
            ManagedLocation::Extent(extent_id) => Some(self.extent(extent_id)?.is_marked()),
        }
    }

    /// Mark one managed location.
    fn mark_location(&mut self, location: ManagedLocation) -> bool {
        match location {
            ManagedLocation::Vacant => false,
            ManagedLocation::Run(slot) => {
                let Some(run) = self.runs.get_mut(slot.run_index()) else {
                    return false;
                };

                run.mark(slot.slot_index());
                true
            }
            ManagedLocation::Extent(extent_id) => {
                let Some(extent) = self.extent_mut(extent_id) else {
                    return false;
                };

                extent.mark();
                true
            }
        }
    }

    // follow one pointer's outgoing references
    fn trace_pointer(&mut self, handle: ManagedReference) {
        let Some(reference_map) = self.reference_map(handle).cloned() else {
            return;
        };
        let Some(bytes) = self.bytes_to_vec(handle) else {
            return;
        };
        let mut references = Vec::new();

        reference_map.for_each_reference(&bytes, |reference| references.push(reference));

        for reference in references {
            self.mark_reference(reference);
        }
    }
}
