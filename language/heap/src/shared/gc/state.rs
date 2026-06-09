use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering};

use crossbeam_deque::{Injector, Steal, Stealer, Worker};
use parking_lot::{Mutex, MutexGuard};

use crate::SharedHeapReference;

use super::GcPhase;

/// One active shared GC state.
#[derive(Debug, Default)]
pub(crate) struct CollectorState {
    /// The shared collector lifecycle gate.
    lifecycle: Mutex<()>,
    /// The reusable collector trace queue.
    pub(crate) trace_queue: MarkQueue,
    /// The current shared collection phase.
    phase: AtomicU8,

    /// Whether shared mark publication is closed for termination.
    mark_closing: AtomicBool,
    /// The number of shared mark publications currently in flight.
    pub(crate) mark_publishers: AtomicUsize,
    /// The number of mark items currently being traced.
    pub(crate) mark_inflight: AtomicUsize,
    /// The active mark epoch.
    mark_epoch: AtomicU64,

    /// The active sweep state.
    sweep: Mutex<SweepState>,
}

impl CollectorState {
    /// Lock the shared collector lifecycle.
    pub(crate) fn lock_lifecycle(&self) -> MutexGuard<'_, ()> {
        self.lifecycle.lock()
    }

    /// Return the current shared collection phase.
    #[inline(always)]
    pub(crate) fn phase(&self) -> GcPhase {
        GcPhase::from_bits(self.phase.load(Ordering::Acquire))
    }

    /// Set the current shared collection phase.
    #[inline(always)]
    pub(crate) fn set_phase(&self, phase: GcPhase) {
        self.phase.store(phase.bits(), Ordering::Release);
    }

    /// Advance and return the active shared mark epoch.
    pub(crate) fn advance_mark_epoch(&self) -> u64 {
        self.mark_epoch.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Return the active shared mark epoch.
    pub(crate) fn mark_epoch(&self) -> u64 {
        self.mark_epoch.load(Ordering::Acquire)
    }

    /// Reset sweep state for a new mark cycle.
    pub(crate) fn reset_sweep(&self) {
        *self.sweep.lock() = SweepState::default();
    }

    /// Start sweeping from the beginning of the shared heap.
    pub(crate) fn start_sweep(&self, small_span_limit: usize, large_limit: usize) {
        *self.sweep.lock() = SweepState {
            cursor: SweepCursor {
                small_span_limit,
                large_limit,
                ..SweepCursor::default()
            },
            ..SweepState::default()
        };
    }

    /// Return the current sweep cursor.
    pub(crate) fn sweep_cursor(&self) -> SweepCursor {
        self.sweep.lock().cursor
    }

    /// Set the current sweep cursor.
    pub(crate) fn set_sweep_cursor(&self, cursor: SweepCursor) {
        self.sweep.lock().cursor = cursor;
    }

    /// Record block bytes freed by sweep.
    pub(crate) fn record_sweep_freed(&self, freed_allocations: usize, freed_bytes: u64) {
        let mut sweep = self.sweep.lock();

        sweep.freed_allocations += freed_allocations;
        sweep.freed_bytes += freed_bytes;
    }

    /// Return the completed sweep free counts.
    pub(crate) fn sweep_freed(&self) -> SweepFreed {
        let sweep = self.sweep.lock();

        SweepFreed {
            allocations: sweep.freed_allocations,
            bytes: sweep.freed_bytes,
        }
    }

    /// Return whether shared mark publication is currently closed.
    pub(crate) fn is_mark_closing(&self) -> bool {
        self.mark_closing.load(Ordering::Acquire)
    }

    /// Open shared mark publication.
    pub(crate) fn open_mark_publication(&self) {
        self.mark_closing.store(false, Ordering::Release);
    }

    /// Close shared mark publication for termination.
    pub(crate) fn close_mark_publication(&self) {
        self.mark_closing.store(true, Ordering::Release);
    }

    /// Return whether the active shared mark phase is fully drained.
    pub(crate) fn mark_drained(&self) -> bool {
        // read all termination counters
        let is_queue_empty = self.trace_queue.is_empty();
        let inflight = self.mark_inflight.load(Ordering::Acquire);
        let publishers = self.mark_publishers.load(Ordering::Acquire);

        is_queue_empty && inflight == 0 && publishers == 0
    }

    /// Begin one shared mark publication and return its lifetime guard.
    pub(crate) fn begin_mark_publication(&self) -> Option<MarkPublication<'_>> {
        // reject inactive or terminating mark cycles
        if self.phase() != GcPhase::Mark || self.is_mark_closing() {
            return None;
        }

        self.mark_publishers.fetch_add(1, Ordering::AcqRel);

        // close the race with mark termination
        if self.phase() != GcPhase::Mark || self.is_mark_closing() {
            self.mark_publishers.fetch_sub(1, Ordering::AcqRel);

            return None;
        }

        Some(MarkPublication { state: self })
    }
}

