use crate::diagnostic::RuntimeResult;
use crate::runtime::WorkerId;
use crate::world::{RuntimeId, WorldState};
use destack_workspace::ExecutionMode;

use super::{HookEvent, TriggeredFault};

impl WorldState {
    /// Decide scenario faults for one hook event.
    pub(crate) fn decide_scenario(
        &mut self,
        mode: ExecutionMode,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        event: &HookEvent,
    ) -> RuntimeResult<Vec<TriggeredFault>> {
        let (runtime_name, runtime_labels, worker_name, worker_labels) = {
            let subject = Self::policy_subject(self.topology(), runtime_id, worker_id, mode)?;

            (
                subject.runtime_name.to_string(),
                subject.runtime_labels.clone(),
                subject.worker_name.to_string(),
                subject.worker_labels.clone(),
            )
        };

        let subject = crate::world::policy::Subject::new(
            &runtime_name,
            &runtime_labels,
            &worker_name,
            &worker_labels,
            mode,
        );
        let mut faults = Vec::new();
        for scenario in &mut self.scenarios {
            faults.extend(scenario.decide_faults(event, subject, &self.random));
        }

        Ok(faults)
    }
}
