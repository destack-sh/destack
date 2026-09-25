use serde::{Deserialize, Serialize};
use tspp_heap as heap;
use tspp_program as program;
use tspp_serde::Reflect;

use super::ObservationScope;
use crate::debugger::ProbeId;
use crate::host::ResourceId;
use crate::runtime::RuntimeId;
use crate::scheduler::RunnableId;
use crate::worker::WorkerId;
use crate::world::policy::RuleId;
use crate::world::time::Instant;
use crate::world::topology::{EdgeId, EdgeKind, EntityId, EntityKind};

/// One emitted observable fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Observation {
    // runtime
    /// runtime.spawned
    RuntimeSpawned {
        /// Created runtime identifier.
        runtime_id: RuntimeId,
        /// Number of workers created with the runtime.
        worker_count: usize,
    },
    /// runtime.removed
    RuntimeRemoved {
        /// Removed runtime identifier.
        runtime_id: RuntimeId,
    },
    /// runtime.worker.spawned
    WorkerSpawned {
        /// Owning runtime identifier.
        runtime_id: RuntimeId,
        /// Created worker identifier.
        worker_id: WorkerId,
    },
    /// runtime.worker.removed
    WorkerRemoved {
        /// Removed worker identifier.
        worker_id: WorkerId,
    },

    // topology
    /// runtime.entity.kind.defined
    EntityKindDefined {
        /// Defined entity kind identifier.
        kind: EntityKind,
    },
    /// runtime.entity.kind.removed
    EntityKindRemoved {
        /// Removed entity kind identifier.
        kind: EntityKind,
    },
    /// runtime.edge.kind.defined
    EdgeKindDefined {
        /// Defined edge kind identifier.
        kind: EdgeKind,
    },
    /// runtime.edge.kind.removed
    EdgeKindRemoved {
        /// Removed edge kind identifier.
        kind: EdgeKind,
    },
    /// runtime.entity.upserted
    EntityUpserted {
        /// Upserted entity identifier.
        entity_id: EntityId,
    },
    /// runtime.entity.removed
    EntityRemoved {
        /// Removed entity identifier.
        entity_id: EntityId,
    },
    /// runtime.edge.upserted
    EdgeUpserted {
        /// Upserted edge identifier.
        edge_id: EdgeId,
    },
    /// runtime.edge.removed
    EdgeRemoved {
        /// Removed edge identifier.
        edge_id: EdgeId,
    },

    // policy
    /// runtime.policy.replaced
    PolicyReplaced,
    /// runtime.policy.rule.added
    RuleAdded {
        /// Added rule identifier.
        rule_id: RuleId,
    },
    /// runtime.policy.rule.removed
    RuleRemoved {
        /// Removed rule identifier.
        rule_id: RuleId,
    },
    /// runtime.policy.rule.enabled
    RuleEnabled {
        /// Enabled rule identifier.
        rule_id: RuleId,
    },
    /// runtime.policy.rule.disabled
    RuleDisabled {
        /// Disabled rule identifier.
        rule_id: RuleId,
    },
    /// runtime.policy.rule.replaced
    RuleReplaced {
        /// Replaced rule identifier.
        rule_id: RuleId,
    },

    // debugger
    /// runtime.debug.breakpoint.added
    BreakpointAdded {
        /// Added breakpoint identifier.
        breakpoint_id: program::BreakpointId,
    },
    /// runtime.debug.breakpoint.updated
    BreakpointUpdated {
        /// Updated breakpoint identifier.
        breakpoint_id: program::BreakpointId,
    },
    /// runtime.debug.breakpoint.removed
    BreakpointRemoved {
        /// Removed breakpoint identifier.
        breakpoint_id: program::BreakpointId,
    },
    /// runtime.debug.watchpoint.added
    WatchpointAdded {
        /// Added watchpoint identifier.
        watchpoint_id: program::WatchpointId,
    },
    /// runtime.debug.watchpoint.updated
    WatchpointUpdated {
        /// Updated watchpoint identifier.
        watchpoint_id: program::WatchpointId,
    },
    /// runtime.debug.watchpoint.removed
    WatchpointRemoved {
        /// Removed watchpoint identifier.
        watchpoint_id: program::WatchpointId,
    },
    /// runtime.debug.probe.added
    ProbeAdded {
        /// Added probe identifier.
        probe_id: ProbeId,
    },
    /// runtime.debug.probe.updated
    ProbeUpdated {
        /// Updated probe identifier.
        probe_id: ProbeId,
    },
    /// runtime.debug.probe.removed
    ProbeRemoved {
        /// Removed probe identifier.
        probe_id: ProbeId,
    },
    /// runtime.debug.probe.hit
    ProbeHit {
        /// Matching probe identifier.
        probe_id: ProbeId,
        /// Durable matching-event count after this hit.
        count: u64,
        /// Runtime that produced the event.
        runtime_id: RuntimeId,
        /// Worker that produced the event.
        worker_id: WorkerId,
        /// Fiber that produced the event when execution has one.
        fiber_id: Option<program::FiberId>,
        /// Matched Program execution event.
        event: program::Event,
    },

    // scheduler
    /// runtime.task.ran
    TaskRan {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that ran the task.
        worker_id: WorkerId,
        /// Task runnable identifier.
        task_id: RunnableId,
    },
    /// runtime.microtask.ran
    MicrotaskRan {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that ran the microtask.
        worker_id: WorkerId,
        /// Microtask runnable identifier.
        microtask_id: RunnableId,
    },
    /// runtime.task.resumed
    TaskResumed {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that resumed the task.
        worker_id: WorkerId,
        /// Resumed task runnable identifier.
        task_id: RunnableId,
    },
    /// runtime.microtask.resumed
    MicrotaskResumed {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that resumed the microtask.
        worker_id: WorkerId,
        /// Resumed microtask runnable identifier.
        microtask_id: RunnableId,
    },

    // execution
    /// runtime.stop.reached
    StopReached {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that reached the stop.
        worker_id: WorkerId,
        /// Fiber that reached the stop.
        fiber_id: program::FiberId,
        /// Stop reason.
        reason: program::StopReason,
    },

    // ingress and time
    /// runtime.ingress.delivered
    IngressDelivered {
        /// Number of delivered host events.
        host_events: usize,
        /// Number of delivered poller events.
        poller_events: usize,
    },
    /// runtime.time.advanced
    TimeAdvanced {
        /// New runtime-controlled deadline.
        deadline: Instant,
    },

    // heap
    /// runtime.gc.local.started
    LocalGcStarted {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that owns the local heap.
        worker_id: WorkerId,
        /// Collector that started.
        collector: heap::GcCollector,
    },
    /// runtime.gc.local.stepped
    LocalGcStepped {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that owns the local heap.
        worker_id: WorkerId,
        /// Completed collector step.
        step: heap::GcStep,
    },
    /// runtime.gc.local.completed
    LocalGcCompleted {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that owns the local heap.
        worker_id: WorkerId,
        /// Completed collector cycle.
        cycle: heap::GcCycle,
    },
    /// runtime.gc.shared.started
    SharedGcStarted {
        /// Runtime that owns the shared heap.
        runtime_id: RuntimeId,
        /// Collector that started.
        collector: heap::GcCollector,
    },
    /// runtime.gc.shared.stepped
    SharedGcStepped {
        /// Runtime that owns the shared heap.
        runtime_id: RuntimeId,
        /// Worker that assisted the shared collector when present.
        worker_id: Option<WorkerId>,
        /// Completed collector step.
        step: heap::GcStep,
    },
    /// runtime.gc.shared.completed
    SharedGcCompleted {
        /// Runtime that owns the shared heap.
        runtime_id: RuntimeId,
        /// Worker that completed the shared collector when present.
        worker_id: Option<WorkerId>,
        /// Completed collector cycle.
        cycle: heap::GcCycle,
    },

    // resources
    /// runtime.resource.attached
    ResourceAttached {
        /// Worker that owns the resource.
        worker_id: WorkerId,
        /// Attached resource identifier.
        resource_id: ResourceId,
    },
    /// runtime.resource.detached
    ResourceDetached {
        /// Worker that owned the resource.
        worker_id: WorkerId,
        /// Detached resource identifier.
        resource_id: ResourceId,
    },
}

