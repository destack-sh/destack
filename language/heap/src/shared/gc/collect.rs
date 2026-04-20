use crate::shared::gc::SharedGcPhase;
use crate::shared::managed::{SharedManagedLocation, SharedManagedSpace};
use crate::{
    GcKind, GcStats, HeapError, HeapResult, HeapSpace, SharedManagedReference,
    visit_shared_references_in_reader, visit_shared_references_in_reader_range,
};

impl SharedManagedSpace {
    /// Start one shared managed mark phase over explicit roots.
    pub(crate) fn start_mark(
        &mut self,
        roots: impl IntoIterator<Item = SharedManagedReference>,
    ) -> HeapResult<()> {
        if self.phase != SharedGcPhase::Idle {
            return Err(HeapError::SharedCollectionActive);
        }

        // cycle state
        self.marks.start_cycle();
        self.trace_queue.clear();
        self.sweep_cursor = 0;
        self.cycle_freed_allocations = 0;
        self.cycle_freed_bytes = 0;
        self.phase = SharedGcPhase::Mark;

        // explicit roots
        for reference in roots {
            if !reference.is_null() && !self.marks.contains(reference) {
                self.trace_queue.push(reference);
            }
        }

        Ok(())
    }

    /// Perform one full shared managed collection over explicit roots.
    pub fn collect_full(
        &mut self,
        roots: impl IntoIterator<Item = SharedManagedReference>,
    ) -> HeapResult<GcStats> {
        self.start_mark(roots)?;

        // concurrent mark
        while !self.mark_idle() {
            self.mark_step([], usize::MAX)?;
        }

        self.start_sweep()?;

        // incremental sweep
        loop {
            if let Some(stats) = self.sweep_step(usize::MAX)? {
                return Ok(stats);
            }
        }
    }

    /// Perform bounded shared mark work.
    pub(crate) fn mark_step(
        &mut self,
        roots: impl IntoIterator<Item = SharedManagedReference>,
        work_items: usize,
    ) -> HeapResult<()> {
        if self.phase != SharedGcPhase::Mark {
            return Err(HeapError::SharedCollectionNotMarking);
        }

        // publish newly discovered roots before draining the queue
        for reference in roots {
            if !reference.is_null() && !self.marks.contains(reference) {
                self.trace_queue.push(reference);
            }
        }

        let mut work_done = 0usize;

        // mark queue
        while work_done < work_items {
            let Some(reference) = self.trace_queue.pop() else {
                break;
            };

            self.trace_reference(reference)?;
            work_done += 1;
        }

        Ok(())
    }

    /// Return whether concurrent mark is currently drained.
    pub(crate) fn mark_idle(&self) -> bool {
        self.phase == SharedGcPhase::Mark && self.trace_queue.is_empty()
    }

    /// Transition from concurrent mark into sweeping.
    pub(crate) fn start_sweep(&mut self) -> HeapResult<()> {
        if self.phase != SharedGcPhase::Mark {
            return Err(HeapError::SharedCollectionNotMarking);
        }

        self.phase = SharedGcPhase::Sweep;
        self.sweep_cursor = 0;

        Ok(())
    }

    /// Perform bounded shared sweep work.
    pub(crate) fn sweep_step(&mut self, work_items: usize) -> HeapResult<Option<GcStats>> {
        let mut work_done = 0usize;

        // reference table
        while self.sweep_cursor < self.references.len() && work_done < work_items {
            let index = self.sweep_cursor;
            self.sweep_cursor += 1;
            work_done += 1;

            let Some(record) = self.references.get(index).copied() else {
                continue;
            };

            let reference_id = index.checked_add(1).ok_or(HeapError::InvariantOverflow {
                context: "shared managed reference id",
            })?;
            let reference_id = u32::try_from(reference_id).map_err(|_| {
                HeapError::InvalidSharedManagedReferenceId {
                    id: reference_id as u64,
                }
            })?;
            let reference = SharedManagedReference::new(reference_id);

            if record.is_vacant() || self.marks.contains(reference) {
                continue;
            }

            let released_bytes = self.free_reference(reference_id)?;

            self.cycle_freed_allocations = self.cycle_freed_allocations.checked_add(1).ok_or(
                HeapError::InvariantOverflow {
                    context: "shared gc freed allocation count",
                },
            )?;
            self.cycle_freed_bytes = self.cycle_freed_bytes.checked_add(released_bytes).ok_or(
                HeapError::InvariantOverflow {
                    context: "shared gc freed bytes",
                },
            )?;
        }

        if self.sweep_cursor == self.references.len() {
            return self.finish_collection().map(Some);
        }

        Ok(None)
    }

