use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::shared::gc::SharedGcPhase;
use crate::shared::space::{SharedHeapPlace, SharedHeapSpace};
use crate::{GcKind, GcProgress, GcStats, HeapError, HeapResult, SharedHeapReference};

impl SharedHeapSpace {
    /// Transition from concurrent mark into sweeping.
    pub(crate) fn try_start_sweep(&self) -> HeapResult<bool> {
        // lifecycle
        let _lifecycle = self.gc.lock_lifecycle();

        // phase
        if self.gc.phase() != SharedGcPhase::Mark {
            return Err(HeapError::SharedCollectionNotMarking);
        }

        // termination
        self.gc.close_mark_publication();
        if !self.gc.mark_drained() {
            self.gc.open_mark_publication();

            return Ok(false);
        }

        // sweep snapshot
        let references = self.live_references()?;
        *self.gc.sweep_references.lock() = Arc::from(references);

        // sweep cursor
        self.gc.sweep_cursor.store(0, Ordering::Release);
        self.gc.set_phase(SharedGcPhase::Sweep);

        Ok(true)
    }

    /// Perform bounded shared sweep work.
    pub(crate) fn sweep_step(&self, budget_bytes: usize) -> HeapResult<GcProgress> {
        // snapshot active sweep candidates
        let references = self.gc.sweep_references.lock().clone();
        let reference_len = references.len();

        let mut swept_bytes = 0usize;
        let mut released_references = Vec::new();

        // sweep cursor
        while swept_bytes < budget_bytes {
            let index = self.gc.sweep_cursor.fetch_add(1, Ordering::AcqRel);
            if index >= reference_len {
                break;
            }

            let Some(reference) = references.get(index).copied() else {
                continue;
            };
            let Some(location) = self.resolve_location(reference) else {
                continue;
            };
            swept_bytes += location.byte_len.max(1);

            // keep reachable allocations
            if self.is_marked_place(location.place)? {
                continue;
            }

            released_references.push(reference);
        }

        // reclaimed allocation
        for reference in released_references {
            let released_bytes = self.free_reference(reference)?;

            self.gc.freed_allocations.fetch_add(1, Ordering::AcqRel);
            self.gc
                .freed_bytes
                .fetch_add(released_bytes, Ordering::AcqRel);
        }

        // end of sweep
        if self.gc.sweep_cursor.load(Ordering::Acquire) >= reference_len {
            return self.finish_collection().map(GcProgress::Complete);
        }

        Ok(GcProgress::Active)
    }

    /// Free one shared heap reference.
    fn free_reference(&self, reference: SharedHeapReference) -> HeapResult<u64> {
        // resolve the live allocation before mutating space state
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        };
        let released_bytes = location.byte_len as u64;
        let mut store = self.state.write();

        // location release
        match location.place {
            SharedHeapPlace::Small(slot) => {
                self.release_small_slot(&mut store, slot)?;
            }
            SharedHeapPlace::Large(allocation_id) => {
                // retire the large allocation record
                let Some(allocation) = store.large.allocations.get(allocation_id.index()?).cloned()
                else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };
                let (first_offset, pages) = {
                    let mut allocation = allocation.write();
                    if !allocation.is_live {
                        return Err(HeapError::MissingLargeAllocation {
                            allocation_id: allocation_id.id(),
                        });
                    }

                    let first_offset = allocation.first_offset;
                    let pages = allocation.pages;
                    allocation.retire();

                    (first_offset, pages)
                };

                // release its backing pages
                store
                    .large
                    .free_large_allocation_ids
                    .push(allocation_id.id());
                self.unmap_page_run(&mut store, first_offset, &pages);
                store
                    .page_run_cache
                    .release_page_run(&self.allocator, pages)?;
            }
        }

        Ok(released_bytes)
    }

    /// Finish one completed shared collection cycle.
    fn finish_collection(&self) -> HeapResult<GcStats> {
        // lifecycle
        let _lifecycle = self.gc.lock_lifecycle();
        let mut store = self.state.write();
        let retained_bytes = self.live_retained_bytes(&store);
        let (live_allocations, allocated_bytes) = self.live_allocated_usage(&store);

        // cycle stats
        let stats = GcStats {
            freed_allocations: self.gc.freed_allocations.load(Ordering::Acquire),
            live_allocations,
            freed_bytes: self.gc.freed_bytes.load(Ordering::Acquire),
            allocated_bytes,
            retained_bytes,
        };

        // cycle reset
        self.gc.close_mark_publication();
        self.gc.sweep_cursor.store(0, Ordering::Release);
        self.gc.mark_publishers.store(0, Ordering::Release);
        self.gc.mark_inflight.store(0, Ordering::Release);
        self.gc.freed_allocations.store(0, Ordering::Release);
        self.gc.freed_bytes.store(0, Ordering::Release);
        *self.gc.sweep_references.lock() = Arc::from([]);
        self.gc.trace_queue.clear();

        // cycle summary
        store.gc.record_cycle(GcKind::Full, stats);

        // idle publication
        self.gc.set_phase(SharedGcPhase::Idle);
        self.gc.open_mark_publication();

        Ok(stats)
    }
}
