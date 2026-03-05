use std::sync::Arc;

use destack_workspace::{
    ExecutionMode, RandomMode, RuntimeAccess, RuntimeIdentitySelector, RuntimeOptions,
    RuntimeSelector, RuntimeWorld, TimeMode,
};

use super::tests::TestEngine;
use crate::host::Host;
use crate::platform::{ResourceEntry, ResourceKind};
use crate::runtime::bindings::BindingDescriptor;
use crate::runtime::policy::{
    Effect, Fault, FaultTarget, FaultType, Hook, Policy, Rule, RuleId, Trigger,
};
use crate::runtime::{
    Agent, BUILTIN_AGENT_OWNS_RESOURCE_EDGE_KIND_ID, BindingCallContext, Runtime, World,
    WorldCommand, WorldEdge, WorldEdgeKindDefinition, WorldEntity, WorldEntityKindDefinition,
};

/// Ensures agents created in one shared world preserve identity and world sharing.
#[test]
fn test_agent_from_options_in_world_tracks_identity() {
    // create one shared world and two agents
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let agent_a = Agent::new_in_world(Vec::new(), &options, world.clone())
        .expect("agent should construct in world");
    let agent_b = Agent::new_in_world(Vec::new(), &options, world.clone())
        .expect("agent should construct in world");

    // both agents should keep unique identities in one shared world
    assert_ne!(agent_a.id, agent_b.id);
    assert!(Arc::ptr_eq(&agent_a.world, &world));
    assert!(Arc::ptr_eq(&agent_b.world, &world));
}

/// Ensures shared worlds expose one shared control state across agents.
#[test]
fn test_agent_from_options_in_world_shares_world_commands() {
    // create one shared world with two agents
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let agent_a = Agent::new_in_world(Vec::new(), &options, world.clone())
        .expect("agent should construct in world");
    let agent_b = Agent::new_in_world(Vec::new(), &options, world.clone())
        .expect("agent should construct in world");

    // install one rule through one world command
    agent_a
        .world()
        .install_rule(Rule {
            id: RuleId("test.shared.world.command".to_string()),
            enabled: true,
            when: Some(RuntimeSelector {
                binding: Some("destack.test.shared.world".to_string()),
                ..RuntimeSelector::default()
            }),
            action: Effect::SetAccess {
                access: RuntimeAccess::Deny,
            },
            trigger: None,
        })
        .expect("world command should apply");

    // both agent world handles should read the same policy view
    assert_eq!(agent_a.world().policy().rules.len(), 1);
    assert_eq!(agent_b.world().policy().rules.len(), 1);
    assert_eq!(world.policy().rules.len(), 1);
}

