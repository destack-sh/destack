use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::binding::BindingDescriptor;
use crate::runtime::WorkerId;
use crate::world::topology::Topology;
use crate::world::{RuntimeId, WorldState};
use destack_workspace::{ConditionSet, ExecutionMode};

use super::{BindingDecision, Subject};

impl WorldState {
    /// Decide one binding call.
    pub(crate) fn decide_binding(
        &self,
        mode: ExecutionMode,
        conditions: &ConditionSet,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        descriptor: BindingDescriptor,
    ) -> RuntimeResult<BindingDecision> {
        let subject =
            Self::policy_subject(&self.topology, runtime_id, worker_id, mode, conditions)?;

        let decision = self.policy.decide_binding(subject, descriptor);

        Ok(decision)
    }

    /// Resolve one subject from world topology metadata.
    pub(crate) fn policy_subject<'a>(
        topology: &'a Topology,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        mode: ExecutionMode,
        conditions: &'a ConditionSet,
    ) -> RuntimeResult<Subject<'a>> {
        let (runtime_name, runtime_labels) = topology
            .runtime_subject(runtime_id)
            .ok_or_else(|| RuntimeError::topology_runtime_missing(runtime_id.0).boxed())?;
        let (worker_name, worker_labels) = topology
            .worker_subject(worker_id)
            .ok_or_else(|| RuntimeError::topology_worker_missing(worker_id.0).boxed())?;

        let subject = Subject::new(
            runtime_name,
            runtime_labels,
            worker_name,
            worker_labels,
            mode,
            conditions,
        );

        Ok(subject)
    }
}
