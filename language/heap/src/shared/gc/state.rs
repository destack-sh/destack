use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, AtomicUsize, Ordering};

use crossbeam_deque::{Injector, Steal, Stealer, Worker};
use parking_lot::{Mutex, RwLock};

use crate::SharedHeapReference;

use super::SharedGcPhase;

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
#[derive(Debug, Clone)]
pub struct SharedGcWorker {
    /// The worker registry key.
    index: usize,
    /// Work owned by this collector worker.
    local: Arc<Mutex<Worker<SharedTraceWork>>>,
    /// Public stealing handle for this worker.
    stealer: Stealer<SharedTraceWork>,
}

/// Shared trace queues for global work and worker-local work.
#[derive(Debug, Default)]
pub(crate) struct SharedTraceQueue {
    /// Work published without a worker context.
    global: Injector<SharedTraceWork>,
    /// Work owned by active collector workers.
    workers: RwLock<BTreeMap<usize, SharedGcWorker>>,
}

impl SharedTraceQueue {
    /// Return the GC worker registered for one runtime worker.
    pub(crate) fn worker(&self, worker_index: usize) -> SharedGcWorker {
        {
            let workers = self.workers.read();
            if let Some(worker) = workers.get(&worker_index) {
                return worker.clone();
            }
        }

        let local = Worker::new_fifo();
        let stealer = local.stealer();
        let worker = SharedGcWorker {
            index: worker_index,
            local: Arc::new(Mutex::new(local)),
            stealer,
        };

        self.workers
            .write()
            .entry(worker_index)
            .or_insert_with(|| worker.clone())
            .clone()
    }

