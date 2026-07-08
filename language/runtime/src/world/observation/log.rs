use crate::diagnostic::RuntimeResult;
use crate::world::{BranchId, Moment, MomentSequence};

use super::{
    Observation, ObservationChunk, ObservationEntry, ObservationSequence, ObservationStore,
};

/// Observation log kept separate from causal replay trace.
#[derive(Debug)]
pub struct ObservationLog {
    /// Observation backing store.
    store: ObservationStore,
}

impl Default for ObservationLog {
    fn default() -> Self {
        Self::new()
    }
}

impl ObservationLog {
    /// Create one empty observation log.
    pub fn new() -> Self {
        Self {
            store: ObservationStore::new(),
        }
    }

    /// Record one observation at one exact execution coordinate.
    pub fn record_at(
        &self,
        moment: Moment,
        observation: Observation,
    ) -> RuntimeResult<ObservationSequence> {
        self.store.record_at(moment, observation)
    }

    /// Return every observation entry after the optional sequence.
    pub fn records_after(&self, after: Option<ObservationSequence>) -> Vec<ObservationEntry> {
        self.store.records_after(after)
    }

    /// Return every observation entry within one moment range.
    pub fn records_between(&self, start: Moment, end: Moment) -> Vec<ObservationEntry> {
        self.store.records_between(start, end)
    }

    /// Drain every observation chunk up to one exact committed sequence.
    pub fn drain_through(
        &self,
        branch_id: BranchId,
        sequence: MomentSequence,
    ) -> Vec<ObservationChunk> {
        self.store.drain_through(branch_id, sequence)
    }

    /// Reset the live observation log.
    pub fn reset(&self) {
        self.store.reset();
    }
}

#[cfg(test)]
mod tests {
    use crate::world::observation::Observation;
    use crate::world::{BranchId, Moment, MomentSequence};

    use super::ObservationLog;

    #[test]
    fn test_drain_through_returns_committed_chunks() {
        let branch_id = BranchId::new(7);
        let log = ObservationLog::new();

        // fill one sealed chunk and leave one active entry
        for sequence in 0..=1024 {
            let moment = Moment::new(branch_id, MomentSequence::new(sequence));
            let observation = Observation::IngressDelivered {
                host_events: sequence as usize,
                poller_events: 0,
            };

            log.record_at(moment, observation).unwrap();
        }

        // drain only the sealed committed chunk
        let drained = log.drain_through(branch_id, MomentSequence::new(1023));
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].entries().len(), 1024);

        // retain the active uncommitted tail
        let retained = log.records_after(None);
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].moment.sequence, MomentSequence::new(1024));
    }
}
