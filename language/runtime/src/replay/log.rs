use std::sync::{Arc, Mutex};

use crate::replay::{BranchId, LogSequence, ReplayEvent};

/// In-memory replay log state.
#[derive(Debug, Clone)]
struct ReplayState {
    /// Branch identifier for this log stream.
    branch_id: BranchId,
    /// Next sequence number to assign.
    next_sequence: LogSequence,
    /// Recorded events for replay.
    events: Vec<ReplayEvent>,
    /// Next event index for replay reads.
    read_index: usize,
}

/// Record and replay log for deterministic execution.
#[derive(Debug, Clone)]
pub struct ReplayLog {
    /// Shared replay log state.
    state: Arc<Mutex<ReplayState>>,
}

impl Default for ReplayLog {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(ReplayState {
                branch_id: BranchId::new(0),
                next_sequence: LogSequence::new(0),
                events: Vec::new(),
                read_index: 0,
            })),
        }
    }
}

impl ReplayLog {
    /// Return the current branch identifier.
    pub fn branch_id(&self) -> BranchId {
        // lock state for reading
        let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.branch_id
    }

    /// Return the next sequence number.
    pub fn next_sequence(&self) -> LogSequence {
        // lock state for reading
        let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.next_sequence
    }

    /// Record an event in the log.
    pub fn record_event(&self, event: ReplayEvent) -> LogSequence {
        // lock state for mutation
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());

        // assign the next sequence
        let sequence = state.next_sequence;
        state.next_sequence = state.next_sequence.next();

        // append the event payload
        state.events.push(event);

        sequence
    }

    /// Read the next recorded event if available.
    pub fn next_event(&self) -> Option<ReplayEvent> {
        // lock state for replay
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());

        // return None when events are exhausted
        if state.read_index >= state.events.len() {
            return None;
        }

        // return the next event in sequence
        let event = state.events[state.read_index].clone();
        state.read_index += 1;

        Some(event)
    }
}

impl crate::replay::ReplayWriter for ReplayLog {
    fn record_event(&mut self, event: ReplayEvent) {
        ReplayLog::record_event(self, event);
    }
}

impl crate::replay::ReplayReader for ReplayLog {
    fn next_event(&mut self) -> Option<ReplayEvent> {
        ReplayLog::next_event(self)
    }
}
