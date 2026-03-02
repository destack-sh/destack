use crate::runtime::AgentId;
use crate::runtime::policy::{
    ActivationWindow, Effect, Fault, FaultTarget, FaultType, Hook, HookEvent, HookState, Lifetime,
    Policy, PolicyIdentity, PolicyState, Rule, RuleId, Trigger,
};
use crate::runtime::random::Random;
use destack_workspace::{ExecutionMode, RandomMode, RuntimeSelector};

/// Ensures activation windows based on call counts gate initial firings.
#[test]
fn test_on_event_respects_after_call_count_activation() {
    // prepare one rule that activates after two matching call events
    let trigger = Trigger {
        on: Hook::BindingBefore,
        activation: Some(ActivationWindow::AfterCallCount { call_count: 2 }),
        lifetime: None,
        activation_ppm: None,
        probability_ppm: None,
        max_occurrences: Some(1),
        cooldown_ns: None,
        burst: None,
        interval_hits: None,
        skip_hits: None,
    };
    let policy = Policy {
        rules: vec![effect_rule("after-call-count", trigger)],
    };
    let random = Random::new(7, RandomMode::Deterministic);
    let mut state = PolicyState::new(policy);

    // first matching call should only advance activation state
    state.on_event(
        &hook_event(10, Hook::BindingBefore, true, 0),
        &test_policy_identity(),
        ExecutionMode::Fast,
        &random,
    );
    assert_eq!(state.matched_decisions_seen_totals(), vec![0]);

    // second matching call should produce exactly one accepted fire
    state.on_event(
        &hook_event(10, Hook::BindingBefore, true, 0),
        &test_policy_identity(),
        ExecutionMode::Fast,
        &random,
    );
    assert_eq!(state.matched_decisions_seen_totals(), vec![1]);
}

/// Ensures cadence and cooldown gates jointly shape accepted firings.
#[test]
fn test_on_event_respects_cadence_and_cooldown() {
    // prepare one rule that skips one hit and then fires every two hits
    let trigger = Trigger {
        on: Hook::SchedulerDequeue,
        activation: None,
        lifetime: None,
        activation_ppm: None,
        probability_ppm: None,
        max_occurrences: None,
        cooldown_ns: Some(10),
        burst: None,
        interval_hits: Some(2),
        skip_hits: Some(1),
    };
    let policy = Policy {
        rules: vec![effect_rule("cadence-cooldown", trigger)],
    };
    let random = Random::new(17, RandomMode::Deterministic);
    let mut state = PolicyState::new(policy);

    // evaluate four matching events across one scope
    state.on_event(
        &hook_event(2, Hook::SchedulerDequeue, false, 0),
        &test_policy_identity(),
        ExecutionMode::Fast,
        &random,
    );
    state.on_event(
        &hook_event(2, Hook::SchedulerDequeue, false, 0),
        &test_policy_identity(),
        ExecutionMode::Fast,
        &random,
    );
    state.on_event(
        &hook_event(2, Hook::SchedulerDequeue, false, 5),
        &test_policy_identity(),
        ExecutionMode::Fast,
        &random,
    );
    state.on_event(
        &hook_event(2, Hook::SchedulerDequeue, false, 10),
        &test_policy_identity(),
        ExecutionMode::Fast,
        &random,
    );

    // cadence and cooldown should allow exactly two accepted firings
    assert_eq!(state.matched_decisions_seen_totals(), vec![2]);
}

/// Ensures call-count lifetime expires rules after the configured active budget.
#[test]
fn test_on_event_respects_call_count_lifetime() {
    // prepare one rule with a two-hit active lifetime
    let trigger = Trigger {
        on: Hook::SchedulerDequeue,
        activation: None,
        lifetime: Some(Lifetime::ForCallCount { call_count: 2 }),
        activation_ppm: None,
        probability_ppm: None,
        max_occurrences: None,
        cooldown_ns: None,
        burst: None,
        interval_hits: None,
        skip_hits: None,
    };
    let policy = Policy {
        rules: vec![effect_rule("lifetime", trigger)],
    };
    let random = Random::new(23, RandomMode::Deterministic);
    let mut state = PolicyState::new(policy);

    // evaluate three matching events for one scope
    state.on_event(
        &hook_event(3, Hook::SchedulerDequeue, false, 0),
        &test_policy_identity(),
        ExecutionMode::Fast,
        &random,
    );
    state.on_event(
        &hook_event(3, Hook::SchedulerDequeue, false, 0),
        &test_policy_identity(),
        ExecutionMode::Fast,
        &random,
    );
    state.on_event(
        &hook_event(3, Hook::SchedulerDequeue, false, 0),
        &test_policy_identity(),
        ExecutionMode::Fast,
        &random,
    );

    // only two firings should be accepted before expiration
    assert_eq!(state.matched_decisions_seen_totals(), vec![2]);
}

/// Ensures policy decisions include deterministic metadata for execution wiring.
#[test]
fn test_on_event_emits_policy_decision_metadata() {
    // prepare one always-on rule with deterministic trigger behavior
    let trigger = Trigger {
        on: Hook::BindingBefore,
        activation: None,
        lifetime: None,
        activation_ppm: None,
        probability_ppm: None,
        max_occurrences: Some(1),
        cooldown_ns: None,
        burst: None,
        interval_hits: None,
        skip_hits: None,
    };
    let policy = Policy {
        rules: vec![effect_rule("metadata", trigger)],
    };
    let random = Random::new(41, RandomMode::Deterministic);
    let mut state = PolicyState::new(policy);

    // fire one matching call event
    let decisions = state.on_event(
        &hook_event(99, Hook::BindingBefore, true, 1234),
        &test_policy_identity(),
        ExecutionMode::Fast,
        &random,
    );

    // the policy decision should carry deterministic event metadata
    assert_eq!(decisions.len(), 1);
    let decision = &decisions[0];
    assert_eq!(decision.rule_id.0, "test.active.metadata");
    assert_eq!(decision.hook, Hook::BindingBefore);
    assert_eq!(decision.agent_id, AgentId(99));
    assert_eq!(decision.policy_revision, 1);
    assert_eq!(decision.event_index, 1);
    assert_eq!(decision.virtual_time_ns, 1234);
    assert_eq!(decision.fire_count, 1);
    assert!(matches!(decision.effect, Effect::Fault { .. }));
}

/// Build one minimal fault effect rule for trigger tests.
fn effect_rule(id_suffix: &str, trigger: Trigger) -> Rule {
    Rule {
        id: RuleId(format!("test.active.{id_suffix}")),
        enabled: true,
        when: RuntimeSelector::default(),
        action: Effect::Fault {
            fault: Fault {
                target: FaultTarget::Call {},
                fault_type: FaultType::Drop {},
            },
        },
        trigger: Some(trigger),
    }
}

/// Build one deterministic identity context for policy tests.
fn test_policy_identity() -> PolicyIdentity {
    PolicyIdentity {
        runtime_name: "test-runtime".to_string(),
        runtime_labels: Default::default(),
        agent_name: "test-agent".to_string(),
        agent_labels: Default::default(),
    }
}

/// Build one hook event for policy trigger tests.
fn hook_event(agent_id: u64, hook: Hook, is_call_event: bool, virtual_time_ns: u64) -> HookEvent {
    HookEvent {
        hook,
        agent_id: AgentId(agent_id),
        descriptor: None,
        state: HookState::empty(),
        is_call_event,
        virtual_time_ns,
    }
}
