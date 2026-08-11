use crate::diagnostic::RuntimeResult;
use crate::world::{BranchId, Moment, MomentSequence};

use super::{
    Observation, ObservationChunk, ObservationEntry, ObservationQuery, ObservationSequence,
    ObservationStore,
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

    /// Return matching Observation entries up to one exact limit.
    pub fn query(&self, query: &ObservationQuery, limit: usize) -> Vec<ObservationEntry> {
        self.store.query(query, limit)
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
    use crate::world::observation::{Observation, ObservationQuery, ObservationSequence};
    use crate::world::{BranchId, Moment, MomentSequence};

    use super::ObservationLog;

    #[test]
    fn test_query_and_drain_observations_across_chunks() {
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

        // bound one query across the sealed and active chunks
        let query = ObservationQuery::after(ObservationSequence::new(1022));
        let queried = log.query(&query, 2);
        let sequences = queried
            .iter()
            .map(|entry| entry.sequence.get())
            .collect::<Vec<_>>();

        assert_eq!(sequences, vec![1023, 1024]);

        // drain only the sealed committed chunk
        let drained = log.drain_through(branch_id, MomentSequence::new(1023));
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].entries().len(), 1024);

        // retain the active uncommitted tail
        let retained = log.query(&ObservationQuery::default(), usize::MAX);
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].moment.sequence, MomentSequence::new(1024));
    }
}
