use std::collections::BTreeMap;

use crate::diagnostic::RuntimeResult;
use crate::runtime::binding::{
    BindingAffinity, BindingDescriptor, BindingEffect, BindingProvider, BindingReplayKind,
    BindingReplayPayload, RuntimeAccess,
};
use crate::runtime::policy::{
    ActivationWindow, CallSelector, EdgeSelector, EntitySelector, Fault, FaultTarget, FaultType,
    Hook, HookEvent, Lifetime, Policy, PolicyCallId, PolicyState, Rule, RuleAction, RuleId,
    Trigger,
};
use crate::runtime::random::Random;
use crate::runtime::world::topology::Topology;
use crate::runtime::{EdgeKind, EntityKind, WorkerId};
use destack_workspace::ExecutionMode;

/// Stable runtime name used by policy tests.
const TEST_RUNTIME_NAME: &str = "test-runtime";
/// Stable worker name used by policy tests.
const TEST_WORKER_NAME: &str = "test-worker";

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
        rules: vec![action_rule("after-call-count", trigger)],
    };
    let random = Random::new(7);
    let mut state = PolicyState::new(policy);

    // first matching call should only advance activation state
    let first_decisions = state.on_event(
        &policy_event(10, Hook::BindingBefore, 0),
        TEST_RUNTIME_NAME,
        &BTreeMap::new(),
        TEST_WORKER_NAME,
        &BTreeMap::new(),
        ExecutionMode::Fast,
        &random,
    );
    assert!(first_decisions.is_empty());

    // second matching call should produce exactly one accepted fire
    let second_decisions = state.on_event(
        &policy_event(10, Hook::BindingBefore, 0),
        TEST_RUNTIME_NAME,
        &BTreeMap::new(),
        TEST_WORKER_NAME,
        &BTreeMap::new(),
        ExecutionMode::Fast,
        &random,
    );
    assert_eq!(second_decisions.len(), 1);
}

/// Ensures call-count activation windows are isolated per worker.
#[test]
fn test_on_event_respects_after_call_count_per_worker() {
    // prepare one rule that activates after two call events per worker
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
        rules: vec![action_rule("after-call-count-per-worker", trigger)],
    };
    let random = Random::new(11);
    let mut state = PolicyState::new(policy);

    // first call for each worker should only advance that worker activation state
    let worker_one_first = state.on_event(
        &policy_event(1, Hook::BindingBefore, 0),
        TEST_RUNTIME_NAME,
        &BTreeMap::new(),
        TEST_WORKER_NAME,
        &BTreeMap::new(),
        ExecutionMode::Fast,
        &random,
    );
    let worker_two_first = state.on_event(
        &policy_event(2, Hook::BindingBefore, 0),
        TEST_RUNTIME_NAME,
        &BTreeMap::new(),
        TEST_WORKER_NAME,
        &BTreeMap::new(),
        ExecutionMode::Fast,
        &random,
    );
    assert!(worker_one_first.is_empty());
    assert!(worker_two_first.is_empty());

    // second call for one worker should activate only that worker rule state
    let worker_two_second = state.on_event(
        &policy_event(2, Hook::BindingBefore, 0),
        TEST_RUNTIME_NAME,
        &BTreeMap::new(),
        TEST_WORKER_NAME,
        &BTreeMap::new(),
        ExecutionMode::Fast,
        &random,
    );
    assert_eq!(worker_two_second.len(), 1);
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
        rules: vec![action_rule("cadence-cooldown", trigger)],
    };
    let random = Random::new(17);
    let mut state = PolicyState::new(policy);

    // evaluate four matching events across one scope
    let first = state.on_event(
        &policy_event(2, Hook::SchedulerDequeue, 0),
        TEST_RUNTIME_NAME,
        &BTreeMap::new(),
        TEST_WORKER_NAME,
        &BTreeMap::new(),
        ExecutionMode::Fast,
        &random,
    );
    let second = state.on_event(
        &policy_event(2, Hook::SchedulerDequeue, 0),
        TEST_RUNTIME_NAME,
        &BTreeMap::new(),
        TEST_WORKER_NAME,
        &BTreeMap::new(),
        ExecutionMode::Fast,
        &random,
    );
    let third = state.on_event(
        &policy_event(2, Hook::SchedulerDequeue, 5),
        TEST_RUNTIME_NAME,
        &BTreeMap::new(),
        TEST_WORKER_NAME,
        &BTreeMap::new(),
        ExecutionMode::Fast,
        &random,
    );
    let fourth = state.on_event(
        &policy_event(2, Hook::SchedulerDequeue, 10),
        TEST_RUNTIME_NAME,
        &BTreeMap::new(),
        TEST_WORKER_NAME,
        &BTreeMap::new(),
        ExecutionMode::Fast,
        &random,
    );

    // cadence and cooldown should allow exactly two accepted firings
    let total_decisions = first.len() + second.len() + third.len() + fourth.len();
    assert_eq!(total_decisions, 2);
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
        rules: vec![action_rule("lifetime", trigger)],
    };
    let random = Random::new(23);
    let mut state = PolicyState::new(policy);

    // evaluate three matching events for one scope
    let first = state.on_event(
        &policy_event(3, Hook::SchedulerDequeue, 0),
        TEST_RUNTIME_NAME,
        &BTreeMap::new(),
        TEST_WORKER_NAME,
        &BTreeMap::new(),
        ExecutionMode::Fast,
        &random,
    );
    let second = state.on_event(
        &policy_event(3, Hook::SchedulerDequeue, 0),
        TEST_RUNTIME_NAME,
        &BTreeMap::new(),
        TEST_WORKER_NAME,
        &BTreeMap::new(),
        ExecutionMode::Fast,
        &random,
    );
    let third = state.on_event(
        &policy_event(3, Hook::SchedulerDequeue, 0),
        TEST_RUNTIME_NAME,
        &BTreeMap::new(),
        TEST_WORKER_NAME,
        &BTreeMap::new(),
        ExecutionMode::Fast,
        &random,
    );

    // only two firings should be accepted before expiration
    let total_decisions = first.len() + second.len() + third.len();
    assert_eq!(total_decisions, 2);
}