impl Observation {
    /// Return the observation scope.
    pub fn scope(&self) -> ObservationScope {
        match self {
            Self::RuntimeSpawned { runtime_id, .. }
            | Self::RuntimeRemoved { runtime_id }
            | Self::SharedGcStarted { runtime_id, .. } => ObservationScope::runtime(*runtime_id),
            Self::WorkerSpawned {
                runtime_id,
                worker_id,
            }
            | Self::TaskRan {
                runtime_id,
                worker_id,
                ..
            }
            | Self::MicrotaskRan {
                runtime_id,
                worker_id,
                ..
            }
            | Self::TaskResumed {
                runtime_id,
                worker_id,
                ..
            }
            | Self::MicrotaskResumed {
                runtime_id,
                worker_id,
                ..
            }
            | Self::LocalGcStarted {
                runtime_id,
                worker_id,
                ..
            }
            | Self::LocalGcStepped {
                runtime_id,
                worker_id,
                ..
            }
            | Self::LocalGcCompleted {
                runtime_id,
                worker_id,
                ..
            }
            | Self::SharedGcStepped {
                runtime_id,
                worker_id: Some(worker_id),
                ..
            }
            | Self::SharedGcCompleted {
                runtime_id,
                worker_id: Some(worker_id),
                ..
            } => ObservationScope::worker(Some(*runtime_id), *worker_id),
            Self::StopReached {
                runtime_id,
                worker_id,
                fiber_id,
                ..
            } => ObservationScope::fiber(*runtime_id, *worker_id, *fiber_id),
            Self::ProbeHit {
                runtime_id,
                worker_id,
                fiber_id: Some(fiber_id),
                ..
            } => ObservationScope::fiber(*runtime_id, *worker_id, *fiber_id),
            Self::ProbeHit {
                runtime_id,
                worker_id,
                fiber_id: None,
                ..
            } => ObservationScope::worker(Some(*runtime_id), *worker_id),
            Self::WorkerRemoved { worker_id } => ObservationScope::worker(None, *worker_id),
            Self::EntityUpserted { entity_id } | Self::EntityRemoved { entity_id } => {
                ObservationScope::entity(entity_id.as_str())
            }
            Self::EdgeUpserted { edge_id } | Self::EdgeRemoved { edge_id } => {
                ObservationScope::edge(edge_id.as_str())
            }
            Self::EntityKindDefined { .. }
            | Self::EntityKindRemoved { .. }
            | Self::EdgeKindDefined { .. }
            | Self::EdgeKindRemoved { .. }
            | Self::PolicyReplaced
            | Self::RuleAdded { .. }
            | Self::RuleRemoved { .. }
            | Self::RuleEnabled { .. }
            | Self::RuleDisabled { .. }
            | Self::RuleReplaced { .. }
            | Self::BreakpointAdded { .. }
            | Self::BreakpointUpdated { .. }
            | Self::BreakpointRemoved { .. }
            | Self::WatchpointAdded { .. }
            | Self::WatchpointUpdated { .. }
            | Self::WatchpointRemoved { .. }
            | Self::ProbeAdded { .. }
            | Self::ProbeUpdated { .. }
            | Self::ProbeRemoved { .. }
            | Self::IngressDelivered { .. }
            | Self::TimeAdvanced { .. } => ObservationScope::world(),
            Self::SharedGcStepped {
                runtime_id,
                worker_id: None,
                ..
            }
            | Self::SharedGcCompleted {
                runtime_id,
                worker_id: None,
                ..
            } => ObservationScope::runtime(*runtime_id),
            Self::ResourceAttached { resource_id, .. }
            | Self::ResourceDetached { resource_id, .. } => {
                ObservationScope::resource(*resource_id)
            }
        }
    }

