use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use super::{ObservationEntry, ObservationScope, ObservationSequence};

/// Selection over one World Observation log.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ObservationQuery {
    /// Sequence preceding the first matching Observation.
    pub after: Option<ObservationSequence>,
    /// Scope to match, or every scope when absent.
    pub scope: Option<ObservationScope>,
}

impl ObservationQuery {
    /// Select Observations after one exact sequence.
    pub const fn after(sequence: ObservationSequence) -> Self {
        Self {
            after: Some(sequence),
            scope: None,
        }
    }

    /// Return whether one Observation entry matches this query.
    pub(crate) fn selects(&self, entry: &ObservationEntry) -> bool {
        entry.is_after(self.after)
            && self
                .scope
                .as_ref()
                .is_none_or(|scope| entry.observation.is_on(scope))
    }
}
