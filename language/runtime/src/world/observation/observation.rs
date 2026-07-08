use serde::{Deserialize, Serialize};

use crate::host::ResourceId;
use crate::runtime::scheduler::RunnableId;
use crate::runtime::time::Instant;
use crate::runtime::{RuntimeId, WorkerId};
use destack_program as program;

use super::ObservationScope;

/// One emitted observable fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Observation {
    /// One runtime was added to the world.
    RuntimeSpawned {
        /// Created runtime identifier.
        runtime_id: RuntimeId,
        /// Number of workers created with the runtime.
        worker_count: u32,
    },
    /// One runtime was removed from the world.
    RuntimeRemoved {
        /// Removed runtime identifier.
        runtime_id: RuntimeId,
    },
    /// One worker was added to a runtime.
    WorkerSpawned {
        /// Owning runtime identifier.
        runtime_id: RuntimeId,
        /// Created worker identifier.
        worker_id: WorkerId,
    },
    /// One worker was removed from the world.
    WorkerRemoved {
        /// Removed worker identifier.
        worker_id: WorkerId,
    },

    /// One task runnable ran.
    TaskRan {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that ran the task.
        worker_id: WorkerId,
        /// Task runnable identifier.
        task_id: RunnableId,
    },

    /// One microtask runnable ran.
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
    /// One stopped task continued.
    TaskContinued {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that continued the task.
        worker_id: WorkerId,
        /// Continued task runnable identifier.
        task_id: RunnableId,
    },
    /// One stopped microtask continued.
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

    /// Execution reached one runtime stop point.
    StopReached {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that reached the stop.
        worker_id: WorkerId,
        /// Stop reason.
        reason: program::StopReason,
    },
    /// Host or poller ingress delivered work.
    IngressDelivered {
        /// Number of delivered host events.
        host_events: usize,
        /// Number of delivered poller events.
        poller_events: usize,
    },
    /// Runtime-controlled time advanced.
    TimeAdvanced {
        /// New runtime-controlled deadline.
        deadline: Instant,
    },
    /// Shared heap GC made progress.
    SharedGcProgressed {
        /// Runtime that owns the shared heap.
        runtime_id: RuntimeId,
    },
    /// One idle worker safepoint ran.
    SafepointRan {
        /// Runtime that owns the worker.
        runtime_id: RuntimeId,
        /// Worker that ran the safepoint.
        worker_id: WorkerId,
    },
    /// One resource was attached to a worker.
    ResourceAttached {
        /// Worker that owns the resource.
        worker_id: WorkerId,
        /// Attached resource identifier.
        resource_id: ResourceId,
    },
    /// One resource was detached from a worker.
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
            | Self::SharedGcProgressed { runtime_id } => ObservationScope::runtime(*runtime_id),
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
            | Self::SafepointRan {
                runtime_id,
                worker_id,
            } => ObservationScope::worker(Some(*runtime_id), *worker_id),
            Self::WorkerRemoved { worker_id } => ObservationScope::worker(None, *worker_id),
            Self::IngressDelivered { .. } | Self::TimeAdvanced { .. } => ObservationScope::world(),
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
                Self::IngressDelivered { .. } | Self::TimeAdvanced { .. }
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
            ObservationScope::Entity(_) | ObservationScope::Edge(_) => false,
            ObservationScope::Resource(resource_id) => self.resource_id() == Some(**resource_id),
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
            | Self::TaskContinued { runtime_id, .. }
            | Self::MicrotaskContinued { runtime_id, .. }
            | Self::StopReached { runtime_id, .. }
            | Self::SharedGcProgressed { runtime_id }
            | Self::SafepointRan { runtime_id, .. } => Some(*runtime_id),
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
            | Self::TaskContinued { worker_id, .. }
            | Self::MicrotaskContinued { worker_id, .. }
            | Self::StopReached { worker_id, .. }
            | Self::SafepointRan { worker_id, .. }
            | Self::ResourceAttached { worker_id, .. }
            | Self::ResourceDetached { worker_id, .. } => Some(*worker_id),
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
            Self::RuntimeSpawned { .. } => "runtime.instance.spawned",
            Self::RuntimeRemoved { .. } => "runtime.instance.removed",
            Self::WorkerSpawned { .. } => "runtime.worker.spawned",
            Self::WorkerRemoved { .. } => "runtime.worker.removed",
            Self::TaskRan { .. } => "runtime.task.ran",
            Self::MicrotaskRan { .. } => "runtime.microtask.ran",
            Self::TaskContinued { .. } | Self::MicrotaskContinued { .. } => {
                "runtime.stop.continued"
            }
            Self::StopReached { .. } => "runtime.stop.reached",
            Self::IngressDelivered { .. } => "runtime.ingress.delivered",
            Self::TimeAdvanced { .. } => "runtime.time.advanced",
            Self::SharedGcProgressed { .. } => "runtime.gc.shared.progressed",
            Self::SafepointRan { .. } => "runtime.safepoint.ran",
            Self::ResourceAttached { .. } => "runtime.resource.attached",
            Self::ResourceDetached { .. } => "runtime.resource.detached",
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
        let observation = Observation::SafepointRan {
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
