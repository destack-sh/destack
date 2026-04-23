use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, AtomicUsize, Ordering};

use parking_lot::{Mutex, RwLock};

use super::{SharedGcPhase, SharedSmallSpanWork, SharedSmallSpanWorkTable, SharedTraceWork};
/// The number of shared trace-queue shards.
const SHARED_TRACE_QUEUE_SHARDS: usize = 8;

/// One sharded shared trace queue.
#[derive(Debug)]
pub(crate) struct SharedTraceQueue {
    /// The per-shard pending work whose outgoing edges still need scanning.
    shards: Box<[Mutex<VecDeque<SharedTraceWork>>]>,
    /// The next shard to probe for pop work.
    next_pop_shard: AtomicUsize,
}

impl Default for SharedTraceQueue {
    fn default() -> Self {
        let shards = (0..SHARED_TRACE_QUEUE_SHARDS)
            .map(|_| Mutex::new(VecDeque::new()))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            shards,
            next_pop_shard: AtomicUsize::new(0),
        }
    }
}

impl SharedTraceQueue {
    /// Push one pending trace work item into its shard.
    pub(crate) fn push(&self, work: SharedTraceWork) {
        if let SharedTraceWork::Reference(reference) = work
            && reference.is_null()
        {
            return;
        }

        let shard_index = self.shard_index(work);
        let mut shard = self.shards[shard_index].lock();

        shard.push_back(work);
    }

    /// Pop one bounded batch of pending work.
    pub(crate) fn pop_batch(&self, batch_len: usize) -> Vec<SharedTraceWork> {
        let mut batch = Vec::with_capacity(batch_len);
        let start = self.next_pop_shard.fetch_add(1, Ordering::AcqRel);

        for shard_offset in 0..self.shards.len() {
            let shard_index = (start + shard_offset) % self.shards.len();
            let mut shard = self.shards[shard_index].lock();

            while batch.len() < batch_len {
                let Some(work) = shard.pop_front() else {
                    break;
                };

                batch.push(work);
            }

            if batch.len() == batch_len {
                break;
            }
        }

        batch
    }

    /// Return whether every shard is currently empty.
    pub(crate) fn is_empty(&self) -> bool {
        for shard in &*self.shards {
            if !shard.lock().is_empty() {
                return false;
            }
        }

        true
    }

    /// Clear every pending work item from every shard.
    pub(crate) fn clear(&self) {
        for shard in &*self.shards {
            shard.lock().clear();
        }
    }

    /// Return the shard index for one queued work item.
    fn shard_index(&self, work: SharedTraceWork) -> usize {
        match work {
            SharedTraceWork::SmallSpan(span_index) => span_index % self.shards.len(),
            SharedTraceWork::Reference(reference) => reference.bits() as usize % self.shards.len(),
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
    /// The per-span pending work state for shared small-span tracing.
    small_span_work: RwLock<SharedSmallSpanWorkTable>,
    /// The current shared collection phase.
    phase: AtomicU8,
    /// Whether shared mark publication is temporarily closed for termination.
    mark_closing: AtomicU8,
    /// The number of shared mark publications currently in flight.
    pub(crate) mark_publishers: AtomicUsize,
    /// The next reference index to sweep.
    pub(crate) sweep_cursor: AtomicUsize,
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

    /// Return one per-span pending work state, growing the table when needed.
    pub(crate) fn ensure_small_span_work(
        &self,
        span_index: usize,
        slot_count: usize,
    ) -> Arc<SharedSmallSpanWork> {
        {
            let work = self.small_span_work.read();

            if let Some(span_work) = work.get(span_index).cloned()
                && span_work.matches_slot_count(slot_count)
            {
                return span_work;
            }
        }

        let mut work = self.small_span_work.write();

        while work.len() <= span_index {
            work.push(Arc::new(SharedSmallSpanWork::new(slot_count)));
        }

        let span_work = Arc::new(SharedSmallSpanWork::new(slot_count));
        work[span_index] = span_work.clone();

        span_work
    }

    /// Return one existing per-span pending work state.
    pub(crate) fn small_span_work(&self, span_index: usize) -> Option<Arc<SharedSmallSpanWork>> {
        self.small_span_work.read().get(span_index).cloned()
    }

    /// Clear every per-span pending work state.
    pub(crate) fn clear_small_span_work(&self) {
        self.small_span_work.write().clear();
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
