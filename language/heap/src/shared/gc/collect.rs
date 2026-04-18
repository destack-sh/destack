use crate::core::{visit_edge_map_in_reader, visit_edge_map_in_reader_range};
use crate::shared::gc::SharedGcPhase;
use crate::shared::managed::{SharedManagedLocation, SharedManagedSpace};
use crate::{GcKind, GcStats, HeapDomain, HeapError, HeapResult, SharedManagedReference, Value};

impl SharedManagedSpace {
    /// Start one shared managed collection over explicit roots.
    pub fn start_collection(
        &mut self,
        roots: impl IntoIterator<Item = SharedManagedReference>,
    ) -> HeapResult<()> {
        if self.phase != SharedGcPhase::Idle {
            return Err(HeapError::SharedCollectionActive);
        }

        // cycle state
        self.marks.clear();
        self.marks.resize(self.references.len(), false);
        self.trace_queue.clear();
        self.sweep_cursor = 0;
        self.cycle_freed_allocations = 0;
        self.cycle_freed_bytes = 0;
        self.phase = SharedGcPhase::Mark;

        // explicit roots
        for reference in roots {
            if !reference.is_null() {
                self.trace_queue.push(reference);
            }
        }

        Ok(())
    }

    /// Perform bounded shared managed collector work.
    pub fn collect_step(&mut self, work_budget: usize) -> HeapResult<Option<GcStats>> {
        if work_budget == 0 {
            return Ok(None);
        }

        match self.phase {
            SharedGcPhase::Idle => Ok(None),
            SharedGcPhase::Mark => self.mark_step(work_budget),
            SharedGcPhase::Sweep => self.sweep_step(work_budget),
        }
    }

    /// Perform one full shared managed collection over explicit roots.
    pub fn collect(
        &mut self,
        roots: impl IntoIterator<Item = SharedManagedReference>,
    ) -> HeapResult<GcStats> {
        self.start_collection(roots)?;

        // concurrent mark
        while !self.is_mark_idle() {
            self.collect_step(usize::MAX)?;
        }

        // mark termination
        self.finish_mark([])?;

        // incremental sweep
        loop {
            if let Some(stats) = self.collect_step(usize::MAX)? {
                return Ok(stats);
            }
        }
    }

    /// Perform bounded shared mark work.
    fn mark_step(&mut self, work_budget: usize) -> HeapResult<Option<GcStats>> {
        let mut work_done = 0usize;

        // mark queue
        while work_done < work_budget {
            let Some(reference) = self.trace_queue.pop() else {
                break;
            };

            self.trace_reference(reference)?;
            work_done += 1;
        }

        Ok(None)
    }

    /// Return whether concurrent mark is currently drained.
    pub fn is_mark_idle(&self) -> bool {
        self.phase == SharedGcPhase::Mark && self.trace_queue.is_empty()
    }

    /// Finish shared marking after the final root handshake.
    pub fn finish_mark(
        &mut self,
        roots: impl IntoIterator<Item = SharedManagedReference>,
    ) -> HeapResult<()> {
        if self.phase != SharedGcPhase::Mark {
            return Err(HeapError::SharedCollectionNotMarking);
        }

        // final roots
        for reference in roots {
            if !reference.is_null() {
                self.trace_queue.push(reference);
            }
        }

        // final drain
        while let Some(reference) = self.trace_queue.pop() {
            self.trace_reference(reference)?;
        }

        self.phase = SharedGcPhase::Sweep;
        self.sweep_cursor = 0;

        Ok(())
    }

