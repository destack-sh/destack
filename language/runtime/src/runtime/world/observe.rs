use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{ResourceBacking, ResourceCapture, ResourcePortability};
use crate::runtime::time::WorldInstant;
use crate::runtime::trace::TraceSequence;
use crate::runtime::{AgentId, RuntimeId};

use super::{Moment, World, WorldResourceId};

/// Stable sequence number for one observation entry.
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

/// Observation category for emitted runtime or user facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationCategory {
    /// Runtime lifecycle and engine diagnostics.
    Runtime,
    /// Topology mutation and graph diagnostics.
    Topology,
    /// Resource lifecycle and capability diagnostics.
    Resource,
    /// Scheduler and execution-lane diagnostics.
    Scheduler,
    /// General diagnostic and policy notices.
    Diagnostic,
    /// Telemetry, tracing, and performance instrumentation.
    Telemetry,
    /// Domain-level user or library observations.
    Domain,
}

/// Scope for one emitted observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope {
    /// One world-scoped observation.
    World,
    /// One runtime-scoped observation.
    Runtime {
        /// Runtime identifier for this scope.
        runtime_id: RuntimeId,
    },
    /// One agent-scoped observation.
    Agent {
        /// Optional owning runtime identifier when known.
        runtime_id: Option<RuntimeId>,
        /// Agent identifier for this scope.
        agent_id: AgentId,
    },
    /// One topology entity-scoped observation.
    Entity {
        /// Entity identifier for this scope.
        entity_id: String,
    },
    /// One topology edge-scoped observation.
    Edge {
        /// Edge identifier for this scope.
        edge_id: String,
    },
    /// One resource-scoped observation.
    Resource {
        /// Agent identifier that owns the resource.
        agent_id: AgentId,
        /// Logical world resource identifier.
        resource_id: WorldResourceId,
    },
}

impl Scope {
    /// Create one world scope.
    pub const fn world() -> Self {
        Self::World
    }

    /// Create one runtime scope.
    pub const fn runtime(runtime_id: RuntimeId) -> Self {
        Self::Runtime { runtime_id }
    }

    /// Create one agent scope.
    pub const fn agent(runtime_id: Option<RuntimeId>, agent_id: AgentId) -> Self {
        Self::Agent {
            runtime_id,
            agent_id,
        }
    }

    /// Create one entity scope.
    pub fn entity(entity_id: impl Into<String>) -> Self {
        Self::Entity {
            entity_id: entity_id.into(),
        }
    }

    /// Create one edge scope.
    pub fn edge(edge_id: impl Into<String>) -> Self {
        Self::Edge {
            edge_id: edge_id.into(),
        }
    }

    /// Create one resource scope.
    pub const fn resource(agent_id: AgentId, resource_id: WorldResourceId) -> Self {
        Self::Resource {
            agent_id,
            resource_id,
        }
    }

    /// Return the runtime id for this scope when present.
    pub const fn runtime_id(&self) -> Option<RuntimeId> {
        match self {
            Self::Runtime { runtime_id } => Some(*runtime_id),
            Self::Agent {
                runtime_id: Some(runtime_id),
                ..
            } => Some(*runtime_id),
            _ => None,
        }
    }

    /// Return the agent id for this scope when present.
    pub const fn agent_id(&self) -> Option<AgentId> {
        match self {
            Self::Agent { agent_id, .. } => Some(*agent_id),
            Self::Resource { agent_id, .. } => Some(*agent_id),
            _ => None,
        }
    }

    /// Return the entity id for this scope when present.
    pub fn entity_id(&self) -> Option<&str> {
        match self {
            Self::Entity { entity_id } => Some(entity_id),
            _ => None,
        }
    }

    /// Return the edge id for this scope when present.
    pub fn edge_id(&self) -> Option<&str> {
        match self {
            Self::Edge { edge_id } => Some(edge_id),
            _ => None,
        }
    }

    /// Return the resource id for this scope when present.
    pub const fn resource_id(&self) -> Option<WorldResourceId> {
        match self {
            Self::Resource { resource_id, .. } => Some(*resource_id),
            _ => None,
        }
    }
}

/// Filter options for one observation subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationOptions {
    /// Include runtime observations.
    pub runtime: bool,
    /// Include topology observations.
    pub topology: bool,
    /// Include resource observations.
    pub resource: bool,
    /// Include scheduler observations.
    pub scheduler: bool,
    /// Include diagnostic observations.
    pub diagnostic: bool,
    /// Include telemetry observations.
    pub telemetry: bool,
    /// Include domain observations.
    pub domain: bool,
}

impl Default for ObservationOptions {
    fn default() -> Self {
        Self {
            runtime: true,
            topology: true,
            resource: true,
            scheduler: true,
            diagnostic: true,
            telemetry: true,
            domain: true,
        }
    }
}