    /// Mark one shared managed reference and queue its children.
    fn trace_reference(&mut self, reference: SharedManagedReference) -> HeapResult<()> {
        let Some(index) = reference.id().checked_sub(1).map(|id| id as usize) else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };
        let Some(record) = self.references.get(index).copied() else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };
        if record.is_vacant() {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        }
        if !self.marks.mark(reference) {
            return Ok(());
        }

        let mut first_reader_error = None;
        let scan = self.scan(reference)?.clone();
        let mut edge_buffer = std::mem::take(&mut self.edge_buffer);
        edge_buffer.clear();
        let trace_result = visit_shared_references_in_reader(
            &scan,
            SharedManagedReference::BYTE_LEN,
            |start, buffer| match self.read_bytes_into(reference, start, buffer) {
                Ok(()) => true,
                Err(error) => {
                    first_reader_error.get_or_insert(error);
                    false
                }
            },
            |reference: SharedManagedReference| {
                if !reference.is_null() {
                    edge_buffer.push(reference);
                }
            },
        );

        if let Err(error) = trace_result {
            edge_buffer.clear();
            self.edge_buffer = edge_buffer;

            if let Some(error) = first_reader_error {
                return Err(error);
            }

            return Err(error);
        }

        // queue newly discovered edges after the read pass
        self.trace_queue.extend(edge_buffer.iter().copied());
        edge_buffer.clear();
        self.edge_buffer = edge_buffer;

        Ok(())
    }

    /// Free one shared managed reference by stable reference id.
    fn free_reference(&mut self, reference_id: u32) -> HeapResult<u64> {
        let reference = SharedManagedReference::new(reference_id);
        let Some(record) = self.reference(reference).copied() else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidSharedManagedReference { reference });
        };
        let released_bytes = record.byte_len() as u64;

        self.usage
            .check_free(released_bytes, HeapSpace::SharedManaged)?;

        // location release
        match location {
            SharedManagedLocation::Small(slot) => {
                self.release_small_slot(slot)?;
            }
            SharedManagedLocation::Large(entry_id) => {
                let Some(entry) = self.large_entry(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };
                let pages = entry.pages;

                let Some(entry) = self.large_entry_mut(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                entry.retire();
                self.large.free_large_entry_ids.push(entry_id.id());
                self.arena.release_page_view(&pages)?;
            }
        }

        // reference release
        self.usage.free(released_bytes, HeapSpace::SharedManaged)?;
        self.retire_reference(reference_id)?;

        Ok(released_bytes)
    }

    /// Finish one completed shared collection cycle.
    fn finish_collection(&mut self) -> HeapResult<GcStats> {
        let stats = GcStats {
            freed_allocations: self.cycle_freed_allocations,
            live_allocations: self.usage.allocation_count(),
            freed_bytes: self.cycle_freed_bytes,
            allocated_bytes: self.usage.allocated_bytes(),
            active_bytes: self.active_bytes(),
        };

        self.phase = SharedGcPhase::Idle;
        self.sweep_cursor = 0;
        self.cycle_freed_allocations = 0;
        self.cycle_freed_bytes = 0;
        self.trace_queue.clear();
        self.gc_state.record_cycle(GcKind::Full, stats)?;

        Ok(stats)
    }

    /// Record one shared managed write barrier after one completed store.
    pub(crate) fn write_shared_barrier(
        &mut self,
        reference: SharedManagedReference,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        if self.phase != SharedGcPhase::Mark {
            return Ok(());
        }

        let scan = self.scan(reference)?.clone();
        let mut first_reader_error = None;
        let mut edge_buffer = std::mem::take(&mut self.edge_buffer);
        edge_buffer.clear();
        let trace_result = visit_shared_references_in_reader_range(
            &scan,
            byte_offset,
            byte_len,
            SharedManagedReference::BYTE_LEN,
            |start, buffer| match self.read_bytes_into(reference, start, buffer) {
                Ok(()) => true,
                Err(error) => {
                    first_reader_error.get_or_insert(error);
                    false
                }
            },
            |reference: SharedManagedReference| {
                if !reference.is_null() {
                    edge_buffer.push(reference);
                }
            },
        );

        if let Err(error) = trace_result {
            edge_buffer.clear();
            self.edge_buffer = edge_buffer;

            if let Some(error) = first_reader_error {
                return Err(error);
            }

            return Err(error);
        }

        // queue any edges published by the completed write
        self.trace_queue.extend(edge_buffer.iter().copied());
        edge_buffer.clear();
        self.edge_buffer = edge_buffer;

        Ok(())
    }
}
