use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};

use crossbeam_deque::{Injector, Steal, Stealer, Worker};
use parking_lot::{Mutex, MutexGuard};

use crate::SharedHeapReference;

use super::SharedGcPhase;

/// One active shared heap collection state.
#[derive(Debug, Default)]
pub(crate) struct SharedGcState {
    /// The shared collector lifecycle gate.
    lifecycle: Mutex<()>,
    /// The reusable collector trace queue.
    pub(crate) trace_queue: SharedTraceQueue,
    /// The current shared collection phase.
    phase: AtomicU8,
    /// Whether shared mark publication is closed for termination.
    mark_closing: AtomicBool,
    /// The number of shared mark publications currently in flight.
    pub(crate) mark_publishers: AtomicUsize,
    /// The number of mark items currently being traced.
    pub(crate) mark_inflight: AtomicUsize,
    /// The active sweep state.
    sweep: Mutex<SharedSweepState>,
}

impl SharedGcState {
    /// Lock the shared collector lifecycle.
    pub(crate) fn lock_lifecycle(&self) -> MutexGuard<'_, ()> {
        self.lifecycle.lock()
    }

    /// Return the current shared collection phase.
    #[inline(always)]
    pub(crate) fn phase(&self) -> SharedGcPhase {
        SharedGcPhase::from_bits(self.phase.load(Ordering::Acquire))
    }

    /// Set the current shared collection phase.
    #[inline(always)]
    pub(crate) fn set_phase(&self, phase: SharedGcPhase) {
        self.phase.store(phase.bits(), Ordering::Release);
    }

    /// Reset sweep state for a new mark cycle.
    pub(crate) fn reset_sweep(&self) {
        *self.sweep.lock() = SharedSweepState::default();
    }

    /// Start sweeping one stable reference snapshot.
    pub(crate) fn start_sweep(&self, references: Vec<SharedHeapReference>) {
        *self.sweep.lock() = SharedSweepState {
            references: Arc::from(references),
            ..SharedSweepState::default()
        };
    }

    /// Return the stable sweep reference snapshot.
    pub(crate) fn sweep_references(&self) -> Arc<[SharedHeapReference]> {
        self.sweep.lock().references.clone()
    }

    /// Return the next reference index to sweep.
    pub(crate) fn sweep_cursor(&self) -> usize {
        self.sweep.lock().cursor
    }

    /// Set the next reference index to sweep.
    pub(crate) fn set_sweep_cursor(&self, cursor: usize) {
        self.sweep.lock().cursor = cursor;
    }

    /// Record allocation bytes freed by sweep.
    pub(crate) fn record_sweep_freed(&self, freed_allocations: usize, freed_bytes: u64) {
        let mut sweep = self.sweep.lock();

        sweep.freed_allocations += freed_allocations;
        sweep.freed_bytes += freed_bytes;
    }

    /// Return the completed sweep free counts.
    pub(crate) fn sweep_freed(&self) -> (usize, u64) {
        let sweep = self.sweep.lock();

        (sweep.freed_allocations, sweep.freed_bytes)
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
    pub(crate) fn begin_mark_publication(&self) -> Option<SharedMarkPublication<'_>> {
        // reject inactive or terminating mark cycles
        if self.phase() != SharedGcPhase::Mark || self.is_mark_closing() {
            return None;
        }

        self.mark_publishers.fetch_add(1, Ordering::AcqRel);

        // close the race with mark termination
        if self.phase() != SharedGcPhase::Mark || self.is_mark_closing() {
            self.mark_publishers.fetch_sub(1, Ordering::AcqRel);

            return None;
        }

        Some(SharedMarkPublication { state: self })
    }
}

/// One queued unit of shared mark work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SharedTraceWork {
    /// One shared small span with marked slots to scan.
    SmallSpan(usize),
    /// One range of one shared large allocation to scan.
    Large {
        /// The shared allocation reference.
        reference: SharedHeapReference,
        /// The range start in bytes.
        start: usize,
    },
}

/// One registered shared GC worker handle.
#[derive(Debug)]
pub struct SharedGcWorker {
    /// The worker registry key.
    index: usize,
    /// Work owned by this collector worker.
    local: Worker<SharedTraceWork>,
}