    /// Perform bounded shared sweep work.
    fn sweep_step(&mut self, work_budget: usize) -> HeapResult<Option<GcStats>> {
        let mut work_done = 0usize;

        // reference table
        while self.sweep_cursor < self.references.len() && work_done < work_budget {
            let index = self.sweep_cursor;
            self.sweep_cursor += 1;
            work_done += 1;

            let Some(record) = self.references.get(index).copied() else {
                continue;
            };

            if record.is_vacant() || self.marks.get(index).copied().unwrap_or(false) {
                continue;
            }

            let reference_id = index.checked_add(1).ok_or(HeapError::InvariantOverflow {
                context: "shared managed reference id",
            })?;
            let reference_id = u32::try_from(reference_id).map_err(|_| {
                HeapError::InvalidSharedManagedReferenceId {
                    id: reference_id as u64,
                }
            })?;
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
        if self.marks[index] {
            return Ok(());
        }

        self.marks[index] = true;
        let mut first_reader_error = None;
        let edge_map = self.edge_map(reference)?.clone();
        let mut trace_buffer = std::mem::take(&mut self.trace_buffer);
        trace_buffer.clear();
        let trace_result = visit_edge_map_in_reader(
            &edge_map,
            SharedManagedReference::BYTE_LEN,
            |start, buffer| match self.read_bytes_into(reference, start, buffer) {
                Ok(()) => true,
                Err(error) => {
                    first_reader_error.get_or_insert(error);
                    false
                }
            },
            decode_shared_reference_window,
            decode_shared_value_reference_window,
            |reference| {
                if !reference.is_null() {
                    trace_buffer.push(reference);
                }
            },
        );

        if let Err(error) = trace_result {
            trace_buffer.clear();
            self.trace_buffer = trace_buffer;

            if let Some(error) = first_reader_error {
                return Err(error);
            }

            return Err(error);
        }

        // queue newly discovered edges after the read pass
        self.trace_queue.extend(trace_buffer.iter().copied());
        trace_buffer.clear();
        self.trace_buffer = trace_buffer;

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

        self.totals.check_free(released_bytes, HeapDomain::Shared)?;

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
        self.totals.free(released_bytes, HeapDomain::Shared)?;
        self.retire_reference(reference_id)?;

        Ok(released_bytes)
    }

    /// Finish one completed shared collection cycle.
    fn finish_collection(&mut self) -> HeapResult<GcStats> {
        let stats = GcStats {
            freed_allocations: self.cycle_freed_allocations,
            live_allocations: self.totals.allocation_count(),
            freed_bytes: self.cycle_freed_bytes,
            allocated_bytes: self.totals.allocated_bytes(),
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

        let edge_map = self.edge_map(reference)?.clone();
        let mut first_reader_error = None;
        let mut trace_buffer = std::mem::take(&mut self.trace_buffer);
        trace_buffer.clear();
        let trace_result = visit_edge_map_in_reader_range(
            &edge_map,
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
            decode_shared_reference_window,
            decode_shared_value_reference_window,
            |reference| {
                if !reference.is_null() {
                    trace_buffer.push(reference);
                }
            },
        );

        if let Err(error) = trace_result {
            trace_buffer.clear();
            self.trace_buffer = trace_buffer;

            if let Some(error) = first_reader_error {
                return Err(error);
            }

            return Err(error);
        }

        // queue any edges published by the completed write
        self.trace_queue.extend(trace_buffer.iter().copied());
        trace_buffer.clear();
        self.trace_buffer = trace_buffer;

        Ok(())
    }
}

/// Decode one direct shared managed reference from one traced window.
fn decode_shared_reference_window(window: &[u8]) -> HeapResult<SharedManagedReference> {
    let mut raw = [0u8; SharedManagedReference::BYTE_LEN];
    raw.copy_from_slice(window);
    let bits = u64::from_le_bytes(raw);

    Ok(SharedManagedReference::from_bits(bits))
}

/// Decode one shared managed reference stored inside one full value window.
fn decode_shared_value_reference_window(
    window: &[u8],
    start: usize,
) -> HeapResult<Option<SharedManagedReference>> {
    let value =
        Value::from_byte_slice(window).ok_or(HeapError::InvalidReferenceValuePayload { start })?;

    Ok(value.as_shared_managed_reference())
}