/// One queued unit of shared mark work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkWork {
    /// One shared small span with marked slots to scan.
    SmallSpan(usize),
    /// One range of one shared large block to scan.
    Large {
        /// The shared block reference.
        reference: SharedHeapReference,
        /// The range start in bytes.
        start: usize,
    },
}

/// One registered shared GC worker handle.
#[derive(Debug)]
pub struct GcWorker {
    /// The worker registry key.
    index: usize,
    /// Work owned by this collector worker.
    local: Worker<MarkWork>,
}

/// Shared trace queues for global work and worker-local work.
#[derive(Debug, Default)]
pub(crate) struct MarkQueue {
    /// Work published without a worker context.
    global: Injector<MarkWork>,
    /// Stealing handles for registered collector workers.
    stealers: Mutex<Vec<Stealer<MarkWork>>>,
}

impl MarkQueue {
    /// Register one GC worker.
    pub(crate) fn register_worker(&self) -> GcWorker {
        let local = Worker::new_fifo();
        let stealer = local.stealer();
        let mut stealers = self.stealers.lock();
        let worker_index = stealers.len();

        // publish the stealing handle for other workers
        stealers.push(stealer);

        GcWorker {
            index: worker_index,
            local,
        }
    }

    /// Push one pending trace work item.
    pub(crate) fn push(&self, worker: Option<&GcWorker>, work: MarkWork) {
        // null large references are not trace work
        if let MarkWork::Large { reference, .. } = work
            && reference.is_null()
        {
            return;
        }

        // worker-local work is preferred for cache locality
        if let Some(worker) = worker {
            worker.local.push(work);

            return;
        }

        self.global.push(work);
    }

    /// Pop one bounded batch of pending work into the caller buffer.
    pub(crate) fn pop_batch(
        &self,
        worker: Option<&GcWorker>,
        batch_len: usize,
        batch: &mut Vec<MarkWork>,
    ) {
        batch.clear();

        // local queue
        if let Some(worker) = worker {
            self.pop_from_worker(worker, batch_len, batch);
        }

        // global queue
        self.pop_from_global(worker, batch_len, batch);

        // other workers
        if batch.len() < batch_len {
            self.steal_from_workers(worker, batch_len, batch);
        }
    }

    /// Return whether every queue is currently empty.
    pub(crate) fn is_empty(&self) -> bool {
        // global queue
        if !self.global.is_empty() {
            return false;
        }

        // worker queues
        let stealers = self.stealers_snapshot();
        for stealer in stealers {
            if !stealer.is_empty() {
                return false;
            }
        }

        true
    }

    /// Clear every pending work item.
    pub(crate) fn clear(&self) {
        // global queue
        self.drain_global();

        // worker queues
        let stealers = self.stealers_snapshot();
        for stealer in stealers {
            self.drain_stealer(&stealer);
        }
    }

    /// Pop work from one worker deque into one batch.
    fn pop_from_worker(&self, worker: &GcWorker, batch_len: usize, batch: &mut Vec<MarkWork>) {
        // drain the local worker queue first
        while batch.len() < batch_len {
            let Some(work) = worker.local.pop() else {
                break;
            };

            batch.push(work);
        }
    }

    /// Pop global work into one batch.
    fn pop_from_global(
        &self,
        worker: Option<&GcWorker>,
        batch_len: usize,
        batch: &mut Vec<MarkWork>,
    ) {
        // no local worker means direct global steals
        let Some(worker) = worker else {
            self.steal_from_global(batch_len, batch);

            return;
        };

        // batch global work through the local deque
        while batch.len() < batch_len {
            let steal = self.global.steal_batch_and_pop(&worker.local);
            match steal {
                Steal::Success(work) => batch.push(work),
                Steal::Empty => return,
                Steal::Retry => continue,
            }
        }
    }

    /// Pop global work without a local worker.
    fn steal_from_global(&self, batch_len: usize, batch: &mut Vec<MarkWork>) {
        // direct global steals
        while batch.len() < batch_len {
            match self.global.steal() {
                Steal::Success(work) => batch.push(work),
                Steal::Empty => return,
                Steal::Retry => continue,
            }
        }
    }