/// Ensures agent state keeps the exact world handle passed at construction.
#[test]
fn test_agent_from_options_in_world_preserves_world_identity() {
    // create one shared world and one agent in that world
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let agent = Agent::new_in_world(Vec::new(), &options, world.clone())
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
    let agent = Agent::new_in_world(Vec::new(), &options, world.clone())
        .expect("agent should construct in world");
    let host = Host::from_runtime_options(&options);
    let descriptor = BindingDescriptor::pure("destack.test.live.policy", "()");

    // baseline policy should allow the call
    let baseline_call_context = BindingCallContext::new(
        &agent,
        agent.event_loop.as_ref(),
        &host,
        agent.bindings.policy_snapshot(),
    );
    let baseline_result = baseline_call_context.on_before_binding(descriptor);
    assert!(baseline_result.is_ok());

    // install one deny rule in the shared world
    world
        .set_policy(Policy {
            rules: vec![Rule {
                id: RuleId("test.runtime.live.policy".to_string()),
                enabled: true,
                when: Some(RuntimeSelector {
                    binding: Some("destack.test.live.policy".to_string()),
                    ..RuntimeSelector::default()
                }),
                action: Effect::SetAccess {
                    access: RuntimeAccess::Deny,
                },
                trigger: None,
            }],
        })
        .expect("policy update should succeed");

    // updated policy should deny the same call without agent refresh
    let refreshed_call_context = BindingCallContext::new(
        &agent,
        agent.event_loop.as_ref(),
        &host,
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
    let agent = Agent::new_in_world(Vec::new(), &options, world.clone())
        .expect("agent should construct in world");
    let host = Host::from_runtime_options(&options);
    let descriptor = BindingDescriptor::pure("destack.test.live.hooks", "()");

    // install one hook-bearing fault rule in the shared world
    world
        .apply(WorldCommand::SetPolicy {
            policy: Policy {
                rules: vec![Rule {
                    id: RuleId("test.runtime.live.hooks".to_string()),
                    enabled: true,
                    when: Some(RuntimeSelector::default()),
                    action: Effect::Fault {
                        fault: Fault {
                            target: FaultTarget::Call {},
                            fault_type: FaultType::Error {
                                code: "EFAULT".to_string(),
                            },
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
            },
        })
        .expect("policy update should succeed");

    // firing the matching hook should enqueue one unapplied policy decision
    let call_context = BindingCallContext::new(
        &agent,
        agent.event_loop.as_ref(),
        &host,
        agent.bindings.policy_snapshot(),
    );
    let hook_result = call_context.on_before_binding(descriptor);
    assert!(hook_result.is_ok());
    assert_eq!(agent.hooks.unapplied_policy_decision_count(), 1);
}

/// Ensures agent selectors match only the targeted agent in one shared world.
#[test]
fn test_agent_world_control_agent_selector() {
    // create two agents attached to one shared world
    let mut options_a = RuntimeOptions::default();
    options_a.primary_agent.name = Some("agent-a".to_string());
    let mut options_b = RuntimeOptions::default();
    options_b.primary_agent.name = Some("agent-b".to_string());
    let world = Arc::new(World::default());
    let mut agent_a = Agent::new_in_world(Vec::new(), &options_a, world.clone())
        .expect("agent should construct in world");
    let mut agent_b = Agent::new_in_world(Vec::new(), &options_b, world.clone())
        .expect("agent should construct in world");
    let host_a = Host::from_runtime_options(&options_a);
    let host_b = Host::from_runtime_options(&options_b);

    // install one scheduler hook rule scoped to agent_a
    world
        .apply(WorldCommand::SetPolicy {
            policy: Policy {
                rules: vec![Rule {
                    id: RuleId("test.runtime.selector.instance".to_string()),
                    enabled: true,
                    when: Some(RuntimeSelector {
                        agent: Some(RuntimeIdentitySelector {
                            name: Some("agent-a".to_string()),
                            labels: None,
                        }),
                        ..RuntimeSelector::default()
                    }),
                    action: Effect::Fault {
                        fault: Fault {
                            target: FaultTarget::Call {},
                            fault_type: FaultType::Error {
                                code: "EFAULT".to_string(),
                            },
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
            },
        })
        .expect("policy update should succeed");

    // apply control updates on both agents
    let mut engine = TestEngine::default();
    let _ = agent_a
        .tick_once(&host_a, &mut engine)
        .expect("tick should refresh policy state");
    let _ = agent_b
        .tick_once(&host_b, &mut engine)
        .expect("tick should refresh policy state");

    // fire the same hook on both agents
    agent_a.hooks.on_scheduler_dequeue();
    agent_b.hooks.on_scheduler_dequeue();

    // only the targeted agent should match the rule
    assert_eq!(agent_a.hooks.unapplied_policy_decision_count(), 1);
    assert_eq!(agent_b.hooks.unapplied_policy_decision_count(), 0);
}

/// Ensures one policy command can install one deny rule.
#[test]
fn test_world_apply_policy_command_updates_rules() {
    // create one agent in one shared world
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let agent = Agent::new_in_world(Vec::new(), &options, world.clone())
        .expect("agent should construct in world");
    let host = Host::from_runtime_options(&options);
    let descriptor = BindingDescriptor::pure("destack.test.program.policy", "()");

    // install one deny rule through one world command
    world
        .apply(WorldCommand::InstallRule {
            rule: Rule {
                id: RuleId("test.runtime.program.policy".to_string()),
                enabled: true,
                when: Some(RuntimeSelector {
                    binding: Some("destack.test.program.policy".to_string()),
                    ..RuntimeSelector::default()
                }),
                action: Effect::SetAccess {
                    access: RuntimeAccess::Deny,
                },
                trigger: None,
            },
        })
        .expect("policy command should apply");

    // the installed rule should deny matching calls
    let call_context = BindingCallContext::new(
        &agent,
        agent.event_loop.as_ref(),
        &host,
        agent.bindings.policy_snapshot(),
    );
    let result = call_context.on_before_binding(descriptor);
    assert!(result.is_err());
}

/// Ensures world topology control supports kind and graph mutation.
#[test]
fn test_world_topology_control_mutates_graph() {
    // create one world
    let world = Arc::new(World::default());

    // define one custom entity and edge kind
    world
        .apply(WorldCommand::DefineEntityKind {
            kind: WorldEntityKindDefinition {
                kind: "app.node".into(),
                labels: std::collections::BTreeMap::new(),
                supported_faults: Default::default(),
            },
        })
        .expect("entity kind should define");
    world
        .apply(WorldCommand::DefineEdgeKind {
            kind: WorldEdgeKindDefinition {
                kind: "app.link".into(),
                labels: std::collections::BTreeMap::new(),
                supported_faults: Default::default(),
            },
        })
        .expect("edge kind should define");

    // insert two entities and one connecting edge
    world
        .apply(WorldCommand::UpsertEntity {
            entity: WorldEntity {
                id: "node-a".into(),
                kind: "app.node".into(),
                labels: std::collections::BTreeMap::new(),
            },
        })
        .expect("first entity should upsert");
    world
        .apply(WorldCommand::UpsertEntity {
            entity: WorldEntity {
                id: "node-b".into(),
                kind: "app.node".into(),
                labels: std::collections::BTreeMap::new(),
            },
        })
        .expect("second entity should upsert");
    world
        .apply(WorldCommand::UpsertEdge {
            edge: WorldEdge {
                id: "link-a-b".into(),
                kind: "app.link".into(),
                from: "node-a".into(),
                to: "node-b".into(),
                labels: std::collections::BTreeMap::new(),
            },
        })
        .expect("edge should upsert");

    // removing one entity should also remove incident edges
    world
        .apply(WorldCommand::RemoveEntity {
            entity_id: "node-a".into(),
        })
        .expect("entity removal should succeed");
    assert!(world.edges().is_empty());
}

/// Ensures one failed topology command does not roll back prior successful commands.
#[test]
fn test_world_topology_command_failure_does_not_revert_prior_commands() {
    // create one world
    let world = Arc::new(World::default());
    let revision_before = world.revision();

    // apply one valid command first
    world
        .apply(WorldCommand::DefineEntityKind {
            kind: WorldEntityKindDefinition {
                kind: "app.atomic.node".into(),
                labels: std::collections::BTreeMap::new(),
                supported_faults: Default::default(),
            },
        })
        .expect("entity kind should define");

    // apply one invalid command next
    let result = world.apply(WorldCommand::UpsertEntity {
        entity: WorldEntity {
            id: "bad-node".into(),
            kind: "app.missing.kind".into(),
            labels: std::collections::BTreeMap::new(),
        },
    });

    // the failing command should not mutate topology, prior command stays committed
    let entity_kinds = world.entity_kinds();
    let entities = world.entities();
    assert!(result.is_err());
    assert!(world.revision() > revision_before);
    assert!(entity_kinds.contains_key("app.atomic.node"));
    assert!(!entities.contains_key("bad-node"));
}

/// Ensures world control exposes CAS revision semantics.
#[test]
fn test_world_control_cas_mismatch_fails() {
    // create one world with default state
    let world = Arc::new(World::default());
    let current_revision = world.revision();

    // apply one command with a stale expected revision
    let result = world.apply_at_revision(
        current_revision.saturating_add(1),
        WorldCommand::InstallRule {
            rule: Rule {
                id: RuleId("test.runtime.policy.cas".to_string()),
                enabled: true,
                when: Some(RuntimeSelector {
                    binding: Some("destack.test.policy.cas".to_string()),
                    ..RuntimeSelector::default()
                }),
                action: Effect::SetAccess {
                    access: RuntimeAccess::Deny,
                },
                trigger: None,
            },
        },
    );

    // stale revisions should fail deterministically
    assert!(result.is_err());
}

/// Ensures resource attach and detach operations synchronize into topology entities and edges.
#[test]
fn test_world_resource_lifecycle_updates_topology() {
    // create one agent and insert one resource
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let agent = Agent::new_in_world(Vec::new(), &options, world.clone())
        .expect("agent should construct in world");
    let resource_id = agent.resources.insert(
        ResourceEntry::new(ResourceKind::Timer).with_label("test-timer"),
        None,
    );

    // verify one resource entity and ownership edge exist in topology
    let entities = world.entities();
    let has_resource_entity = entities.values().any(|entity| {
        entity.kind.as_str() == ResourceKind::Timer.kind_id()
            && entity.labels.get("agent.id") == Some(&agent.id.0.to_string())
            && entity.labels.get("resource.id") == Some(&resource_id.0.to_string())
    });
    let edges = world.edges();
    let has_resource_edge = edges
        .values()
        .any(|edge| edge.kind.as_str() == BUILTIN_AGENT_OWNS_RESOURCE_EDGE_KIND_ID);
    assert!(has_resource_entity);
    assert!(has_resource_edge);

    // remove the resource and verify the resource entity is removed
    let removed = agent.resources.remove(resource_id, None);
    assert!(removed.is_some());
    let entities = world.entities();
    let has_resource_entity = entities.values().any(|entity| {
        entity.kind.as_str() == ResourceKind::Timer.kind_id()
            && entity.labels.get("agent.id") == Some(&agent.id.0.to_string())
            && entity.labels.get("resource.id") == Some(&resource_id.0.to_string())
    });
    assert!(!has_resource_entity);
}

/// Ensures dropping one detached agent deregisters topology identity and runtime ownership.
#[test]
fn test_world_agent_drop_cleans_topology() {
    // create one world and one detached agent
    let world = Arc::new(World::default());
    let options = RuntimeOptions::default();
    let agent = Agent::new_in_world(Vec::new(), &options, world.clone())
        .expect("agent should construct in world");
    let runtime_id = agent.runtime_id;
    let agent_id = agent.id;

    // agent and runtime entities should exist before drop
    let before = world.entities();
    assert!(before.contains_key(format!("agent.{}", agent_id.0).as_str()));
    assert!(before.contains_key(format!("runtime.{}", runtime_id.0).as_str()));

    // dropping the agent should remove agent and runtime entities
    drop(agent);
    let after = world.entities();
    assert!(!after.contains_key(format!("agent.{}", agent_id.0).as_str()));
    assert!(!after.contains_key(format!("runtime.{}", runtime_id.0).as_str()));
}

/// Ensures failed world commands do not append replay events.
#[test]
fn test_world_apply_record_failure_does_not_append_replay_events() {
    // create one record-mode world
    let mut options = RuntimeOptions::default();
    options.execution = ExecutionMode::Record;
    let world = World::new(&options, None).expect("world should construct");

    // apply one successful command first
    world
        .apply(WorldCommand::DefineEntityKind {
            kind: WorldEntityKindDefinition {
                kind: "app.record.atomic.node".into(),
                labels: Default::default(),
                supported_faults: Default::default(),
            },
        })
        .expect("define entity kind should succeed");
    let sequence_after_success = world.replay().log().next_sequence().get();
    assert_eq!(sequence_after_success, 1);

    // apply one failing command after that
    let result = world.apply(WorldCommand::UpsertEntity {
        entity: WorldEntity {
            id: "missing-kind-node".into(),
            kind: "app.record.atomic.missing".into(),
            labels: Default::default(),
        },
    });
    assert!(result.is_err());

    // replay log should not advance after failure
    let next_sequence = world.replay().log().next_sequence().get();
    assert_eq!(next_sequence, sequence_after_success);
}

/// Ensures world revisions advance for runtime and resource lifecycle mutations.
#[test]
fn test_world_revision_advances_for_runtime_lifecycle_mutations() {
    // create one world and one runtime in that world
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let mut runtime = Runtime::from_options_in_world(Vec::new(), &options, world.clone())
        .expect("runtime should construct in world");

    // runtime bootstrap should advance world revision
    let revision_after_runtime = world.revision();
    assert!(revision_after_runtime > 1);

    // spawning one agent should advance revision
    let spawned_agent_id = runtime.spawn_agent().expect("agent spawn should succeed");
    let revision_after_spawn = world.revision();
    assert!(revision_after_spawn > revision_after_runtime);

    // attaching and detaching resources should advance revision
    let agent = runtime
        .agent(spawned_agent_id)
        .expect("spawned agent should exist");
    let resource_id = agent.resources.insert(
        ResourceEntry::new(ResourceKind::Timer).with_label("revision-test"),
        None,
    );
    let revision_after_attach = world.revision();
    assert!(revision_after_attach > revision_after_spawn);

    let removed = agent.resources.remove(resource_id, None);
    assert!(removed.is_some());
    let revision_after_detach = world.revision();
    assert!(revision_after_detach > revision_after_attach);
}

/// Ensures spawned agents inherit world-scoped runtime options.
#[test]
fn test_runtime_spawn_agent_aligns_world_scoped_options() {
    // create one runtime with one shared world
    let options = RuntimeOptions::default();
    let mut runtime = Runtime::from_options(Vec::new(), &options).expect("runtime builds");

    // request one conflicting option set for spawn
    let mut spawn_options = RuntimeOptions::default();
    spawn_options.execution = ExecutionMode::Replay;
    spawn_options.access = RuntimeAccess::Deny;
    spawn_options.world = RuntimeWorld::Simulation;
    spawn_options.random.mode = RandomMode::Host;
    spawn_options.time.mode = TimeMode::Host;

    // spawned agent should keep runtime world-scoped settings
    let spawned_agent_id = runtime
        .spawn_agent_with_options(&spawn_options)
        .expect("spawn should succeed");
    let spawned_agent = runtime
        .agent(spawned_agent_id)
        .expect("spawned agent should exist");
    assert_eq!(spawned_agent.options.execution, options.execution);
    assert_eq!(spawned_agent.options.access, options.access);
    assert_eq!(spawned_agent.options.world, options.world);
    assert_eq!(spawned_agent.options.replay, options.replay);
    assert_eq!(spawned_agent.options.random, options.random);
    assert_eq!(spawned_agent.options.time, options.time);
}

/// Ensures deterministic worlds reject secure randomness bindings by default.
#[test]
fn test_world_deterministic_mode_rejects_secure_randomness() {
    // construct one deterministic-random world
    let mut options = RuntimeOptions::default();
    options.random.mode = RandomMode::Deterministic;
    let world = World::new(&options, None).expect("world should construct");

    // secure host randomness should fail in deterministic mode
    let mut bytes = [0u8; 16];
    assert!(world.fill_secure_bytes(&mut bytes).is_err());
    assert!(world.try_fill_secure_bytes(&mut bytes).is_err());
}

/// Ensures capability profiles configure binding policy capability enforcement.
#[test]
fn test_agent_capability_profile_configures_binding_policy() {
    let mut options = RuntimeOptions::default();
    options.security.capability_profile = Some("fs.read,net.connect".to_string());

    let agent = Agent::new(Vec::new(), &options).expect("agent should construct");
    let policy = agent.bindings.policy_snapshot();

    assert!(policy.is_capability_requirements_enforced());
    assert!(policy.capabilities().contains_name("fs.read"));
    assert!(policy.capabilities().contains_name("net.connect"));
    assert_eq!(policy.capabilities().len(), 2);
}