impl ObservationOptions {
    /// Return whether this filter allows one observation category.
    pub const fn allows(self, category: ObservationCategory) -> bool {
        match category {
            ObservationCategory::Runtime => self.runtime,
            ObservationCategory::Topology => self.topology,
            ObservationCategory::Resource => self.resource,
            ObservationCategory::Scheduler => self.scheduler,
            ObservationCategory::Diagnostic => self.diagnostic,
            ObservationCategory::Telemetry => self.telemetry,
            ObservationCategory::Domain => self.domain,
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

/// Payload data for one emitted observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationData {
    /// One summary message.
    Summary {
        /// Short summary payload.
        message: String,
    },
    /// One resource lifecycle payload.
    ResourceLifecycle {
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
    /// One scheduler progress payload.
    Scheduler {
        /// Scheduler outcome classification.
        outcome: ObservationSchedulerOutcome,
        /// Optional virtual-time deadline reached by the scheduler.
        deadline: Option<WorldInstant>,
    },
    /// One generic structured field payload.
    Fields {
        /// Structured observation fields.
        fields: BTreeMap<String, String>,
    },
}

/// One emitted observable fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    /// The observation category.
    pub category: ObservationCategory,
    /// The observation scope.
    pub scope: Scope,
    /// Stable observation name.
    pub name: String,
    /// Structured observation tags.
    pub tags: BTreeMap<String, String>,
    /// Observation payload data.
    pub data: ObservationData,
}

impl Observation {
    /// Create one observation from explicit parts.
    pub fn new(
        category: ObservationCategory,
        scope: Scope,
        name: impl Into<String>,
        data: ObservationData,
    ) -> Self {
        Self {
            category,
            scope,
            name: name.into(),
            tags: BTreeMap::new(),
            data,
        }
    }

    /// Create one summary observation.
    pub fn summary(
        category: ObservationCategory,
        scope: Scope,
        name: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            category,
            scope,
            name,
            ObservationData::Summary {
                message: message.into(),
            },
        )
    }

    /// Create one world-scoped summary observation.
    pub fn world_summary(
        category: ObservationCategory,
        name: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::summary(category, Scope::world(), name, message)
    }

    /// Create one world-scoped structured-field observation.
    pub fn world_fields<K, V>(
        category: ObservationCategory,
        name: impl Into<String>,
        fields: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        Self::fields(category, Scope::world(), name, fields)
    }

    /// Create one runtime-scoped summary observation.
    pub fn runtime_summary(
        category: ObservationCategory,
        runtime_id: RuntimeId,
        name: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::summary(category, Scope::runtime(runtime_id), name, message)
    }

    /// Create one runtime-scoped structured-field observation.
    pub fn runtime_fields<K, V>(
        category: ObservationCategory,
        runtime_id: RuntimeId,
        name: impl Into<String>,
        fields: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        Self::fields(category, Scope::runtime(runtime_id), name, fields)
    }

    /// Create one agent-scoped summary observation.
    pub fn agent_summary(
        category: ObservationCategory,
        runtime_id: Option<RuntimeId>,
        agent_id: AgentId,
        name: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::summary(category, Scope::agent(runtime_id, agent_id), name, message)
    }

    /// Create one agent-scoped structured-field observation.
    pub fn agent_fields<K, V>(
        category: ObservationCategory,
        runtime_id: Option<RuntimeId>,
        agent_id: AgentId,
        name: impl Into<String>,
        fields: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        Self::fields(category, Scope::agent(runtime_id, agent_id), name, fields)
    }

    /// Create one entity-scoped summary observation.
    pub fn entity_summary(
        category: ObservationCategory,
        entity_id: impl Into<String>,
        name: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::summary(category, Scope::entity(entity_id), name, message)
    }

    /// Create one entity-scoped structured-field observation.
    pub fn entity_fields<K, V>(
        category: ObservationCategory,
        entity_id: impl Into<String>,
        name: impl Into<String>,
        fields: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        Self::fields(category, Scope::entity(entity_id), name, fields)
    }

    /// Create one edge-scoped summary observation.
    pub fn edge_summary(
        category: ObservationCategory,
        edge_id: impl Into<String>,
        name: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::summary(category, Scope::edge(edge_id), name, message)
    }

    /// Create one edge-scoped structured-field observation.
    pub fn edge_fields<K, V>(
        category: ObservationCategory,
        edge_id: impl Into<String>,
        name: impl Into<String>,
        fields: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        Self::fields(category, Scope::edge(edge_id), name, fields)
    }

    /// Create one structured-field observation.
    pub fn fields<K, V>(
        category: ObservationCategory,
        scope: Scope,
        name: impl Into<String>,
        fields: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        let fields = fields
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect();

        Self::new(category, scope, name, ObservationData::Fields { fields })
    }

    /// Create one resource lifecycle observation.
    pub fn resource_lifecycle(
        agent_id: AgentId,
        resource_id: WorldResourceId,
        is_attach: bool,
        backing: ResourceBacking,
        capture: ResourceCapture,
        portability: ResourcePortability,
    ) -> Self {
        let name = if is_attach {
            "resource.attach"
        } else {
            "resource.detach"
        };

        Self::new(
            ObservationCategory::Resource,
            Scope::resource(agent_id, resource_id),
            name,
            ObservationData::ResourceLifecycle {
                agent_id,
                resource_id,
                is_attach,
                backing,
                capture,
                portability,
            },
        )
    }

    /// Create one scheduler observation.
    pub fn scheduler(outcome: ObservationSchedulerOutcome, deadline: Option<WorldInstant>) -> Self {
        let name = match outcome {
            ObservationSchedulerOutcome::Progressed => "scheduler.progressed",
            ObservationSchedulerOutcome::AdvancedTime => "scheduler.advanced_time",
        };

        Self::new(
            ObservationCategory::Scheduler,
            Scope::world(),
            name,
            ObservationData::Scheduler { outcome, deadline },
        )
    }

    /// Return one copy of this observation with one additional tag.
    pub fn tagged(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    /// Return the value for one observation tag when present.
    pub fn tag(&self, key: &str) -> Option<&str> {
        self.tags.get(key).map(String::as_str)
    }
}

/// One recorded observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationRecord {
    /// Observation sequence number.
    pub sequence: ObservationSequence,
    /// Execution coordinate where this observation was emitted.
    pub moment: Moment,
    /// Emitted observation payload.
    pub observation: Observation,
}

/// World-owned observation log kept separate from causal trace.
#[derive(Debug, Default)]
pub struct ObservationLog {
    /// Next observation sequence number.
    next_sequence: AtomicU64,
    /// Next observation subscription identifier.
    next_subscription_id: AtomicU64,
    /// Recorded observation entries.
    entries: RwLock<Vec<ObservationRecord>>,
    /// Live observation subscriptions keyed by identifier.
    subscriptions: RwLock<BTreeMap<ObservationSubscriptionId, ObservationSubscription>>,
}

impl ObservationLog {
    /// Record one observation at one exact execution coordinate.
    pub fn record_at(&self, moment: Moment, observation: Observation) -> ObservationSequence {
        let sequence = ObservationSequence::new(self.next_sequence.fetch_add(1, Ordering::SeqCst));
        let mut entries = self.entries.write();
        entries.push(ObservationRecord {
            sequence,
            moment,
            observation,
        });

        sequence
    }

