use serde::{Deserialize, Serialize};

/// One explicit drop reason recorded by runtime owners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DropReason {
    /// Events dropped before delivery because an upstream queue overflowed.
    QueuePressure,
    /// Ingress dropped because no worker declared interest.
    UnmatchedIngress,
    /// Events dropped inside one event loop because no dispatch watch matched.
    UnwatchedDispatch,
}

/// Drop accounting grouped by reason at one ownership boundary.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DropCounts {
    /// Events dropped because an upstream queue overflowed.
    pub queue_pressure: u64,
    /// Ingress dropped because no worker declared interest.
    pub unmatched_ingress: u64,
    /// Events dropped because no dispatch watch matched.
    pub unwatched_dispatch: u64,
}

impl DropCounts {
    /// Record one or more drops for the given reason.
    pub fn record(&mut self, reason: DropReason, count: u64) {
        if count == 0 {
            return;
        }

        match reason {
            DropReason::QueuePressure => {
                self.queue_pressure = self.queue_pressure.saturating_add(count);
            }
            DropReason::UnmatchedIngress => {
                self.unmatched_ingress = self.unmatched_ingress.saturating_add(count);
            }
            DropReason::UnwatchedDispatch => {
                self.unwatched_dispatch = self.unwatched_dispatch.saturating_add(count);
            }
        }
    }

    /// Return the accumulated count for one explicit reason.
    pub const fn count(self, reason: DropReason) -> u64 {
        match reason {
            DropReason::QueuePressure => self.queue_pressure,
            DropReason::UnmatchedIngress => self.unmatched_ingress,
            DropReason::UnwatchedDispatch => self.unwatched_dispatch,
        }
    }

    /// Return the total number of recorded drops.
    pub const fn total(self) -> u64 {
        self.queue_pressure
            .saturating_add(self.unmatched_ingress)
            .saturating_add(self.unwatched_dispatch)
    }
}
