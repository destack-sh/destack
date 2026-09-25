use tspp_program as program;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::RuntimeId;
use crate::worker::WorkerId;
use crate::world::WorldState;
use crate::world::topology::Topology;
use tspp_repository::{ConditionSet, ExecutionMode};

use super::{Decision, Subject};

impl WorldState {
    /// Decide one binding call.
    pub(crate) fn decide_binding(
        &self,
        conditions: &ConditionSet,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        program: &program::Program,
        binding: &program::Binding,
    ) -> RuntimeResult<Decision> {
        let mode = self.trace.mode();
        let subject =
            Self::policy_subject(&self.topology, runtime_id, worker_id, mode, conditions)?;

        self.policy.decide_binding(subject, program, binding)
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