    /// Return every observation entry after the optional sequence.
    pub fn records_after(&self, after: Option<ObservationSequence>) -> Vec<ObservationRecord> {
        self.records_after_with_options(after, ObservationOptions::default())
    }

    /// Return every filtered observation entry after the optional sequence.
    pub fn records_after_with_options(
        &self,
        after: Option<ObservationSequence>,
        options: ObservationOptions,
    ) -> Vec<ObservationRecord> {
        let entries = self.entries.read();

        let start_index = match after {
            Some(after) => entries.partition_point(|entry| entry.sequence <= after),
            None => 0,
        };

        entries[start_index..]
            .iter()
            .filter(|entry| options.allows(entry.observation.category))
            .cloned()
            .collect()
    }

    /// Return every observation entry within one moment range.
    pub fn records_between(&self, start: Moment, end: Moment) -> Vec<ObservationRecord> {
        let entries = self.entries.read();

        entries
            .iter()
            .filter(|entry| {
                entry.moment.branch_id == start.branch_id
                    && entry.moment.sequence.get() > start.sequence.get()
                    && entry.moment.sequence.get() <= end.sequence.get()
            })
            .cloned()
            .collect()
    }

    /// Drain every observation entry up to one exact committed sequence.
    pub fn drain_through(
        &self,
        branch_id: super::BranchId,
        sequence: TraceSequence,
    ) -> Vec<ObservationRecord> {
        let mut entries = self.entries.write();
        let split_index = entries.partition_point(|entry| {
            entry.moment.branch_id == branch_id && entry.moment.sequence.get() <= sequence.get()
        });

        entries.drain(..split_index).collect()
    }

    /// Reset the live observation tail and subscriptions.
    pub fn reset(&self) {
        self.next_sequence.store(0, Ordering::SeqCst);
        self.next_subscription_id.store(0, Ordering::SeqCst);
        self.entries.write().clear();
        self.subscriptions.write().clear();
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

    /// Read the next batch of observation entries from one subscription.
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

        let entries = self.entries.read();
        let start_index =
            entries.partition_point(|entry| entry.sequence < subscription.next_sequence);
        let mut batch = Vec::new();

        // scan the shared observation log from the subscription cursor
        for entry in &entries[start_index..] {
            if !subscription.options.allows(entry.observation.category) {
                continue;
            }

            batch.push(entry.clone());
            if batch.len() == limit {
                break;
            }
        }

        // advance the subscription cursor after a successful batch
        if let Some(entry) = batch.last() {
            subscription.next_sequence = entry.sequence.next();
        }

        Ok(batch)
    }
}

impl World {
    /// Emit one observation at the current execution coordinate.
    pub fn observe(&self, observation: Observation) -> ObservationSequence {
        let moment = self.moment();
        self.observations.record_at(moment, observation)
    }
}
