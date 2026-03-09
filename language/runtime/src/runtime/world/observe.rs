use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::platform::{ResourceBacking, ResourceCapture, ResourcePortability};
use crate::runtime::AgentId;

use super::{BranchId, WorldResourceId};

/// Stable sequence number for one observation record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObserveSequence(u64);

impl ObserveSequence {
    /// Create one observation sequence number.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw observation sequence.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// High-volume runtime observation event kept separate from causal trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObserveEvent {
    /// One world control mutation.
    Control {
        /// Branch that observed the mutation.
        branch_id: BranchId,
        /// Debug summary for the command.
        summary: String,
    },
    /// One resource lifecycle event.
    Resource {
        /// Branch that observed the event.
        branch_id: BranchId,
        /// Agent that owns the resource.
        agent_id: AgentId,
        /// Logical world resource identifier.
        resource_id: WorldResourceId,
        /// Whether the resource was attached or detached.
        is_attach: bool,
        /// Resource backing model.
        backing: ResourceBacking,
        /// Resource capture model.
        capture: ResourceCapture,
        /// Resource portability model.
        portability: ResourcePortability,
    },
}

/// One recorded observation entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserveRecord {
    /// Observation sequence number.
    pub sequence: ObserveSequence,
    /// Observation event payload.
    pub event: ObserveEvent,
}

/// World-owned observation stream kept separate from causal trace.
#[derive(Debug, Default)]
pub struct Observe {
    /// Next observation sequence number.
    next_sequence: AtomicU64,
    /// Recorded observation entries.
    records: RwLock<Vec<ObserveRecord>>,
}

impl Observe {
    /// Record one observation event and return its sequence number.
    pub fn record(&self, event: ObserveEvent) -> ObserveSequence {
        let sequence = ObserveSequence::new(self.next_sequence.fetch_add(1, Ordering::SeqCst));
        let mut records = self.records.write();
        records.push(ObserveRecord { sequence, event });

        sequence
    }

    /// Return every observation record after the optional sequence.
    pub fn records_after(&self, after: Option<ObserveSequence>) -> Vec<ObserveRecord> {
        let records = self.records.read();

        match after {
            Some(after) => records
                .iter()
                .filter(|record| record.sequence > after)
                .cloned()
                .collect(),
            None => records.clone(),
        }
    }
}
