use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::WorkerId;
use crate::runtime::bindings::{BindingDescriptor, BindingEngine, BindingReplayPayload};
use crate::runtime::policy::{BindingDispatchDecision, HookEvent, PolicyDecision, RuleSubject};
use destack_workspace::{ExecutionMode, RuntimeAccess, RuntimeWorld};

use super::{RuntimeId, WorldRef};

impl WorldRef {
    /// Resolve binding dispatch decisions for one binding call.
    pub(crate) fn resolve_binding_dispatch(
        &self,
        mode: ExecutionMode,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        descriptor: BindingDescriptor,
        engine: Option<BindingEngine>,
        default_access: RuntimeAccess,
        default_world: RuntimeWorld,
        default_replay_payload: BindingReplayPayload,
    ) -> RuntimeResult<BindingDispatchDecision> {
        let subject = Self::resolve_rule_subject(self.topology(), runtime_id, worker_id, mode)?;

        // evaluate dispatch decision against active policy
        let decision = self.policy().resolve_binding_dispatch_for_subject(
            subject,
            descriptor,
            engine,
            default_access,
            default_world,
            default_replay_payload,
        );

        Ok(decision)
    }

    /// Evaluate one policy event for one worker under one world lock.
    pub(crate) fn evaluate_policy_event(
        &self,
        mode: ExecutionMode,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        event: &HookEvent,
    ) -> RuntimeResult<Vec<PolicyDecision>> {
        let (runtime_name, runtime_labels, worker_name, worker_labels) = {
            let (runtime_name, runtime_labels) =
                self.topology().runtime_subject(runtime_id).ok_or_else(|| {
                    RuntimeError::TopologyRuntimeMissing {
                        runtime_id: runtime_id.0,
                    }
                    .boxed()
                })?;
            let (worker_name, worker_labels) =
                self.topology().worker_subject(worker_id).ok_or_else(|| {
                    RuntimeError::TopologyWorkerMissing {
                        worker_id: worker_id.0,
                    }
                    .boxed()
                })?;

            (
                runtime_name.to_string(),
                runtime_labels.clone(),
                worker_name.to_string(),
                worker_labels.clone(),
            )
        };

        // evaluate one policy event with world-randomness context
        let subject = RuleSubject::new(
            &runtime_name,
            &runtime_labels,
            &worker_name,
            &worker_labels,
            mode,
        );
        let decisions = self
            .policy_mut()
            .on_event_for_subject(event, subject, self.random());

        Ok(decisions)
    }

    /// Resolve one rule subject from world topology metadata.
    fn resolve_rule_subject<'a>(
        topology: &'a super::topology::Topology,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        mode: ExecutionMode,
    ) -> RuntimeResult<RuleSubject<'a>> {
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

        Ok(RuleSubject::new(
            runtime_name,
            runtime_labels,
            worker_name,
            worker_labels,
            mode,
        ))
    }
}
