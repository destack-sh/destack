use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering};

use crossbeam_deque::{Injector, Steal, Stealer, Worker};
use parking_lot::{Mutex, MutexGuard};

use crate::SharedHeapReference;

use crate::{DropCursor, DropReference, GcDrop, GcPhase, HeapError, HeapResult};

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
    is_mark_closing: AtomicBool,
    /// The number of shared mark publications currently in flight.
    pub(crate) mark_publishers: AtomicUsize,
    /// The number of mark items currently being traced.
    pub(crate) mark_inflight: AtomicUsize,
    /// The active mark epoch.
    mark_epoch: AtomicU64,

    /// The active post-mark reclamation state.
    reclaim: Mutex<ReclaimState>,
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

    /// Reset reclamation state for a new mark cycle.
    pub(crate) fn reset_reclaim(&self) {
        *self.reclaim.lock() = ReclaimState::default();
    }

    /// Start one post-mark heap pass from the beginning.
    pub(crate) fn start_reclaim(&self, small_span_limit: usize, large_limit: usize) {
        *self.reclaim.lock() = ReclaimState {
            cursor: ReclaimCursor {
                small_span_limit,
                large_limit,
                ..ReclaimCursor::default()
            },
            ..ReclaimState::default()
        };
    }

    /// Restart the active post-mark cursor from its captured table limits.
    pub(crate) fn restart_reclaim(&self) {
        let mut reclaim = self.reclaim.lock();
        let small_span_limit = reclaim.cursor.small_span_limit;
        let large_limit = reclaim.cursor.large_limit;
        reclaim.cursor = ReclaimCursor {
            small_span_limit,
            large_limit,
            ..ReclaimCursor::default()
        };
    }

    /// Return the current reclamation cursor.
    pub(crate) fn reclaim_cursor(&self) -> ReclaimCursor {
        self.reclaim.lock().cursor
    }

    /// Set the current reclamation cursor.
    pub(crate) fn set_reclaim_cursor(&self, cursor: ReclaimCursor) {
        self.reclaim.lock().cursor = cursor;
    }

    /// Claim the first value from one shared allocation with the advanced heap cursor.
    pub(crate) fn claim_drop(
        &self,
        reclaim_cursor: ReclaimCursor,
        mut drop_cursor: DropCursor,
        budget_bytes: usize,
    ) -> HeapResult<GcDrop> {
        let mut reclaim = self.reclaim.lock();

        // reject overlapping allocation cursors
        if reclaim.pending_drop.is_some() {
            return Err(HeapError::internal("shared Drop is already claimed"));
        }

        // publish the cursor only after its first claim succeeds
        let drop = drop_cursor.claim(budget_bytes)?;
        reclaim.cursor = reclaim_cursor;
        reclaim.pending_drop = Some(drop_cursor);

        Ok(drop)
    }

    /// Claim the next value from the pending shared allocation.
    pub(crate) fn continue_drop(&self, budget_bytes: usize) -> HeapResult<Option<GcDrop>> {
        let mut reclaim = self.reclaim.lock();
        let Some(cursor) = &mut reclaim.pending_drop else {
            return Ok(None);
        };

        // wait for the runtime to complete the active value
        if cursor.is_claimed() {
            return Ok(None);
        }

        cursor.claim(budget_bytes).map(Some)
    }

    /// Return whether one shared allocation still has values to drop.
    pub(crate) fn has_pending_drop(&self) -> bool {
        self.reclaim.lock().pending_drop.is_some()
    }

    /// Return whether one shared value is claimed for Drop.
    pub(crate) fn is_drop_claimed(&self) -> bool {
        self.reclaim
            .lock()
            .pending_drop
            .is_some_and(|cursor| cursor.is_claimed())
    }

    /// Complete the currently claimed shared value.
    pub(crate) fn complete_drop(&self, reference: DropReference) -> HeapResult<()> {
        let mut reclaim = self.reclaim.lock();
        let Some(cursor) = &mut reclaim.pending_drop else {
            return Err(HeapError::internal("shared Drop cursor is missing"));
        };
        cursor.complete(reference)?;

        Ok(())
    }

    /// Retire the shared allocation cursor whose values completed Drop.
    pub(crate) fn retire_completed_drop(&self) {
        let mut reclaim = self.reclaim.lock();
        let is_complete = reclaim
            .pending_drop
            .is_some_and(|cursor| cursor.is_complete() && !cursor.is_claimed());
        if is_complete {
            reclaim.pending_drop = None;
        }
    }

    /// Record blocks reclaimed by sweep.
    pub(crate) fn record_reclaimed(&self, allocation_count: usize, byte_count: u64) {
        let mut reclaim = self.reclaim.lock();

        reclaim.freed_allocations += allocation_count;
        reclaim.freed_bytes += byte_count;
    }

    /// Return the blocks reclaimed by the active cycle.
    pub(crate) fn reclaimed(&self) -> (usize, u64) {
        let reclaim = self.reclaim.lock();

        (reclaim.freed_allocations, reclaim.freed_bytes)
    }

    /// Return whether shared mark publication is currently closed.
    pub(crate) fn is_mark_closing(&self) -> bool {
        self.is_mark_closing.load(Ordering::Acquire)
    }

    /// Open shared mark publication.
    pub(crate) fn open_mark_publication(&self) {
        self.is_mark_closing.store(false, Ordering::Release);
    }

    /// Close shared mark publication for termination.
    pub(crate) fn close_mark_publication(&self) {
        self.is_mark_closing.store(true, Ordering::Release);
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

/// One registered shared mark worker handle.
#[derive(Debug)]
pub struct SharedMarkWorker {
    /// The worker registry key.
    index: usize,
    /// Work owned by this mark worker.
    local: Worker<MarkWork>,
    /// Reusable batch storage for incremental mark work.
    batch: Mutex<Vec<MarkWork>>,
}

/// Shared trace queues for global work and worker-local work.
#[derive(Debug, Default)]
pub(crate) struct MarkQueue {
    /// Work published without a worker context.
    global: Injector<MarkWork>,
    /// Stealing handles for registered mark workers.
    stealers: Mutex<Vec<Stealer<MarkWork>>>,
    /// Reusable batch storage for coordinator mark work.
    batch: Mutex<Vec<MarkWork>>,
}

impl MarkQueue {
    /// Register one mark worker.
    pub(crate) fn register_worker(&self) -> SharedMarkWorker {
        let local = Worker::new_fifo();
        let stealer = local.stealer();
        let mut stealers = self.stealers.lock();
        let worker_index = stealers.len();

        // publish the stealing handle for other workers
        stealers.push(stealer);

        SharedMarkWorker {
            index: worker_index,
            local,
            batch: Mutex::new(Vec::new()),
        }
    }

    /// Lock the reusable batch for one mark worker or the coordinator.
    pub(crate) fn batch<'a>(
        &'a self,
        worker: Option<&'a SharedMarkWorker>,
    ) -> MutexGuard<'a, Vec<MarkWork>> {
        match worker {
            Some(worker) => worker.batch.lock(),
            None => self.batch.lock(),
        }
    }

    /// Push one pending trace work item.
    pub(crate) fn push(&self, worker: Option<&SharedMarkWorker>, work: MarkWork) {
        // nullish large references are not trace work
        if let MarkWork::Large { reference, .. } = work
            && reference.is_nullish()
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
        worker: Option<&SharedMarkWorker>,
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
        let stealers = self.stealers.lock();
        for stealer in stealers.iter() {
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
        let stealers = self.stealers.lock();
        for stealer in stealers.iter() {
            self.drain_stealer(stealer);
        }
    }

    /// Pop work from one worker deque into one batch.
    fn pop_from_worker(
        &self,
        worker: &SharedMarkWorker,
        batch_len: usize,
        batch: &mut Vec<MarkWork>,
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
        worker: Option<&SharedMarkWorker>,
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
        local_worker: Option<&SharedMarkWorker>,
        batch_len: usize,
        batch: &mut Vec<MarkWork>,
    ) {
        // scan the stable worker registry without allocating a snapshot
        let stealers = self.stealers.lock();
        for (worker_index, stealer) in stealers.iter().enumerate() {
            if local_worker.is_some_and(|local_worker| local_worker.index == worker_index) {
                continue;
            }

            self.steal_from_worker(local_worker, stealer, batch_len, batch);

            if batch.len() == batch_len {
                break;
            }
        }
    }

    /// Steal work from one worker queue into one batch.
    fn steal_from_worker(
        &self,
        local_worker: Option<&SharedMarkWorker>,
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
}

/// One active shared mark publication guard.
#[derive(Debug)]
pub(crate) struct MarkPublication<'a> {
    /// The owning shared collection state.
    state: &'a CollectorState,
}

/// Active shared post-mark heap cursor.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ReclaimCursor {
    /// The next small span index to visit.
    pub(crate) small_span_index: usize,
    /// The next small slot index to visit inside the current span.
    pub(crate) small_slot_index: usize,
    /// The small span table length captured when reclamation started.
    pub(crate) small_span_limit: usize,
    /// The next large block index to visit.
    pub(crate) large_index: usize,
    /// The large block table length captured when reclamation started.
    pub(crate) large_limit: usize,
}

/// Active shared post-mark reclamation state.
#[derive(Debug, Default)]
struct ReclaimState {
    /// The next heap block to visit.
    cursor: ReclaimCursor,
    /// Incremental Drop progress for one shared allocation.
    pending_drop: Option<DropCursor>,
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
