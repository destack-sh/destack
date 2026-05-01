use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{GcPacer, GcStats, HeapOptions};

/// Shared collector pacing state.
#[derive(Debug, Default)]
pub(crate) struct SharedGcPacer {
    /// The live shared heap bytes after the last completed cycle.
    live_bytes: AtomicUsize,
    /// Estimated shared collector work for one complete cycle.
    estimated_work_bytes: AtomicUsize,
    /// Estimated shared collector work not yet issued to collector steps.
    remaining_work_bytes: AtomicUsize,
    /// Pending shared collector assist debt in work bytes.
    assist_debt_bytes: AtomicUsize,
}

impl SharedGcPacer {
    /// Derive shared pacing targets from the current live heap size.
    pub(crate) fn set_live_bytes(&self, options: &HeapOptions, live_bytes: u64) {
        // derive generic pacer targets
        let mut gc_pacer = GcPacer::default();
        gc_pacer.set_live_bytes(options.gc, live_bytes);

        // publish shared pacer state
        self.live_bytes
            .store(gc_work_usize(gc_pacer.live_bytes), Ordering::Release);
        self.estimated_work_bytes.store(
            gc_work_usize(gc_pacer.estimated_work_bytes),
            Ordering::Release,
        );
    }

    /// Copy this shared pacer into one independent heap.
    pub(crate) fn fork(&self) -> Self {
        Self {
            live_bytes: AtomicUsize::new(self.live_bytes.load(Ordering::Acquire)),
            estimated_work_bytes: AtomicUsize::new(
                self.estimated_work_bytes.load(Ordering::Acquire),
            ),
            remaining_work_bytes: AtomicUsize::new(
                self.remaining_work_bytes.load(Ordering::Acquire),
            ),
            assist_debt_bytes: AtomicUsize::new(self.assist_debt_bytes.load(Ordering::Acquire)),
        }
    }

    /// Return one pacer snapshot from current shared heap bytes.
    pub(crate) fn snapshot(&self, options: &HeapOptions, heap_bytes: u64) -> GcPacer {
        // rebuild the generic pacer from atomic shared state
        let live_bytes = self.live_bytes.load(Ordering::Acquire) as u64;
        let mut gc_pacer = GcPacer {
            estimated_work_bytes: self.estimated_work_bytes.load(Ordering::Acquire) as u64,
            remaining_work_bytes: self.remaining_work_bytes.load(Ordering::Acquire) as u64,
            assist_debt_bytes: self.assist_debt_bytes.load(Ordering::Acquire) as u64,
            ..GcPacer::default()
        };

        // keep an empty heap able to start its first cycle
        gc_pacer.set_live_bytes(options.gc, live_bytes);
        if gc_pacer.goal_bytes == 0 && heap_bytes == 0 {
            gc_pacer.set_live_bytes(options.gc, heap_bytes);
        }

        gc_pacer
    }

    /// Start one shared collection cycle.
    pub(crate) fn begin_cycle(&self, options: &HeapOptions, heap_bytes: u64) {
        // derive cycle work from current heap usage
        let mut gc_pacer = self.snapshot(options, heap_bytes);
        gc_pacer.begin_cycle(options.gc, heap_bytes);

        // publish new cycle work counters
        self.estimated_work_bytes.store(
            gc_work_usize(gc_pacer.estimated_work_bytes),
            Ordering::Release,
        );
        self.remaining_work_bytes.store(
            gc_work_usize(gc_pacer.remaining_work_bytes),
            Ordering::Release,
        );
        self.assist_debt_bytes.store(0, Ordering::Release);
    }

    /// Record one completed shared collection cycle.
    pub(crate) fn record_cycle(&self, options: &HeapOptions, stats: GcStats) {
        // fold completed stats into the generic pacer
        let mut gc_pacer = self.snapshot(options, stats.allocated_bytes);
        gc_pacer.record_cycle(options.gc, stats);

        // publish idle cycle state
        self.estimated_work_bytes.store(
            gc_work_usize(gc_pacer.estimated_work_bytes),
            Ordering::Release,
        );
        self.live_bytes
            .store(gc_work_usize(gc_pacer.live_bytes), Ordering::Release);
        self.remaining_work_bytes.store(0, Ordering::Release);
        self.assist_debt_bytes.store(0, Ordering::Release);
    }

    /// Charge one shared allocation against current collection runway.
    pub(crate) fn charge_allocation(
        &self,
        options: &HeapOptions,
        heap_bytes: u64,
        byte_len: usize,
    ) {
        // compute new assist debt from allocation pressure
        let gc_pacer = self.snapshot(options, heap_bytes);
        let debt_bytes = gc_pacer.allocation_debt_bytes(options.gc, byte_len);

        self.assist_debt_bytes
            .fetch_add(gc_work_usize(debt_bytes), Ordering::AcqRel);
    }

    /// Consume pending collector assist debt as bytes.
    pub(crate) fn take_assist_budget_bytes(&self, budget_bytes: usize) -> usize {
        let mut consumed = 0usize;

        // claim one bounded slice of outstanding allocation debt
        let _ =
            self.assist_debt_bytes
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |pending| {
                    consumed = pending.min(budget_bytes);

                    Some(pending - consumed)
                });

        self.consume_work(consumed);

        consumed
    }

    /// Return and consume one shared collection work budget.
    pub(crate) fn take_collection_budget_bytes(
        &self,
        options: &HeapOptions,
        heap_bytes: u64,
        worker_count: usize,
    ) -> usize {
        // combine scheduled work with caller-paid assist work
        let gc_pacer = self.snapshot(options, heap_bytes);
        let base_bytes = gc_pacer.base_budget_bytes(options.gc, worker_count);
        let assist_bytes = self.take_assist_budget_bytes(base_bytes);
        let budget_bytes = base_bytes + assist_bytes;

        // base work is issued even if no assist debt exists
        self.consume_work(base_bytes);

        budget_bytes
    }

    /// Return one shared collector budget without consuming allocation debt.
    pub(crate) fn base_budget_bytes(
        &self,
        options: &HeapOptions,
        heap_bytes: u64,
        worker_count: usize,
    ) -> usize {
        // issue one scheduler-driven collector budget
        let gc_pacer = self.snapshot(options, heap_bytes);
        let budget_bytes = gc_pacer.base_budget_bytes(options.gc, worker_count);

        self.consume_work(budget_bytes);

        budget_bytes
    }

    /// Consume issued collector work from the remaining cycle estimate.
    fn consume_work(&self, budget_bytes: usize) {
        // clamp overspent work at zero
        let _ = self.remaining_work_bytes.fetch_update(
            Ordering::AcqRel,
            Ordering::Acquire,
            |pending| {
                if budget_bytes >= pending {
                    Some(0)
                } else {
                    Some(pending - budget_bytes)
                }
            },
        );
    }
}

/// Convert collector work bytes to platform usize.
fn gc_work_usize(bytes: u64) -> usize {
    bytes.min(usize::MAX as u64) as usize
}