/// Ensures policy decisions carry the minimum execution payload.
#[test]
fn test_on_event_emits_policy_decision_payload() {
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
        rules: vec![action_rule("metadata", trigger)],
    };
    let random = Random::new(41);
    let mut state = PolicyState::new(policy);

    // fire one matching call event
    let decisions = state.on_event(
        &policy_event(99, Hook::BindingBefore, 1234),
        TEST_RUNTIME_NAME,
        &BTreeMap::new(),
        TEST_WORKER_NAME,
        &BTreeMap::new(),
        ExecutionMode::Fast,
        &random,
    );

    // the policy decision should carry execution payload
    assert_eq!(decisions.len(), 1);
    let decision = &decisions[0];
    assert_eq!(decision.rule_id.0, "test.active.metadata");
    assert_eq!(decision.hook, Hook::BindingBefore);
    assert_eq!(decision.worker_id, WorkerId(99));
    assert!(decision.call_id.is_some());
    assert!(matches!(decision.action, RuleAction::Fault { .. }));
}

/// Ensures fault rules require explicit triggers.
#[test]
fn test_policy_validate_rejects_fault_rule_without_trigger() {
    // configure one fault rule with no trigger
    let policy = Policy {
        rules: vec![Rule {
            id: RuleId("test.policy.shape.fault_missing_trigger".to_string()),
            enabled: true,
            call: Some(CallSelector::default()),
            action: RuleAction::Fault {
                fault: Fault {
                    target: FaultTarget::Call {},
                    fault_type: FaultType::Error {
                        code: "EFAULT".to_string(),
                    },
                },
            },
            trigger: None,
        }],
    };

    // validation should reject missing trigger shape
    let result = validate_policy(&policy);
    assert!(result.is_err());
}

/// Ensures static binding decision rules reject triggers.
#[test]
fn test_policy_validate_rejects_binding_rule_with_trigger() {
    // configure one binding decision rule with one trigger
    let policy = Policy {
        rules: vec![Rule {
            id: RuleId("test.policy.shape.binding_with_trigger".to_string()),
            enabled: true,
            call: Some(CallSelector::default()),
            action: RuleAction::SetAccess {
                access: RuntimeAccess::Allow,
            },
            trigger: Some(test_trigger()),
        }],
    };

    // validation should reject binding trigger shape
    let result = validate_policy(&policy);
    assert!(result.is_err());
}

