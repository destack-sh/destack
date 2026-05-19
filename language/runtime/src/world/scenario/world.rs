use crate::diagnostic::RuntimeResult;
use crate::runtime::WorkerId;
use crate::world::{RuntimeId, WorldState};
use destack_workspace::{ConditionSet, ExecutionMode};

use super::{RuntimeEvent, TriggeredFault};

impl WorldState {
    /// Decide scenario faults for one runtime event.
    pub(crate) fn decide_scenario(
        &mut self,
        mode: ExecutionMode,
        conditions: &ConditionSet,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        event: &RuntimeEvent,
    ) -> RuntimeResult<Vec<TriggeredFault>> {
        let subject =
            Self::policy_subject(&self.topology, runtime_id, worker_id, mode, conditions)?;
        let mut faults = Vec::new();
        for scenario in &mut self.scenarios {
            faults.extend(scenario.decide_faults(event, subject, &self.random));
        }

        Ok(faults)
    }
}
