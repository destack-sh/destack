use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::diagnostic::RuntimeResult;
use crate::world::{BranchId, Moment, MomentSequence};

use super::{
    Observation, ObservationChunk, ObservationEntry, ObservationQuery, ObservationSequence,
};

/// Default maximum observation entries per chunk.
const DEFAULT_MAX_ENTRIES_PER_CHUNK: usize = 1024;

/// One live observation-log state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ObservationState {
    /// Next observation sequence number.
    next_sequence: ObservationSequence,
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
            next_sequence: ObservationSequence::new(0),
            sealed: Vec::new(),
            active: ObservationChunk::new(),
            max_entries_per_chunk: DEFAULT_MAX_ENTRIES_PER_CHUNK,
        }
    }
}

impl ObservationState {
    /// Allocate one sequence number.
    fn allocate_sequence(&mut self) -> RuntimeResult<ObservationSequence> {
        let sequence = self.next_sequence;
        self.next_sequence = sequence.next()?;

        Ok(sequence)
    }

    /// Append matching entries up to one exact output limit.
    fn append_query(
        &self,
        output: &mut Vec<ObservationEntry>,
        query: &ObservationQuery,
        limit: usize,
    ) {
        for chunk in &self.sealed {
            chunk.append_query(output, query, limit);
            if output.len() == limit {
                return;
            }
        }

        if !self.active.is_empty() {
            self.active.append_query(output, query, limit);
        }
    }

    /// Append matching entries inside one moment range.
    fn append_records_between(
        &self,
        output: &mut Vec<ObservationEntry>,
        start: Moment,
        end: Moment,
    ) {
        for chunk in &self.sealed {
            chunk.append_records_between(output, start, end);
        }

        if !self.active.is_empty() {
            self.active.append_records_between(output, start, end);
        }
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
        let mut state = self.state.write();
        let sequence = state.allocate_sequence()?;
        let entry = ObservationEntry {
            sequence,
            moment,
            observation,
        };

        if state
            .active
            .should_rotate_for_entry(state.max_entries_per_chunk)
        {
            state.seal_active_chunk();
        }

        state.active.push(entry);

        Ok(sequence)
    }

    /// Return matching Observation entries up to one exact limit.
    pub(crate) fn query(&self, query: &ObservationQuery, limit: usize) -> Vec<ObservationEntry> {
        let state = self.state.read();
        let mut records = Vec::new();

        state.append_query(&mut records, query, limit);

        records
    }

    /// Return every observation entry within one moment range.
    pub(crate) fn records_between(&self, start: Moment, end: Moment) -> Vec<ObservationEntry> {
        let state = self.state.read();
        let mut records = Vec::new();

        state.append_records_between(&mut records, start, end);

        records
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
        *self.state.write() = ObservationState::default();
    }
}