    /// Push one pending trace work item.
    pub(crate) fn push(&self, worker: Option<&SharedGcWorker>, work: SharedTraceWork) {
        if let SharedTraceWork::Large { reference, .. } = work
            && reference.is_null()
        {
            return;
        }

        if let Some(worker) = worker {
            worker.local.lock().push(work);

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

        if let Some(worker) = worker {
            self.pop_from_worker(worker, batch_len, &mut batch);
        }

        self.pop_from_global(worker, batch_len, &mut batch);

        if batch.len() < batch_len {
            self.steal_from_workers(worker, batch_len, &mut batch);
        }

        batch
    }

    /// Return whether every queue is currently empty.
    pub(crate) fn is_empty(&self) -> bool {
        if !self.global.is_empty() {
            return false;
        }

        for worker in self.workers.read().values() {
            if !worker.stealer.is_empty() {
                return false;
            }
        }

        true
    }

    /// Clear every pending work item.
    pub(crate) fn clear(&self) {
        self.drain_global();

        for worker in self.workers.read().values() {
            self.drain_stealer(&worker.stealer);
        }
    }

    /// Pop work from one worker deque into one batch.
    fn pop_from_worker(
        &self,
        worker: &SharedGcWorker,
        batch_len: usize,
        batch: &mut Vec<SharedTraceWork>,
    ) {
        let local = worker.local.lock();

        while batch.len() < batch_len {
            let Some(work) = local.pop() else {
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
        let Some(worker) = worker else {
            self.steal_from_global(batch_len, batch);

            return;
        };

        while batch.len() < batch_len {
            let local = worker.local.lock();
            let steal = self.global.steal_batch_and_pop(&local);
            drop(local);

            match steal {
                Steal::Success(work) => batch.push(work),
                Steal::Empty => return,
                Steal::Retry => continue,
            }
        }
    }

    /// Pop global work without a local worker.
    fn steal_from_global(&self, batch_len: usize, batch: &mut Vec<SharedTraceWork>) {
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
        loop {
            match self.global.steal() {
                Steal::Success(_) | Steal::Retry => continue,
                Steal::Empty => return,
            }
        }
    }

    /// Drain one worker through its public stealer.
    fn drain_stealer(&self, stealer: &Stealer<SharedTraceWork>) {
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
        let workers = self.workers.read();
        for (worker_index, worker) in workers.iter() {
            if local_worker.is_some_and(|worker| worker.index == *worker_index) {
                continue;
            }

            self.steal_from_worker(local_worker, &worker.stealer, batch_len, batch);

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

        while batch.len() < batch_len {
            let local = local_worker.local.lock();
            let steal = stealer.steal_batch_and_pop(&local);
            drop(local);

            match steal {
                Steal::Success(work) => batch.push(work),
                Steal::Empty => return,
                Steal::Retry => continue,
            }
        }
    }
}

/// One active shared heap collection state.
#[derive(Debug, Default)]
pub(crate) struct SharedGcState {
    /// The shared collector lifecycle gate.
    lifecycle: Mutex<()>,
    /// The reusable collector trace queue.
    pub(crate) trace_queue: SharedTraceQueue,
    /// The current shared collection phase.
    phase: AtomicU8,
    /// Whether shared mark publication is temporarily closed for termination.
    mark_closing: AtomicU8,
    /// The number of shared mark publications currently in flight.
    pub(crate) mark_publishers: AtomicUsize,
    /// The next reference index to sweep.
    pub(crate) sweep_cursor: AtomicUsize,
    /// The shared reference snapshot for the active sweep.
    pub(crate) sweep_references: Mutex<Arc<[crate::SharedHeapReference]>>,
    /// The number of mark items currently being traced.
    pub(crate) mark_inflight: AtomicUsize,
    /// The allocations freed so far in the active cycle.
    pub(crate) freed_allocations: AtomicUsize,
    /// The bytes freed so far in the active cycle.
    pub(crate) freed_bytes: AtomicU64,
}

impl SharedGcState {
    /// Lock the shared collector lifecycle.
    pub(crate) fn lock_lifecycle(&self) -> parking_lot::MutexGuard<'_, ()> {
        self.lifecycle.lock()
    }

    /// Return the current shared collection phase.
    pub(crate) fn phase(&self) -> SharedGcPhase {
        SharedGcPhase::from_bits(self.phase.load(Ordering::Acquire))
    }

    /// Set the current shared collection phase.
    pub(crate) fn set_phase(&self, phase: SharedGcPhase) {
        self.phase.store(phase.bits(), Ordering::Release);
    }

    /// Return whether shared mark publication is currently closed.
    pub(crate) fn is_mark_closing(&self) -> bool {
        self.mark_closing.load(Ordering::Acquire) != 0
    }

    /// Open shared mark publication.
    pub(crate) fn open_mark_publication(&self) {
        self.mark_closing.store(0, Ordering::Release);
    }

    /// Close shared mark publication for termination.
    pub(crate) fn close_mark_publication(&self) {
        self.mark_closing.store(1, Ordering::Release);
    }

    /// Return whether the active shared mark phase is fully drained.
    pub(crate) fn mark_drained(&self) -> bool {
        let is_queue_empty = self.trace_queue.is_empty();
        let inflight = self.mark_inflight.load(Ordering::Acquire);
        let publishers = self.mark_publishers.load(Ordering::Acquire);

        is_queue_empty && inflight == 0 && publishers == 0
    }

    /// Begin one shared mark publication and return its lifetime guard.
    pub(crate) fn begin_mark_publication(&self) -> Option<SharedMarkPublication<'_>> {
        if self.phase() != SharedGcPhase::Mark || self.is_mark_closing() {
            return None;
        }

        self.mark_publishers.fetch_add(1, Ordering::AcqRel);

        if self.phase() != SharedGcPhase::Mark || self.is_mark_closing() {
            self.mark_publishers.fetch_sub(1, Ordering::AcqRel);

            return None;
        }

        Some(SharedMarkPublication { state: self })
    }
}

/// One active shared mark publication guard.
#[derive(Debug)]
pub(crate) struct SharedMarkPublication<'a> {
    /// The owning shared collection state.
    state: &'a SharedGcState,
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

    /// Reuse worker-local queues for stable runtime worker ids.
    #[test]
    fn test_reuse_worker_queue_by_index() {
        let queue = SharedTraceQueue::default();
        let worker = queue.worker(7);
        queue.push(
            Some(&worker),
            SharedTraceWork::Large {
                reference: SharedHeapReference::new(11),
                start: 0,
            },
        );

        let worker = queue.worker(7);
        let batch = queue.pop_batch(Some(&worker), 1);

        assert_eq!(
            batch,
            vec![SharedTraceWork::Large {
                reference: SharedHeapReference::new(11),
                start: 0,
            }]
        );
    }
}
