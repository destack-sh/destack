use std::sync::Arc;

use destack_workspace::RuntimeOptions;

use super::tests::TestEngine;
use crate::platform::PlatformContext;
use crate::runtime::bindings::BindingDescriptor;
use crate::runtime::policy::{
    Effect, Fault, FaultTarget, FaultType, Hook, Policy, Rule, RuleId, Trigger,
};
use crate::runtime::{Agent, BindingCallContext, HookState, World};
use destack_workspace::{RuntimeAccess, RuntimeIdentitySelector, RuntimeSelector};

/// Ensures agents created in one shared world preserve identity and world sharing.
#[test]
fn test_agent_from_options_in_world_tracks_identity() {
    // create one shared world and two agents
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let agent_a =
        Agent::from_options_in_world(PlatformContext::new(Vec::new()), &options, world.clone())
            .expect("agent should construct in world");
    let agent_b =
        Agent::from_options_in_world(PlatformContext::new(Vec::new()), &options, world.clone())
            .expect("agent should construct in world");

    // both agents should keep unique identities in one shared world
    assert_ne!(agent_a.id, agent_b.id);
    assert!(Arc::ptr_eq(&agent_a.world, &world));
    assert!(Arc::ptr_eq(&agent_b.world, &world));
}

/// Ensures shared worlds expose one shared simulation state across agents.
#[test]
fn test_agent_from_options_in_world_shares_simulation() {
    // create one shared world with two agents
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let agent_a =
        Agent::from_options_in_world(PlatformContext::new(Vec::new()), &options, world.clone())
            .expect("agent should construct in world");
    let agent_b =
        Agent::from_options_in_world(PlatformContext::new(Vec::new()), &options, world.clone())
            .expect("agent should construct in world");

    // write one value through one agent world handle
    agent_a.world().write_simulation().version = 42;

    // both agent world handles should read the same value
    assert_eq!(agent_a.world().simulation().version, 42);
    assert_eq!(agent_b.world().simulation().version, 42);
    assert_eq!(world.simulation().version, 42);
}

/// Ensures agent state keeps the exact world handle passed at construction.
#[test]
fn test_agent_from_options_in_world_preserves_world_identity() {
    // create one shared world and one agent in that world
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let agent =
        Agent::from_options_in_world(PlatformContext::new(Vec::new()), &options, world.clone())
            .expect("agent should construct in world");

    // agent world identity should equal constructor world identity
    assert!(Arc::ptr_eq(&agent.world, &world));
}

/// Ensures live world policy updates affect binding checks for existing agents.
#[test]
fn test_agent_world_control_update_refreshes_policy() {
    // create one agent in one shared world
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let agent =
        Agent::from_options_in_world(PlatformContext::new(Vec::new()), &options, world.clone())
            .expect("agent should construct in world");
    let descriptor = BindingDescriptor::pure("destack.test.live.policy", "()");

    // baseline policy should allow the call
    let baseline_call_context = BindingCallContext::new(
        &agent,
        agent.event_loop.as_ref(),
        agent.bindings.policy_snapshot(),
    );
    let baseline_result = baseline_call_context.on_before_binding(descriptor);
    assert!(baseline_result.is_ok());

    // install one deny rule in the shared world
    world.set_policy(Policy {
        rules: vec![Rule {
            id: RuleId("test.runtime.live.policy".to_string()),
            enabled: true,
            when: RuntimeSelector {
                binding: Some("destack.test.live.policy".to_string()),
                ..RuntimeSelector::default()
            },
            action: Effect::SetAccess {
                access: RuntimeAccess::Deny,
            },
            trigger: None,
        }],
    });

    // updated policy should deny the same call without agent refresh
    let refreshed_call_context = BindingCallContext::new(
        &agent,
        agent.event_loop.as_ref(),
        agent.bindings.policy_snapshot(),
    );
    let refreshed_result = refreshed_call_context.on_before_binding(descriptor);
    assert!(refreshed_result.is_err());
}

/// Ensures live world policy updates affect hook plans for existing agents.
#[test]
fn test_agent_world_control_update_refreshes_hooks() {
    // create one agent in one shared world
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let agent =
        Agent::from_options_in_world(PlatformContext::new(Vec::new()), &options, world.clone())
            .expect("agent should construct in world");
    assert_eq!(agent.hooks.rule_count(), 0);

    // install one hook-bearing fault rule in the shared world
    world.set_policy(Policy {
        rules: vec![Rule {
            id: RuleId("test.runtime.live.hooks".to_string()),
            enabled: true,
            when: RuntimeSelector::default(),
            action: Effect::Fault {
                fault: Fault {
                    target: FaultTarget::Call {},
                    fault_type: FaultType::Drop {},
                },
            },
            trigger: Some(Trigger {
                on: Hook::BindingBefore,
                activation: None,
                lifetime: None,
                activation_ppm: None,
                probability_ppm: None,
                max_occurrences: None,
                cooldown_ns: None,
                burst: None,
                interval_hits: None,
                skip_hits: None,
            }),
        }],
    });

    // hook plan should now contain the installed rule
    assert_eq!(agent.hooks.rule_count(), 1);
}

/// Ensures agent selectors match only the targeted agent in one shared world.
#[test]
fn test_agent_world_control_agent_selector() {
    // create two agents attached to one shared world
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let mut agent_a =
        Agent::from_options_in_world(PlatformContext::new(Vec::new()), &options, world.clone())
            .expect("agent should construct in world");
    let mut agent_b =
        Agent::from_options_in_world(PlatformContext::new(Vec::new()), &options, world.clone())
            .expect("agent should construct in world");

    // install one scheduler hook rule scoped to agent_a
    world.set_policy(Policy {
        rules: vec![Rule {
            id: RuleId("test.runtime.selector.instance".to_string()),
            enabled: true,
            when: RuntimeSelector {
                agent: Some(RuntimeIdentitySelector {
                    name: Some(agent_a.name.clone()),
                    labels: None,
                }),
                ..RuntimeSelector::default()
            },
            action: Effect::Fault {
                fault: Fault {
                    target: FaultTarget::Call {},
                    fault_type: FaultType::Drop {},
                },
            },
            trigger: Some(Trigger {
                on: Hook::SchedulerDequeue,
                activation: None,
                lifetime: None,
                activation_ppm: None,
                probability_ppm: None,
                max_occurrences: None,
                cooldown_ns: None,
                burst: None,
                interval_hits: None,
                skip_hits: None,
            }),
        }],
    });

    // apply control updates on both agents
    let mut engine = TestEngine::default();
    let _ = agent_a
        .tick_once(&mut engine)
        .expect("tick should refresh policy state");
    let _ = agent_b
        .tick_once(&mut engine)
        .expect("tick should refresh policy state");

    // fire the same hook on both agents
    agent_a.hooks.on_scheduler_dequeue(HookState::empty());
    agent_b.hooks.on_scheduler_dequeue(HookState::empty());

    // only the targeted agent should match the rule
    assert_eq!(world.policy_matched_effects_seen(), vec![1]);
}