/// Ensures binding decision rules require one explicit call selector.
#[test]
fn test_policy_validate_rejects_binding_rule_without_call_selector() {
    // configure one binding decision rule with no call selector
    let policy = Policy {
        rules: vec![Rule {
            id: RuleId("test.policy.shape.binding_missing_call_selector".to_string()),
            enabled: true,
            call: None,
            action: RuleAction::SetAccess {
                access: RuntimeAccess::Allow,
            },
            trigger: None,
        }],
    };

    // validation should reject missing call selector for binding decision rules
    let result = validate_policy(&policy);
    assert!(result.is_err());
}

/// Ensures call-target faults require one explicit call selector.
#[test]
fn test_policy_validate_rejects_call_fault_without_call_selector() {
    // configure one call fault rule with no call selector
    let policy = Policy {
        rules: vec![Rule {
            id: RuleId("test.policy.shape.call_fault_missing_call_selector".to_string()),
            enabled: true,
            call: None,
            action: RuleAction::Fault {
                fault: Fault {
                    target: FaultTarget::Call {},
                    fault_type: FaultType::Error {
                        code: "EFAULT".to_string(),
                    },
                },
            },
            trigger: Some(test_trigger()),
        }],
    };

    // validation should reject missing call selector for call-target faults
    let result = validate_policy(&policy);
    assert!(result.is_err());
}

/// Ensures unknown simulation entity kinds are rejected during policy validation.
#[test]
fn test_policy_validate_rejects_unknown_entity_kind() {
    // configure one rule with one unknown entity kind id
    let policy = Policy {
        rules: vec![Rule {
            id: RuleId("test.policy.kind.unknown".to_string()),
            enabled: true,
            call: None,
            action: RuleAction::Fault {
                fault: Fault {
                    target: FaultTarget::Entity {
                        kind: EntityKind("unknown.kind".to_string()),
                        selector: EntitySelector::Any,
                    },
                    fault_type: FaultType::Drop {},
                },
            },
            trigger: Some(test_trigger()),
        }],
    };

    // validation should reject unknown kind ids
    let result = validate_policy(&policy);
    assert!(result.is_err());
}

/// Ensures registered simulation kinds accept compatible fault classes.
#[test]
fn test_policy_validate_accepts_registered_compatible_fault_kind() {
    // configure one rule with one registered transport-capable entity kind
    let policy = Policy {
        rules: vec![Rule {
            id: RuleId("test.policy.kind.compatible".to_string()),
            enabled: true,
            call: None,
            action: RuleAction::Fault {
                fault: Fault {
                    target: FaultTarget::Entity {
                        kind: EntityKind("host.net.socket".to_string()),
                        selector: EntitySelector::Any,
                    },
                    fault_type: FaultType::Drop {},
                },
            },
            trigger: Some(test_trigger()),
        }],
    };

    // validation should accept compatible kind and fault pairs
    let result = validate_policy(&policy);
    assert!(result.is_ok());
}

/// Ensures registered simulation kinds reject incompatible fault classes.
#[test]
fn test_policy_validate_rejects_registered_incompatible_fault_kind() {
    // configure one rule with one durability-only entity kind and one transport fault
    let policy = Policy {
        rules: vec![Rule {
            id: RuleId("test.policy.kind.incompatible".to_string()),
            enabled: true,
            call: None,
            action: RuleAction::Fault {
                fault: Fault {
                    target: FaultTarget::Entity {
                        kind: EntityKind("host.fs.inode".to_string()),
                        selector: EntitySelector::Any,
                    },
                    fault_type: FaultType::Drop {},
                },
            },
            trigger: Some(test_trigger()),
        }],
    };

    // validation should reject incompatible kind and fault pairs
    let result = validate_policy(&policy);
    assert!(result.is_err());
}

/// Ensures registered simulation edge kinds accept compatible fault classes.
#[test]
fn test_policy_validate_accepts_registered_compatible_fault_edge_kind() {
    // configure one rule with one registered transport-capable edge kind
    let policy = Policy {
        rules: vec![Rule {
            id: RuleId("test.policy.edge.compatible".to_string()),
            enabled: true,
            call: None,
            action: RuleAction::Fault {
                fault: Fault {
                    target: FaultTarget::Edge {
                        kind: EdgeKind("host.net.stream_link".to_string()),
                        selector: EdgeSelector::Any,
                        direction: None,
                    },
                    fault_type: FaultType::Drop {},
                },
            },
            trigger: Some(test_trigger()),
        }],
    };

    // validation should accept compatible edge and fault pairs
    let result = validate_policy(&policy);
    assert!(result.is_ok());
}