    /// Return whether this observation is on one exact scope.
    pub fn is_on(&self, scope: &ObservationScope) -> bool {
        match scope {
            ObservationScope::World => matches!(
                self,
                Self::EntityKindDefined { .. }
                    | Self::EntityKindRemoved { .. }
                    | Self::EdgeKindDefined { .. }
                    | Self::EdgeKindRemoved { .. }
                    | Self::PolicyReplaced
                    | Self::RuleAdded { .. }
                    | Self::RuleRemoved { .. }
                    | Self::RuleEnabled { .. }
                    | Self::RuleDisabled { .. }
                    | Self::RuleReplaced { .. }
                    | Self::BreakpointAdded { .. }
                    | Self::BreakpointUpdated { .. }
                    | Self::BreakpointRemoved { .. }
                    | Self::WatchpointAdded { .. }
                    | Self::WatchpointUpdated { .. }
                    | Self::WatchpointRemoved { .. }
                    | Self::ProbeAdded { .. }
                    | Self::ProbeUpdated { .. }
                    | Self::ProbeRemoved { .. }
                    | Self::IngressDelivered { .. }
                    | Self::TimeAdvanced { .. }
            ),
            ObservationScope::Runtime { runtime_id } => {
                self.runtime_id() == Some(*runtime_id) && self.worker_id().is_none()
            }
            ObservationScope::Worker { worker_id } => {
                self.worker_id() == Some(*worker_id) && self.runtime_id().is_none()
            }
            ObservationScope::RuntimeWorker {
                runtime_id,
                worker_id,
            } => {
                self.runtime_id() == Some(*runtime_id)
                    && self.worker_id() == Some(*worker_id)
                    && self.fiber_id().is_none()
            }
            ObservationScope::Fiber {
                runtime_id,
                worker_id,
                fiber_id,
            } => {
                self.runtime_id() == Some(*runtime_id)
                    && self.worker_id() == Some(*worker_id)
                    && self.fiber_id() == Some(*fiber_id)
            }
            ObservationScope::Entity { entity_id } => self.entity_id() == Some(entity_id.as_str()),
            ObservationScope::Edge { edge_id } => self.edge_id() == Some(edge_id.as_str()),
            ObservationScope::Resource { resource_id } => self.resource_id() == Some(*resource_id),
        }
    }

