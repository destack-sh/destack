use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::binding::{BindingDescriptor, BindingEngine};
use crate::runtime::WorkerId;
use crate::world::topology::Topology;
use crate::world::{RuntimeId, WorldState};
use destack_workspace::ExecutionMode;

use super::{BindingDecision, Subject};

impl WorldState {
    /// Decide one binding call.
    pub(crate) fn decide_binding(
        &self,
        mode: ExecutionMode,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        descriptor: BindingDescriptor,
        engine: Option<BindingEngine>,
    ) -> RuntimeResult<BindingDecision> {
        let subject = Self::policy_subject(self.topology(), runtime_id, worker_id, mode)?;

        let decision = self.policy().decide_binding(subject, descriptor, engine);

        Ok(decision)
    }

    /// Resolve one subject from world topology metadata.
    pub(crate) fn policy_subject<'a>(
        topology: &'a Topology,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        mode: ExecutionMode,
    ) -> RuntimeResult<Subject<'a>> {
        let (runtime_name, runtime_labels) =
            topology.runtime_subject(runtime_id).ok_or_else(|| {
                RuntimeError::TopologyRuntimeMissing {
                    runtime_id: runtime_id.0,
                }
                .boxed()
            })?;
        let (worker_name, worker_labels) = topology.worker_subject(worker_id).ok_or_else(|| {
            RuntimeError::TopologyWorkerMissing {
                worker_id: worker_id.0,
            }
            .boxed()
        })?;

        Ok(Subject::new(
            runtime_name,
            runtime_labels,
            worker_name,
            worker_labels,
            mode,
        ))
    }
}
