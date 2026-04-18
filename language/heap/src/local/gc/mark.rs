use std::collections::VecDeque;

use crate::value::ManagedReference;

/// The number of reference ids covered by one mark chunk.
const MARK_CHUNK_BITS: usize = 1024;
/// The number of bits inside one mark word.
const MARK_WORD_BITS: usize = u64::BITS as usize;
/// The number of mark words inside one mark chunk.
const MARK_CHUNK_WORDS: usize = MARK_CHUNK_BITS / MARK_WORD_BITS;

/// One logical mark epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MarkEpoch(u32);

impl MarkEpoch {
    /// Return the first usable mark epoch.
    const fn first() -> Self {
        Self(1)
    }

    /// Return the next mark epoch.
    fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// One lazily cleared mark chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MarkChunk {
    /// The epoch currently represented by this chunk.
    epoch: MarkEpoch,
    /// The packed mark bits for this chunk.
    words: Box<[u64; MARK_CHUNK_WORDS]>,
}

impl Default for MarkChunk {
    fn default() -> Self {
        Self {
            epoch: MarkEpoch(0),
            words: Box::new([0; MARK_CHUNK_WORDS]),
        }
    }
}

/// Collector-owned mark set keyed by stable managed-reference id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MarkSet {
    /// The active logical mark epoch.
    epoch: MarkEpoch,
    /// The lazily cleared mark chunks.
    chunks: Vec<MarkChunk>,
}

impl Default for MarkSet {
    fn default() -> Self {
        Self {
            epoch: MarkEpoch::first(),
            chunks: Vec::new(),
        }
    }
}

impl MarkSet {
    /// Start a new logical mark cycle.
    pub(crate) fn start_cycle(&mut self) {
        if let Some(epoch) = self.epoch.next() {
            self.epoch = epoch;

            return;
        }

        for chunk in &mut self.chunks {
            chunk.epoch = MarkEpoch(0);
            chunk.words.fill(0);
        }

        self.epoch = MarkEpoch::first();
    }

    /// Return whether the reference has been marked in this cycle.
    pub(crate) fn contains(&self, reference: ManagedReference) -> bool {
        let Some((chunk_index, word_index, bit_mask)) = self.resolve(reference) else {
            return false;
        };
        let Some(chunk) = self.chunks.get(chunk_index) else {
            return false;
        };

        chunk.epoch == self.epoch && chunk.words[word_index] & bit_mask != 0
    }

    /// Mark the reference and return whether this was the first mark in this cycle.
    pub(crate) fn mark(&mut self, reference: ManagedReference) -> bool {
        let Some((chunk_index, word_index, bit_mask)) = self.resolve(reference) else {
            return false;
        };

        if chunk_index >= self.chunks.len() {
            self.chunks
                .resize_with(chunk_index.saturating_add(1), MarkChunk::default);
        }

        let chunk = &mut self.chunks[chunk_index];

        if chunk.epoch != self.epoch {
            chunk.epoch = self.epoch;
            chunk.words.fill(0);
        }

        let was_marked = chunk.words[word_index] & bit_mask != 0;
        chunk.words[word_index] |= bit_mask;

        !was_marked
    }

    /// Resolve one reference into its chunk, word, and bit mask.
    fn resolve(&self, reference: ManagedReference) -> Option<(usize, usize, u64)> {
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

/// Trace queue for reachable managed references.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TraceQueue {
    /// The pending references whose outgoing edges still need scanning.
    pending: VecDeque<ManagedReference>,
}

impl TraceQueue {
    /// Push one pending reference.
    pub(crate) fn push(&mut self, reference: ManagedReference) {
        if !reference.is_null() {
            self.pending.push_back(reference);
        }
    }

    /// Pop the next pending reference.
    pub(crate) fn pop(&mut self) -> Option<ManagedReference> {
        self.pending.pop_front()
    }

    /// Clear every pending reference.
    pub(crate) fn clear(&mut self) {
        self.pending.clear();
    }
}