    /// Return the runtime id for this observation when present.
    pub const fn runtime_id(&self) -> Option<RuntimeId> {
        match self {
            Self::RuntimeSpawned { runtime_id, .. }
            | Self::RuntimeRemoved { runtime_id }
            | Self::WorkerSpawned { runtime_id, .. }
            | Self::TaskRan { runtime_id, .. }
            | Self::MicrotaskRan { runtime_id, .. }
            | Self::TaskResumed { runtime_id, .. }
            | Self::MicrotaskResumed { runtime_id, .. }
            | Self::ProbeHit { runtime_id, .. }
            | Self::StopReached { runtime_id, .. }
            | Self::LocalGcStarted { runtime_id, .. }
            | Self::LocalGcStepped { runtime_id, .. }
            | Self::LocalGcCompleted { runtime_id, .. }
            | Self::SharedGcStarted { runtime_id, .. }
            | Self::SharedGcStepped { runtime_id, .. }
            | Self::SharedGcCompleted { runtime_id, .. } => Some(*runtime_id),
            _ => None,
        }
    }

    /// Return the worker id for this observation when present.
    pub const fn worker_id(&self) -> Option<WorkerId> {
        match self {
            Self::WorkerSpawned { worker_id, .. }
            | Self::WorkerRemoved { worker_id }
            | Self::TaskRan { worker_id, .. }
            | Self::MicrotaskRan { worker_id, .. }
            | Self::TaskResumed { worker_id, .. }
            | Self::MicrotaskResumed { worker_id, .. }
            | Self::ProbeHit { worker_id, .. }
            | Self::StopReached { worker_id, .. }
            | Self::LocalGcStarted { worker_id, .. }
            | Self::LocalGcStepped { worker_id, .. }
            | Self::LocalGcCompleted { worker_id, .. }
            | Self::SharedGcStepped {
                worker_id: Some(worker_id),
                ..
            }
            | Self::SharedGcCompleted {
                worker_id: Some(worker_id),
                ..
            }
            | Self::ResourceAttached { worker_id, .. }
            | Self::ResourceDetached { worker_id, .. } => Some(*worker_id),
            _ => None,
        }
    }

    /// Return the fiber id for this observation when present.
    pub const fn fiber_id(&self) -> Option<program::FiberId> {
        match self {
            Self::ProbeHit { fiber_id, .. } => *fiber_id,
            Self::StopReached { fiber_id, .. } => Some(*fiber_id),
            _ => None,
        }
    }

    /// Return the topology entity id for this observation when present.
    pub fn entity_id(&self) -> Option<&str> {
        match self {
            Self::EntityUpserted { entity_id } | Self::EntityRemoved { entity_id } => {
                Some(entity_id.as_str())
            }
            _ => None,
        }
    }

    /// Return the topology edge id for this observation when present.
    pub fn edge_id(&self) -> Option<&str> {
        match self {
            Self::EdgeUpserted { edge_id } | Self::EdgeRemoved { edge_id } => {
                Some(edge_id.as_str())
            }
            _ => None,
        }
    }

    /// Return the resource id for this observation when present.
    pub const fn resource_id(&self) -> Option<ResourceId> {
        match self {
            Self::ResourceAttached { resource_id, .. }
            | Self::ResourceDetached { resource_id, .. } => Some(*resource_id),
            _ => None,
        }
    }

