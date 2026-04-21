use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use parking_lot::{Mutex, RwLock};

use super::SharedGcPhase;
use crate::SharedManagedReference;

/// The number of shared trace-queue shards.
const SHARED_TRACE_QUEUE_SHARDS: usize = 8;
/// The number of reference ids covered by one mark chunk.
const MARK_CHUNK_BITS: usize = 1024;
/// The number of bits inside one mark word.
const MARK_WORD_BITS: usize = u64::BITS as usize;
/// The number of mark words inside one mark chunk.
const MARK_CHUNK_WORDS: usize = MARK_CHUNK_BITS / MARK_WORD_BITS;
/// The first usable shared mark cycle.
const FIRST_SHARED_MARK_CYCLE: u32 = 1;
/// The cleared shared mark cycle sentinel.
const EMPTY_SHARED_MARK_CYCLE: u32 = 0;

/// One atomic shared mark chunk.
#[derive(Debug)]
struct SharedMarkChunk {
    /// The cycle currently represented by this chunk.
    cycle: AtomicU32,
    /// The packed mark bits for this chunk.
    words: Box<[AtomicU64]>,
    /// The reset gate for one cycle rollover.
    reset: Mutex<()>,
}

impl SharedMarkChunk {
    /// Create one empty atomic shared mark chunk.
    fn new() -> Self {
        let words = std::iter::repeat_with(|| AtomicU64::new(0))
            .take(MARK_CHUNK_WORDS)
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            cycle: AtomicU32::new(EMPTY_SHARED_MARK_CYCLE),
            words,
            reset: Mutex::new(()),
        }
    }

    /// Reset this chunk for one new logical cycle.
    fn reset_for_cycle(&self, cycle: u32) {
        let _reset = self.reset.lock();
        let chunk_cycle = self.cycle.load(Ordering::Acquire);

        if chunk_cycle == cycle {
            return;
        }

        for word in &*self.words {
            word.store(0, Ordering::Release);
        }

        self.cycle.store(cycle, Ordering::Release);
    }
}

/// One atomic shared mark set keyed by stable reference id.
#[derive(Debug)]
pub(crate) struct SharedMarkSet {
    /// The active logical mark cycle.
    cycle: AtomicU32,
    /// The shared mark chunks.
    chunks: RwLock<Vec<Arc<SharedMarkChunk>>>,
}

impl Default for SharedMarkSet {
    fn default() -> Self {
        Self {
            cycle: AtomicU32::new(FIRST_SHARED_MARK_CYCLE),
            chunks: RwLock::new(Vec::new()),
        }
    }
}

impl SharedMarkSet {
    /// Start one new logical mark cycle.
    pub(crate) fn start_cycle(&self) {
        let next_cycle = self.cycle.fetch_add(1, Ordering::AcqRel).wrapping_add(1);

        if next_cycle != EMPTY_SHARED_MARK_CYCLE {
            return;
        }

        let chunks = self.chunks.write();

        for chunk in &*chunks {
            for word in &*chunk.words {
                word.store(0, Ordering::Release);
            }

            chunk
                .cycle
                .store(EMPTY_SHARED_MARK_CYCLE, Ordering::Release);
        }

        self.cycle.store(FIRST_SHARED_MARK_CYCLE, Ordering::Release);
    }

    /// Return whether one reference has been marked in the current cycle.
    pub(crate) fn contains(&self, reference: SharedManagedReference) -> bool {
        let Some((chunk_index, word_index, bit_mask)) = Self::resolve(reference) else {
            return false;
        };
        let Some(chunk) = self.chunk(chunk_index) else {
            return false;
        };
        let cycle = self.cycle.load(Ordering::Acquire);
        let chunk_cycle = chunk.cycle.load(Ordering::Acquire);

        if chunk_cycle != cycle {
            return false;
        }

        chunk.words[word_index].load(Ordering::Acquire) & bit_mask != 0
    }

    /// Mark one reference and return whether this was the first mark in the current cycle.
    pub(crate) fn mark(&self, reference: SharedManagedReference) -> bool {
        let Some((chunk_index, word_index, bit_mask)) = Self::resolve(reference) else {
            return false;
        };
        let chunk = self.ensure_chunk(chunk_index);
        let cycle = self.cycle.load(Ordering::Acquire);
        let chunk_cycle = chunk.cycle.load(Ordering::Acquire);

        if chunk_cycle != cycle {
            chunk.reset_for_cycle(cycle);
        }

        let previous = chunk.words[word_index].fetch_or(bit_mask, Ordering::AcqRel);

        previous & bit_mask == 0
    }

