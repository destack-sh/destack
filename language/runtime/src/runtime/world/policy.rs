use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::AgentId;
use crate::runtime::bindings::{BindingDescriptor, BindingEngine, BindingReplayPayload};
use crate::runtime::policy::{BindingDispatchDecision, HookEvent, PolicyDecision, RuleSubject};
use destack_workspace::{ExecutionMode, RuntimeAccess, RuntimeWorld};

use super::{RuntimeId, World};

impl World {
    /// Resolve binding dispatch decisions for one binding call.
    pub(crate) fn resolve_binding_dispatch(
        &self,
        mode: ExecutionMode,
        runtime_id: RuntimeId,
        agent_id: AgentId,
        descriptor: BindingDescriptor,
        engine: Option<BindingEngine>,
        default_access: RuntimeAccess,
        default_world: RuntimeWorld,
        default_replay_payload: BindingReplayPayload,
    ) -> RuntimeResult<BindingDispatchDecision> {
        let topology = self.topology.borrow();
        let subject = Self::resolve_rule_subject(&topology, runtime_id, agent_id, mode)?;

        // evaluate dispatch decision against active policy
        let decision = self.policy.borrow().resolve_binding_dispatch_for_subject(
            subject,
            descriptor,
            engine,
            default_access,
            default_world,
            default_replay_payload,
        );

        Ok(decision)
    }

    /// Evaluate one policy event for one agent under one world lock.
    pub(crate) fn evaluate_policy_event(
        &self,
        mode: ExecutionMode,
        runtime_id: RuntimeId,
        agent_id: AgentId,
        event: &HookEvent,
    ) -> RuntimeResult<Vec<PolicyDecision>> {
        let topology = self.topology.borrow();
        let subject = Self::resolve_rule_subject(&topology, runtime_id, agent_id, mode)?;

        // evaluate one policy event with world-randomness context
        let mut policy = self.policy.borrow_mut();
        let decisions = policy.on_event_for_subject(event, subject, &self.random);

        Ok(decisions)
    }

    /// Resolve one rule subject from world topology metadata.
    fn resolve_rule_subject<'a>(
        topology: &'a super::topology::Topology,
        runtime_id: RuntimeId,
        agent_id: AgentId,
        mode: ExecutionMode,
    ) -> RuntimeResult<RuleSubject<'a>> {
        let (runtime_name, runtime_labels) =
            topology.runtime_subject(runtime_id).ok_or_else(|| {
                RuntimeError::TopologyRuntimeMissing {
                    runtime_id: runtime_id.0,
                }
                .boxed()
            })?;
        let (agent_name, agent_labels) = topology.agent_subject(agent_id).ok_or_else(|| {
            RuntimeError::TopologyAgentMissing {
                agent_id: agent_id.0,
            }
            .boxed()
        })?;

        Ok(RuleSubject::new(
            runtime_name,
            runtime_labels,
            agent_name,
            agent_labels,
            mode,
        ))
    }
}
