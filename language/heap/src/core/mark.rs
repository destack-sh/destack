use std::collections::VecDeque;
use std::marker::PhantomData;

use crate::{ManagedReference, SharedManagedReference};

/// The number of reference ids covered by one mark chunk.
const MARK_CHUNK_BITS: usize = 1024;
/// The number of bits inside one mark word.
const MARK_WORD_BITS: usize = u64::BITS as usize;
/// The number of mark words inside one mark chunk.
const MARK_CHUNK_WORDS: usize = MARK_CHUNK_BITS / MARK_WORD_BITS;
/// The first usable mark cycle.
const FIRST_MARK_CYCLE: u32 = 1;
/// The cleared mark cycle sentinel.
const EMPTY_MARK_CYCLE: u32 = 0;

/// One reference type with a stable allocation id.
pub(crate) trait ReferenceId: Copy {
    /// Return the stable allocation id.
    fn id(self) -> u32;
}

impl ReferenceId for ManagedReference {
    fn id(self) -> u32 {
        ManagedReference::id(&self)
    }
}

impl ReferenceId for SharedManagedReference {
    fn id(self) -> u32 {
        SharedManagedReference::id(&self)
    }
}

/// Collector-owned mark set keyed by stable reference id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MarkSet<R> {
    /// The active logical mark cycle.
    cycle: u32,
    /// The cycle currently represented by each chunk.
    chunk_cycles: Vec<u32>,
    /// The packed mark bits for each chunk.
    chunk_words: Vec<[u64; MARK_CHUNK_WORDS]>,
    /// The reference type carried by this set.
    reference: PhantomData<fn(R)>,
}

impl<R> Default for MarkSet<R> {
    fn default() -> Self {
        Self {
            cycle: FIRST_MARK_CYCLE,
            chunk_cycles: Vec::new(),
            chunk_words: Vec::new(),
            reference: PhantomData,
        }
    }
}

impl<R: ReferenceId> MarkSet<R> {
    /// Start a new logical mark cycle.
    pub(crate) fn start_cycle(&mut self) {
        if let Some(cycle) = self.cycle.checked_add(1) {
            self.cycle = cycle;

            return;
        }

        for cycle in &mut self.chunk_cycles {
            *cycle = EMPTY_MARK_CYCLE;
        }

        for words in &mut self.chunk_words {
            words.fill(0);
        }

        self.cycle = FIRST_MARK_CYCLE;
    }

    /// Return whether the reference has been marked in this cycle.
    pub(crate) fn contains(&self, reference: R) -> bool {
        let Some((chunk_index, word_index, bit_mask)) = self.resolve(reference) else {
            return false;
        };
        let Some(chunk_cycle) = self.chunk_cycles.get(chunk_index) else {
            return false;
        };
        let Some(chunk_words) = self.chunk_words.get(chunk_index) else {
            return false;
        };

        *chunk_cycle == self.cycle && chunk_words[word_index] & bit_mask != 0
    }

    /// Mark the reference and return whether this was the first mark in this cycle.
    pub(crate) fn mark(&mut self, reference: R) -> bool {
        let Some((chunk_index, word_index, bit_mask)) = self.resolve(reference) else {
            return false;
        };

        if chunk_index >= self.chunk_words.len() {
            let next_len = chunk_index.saturating_add(1);
            self.chunk_cycles.resize(next_len, EMPTY_MARK_CYCLE);
            self.chunk_words.resize(next_len, [0; MARK_CHUNK_WORDS]);
        }

        if self.chunk_cycles[chunk_index] != self.cycle {
            self.chunk_cycles[chunk_index] = self.cycle;
            self.chunk_words[chunk_index].fill(0);
        }

        let was_marked = self.chunk_words[chunk_index][word_index] & bit_mask != 0;
        self.chunk_words[chunk_index][word_index] |= bit_mask;

        !was_marked
    }

    /// Resolve one reference into its chunk, word, and bit mask.
    fn resolve(&self, reference: R) -> Option<(usize, usize, u64)> {
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

/// Trace queue for reachable references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TraceQueue<R> {
    /// The pending references whose outgoing edges still need scanning.
    pending: VecDeque<R>,
}

impl<R> Default for TraceQueue<R> {
    fn default() -> Self {
        Self {
            pending: VecDeque::new(),
        }
    }
}

impl<R: ReferenceId> TraceQueue<R> {
    /// Push one pending reference.
    pub(crate) fn push(&mut self, reference: R) {
        if reference.id() != 0 {
            self.pending.push_back(reference);
        }
    }

    /// Extend the pending queue with more references.
    pub(crate) fn extend(&mut self, references: impl IntoIterator<Item = R>) {
        for reference in references {
            self.push(reference);
        }
    }

    /// Pop the next pending reference.
    pub(crate) fn pop(&mut self) -> Option<R> {
        self.pending.pop_front()
    }

    /// Return whether the queue is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Clear every pending reference.
    pub(crate) fn clear(&mut self) {
        self.pending.clear();
    }
}