    /// Drain global work.
    fn drain_global(&self) {
        // consume until empty
        loop {
            match self.global.steal() {
                Steal::Success(_) | Steal::Retry => continue,
                Steal::Empty => return,
            }
        }
    }

    /// Drain one worker through its public stealer.
    fn drain_stealer(&self, stealer: &Stealer<MarkWork>) {
        // consume until empty
        loop {
            match stealer.steal() {
                Steal::Success(_) | Steal::Retry => continue,
                Steal::Empty => return,
            }
        }
    }

    /// Steal work from other worker queues into one batch.
    fn steal_from_workers(
        &self,
        local_worker: Option<&GcWorker>,
        batch_len: usize,
        batch: &mut Vec<MarkWork>,
    ) {
        // snapshot avoids holding the registry lock while stealing
        let stealers = self.stealers_snapshot();
        for (worker_index, stealer) in stealers.into_iter().enumerate() {
            if local_worker.is_some_and(|local_worker| local_worker.index == worker_index) {
                continue;
            }

            self.steal_from_worker(local_worker, &stealer, batch_len, batch);

            if batch.len() == batch_len {
                break;
            }
        }
    }

    /// Steal work from one worker queue into one batch.
    fn steal_from_worker(
        &self,
        local_worker: Option<&GcWorker>,
        stealer: &Stealer<MarkWork>,
        batch_len: usize,
        batch: &mut Vec<MarkWork>,
    ) {
        // no local worker means direct steals
        let Some(local_worker) = local_worker else {
            while batch.len() < batch_len {
                match stealer.steal() {
                    Steal::Success(work) => batch.push(work),
                    Steal::Empty => return,
                    Steal::Retry => continue,
                }
            }

            return;
        };

        // batch stolen work through the local worker deque
        while batch.len() < batch_len {
            let steal = stealer.steal_batch_and_pop(&local_worker.local);

            match steal {
                Steal::Success(work) => batch.push(work),
                Steal::Empty => return,
                Steal::Retry => continue,
            }
        }
    }

    /// Return the currently registered worker stealers.
    fn stealers_snapshot(&self) -> Vec<Stealer<MarkWork>> {
        self.stealers.lock().clone()
    }
}

/// One active shared mark publication guard.
#[derive(Debug)]
pub(crate) struct MarkPublication<'a> {
    /// The owning shared collection state.
    state: &'a CollectorState,
}

/// Active shared sweep cursor.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct SweepCursor {
    /// The next small span index to sweep.
    pub(crate) small_span_index: usize,
    /// The next small slot index to sweep inside the current span.
    pub(crate) small_slot_index: usize,
    /// The small span table length captured when sweep started.
    pub(crate) small_span_limit: usize,
    /// The next large block index to sweep.
    pub(crate) large_index: usize,
    /// The large block table length captured when sweep started.
    pub(crate) large_limit: usize,
}

/// Block counts freed by one shared sweep cycle.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SweepFreed {
    /// The blocks freed by the sweep cycle.
    pub(crate) allocations: usize,
    /// The bytes freed by the sweep cycle.
    pub(crate) bytes: u64,
}

/// Active shared sweep state.
#[derive(Debug, Default)]
struct SweepState {
    /// The next block to sweep.
    cursor: SweepCursor,
    /// The blocks freed so far in the active cycle.
    freed_allocations: usize,
    /// The bytes freed so far in the active cycle.
    freed_bytes: u64,
}

impl Drop for MarkPublication<'_> {
    fn drop(&mut self) {
        self.state.mark_publishers.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use super::{MarkQueue, MarkWork};
    use crate::SharedHeapReference;

    /// Pop worker-local work before global work.
    #[test]
    fn test_pop_worker_local_work_first() {
        let queue = MarkQueue::default();
        let worker = queue.register_worker();
        queue.push(
            Some(&worker),
            MarkWork::Large {
                reference: SharedHeapReference::new(11),
                start: 0,
            },
        );
        queue.push(
            None,
            MarkWork::Large {
                reference: SharedHeapReference::new(13),
                start: 0,
            },
        );

        let mut batch = Vec::new();
        queue.pop_batch(Some(&worker), 2, &mut batch);

        assert_eq!(
            batch,
            vec![
                MarkWork::Large {
                    reference: SharedHeapReference::new(11),
                    start: 0,
                },
                MarkWork::Large {
                    reference: SharedHeapReference::new(13),
                    start: 0,
                },
            ]
        );
    }
}
