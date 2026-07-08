use serde::{Deserialize, Serialize};

use crate::world::{BranchId, Moment, MomentSequence};

use super::{ObservationEntry, ObservationSequence};

/// Observation chunk payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObservationChunk {
    /// Observation entries stored in this chunk.
    entries: Vec<ObservationEntry>,
}

impl ObservationChunk {
    /// Create one empty observation chunk.
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Create one chunk from existing entries.
    pub(crate) fn from_entries(entries: Vec<ObservationEntry>) -> Self {
        Self { entries }
    }

    /// Return whether this chunk stores no entries.
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return the stored observation entries.
    pub(crate) fn entries(&self) -> &[ObservationEntry] {
        &self.entries
    }

    /// Append entries after one optional sequence.
    pub(crate) fn append_records_after(
        &self,
        output: &mut Vec<ObservationEntry>,
        after: Option<ObservationSequence>,
    ) {
        for entry in &self.entries {
            if entry.is_after(after) {
                output.push(entry.clone());
            }
        }
    }

    /// Append entries inside one moment range.
    pub(crate) fn append_records_between(
        &self,
        output: &mut Vec<ObservationEntry>,
        start: Moment,
        end: Moment,
    ) {
        for entry in &self.entries {
            if entry.is_between(start, end) {
                output.push(entry.clone());
            }
        }
    }

    /// Append one observation entry.
    pub(crate) fn push(&mut self, entry: ObservationEntry) {
        self.entries.push(entry);
    }

    /// Return whether this chunk should rotate before appending one entry.
    pub(crate) fn should_rotate_for_entry(&self, max_entries_per_chunk: usize) -> bool {
        self.entries.len() >= max_entries_per_chunk
    }

    /// Split this chunk at one committed moment frontier.
    pub(crate) fn split_through(
        mut self,
        branch_id: BranchId,
        sequence: MomentSequence,
    ) -> (Option<Self>, Option<Self>) {
        let split_index = self.entries.partition_point(|entry| {
            entry.moment.branch_id == branch_id && entry.moment.sequence.get() <= sequence.get()
        });

        // keep the whole chunk when nothing is committed yet
        if split_index == 0 {
            (None, Some(self))
        }
        // drain the whole chunk when every entry is committed
        else if split_index == self.entries.len() {
            (Some(self), None)
        }
        // split mixed chunks at the committed frontier
        else {
            let retained = Self {
                entries: self.entries.split_off(split_index),
            };

            (Some(self), Some(retained))
        }
    }
}
