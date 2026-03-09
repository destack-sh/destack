use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{ResourceBacking, ResourceCapture, ResourcePortability};
use crate::runtime::AgentId;
use crate::runtime::time::WorldInstant;

use super::{BranchId, WorldResourceId};

/// Stable sequence number for one observation record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObservationSequence(u64);

impl ObservationSequence {
    /// Create one observation sequence number.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw observation sequence.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Return the next observation sequence.
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// Stable identifier for one observation subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObservationSubscriptionId(u64);

impl ObservationSubscriptionId {
    /// Create one observation subscription identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw observation subscription identifier.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Observation event class kept separate from causal trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationKind {
    /// Trace-adjacent runtime event.
    Trace,
    /// Topology mutation event.
    Topology,
    /// Resource lifecycle event.
    Resource,
    /// Scheduler or execution event.
    Scheduler,
    /// Diagnostic or policy event.
    Diagnostic,
    /// Profiling event.
    Profile,
}

/// Filter options for one observation subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationOptions {
    /// Include trace-adjacent events.
    pub trace: bool,
    /// Include topology mutation events.
    pub topology: bool,
    /// Include resource lifecycle events.
    pub resource: bool,
    /// Include scheduler and execution events.
    pub scheduler: bool,
    /// Include diagnostics and policy events.
    pub diagnostic: bool,
    /// Include profiling events.
    pub profile: bool,
}

impl Default for ObservationOptions {
    fn default() -> Self {
        Self {
            trace: true,
            topology: true,
            resource: true,
            scheduler: true,
            diagnostic: true,
            profile: true,
        }
    }
}

impl ObservationOptions {
    /// Return whether this filter allows one observation class.
    pub const fn allows(self, kind: ObservationKind) -> bool {
        match kind {
            ObservationKind::Trace => self.trace,
            ObservationKind::Topology => self.topology,
            ObservationKind::Resource => self.resource,
            ObservationKind::Scheduler => self.scheduler,
            ObservationKind::Diagnostic => self.diagnostic,
            ObservationKind::Profile => self.profile,
        }
    }
}

/// Live cursor state for one observation subscription.
#[derive(Debug, Clone, Copy)]
struct ObservationSubscription {
    /// Filter options for this subscription.
    options: ObservationOptions,
    /// Next sequence visible through this subscription.
    next_sequence: ObservationSequence,
}

/// Scheduler observation outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationSchedulerOutcome {
    /// Runnable work or ingress progressed.
    Progressed,
    /// Virtual time advanced to one deadline.
    AdvancedTime,
}

/// High-volume runtime observation event kept separate from causal trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationEvent {
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
    /// One scheduler progress event.
    Scheduler {
        /// Branch that observed the scheduler event.
        branch_id: BranchId,
        /// Scheduler outcome classification.
        outcome: ObservationSchedulerOutcome,
        /// Optional virtual-time deadline reached by the scheduler.
        deadline: Option<WorldInstant>,
    },
}

impl ObservationEvent {
    /// Return the observation class for this event.
    pub const fn kind(&self) -> ObservationKind {
        match self {
            Self::Control { .. } => ObservationKind::Diagnostic,
            Self::Resource { .. } => ObservationKind::Resource,
            Self::Scheduler { .. } => ObservationKind::Scheduler,
        }
    }
}

/// One recorded observation entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationRecord {
    /// Observation sequence number.
    pub sequence: ObservationSequence,
    /// Observation event class.
    pub kind: ObservationKind,
    /// Observation event payload.
    pub event: ObservationEvent,
}

/// World-owned observation stream kept separate from causal trace.
#[derive(Debug, Default)]
pub struct Observation {
    /// Next observation sequence number.
    next_sequence: AtomicU64,
    /// Next observation subscription identifier.
    next_subscription_id: AtomicU64,
    /// Recorded observation entries.
    records: RwLock<Vec<ObservationRecord>>,
    /// Live observation subscriptions keyed by identifier.
    subscriptions: RwLock<BTreeMap<ObservationSubscriptionId, ObservationSubscription>>,
}

impl Observation {
    /// Record one observation event and return its sequence number.
    pub fn record(&self, event: ObservationEvent) -> ObservationSequence {
        let sequence = ObservationSequence::new(self.next_sequence.fetch_add(1, Ordering::SeqCst));
        let kind = event.kind();
        let mut records = self.records.write();
        records.push(ObservationRecord {
            sequence,
            kind,
            event,
        });

        sequence
    }

    /// Return every observation record after the optional sequence.
    pub fn records_after(&self, after: Option<ObservationSequence>) -> Vec<ObservationRecord> {
        self.records_after_with_options(after, ObservationOptions::default())
    }

    /// Return every filtered observation record after the optional sequence.
    pub fn records_after_with_options(
        &self,
        after: Option<ObservationSequence>,
        options: ObservationOptions,
    ) -> Vec<ObservationRecord> {
        let records = self.records.read();

        let start_index = match after {
            Some(after) => records.partition_point(|record| record.sequence <= after),
            None => 0,
        };

        records[start_index..]
            .iter()
            .filter(|record| options.allows(record.kind))
            .cloned()
            .collect()
    }

    /// Open one live observation subscription.
    pub fn open(&self, options: ObservationOptions) -> ObservationSubscriptionId {
        let subscription_id = ObservationSubscriptionId::new(
            self.next_subscription_id.fetch_add(1, Ordering::SeqCst),
        );
        let next_sequence = ObservationSequence::new(self.next_sequence.load(Ordering::SeqCst));
        let mut subscriptions = self.subscriptions.write();
        subscriptions.insert(
            subscription_id,
            ObservationSubscription {
                options,
                next_sequence,
            },
        );

        subscription_id
    }

    /// Close one live observation subscription.
    pub fn close(&self, subscription_id: ObservationSubscriptionId) -> RuntimeResult<()> {
        let mut subscriptions = self.subscriptions.write();
        if subscriptions.remove(&subscription_id).is_none() {
            return Err(RuntimeError::ObservationSubscriptionNotFound {
                subscription_id: subscription_id.get(),
            }
            .boxed());
        }

        Ok(())
    }

    /// Read the next batch of observation records from one subscription.
    pub fn next(
        &self,
        subscription_id: ObservationSubscriptionId,
        limit: usize,
    ) -> RuntimeResult<Vec<ObservationRecord>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let mut subscriptions = self.subscriptions.write();
        let Some(subscription) = subscriptions.get_mut(&subscription_id) else {
            return Err(RuntimeError::ObservationSubscriptionNotFound {
                subscription_id: subscription_id.get(),
            }
            .boxed());
        };

        let records = self.records.read();
        let start_index =
            records.partition_point(|record| record.sequence < subscription.next_sequence);
        let mut batch = Vec::new();

        // scan the shared observation log from the subscription cursor
        for record in &records[start_index..] {
            if !subscription.options.allows(record.kind) {
                continue;
            }

            batch.push(record.clone());
            if batch.len() == limit {
                break;
            }
        }

        // advance the subscription cursor after a successful batch
        if let Some(record) = batch.last() {
            subscription.next_sequence = record.sequence.next();
        }

        Ok(batch)
    }
}
