use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::world::Moment;

use super::{Observation, ObservationSequence};

/// One recorded observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ObservationEntry {
    /// Observation sequence number.
    pub sequence: ObservationSequence,
    /// Execution coordinate where this observation was emitted.
    pub moment: Moment,
    /// Emitted observation payload.
    pub observation: Observation,
}

impl ObservationEntry {
    /// Return whether this entry follows one optional observation sequence.
    pub(crate) fn is_after(&self, sequence: Option<ObservationSequence>) -> bool {
        match sequence {
            Some(sequence) => self.sequence > sequence,
            None => true,
        }
    }

    /// Return whether this entry falls inside one branch-local moment range.
    pub(crate) fn is_between(&self, start: Moment, end: Moment) -> bool {
        self.moment.branch_id == start.branch_id
            && self.moment.sequence.get() > start.sequence.get()
            && self.moment.sequence.get() <= end.sequence.get()
    }
}