    /// Return one mark chunk by index when it already exists.
    fn chunk(&self, chunk_index: usize) -> Option<Arc<SharedMarkChunk>> {
        let chunks = self.chunks.read();

        chunks.get(chunk_index).cloned()
    }

    /// Return one mark chunk by index, growing the table when needed.
    fn ensure_chunk(&self, chunk_index: usize) -> Arc<SharedMarkChunk> {
        if let Some(chunk) = self.chunk(chunk_index) {
            return chunk;
        }

        let mut chunks = self.chunks.write();

        while chunks.len() <= chunk_index {
            chunks.push(Arc::new(SharedMarkChunk::new()));
        }

        chunks[chunk_index].clone()
    }

    /// Resolve one shared managed reference into its chunk, word, and bit mask.
    fn resolve(reference: SharedManagedReference) -> Option<(usize, usize, u64)> {
        let reference_id = reference.id();

        if reference_id == 0 {
            return None;
        }

        let offset = reference_id.checked_sub(1)? as usize;
        let chunk_index = offset / MARK_CHUNK_BITS;
        let chunk_offset = offset % MARK_CHUNK_BITS;
        let word_index = chunk_offset / MARK_WORD_BITS;
        let bit_offset = chunk_offset % MARK_WORD_BITS;

        Some((chunk_index, word_index, 1_u64 << bit_offset))
    }
}

/// One sharded shared trace queue.
#[derive(Debug)]
pub(crate) struct SharedTraceQueue {
    /// The per-shard pending references whose outgoing edges still need scanning.
    shards: Box<[Mutex<VecDeque<SharedManagedReference>>]>,
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
    /// Push one pending reference into its shard.
    pub(crate) fn push(&self, reference: SharedManagedReference) {
        if reference.id() == 0 {
            return;
        }

        let shard_index = self.shard_index(reference);
        let mut shard = self.shards[shard_index].lock();

        shard.push_back(reference);
    }

    /// Pop one bounded batch of pending references.
    pub(crate) fn pop_batch(&self, batch_len: usize) -> Vec<SharedManagedReference> {
        let mut batch = Vec::with_capacity(batch_len);
        let start = self.next_pop_shard.fetch_add(1, Ordering::AcqRel);

        for shard_offset in 0..self.shards.len() {
            let shard_index = (start + shard_offset) % self.shards.len();
            let mut shard = self.shards[shard_index].lock();

            while batch.len() < batch_len {
                let Some(reference) = shard.pop_front() else {
                    break;
                };

                batch.push(reference);
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

    /// Clear every pending reference from every shard.
    pub(crate) fn clear(&self) {
        for shard in &*self.shards {
            shard.lock().clear();
        }
    }

    /// Return the shard index for one shared managed reference.
    fn shard_index(&self, reference: SharedManagedReference) -> usize {
        reference.id() as usize % self.shards.len()
    }
}

/// One active shared managed collection state.
#[derive(Debug, Default)]
pub(crate) struct SharedGcState {
    /// The shared collector lifecycle gate.
    lifecycle: Mutex<()>,
    /// The reusable collector mark set.
    pub(crate) marks: SharedMarkSet,
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

    /// Join one active shared mark publication.
    pub(crate) fn begin_mark_publication(&self) -> Option<SharedMarkPublication<'_>> {
        loop {
            if self.phase() != SharedGcPhase::Mark {
                return None;
            }

            if self.is_mark_closing() {
                std::hint::spin_loop();
                continue;
            }

            self.mark_publishers.fetch_add(1, Ordering::AcqRel);

            if self.phase() == SharedGcPhase::Mark && !self.is_mark_closing() {
                return Some(SharedMarkPublication { gc: self });
            }

            self.mark_publishers.fetch_sub(1, Ordering::AcqRel);

            if self.phase() != SharedGcPhase::Mark {
                return None;
            }

            std::hint::spin_loop();
        }
    }
}

/// One active shared mark publication guard.
#[derive(Debug)]
pub(crate) struct SharedMarkPublication<'a> {
    /// The shared collector state for this publication.
    gc: &'a SharedGcState,
}

impl Drop for SharedMarkPublication<'_> {
    fn drop(&mut self) {
        self.gc.mark_publishers.fetch_sub(1, Ordering::AcqRel);
    }
}