/// Shared trace queues for global work and worker-local work.
#[derive(Debug, Default)]
pub(crate) struct SharedTraceQueue {
    /// Work published without a worker context.
    global: Injector<SharedTraceWork>,
    /// Stealing handles for registered collector workers.
    stealers: Mutex<Vec<Stealer<SharedTraceWork>>>,
}

impl SharedTraceQueue {
    /// Register one GC worker.
    pub(crate) fn register_worker(&self) -> SharedGcWorker {
        let local = Worker::new_fifo();
        let stealer = local.stealer();
        let mut stealers = self.stealers.lock();
        let worker_index = stealers.len();

        // publish the stealing handle for other workers
        stealers.push(stealer);

        SharedGcWorker {
            index: worker_index,
            local,
        }
    }

    /// Push one pending trace work item.
    pub(crate) fn push(&self, worker: Option<&SharedGcWorker>, work: SharedTraceWork) {
        // null large references are not trace work
        if let SharedTraceWork::Large { reference, .. } = work
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

    /// Pop one bounded batch of pending work.
    pub(crate) fn pop_batch(
        &self,
        worker: Option<&SharedGcWorker>,
        batch_len: usize,
    ) -> Vec<SharedTraceWork> {
        let mut batch = Vec::with_capacity(batch_len);

        // local queue
        if let Some(worker) = worker {
            self.pop_from_worker(worker, batch_len, &mut batch);
        }

        // global queue
        self.pop_from_global(worker, batch_len, &mut batch);

        // other workers
        if batch.len() < batch_len {
            self.steal_from_workers(worker, batch_len, &mut batch);
        }

        batch
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
    fn pop_from_worker(
        &self,
        worker: &SharedGcWorker,
        batch_len: usize,
        batch: &mut Vec<SharedTraceWork>,
    ) {
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
        worker: Option<&SharedGcWorker>,
        batch_len: usize,
        batch: &mut Vec<SharedTraceWork>,
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
    fn steal_from_global(&self, batch_len: usize, batch: &mut Vec<SharedTraceWork>) {
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
    fn drain_stealer(&self, stealer: &Stealer<SharedTraceWork>) {
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
        local_worker: Option<&SharedGcWorker>,
        batch_len: usize,
        batch: &mut Vec<SharedTraceWork>,
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
        local_worker: Option<&SharedGcWorker>,
        stealer: &Stealer<SharedTraceWork>,
        batch_len: usize,
        batch: &mut Vec<SharedTraceWork>,
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
    fn stealers_snapshot(&self) -> Vec<Stealer<SharedTraceWork>> {
        self.stealers.lock().clone()
    }
}

/// One active shared mark publication guard.
#[derive(Debug)]
pub(crate) struct SharedMarkPublication<'a> {
    /// The owning shared collection state.
    state: &'a SharedGcState,
}

/// Active shared sweep state.
#[derive(Debug, Default)]
struct SharedSweepState {
    /// The stable shared reference snapshot for the active sweep.
    references: Arc<[SharedHeapReference]>,
    /// The next reference index to sweep.
    cursor: usize,
    /// The allocations freed so far in the active cycle.
    freed_allocations: usize,
    /// The bytes freed so far in the active cycle.
    freed_bytes: u64,
}

impl Drop for SharedMarkPublication<'_> {
    fn drop(&mut self) {
        self.state.mark_publishers.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use super::{SharedTraceQueue, SharedTraceWork};
    use crate::SharedHeapReference;

    /// Pop worker-local work before global work.
    #[test]
    fn test_pop_worker_local_work_first() {
        let queue = SharedTraceQueue::default();
        let worker = queue.register_worker();
        queue.push(
            Some(&worker),
            SharedTraceWork::Large {
                reference: SharedHeapReference::new(11),
                start: 0,
            },
        );
        queue.push(
            None,
            SharedTraceWork::Large {
                reference: SharedHeapReference::new(13),
                start: 0,
            },
        );

        let batch = queue.pop_batch(Some(&worker), 2);

        assert_eq!(
            batch,
            vec![
                SharedTraceWork::Large {
                    reference: SharedHeapReference::new(11),
                    start: 0,
                },
                SharedTraceWork::Large {
                    reference: SharedHeapReference::new(13),
                    start: 0,
                },
            ]
        );
    }
}
