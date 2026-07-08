use serde::{Deserialize, Serialize};

use crate::host::ResourceId;
use crate::host::binding::{BindingId, CodecId};
use crate::runtime::scheduler::RunnableId;
use crate::runtime::time::Instant;
use crate::runtime::worker::RunnableScope;
use crate::runtime::{RuntimeId, WorkerId};
use crate::world::policy::RuleId;
use crate::world::topology::{EdgeId, EdgeKind, EntityId, EntityKind};
use destack_heap as heap;
use destack_program as program;

use super::ObservationScope;

/// One emitted observable fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Observation {
    // runtime
    /// runtime.spawned
    RuntimeSpawned {
        /// Created runtime identifier.
        runtime_id: RuntimeId,
        /// Number of workers created with the runtime.
        worker_count: u32,
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

    // scheduler
    /// runtime.task.scheduled
    TaskScheduled {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that scheduled the task.
        worker_id: WorkerId,
        /// Task runnable identifier.
        task_id: RunnableId,
    },
    /// runtime.task.ran
    TaskRan {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that ran the task.
        worker_id: WorkerId,
        /// Task runnable identifier.
        task_id: RunnableId,
    },
    /// runtime.task.completed
    TaskCompleted {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that completed the task.
        worker_id: WorkerId,
        /// Task runnable identifier.
        task_id: RunnableId,
    },
    /// runtime.task.failed
    TaskFailed {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that failed the task.
        worker_id: WorkerId,
        /// Task runnable identifier.
        task_id: RunnableId,
        /// Human-readable failure detail.
        message: Box<str>,
    },
    /// runtime.microtask.scheduled
    MicrotaskScheduled {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that scheduled the microtask.
        worker_id: WorkerId,
        /// Microtask runnable identifier.
        microtask_id: RunnableId,
        /// Nested microtask execution depth.
        depth: u32,
    },
    /// runtime.microtask.ran
    MicrotaskRan {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that ran the microtask.
        worker_id: WorkerId,
        /// Microtask runnable identifier.
        microtask_id: RunnableId,
        /// Nested microtask execution depth.
        depth: u32,
    },
    /// runtime.microtask.completed
    MicrotaskCompleted {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that completed the microtask.
        worker_id: WorkerId,
        /// Microtask runnable identifier.
        microtask_id: RunnableId,
        /// Nested microtask execution depth.
        depth: u32,
    },
    /// runtime.microtask.failed
    MicrotaskFailed {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that failed the microtask.
        worker_id: WorkerId,
        /// Microtask runnable identifier.
        microtask_id: RunnableId,
        /// Nested microtask execution depth.
        depth: u32,
        /// Human-readable failure detail.
        message: Box<str>,
    },
    /// runtime.task.continued
    TaskContinued {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that continued the task.
        worker_id: WorkerId,
        /// Continued task runnable identifier.
        task_id: RunnableId,
    },
    /// runtime.microtask.continued
    MicrotaskContinued {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that continued the microtask.
        worker_id: WorkerId,
        /// Continued microtask runnable identifier.
        microtask_id: RunnableId,
        /// Nested microtask execution depth.
        depth: u32,
    },

    // execution
    /// runtime.stop.reached
    StopReached {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that reached the stop.
        worker_id: WorkerId,
        /// Stop reason.
        reason: program::StopReason,
    },
    /// runtime.stop.continued
    StopContinued {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that continued.
        worker_id: WorkerId,
        /// Stop reason that was continued.
        reason: program::StopReason,
    },
    /// runtime.panic.raised
    PanicRaised {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that raised the panic.
        worker_id: WorkerId,
        /// Human-readable panic detail.
        message: Box<str>,
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
    /// runtime.timer.scheduled
    TimerScheduled {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that scheduled the timer.
        worker_id: WorkerId,
        /// Timer resource identifier.
        timer_id: ResourceId,
        /// Timer deadline.
        deadline: Instant,
    },
    /// runtime.timer.fired
    TimerFired {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that received the timer.
        worker_id: WorkerId,
        /// Timer resource identifier.
        timer_id: ResourceId,
    },
    /// runtime.timer.cancelled
    TimerCancelled {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that cancelled the timer.
        worker_id: WorkerId,
        /// Timer resource identifier.
        timer_id: ResourceId,
    },

    // bindings
    /// runtime.binding.called
    BindingCalled {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that called the binding.
        worker_id: WorkerId,
        /// Runnable scope active at the binding boundary.
        scope: RunnableScope,
        /// Binding identifier.
        binding_id: BindingId,
        /// Binding codec identifier.
        codec: CodecId,
    },
    /// runtime.binding.returned
    BindingReturned {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that called the binding.
        worker_id: WorkerId,
        /// Runnable scope active at the binding boundary.
        scope: RunnableScope,
        /// Binding identifier.
        binding_id: BindingId,
        /// Binding codec identifier.
        codec: CodecId,
        /// Encoded result byte length.
        byte_len: usize,
    },
    /// runtime.binding.failed
    BindingFailed {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that called the binding.
        worker_id: WorkerId,
        /// Runnable scope active at the binding boundary.
        scope: RunnableScope,
        /// Binding identifier.
        binding_id: BindingId,
        /// Binding codec identifier.
        codec: CodecId,
        /// Human-readable failure detail.
        message: Box<str>,
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
    /// runtime.heap.limit.reached
    HeapLimitReached {
        /// Runtime that owns the heap.
        runtime_id: RuntimeId,
        /// Worker that owns the local heap when present.
        worker_id: Option<WorkerId>,
        /// Retained bytes at the failure point.
        used_bytes: u64,
        /// Configured hard limit in bytes.
        max_bytes: u64,
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
            | Self::TaskScheduled {
                runtime_id,
                worker_id,
                ..
            }
            | Self::TaskRan {
                runtime_id,
                worker_id,
                ..
            }
            | Self::TaskCompleted {
                runtime_id,
                worker_id,
                ..
            }
            | Self::TaskFailed {
                runtime_id,
                worker_id,
                ..
            }
            | Self::MicrotaskScheduled {
                runtime_id,
                worker_id,
                ..
            }
            | Self::MicrotaskRan {
                runtime_id,
                worker_id,
                ..
            }
            | Self::MicrotaskCompleted {
                runtime_id,
                worker_id,
                ..
            }
            | Self::MicrotaskFailed {
                runtime_id,
                worker_id,
                ..
            }
            | Self::TaskContinued {
                runtime_id,
                worker_id,
                ..
            }
            | Self::MicrotaskContinued {
                runtime_id,
                worker_id,
                ..
            }
            | Self::StopReached {
                runtime_id,
                worker_id,
                ..
            }
            | Self::StopContinued {
                runtime_id,
                worker_id,
                ..
            }
            | Self::BindingCalled {
                runtime_id,
                worker_id,
                ..
            }
            | Self::BindingReturned {
                runtime_id,
                worker_id,
                ..
            }
            | Self::BindingFailed {
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
            }
            | Self::PanicRaised {
                runtime_id,
                worker_id,
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
            | Self::ResourceDetached { resource_id, .. }
            | Self::TimerScheduled {
                timer_id: resource_id,
                ..
            }
            | Self::TimerFired {
                timer_id: resource_id,
                ..
            }
            | Self::TimerCancelled {
                timer_id: resource_id,
                ..
            } => ObservationScope::resource(*resource_id),
            Self::HeapLimitReached {
                runtime_id,
                worker_id: Some(worker_id),
                ..
            } => ObservationScope::worker(Some(*runtime_id), *worker_id),
            Self::HeapLimitReached {
                runtime_id,
                worker_id: None,
                ..
            } => ObservationScope::runtime(*runtime_id),
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
                    | Self::IngressDelivered { .. }
                    | Self::TimeAdvanced { .. }
            ),
            ObservationScope::Runtime { runtime_id } => {
                self.runtime_id() == Some(*runtime_id) && self.worker_id().is_none()
            }
            ObservationScope::Worker { worker_id } => {
                self.worker_id() == Some(*worker_id) && self.runtime_id().is_none()
            }
            ObservationScope::RuntimeWorker(scope) => {
                self.runtime_id() == Some(scope.runtime_id)
                    && self.worker_id() == Some(scope.worker_id)
            }
            ObservationScope::Entity(entity_id) => self.entity_id() == Some(entity_id.as_str()),
            ObservationScope::Edge(edge_id) => self.edge_id() == Some(edge_id.as_str()),
            ObservationScope::Resource(resource_id) => self.resource_id() == Some(**resource_id),
        }
    }

    /// Return the runtime id for this observation when present.
    pub const fn runtime_id(&self) -> Option<RuntimeId> {
        match self {
            Self::RuntimeSpawned { runtime_id, .. }
            | Self::RuntimeRemoved { runtime_id }
            | Self::WorkerSpawned { runtime_id, .. }
            | Self::TaskScheduled { runtime_id, .. }
            | Self::TaskRan { runtime_id, .. }
            | Self::TaskCompleted { runtime_id, .. }
            | Self::TaskFailed { runtime_id, .. }
            | Self::MicrotaskScheduled { runtime_id, .. }
            | Self::MicrotaskRan { runtime_id, .. }
            | Self::MicrotaskCompleted { runtime_id, .. }
            | Self::MicrotaskFailed { runtime_id, .. }
            | Self::TaskContinued { runtime_id, .. }
            | Self::MicrotaskContinued { runtime_id, .. }
            | Self::StopReached { runtime_id, .. }
            | Self::StopContinued { runtime_id, .. }
            | Self::TimerScheduled { runtime_id, .. }
            | Self::TimerFired { runtime_id, .. }
            | Self::TimerCancelled { runtime_id, .. }
            | Self::BindingCalled { runtime_id, .. }
            | Self::BindingReturned { runtime_id, .. }
            | Self::BindingFailed { runtime_id, .. }
            | Self::LocalGcStarted { runtime_id, .. }
            | Self::LocalGcStepped { runtime_id, .. }
            | Self::LocalGcCompleted { runtime_id, .. }
            | Self::SharedGcStarted { runtime_id, .. }
            | Self::SharedGcStepped { runtime_id, .. }
            | Self::SharedGcCompleted { runtime_id, .. }
            | Self::HeapLimitReached { runtime_id, .. }
            | Self::PanicRaised { runtime_id, .. } => Some(*runtime_id),
            _ => None,
        }
    }

    /// Return the worker id for this observation when present.
    pub const fn worker_id(&self) -> Option<WorkerId> {
        match self {
            Self::WorkerSpawned { worker_id, .. }
            | Self::WorkerRemoved { worker_id }
            | Self::TaskScheduled { worker_id, .. }
            | Self::TaskRan { worker_id, .. }
            | Self::TaskCompleted { worker_id, .. }
            | Self::TaskFailed { worker_id, .. }
            | Self::MicrotaskScheduled { worker_id, .. }
            | Self::MicrotaskRan { worker_id, .. }
            | Self::MicrotaskCompleted { worker_id, .. }
            | Self::MicrotaskFailed { worker_id, .. }
            | Self::TaskContinued { worker_id, .. }
            | Self::MicrotaskContinued { worker_id, .. }
            | Self::StopReached { worker_id, .. }
            | Self::StopContinued { worker_id, .. }
            | Self::TimerScheduled { worker_id, .. }
            | Self::TimerFired { worker_id, .. }
            | Self::TimerCancelled { worker_id, .. }
            | Self::BindingCalled { worker_id, .. }
            | Self::BindingReturned { worker_id, .. }
            | Self::BindingFailed { worker_id, .. }
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
            | Self::HeapLimitReached {
                worker_id: Some(worker_id),
                ..
            }
            | Self::ResourceAttached { worker_id, .. }
            | Self::ResourceDetached { worker_id, .. }
            | Self::PanicRaised { worker_id, .. } => Some(*worker_id),
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
            | Self::ResourceDetached { resource_id, .. }
            | Self::TimerScheduled {
                timer_id: resource_id,
                ..
            }
            | Self::TimerFired {
                timer_id: resource_id,
                ..
            }
            | Self::TimerCancelled {
                timer_id: resource_id,
                ..
            } => Some(*resource_id),
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
            Self::TaskScheduled { .. } => "runtime.task.scheduled",
            Self::TaskRan { .. } => "runtime.task.ran",
            Self::TaskCompleted { .. } => "runtime.task.completed",
            Self::TaskFailed { .. } => "runtime.task.failed",
            Self::MicrotaskScheduled { .. } => "runtime.microtask.scheduled",
            Self::MicrotaskRan { .. } => "runtime.microtask.ran",
            Self::MicrotaskCompleted { .. } => "runtime.microtask.completed",
            Self::MicrotaskFailed { .. } => "runtime.microtask.failed",
            Self::TaskContinued { .. } => "runtime.task.continued",
            Self::MicrotaskContinued { .. } => "runtime.microtask.continued",
            Self::StopReached { .. } => "runtime.stop.reached",
            Self::StopContinued { .. } => "runtime.stop.continued",
            Self::IngressDelivered { .. } => "runtime.ingress.delivered",
            Self::TimeAdvanced { .. } => "runtime.time.advanced",
            Self::TimerScheduled { .. } => "runtime.timer.scheduled",
            Self::TimerFired { .. } => "runtime.timer.fired",
            Self::TimerCancelled { .. } => "runtime.timer.cancelled",
            Self::BindingCalled { .. } => "runtime.binding.called",
            Self::BindingReturned { .. } => "runtime.binding.returned",
            Self::BindingFailed { .. } => "runtime.binding.failed",
            Self::LocalGcStarted { .. } => "runtime.gc.local.started",
            Self::LocalGcStepped { .. } => "runtime.gc.local.stepped",
            Self::LocalGcCompleted { .. } => "runtime.gc.local.completed",
            Self::SharedGcStarted { .. } => "runtime.gc.shared.started",
            Self::SharedGcStepped { .. } => "runtime.gc.shared.stepped",
            Self::SharedGcCompleted { .. } => "runtime.gc.shared.completed",
            Self::HeapLimitReached { .. } => "runtime.heap.limit.reached",
            Self::ResourceAttached { .. } => "runtime.resource.attached",
            Self::ResourceDetached { .. } => "runtime.resource.detached",
            Self::PanicRaised { .. } => "runtime.panic.raised",
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::host::ResourceId;
    use crate::runtime::{RuntimeId, WorkerId};
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