    /// Return the stable observation name.
    pub fn name(&self) -> &str {
        match self {
            Self::RuntimeSpawned { .. } => "runtime.spawned",
            Self::RuntimeRemoved { .. } => "runtime.removed",
            Self::WorkerSpawned { .. } => "runtime.worker.spawned",
            Self::WorkerRemoved { .. } => "runtime.worker.removed",
            Self::EntityKindDefined { .. } => "runtime.entity.kind.defined",
            Self::EntityKindRemoved { .. } => "runtime.entity.kind.removed",
            Self::EdgeKindDefined { .. } => "runtime.edge.kind.defined",
            Self::EdgeKindRemoved { .. } => "runtime.edge.kind.removed",
            Self::EntityUpserted { .. } => "runtime.entity.upserted",
            Self::EntityRemoved { .. } => "runtime.entity.removed",
            Self::EdgeUpserted { .. } => "runtime.edge.upserted",
            Self::EdgeRemoved { .. } => "runtime.edge.removed",
            Self::PolicyReplaced => "runtime.policy.replaced",
            Self::RuleAdded { .. } => "runtime.policy.rule.added",
            Self::RuleRemoved { .. } => "runtime.policy.rule.removed",
            Self::RuleEnabled { .. } => "runtime.policy.rule.enabled",
            Self::RuleDisabled { .. } => "runtime.policy.rule.disabled",
            Self::RuleReplaced { .. } => "runtime.policy.rule.replaced",
            Self::BreakpointAdded { .. } => "runtime.debug.breakpoint.added",
            Self::BreakpointUpdated { .. } => "runtime.debug.breakpoint.updated",
            Self::BreakpointRemoved { .. } => "runtime.debug.breakpoint.removed",
            Self::WatchpointAdded { .. } => "runtime.debug.watchpoint.added",
            Self::WatchpointUpdated { .. } => "runtime.debug.watchpoint.updated",
            Self::WatchpointRemoved { .. } => "runtime.debug.watchpoint.removed",
            Self::ProbeAdded { .. } => "runtime.debug.probe.added",
            Self::ProbeUpdated { .. } => "runtime.debug.probe.updated",
            Self::ProbeRemoved { .. } => "runtime.debug.probe.removed",
            Self::ProbeHit { .. } => "runtime.debug.probe.hit",
            Self::TaskRan { .. } => "runtime.task.ran",
            Self::MicrotaskRan { .. } => "runtime.microtask.ran",
            Self::TaskResumed { .. } => "runtime.task.resumed",
            Self::MicrotaskResumed { .. } => "runtime.microtask.resumed",
            Self::StopReached { .. } => "runtime.stop.reached",
            Self::IngressDelivered { .. } => "runtime.ingress.delivered",
            Self::TimeAdvanced { .. } => "runtime.time.advanced",
            Self::LocalGcStarted { .. } => "runtime.gc.local.started",
            Self::LocalGcStepped { .. } => "runtime.gc.local.stepped",
            Self::LocalGcCompleted { .. } => "runtime.gc.local.completed",
            Self::SharedGcStarted { .. } => "runtime.gc.shared.started",
            Self::SharedGcStepped { .. } => "runtime.gc.shared.stepped",
            Self::SharedGcCompleted { .. } => "runtime.gc.shared.completed",
            Self::ResourceAttached { .. } => "runtime.resource.attached",
            Self::ResourceDetached { .. } => "runtime.resource.detached",
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::host::ResourceId;
    use crate::runtime::RuntimeId;
    use crate::worker::WorkerId;
    use crate::world::observation::{Observation, ObservationScope};

    #[test]
    fn test_match_typed_observation_scope() {
        // build one worker-scoped typed observation
        let runtime_id = RuntimeId(7);
        let worker_id = WorkerId(11);
        let observation = Observation::WorkerSpawned {
            runtime_id,
            worker_id,
        };

        // query identifiers without rebuilding an owned scope
        assert_eq!(observation.runtime_id(), Some(runtime_id));
        assert_eq!(observation.worker_id(), Some(worker_id));
        assert!(observation.is_on(&ObservationScope::worker(Some(runtime_id), worker_id)));
        assert!(!observation.is_on(&ObservationScope::worker(None, worker_id)));
    }

    #[test]
    fn test_match_resource_observation_scope() {
        // build one resource lifecycle observation
        let worker_id = WorkerId(3);
        let resource_id = ResourceId::new(worker_id, 17);
        let observation = Observation::ResourceAttached {
            worker_id,
            resource_id,
        };

        // resource observations retain both worker and resource identity
        assert_eq!(observation.worker_id(), Some(worker_id));
        assert_eq!(observation.resource_id(), Some(resource_id));
        assert!(observation.is_on(&ObservationScope::resource(resource_id)));
    }
}