/// Ensures registered simulation edge kinds reject incompatible fault classes.
#[test]
fn test_policy_validate_rejects_registered_incompatible_fault_edge_kind() {
    // configure one rule with one durability-only edge kind and one transport fault
    let policy = Policy {
        rules: vec![Rule {
            id: RuleId("test.policy.edge.incompatible".to_string()),
            enabled: true,
            call: None,
            action: RuleAction::Fault {
                fault: Fault {
                    target: FaultTarget::Edge {
                        kind: EdgeKind("host.fs.parent_child".to_string()),
                        selector: EdgeSelector::Any,
                        direction: None,
                    },
                    fault_type: FaultType::Drop {},
                },
            },
            trigger: Some(test_trigger()),
        }],
    };

    // validation should reject incompatible edge and fault pairs
    let result = validate_policy(&policy);
    assert!(result.is_err());
}

/// Build one minimal fault action rule for trigger tests.
fn action_rule(id_suffix: &str, trigger: Trigger) -> Rule {
    Rule {
        id: RuleId(format!("test.active.{id_suffix}")),
        enabled: true,
        call: Some(CallSelector::default()),
        action: RuleAction::Fault {
            fault: Fault {
                target: FaultTarget::Call {},
                fault_type: FaultType::Error {
                    code: "EFAULT".to_string(),
                },
            },
        },
        trigger: Some(trigger),
    }
}

/// Build one default trigger used by fault validation tests.
fn test_trigger() -> Trigger {
    Trigger {
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
    }
}

/// Validate one policy against standard world topology kinds.
fn validate_policy(policy: &Policy) -> RuntimeResult<()> {
    let topology = Topology::new();
    policy.validate_with_kind_catalog(&topology)
}

/// Build one policy event for policy trigger tests.
fn policy_event(worker_id: u64, hook: Hook, virtual_time_ns: u64) -> HookEvent {
    match hook {
        Hook::BindingBefore => HookEvent::BindingBefore {
            worker_id: WorkerId(worker_id),
            call_id: PolicyCallId(1),
            descriptor: BindingDescriptor::new(
                "destack.test",
                "()",
                BindingEffect::Pure,
                BindingReplayKind::BindingCall,
                BindingReplayPayload::Results,
                &[],
                BindingProvider::Runtime,
                BindingAffinity::None,
            ),
            engine: None,
            virtual_time_ns,
        },
        Hook::BindingAfter => HookEvent::BindingAfter {
            worker_id: WorkerId(worker_id),
            call_id: PolicyCallId(1),
            descriptor: BindingDescriptor::new(
                "destack.test",
                "()",
                BindingEffect::Pure,
                BindingReplayKind::BindingCall,
                BindingReplayPayload::Results,
                &[],
                BindingProvider::Runtime,
                BindingAffinity::None,
            ),
            engine: None,
            virtual_time_ns,
        },
        Hook::SchedulerEnqueue => HookEvent::SchedulerEnqueue {
            worker_id: WorkerId(worker_id),
            virtual_time_ns,
        },
        Hook::SchedulerDequeue => HookEvent::SchedulerDequeue {
            worker_id: WorkerId(worker_id),
            virtual_time_ns,
        },
        Hook::SchedulerTimerFire => HookEvent::SchedulerTimerFire {
            worker_id: WorkerId(worker_id),
            virtual_time_ns,
        },
        Hook::IngressEnqueue => HookEvent::IngressEnqueue {
            worker_id: WorkerId(worker_id),
            virtual_time_ns,
        },
        Hook::TimeRead => HookEvent::TimeRead {
            worker_id: WorkerId(worker_id),
            engine: None,
            virtual_time_ns,
        },
        Hook::RandomRead => HookEvent::RandomRead {
            worker_id: WorkerId(worker_id),
            engine: None,
            virtual_time_ns,
        },
        Hook::ResourceAttach => HookEvent::ResourceAttach {
            worker_id: WorkerId(worker_id),
            virtual_time_ns,
        },
        Hook::ResourceDetach => HookEvent::ResourceDetach {
            worker_id: WorkerId(worker_id),
            virtual_time_ns,
        },
    }
}
