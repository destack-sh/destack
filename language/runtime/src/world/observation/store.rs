use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::{BranchId, Moment, MomentSequence};

use super::{
    Observation, ObservationChunk, ObservationEntry, ObservationOptions, ObservationSequence,
};

/// Default maximum observation entries per chunk.
const DEFAULT_MAX_ENTRIES_PER_CHUNK: usize = 1024;

/// One live observation-log state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ObservationState {
    /// Completed chunks in observation order.
    sealed: Vec<ObservationChunk>,
    /// Current mutable append chunk.
    active: ObservationChunk,
    /// Maximum entries stored in one chunk.
    max_entries_per_chunk: usize,
}

impl Default for ObservationState {
    fn default() -> Self {
        Self {
            sealed: Vec::new(),
            active: ObservationChunk::new(),
            max_entries_per_chunk: DEFAULT_MAX_ENTRIES_PER_CHUNK,
        }
    }
}

impl ObservationState {
    /// Return the total number of visible chunks.
    fn chunk_count(&self) -> usize {
        let active_chunk_count = usize::from(!self.active.is_empty());

        self.sealed.len() + active_chunk_count
    }

    /// Return the visible chunks in observation order.
    fn chunks(&self) -> Vec<ObservationChunk> {
        let mut chunks = Vec::with_capacity(self.chunk_count());
        chunks.extend(self.sealed.iter().cloned());

        if !self.active.is_empty() {
            chunks.push(self.active.clone());
        }

        chunks
    }

    /// Finalize one non-empty active chunk into the completed chunk list.
    fn seal_active_chunk(&mut self) {
        if self.active.is_empty() {
            return;
        }

        let sealed_chunk = std::mem::replace(&mut self.active, ObservationChunk::new());
        self.sealed.push(sealed_chunk);
    }
}

/// Backing store for live observation chunks.
#[derive(Debug, Default)]
pub(crate) struct ObservationStore {
    /// Next observation sequence number.
    next_sequence: AtomicU64,
    /// Mutable observation store state.
    state: RwLock<ObservationState>,
}

impl ObservationStore {
    /// Create one empty observation store.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Record one observation at one exact execution coordinate.
    pub(crate) fn record_at(
        &self,
        moment: Moment,
        observation: Observation,
    ) -> RuntimeResult<ObservationSequence> {
        let sequence = self.next_sequence()?;
        let entry = ObservationEntry {
            sequence,
            moment,
            observation,
        };
        let mut state = self.state.write();

        if state
            .active
            .should_rotate_for_entry(state.max_entries_per_chunk)
        {
            state.seal_active_chunk();
        }

        state.active.push(entry);

        Ok(sequence)
    }

    /// Return every filtered observation entry after the optional sequence.
    pub(crate) fn records_after(
        &self,
        after: Option<ObservationSequence>,
        options: ObservationOptions,
    ) -> Vec<ObservationEntry> {
        let state = self.state.read();

        state
            .chunks()
            .into_iter()
            .flat_map(ObservationChunk::into_entries)
            .filter(|entry| entry.is_after(after))
            .filter(|entry| options.allows(entry.observation.category))
            .collect()
    }

    /// Return every observation entry within one moment range.
    pub(crate) fn records_between(&self, start: Moment, end: Moment) -> Vec<ObservationEntry> {
        let state = self.state.read();

        state
            .chunks()
            .into_iter()
            .flat_map(ObservationChunk::into_entries)
            .filter(|entry| entry.is_between(start, end))
            .collect()
    }

    /// Drain every observation chunk up to one exact committed sequence.
    pub(crate) fn drain_through(
        &self,
        branch_id: BranchId,
        sequence: MomentSequence,
    ) -> Vec<ObservationChunk> {
        let mut state = self.state.write();
        let mut drained = Vec::new();

        // split sealed chunks before the mutable frontier
        let mut retained_sealed = Vec::with_capacity(state.sealed.len());
        for chunk in std::mem::take(&mut state.sealed) {
            let (drained_chunk, retained_chunk) = chunk.split_through(branch_id, sequence);

            if let Some(drained_chunk) = drained_chunk {
                drained.push(drained_chunk);
            }

            if let Some(retained_chunk) = retained_chunk {
                retained_sealed.push(retained_chunk);
            }
        }
        state.sealed = retained_sealed;

        // split the active chunk at the committed frontier
        let active = std::mem::replace(&mut state.active, ObservationChunk::new());
        let (drained_chunk, retained_chunk) = active.split_through(branch_id, sequence);

        if let Some(drained_chunk) = drained_chunk {
            drained.push(drained_chunk);
        }

        state.active = retained_chunk.unwrap_or_else(ObservationChunk::new);

        drained
    }

    /// Reset the observation store.
    pub(crate) fn reset(&self) {
        self.next_sequence.store(0, Ordering::SeqCst);
        *self.state.write() = ObservationState::default();
    }

    /// Allocate one sequence number.
    fn next_sequence(&self) -> RuntimeResult<ObservationSequence> {
        self.next_sequence
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
                value.checked_add(1)
            })
            .map(ObservationSequence::new)
            .map_err(|_| {
                RuntimeError::Internal {
                    message: "observation sequence space exhausted".to_string(),
                }
                .boxed()
            })
    }
}
