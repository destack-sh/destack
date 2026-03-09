#![allow(clippy::arc_with_non_send_sync)]

use std::sync::Arc;

use destack_base::LocalStringPool;
use destack_mir::NodeTree;
use destack_workspace::{
    ExecutionMode, RandomMode, RuntimeAccess, RuntimeIdentitySelector, RuntimeOptions,
    RuntimeSelector, RuntimeWorld, TimeMode,
};
use {destack_heap as heap, destack_vm as vm};

use super::tests::TestEngine;
use crate::host::Host;
use crate::platform::{ResourceEntry, ResourceId, ResourceKind};
use crate::runtime::bindings::BindingDescriptor;
use crate::runtime::policy::{
    Effect, Fault, FaultTarget, FaultType, Hook, Policy, Rule, RuleId, Trigger,
};
use crate::runtime::poller::{
    PollerEvent, PollerEventFlags, PollerEventMask, PollerEventPayload, PollerEventSource,
    PollerToken,
};
use crate::runtime::scheduler::Runnable;
use crate::runtime::time::WorldInstant;
use crate::runtime::{
    Agent, BindingCallContext, BranchId, ObserveEvent, World, WorldEdge, WorldEdgeKindDefinition,
    WorldEntity, WorldEntityKindDefinition,
};

/// Build one empty VM engine for checkpoint tests.
fn vm_engine() -> vm::Isolate {
    let tree = NodeTree::new();
    let strings = LocalStringPool::new().into_immutable();

    vm::Isolate::build(tree, strings).expect("vm engine should build")
}

/// Allocate one managed heap cell in the primary agent VM engine.
fn allocate_vm_heap_cell(world: &Arc<World>, runtime_id: crate::runtime::RuntimeId) {
    // mutate the primary agent engine directly
    world
        .with_runtime_mut(runtime_id, |runtime| {
            let agent_id = runtime.primary_agent_id();
            let agent = runtime
                .agent_mut(agent_id)
                .expect("runtime should keep its primary agent");
            let engine = &mut *agent.engine as &mut dyn std::any::Any;
            let isolate = engine
                .downcast_mut::<vm::Isolate>()
                .expect("agent should use a vm engine");
            let _ = isolate.allocate_single(&mut agent.heap, heap::Value::int32(7));

            Ok(())
        })
        .expect("vm heap mutation should succeed");
}

/// Return the managed heap cell count for the primary agent VM engine.
fn vm_heap_cell_count(world: &Arc<World>, runtime_id: crate::runtime::RuntimeId) -> usize {
    // inspect the primary agent engine directly
    world
        .with_runtime_mut(runtime_id, |runtime| {
            let agent_id = runtime.primary_agent_id();
            let agent = runtime
                .agent_mut(agent_id)
                .expect("runtime should keep its primary agent");
            let engine = &mut *agent.engine as &mut dyn std::any::Any;
            let isolate = engine
                .downcast_mut::<vm::Isolate>()
                .expect("agent should use a vm engine");
            let _ = isolate;

            Ok(agent.heap.managed().cell_count())
        })
        .expect("vm heap inspection should succeed")
}

/// Ensures new worlds start on one real root branch.
#[test]
fn test_world_starts_on_root_branch() {
    // create one new world
    let world = Arc::new(World::default());

    // the active branch should be the root branch
    assert_eq!(world.branch_id(), BranchId::new(0));
    assert_eq!(world.branch().name, "root");
    assert!(world.branch().labels.is_empty());
    assert_eq!(world.branch_ids(), vec![BranchId::new(0)]);
}

/// Ensures empty worlds can checkpoint, rewind, and fork exactly.
#[test]
fn test_world_checkpoint_and_fork_empty_world() {
    // create one new world
    let world = Arc::new(World::default());

    // capture one empty-world checkpoint
    let checkpoint_id = world
        .checkpoint("steady")
        .expect("checkpoint should succeed");
    let checkpoint = world
        .checkpoint_info(checkpoint_id)
        .expect("checkpoint metadata should exist");
    let checkpoint_revision = world
        .revision_info(checkpoint.revision_id)
        .expect("checkpoint revision should exist");
    assert_eq!(checkpoint_revision.branch_id, BranchId::new(0));
    assert_eq!(checkpoint.name, "steady");

    // rewind should restore the same empty state
    world
        .rewind(checkpoint_id)
        .expect("rewind should restore the checkpoint");

    // forking should create one child world on one child branch
    let child = world
        .fork(checkpoint_id, "child")
        .expect("fork should succeed");
    assert_eq!(child.branch().name, "child");
    assert_eq!(child.branch_ids(), vec![BranchId::new(0), BranchId::new(1)]);
    assert_eq!(world.branch_ids(), vec![BranchId::new(0), BranchId::new(1)]);
}

/// Ensures checkpoints restore VM-backed runtime and heap state exactly.
#[test]
fn test_world_rewind_restores_vm_runtime_state() {
    // build one vm-backed runtime
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let runtime_id = world
        .spawn_runtime(Vec::new(), &options, vm_engine())
        .expect("runtime should spawn in world");

    // create one baseline managed allocation before the checkpoint
    allocate_vm_heap_cell(&world, runtime_id);
    assert_eq!(vm_heap_cell_count(&world, runtime_id), 1);

    // capture one checkpoint at the baseline state
    let checkpoint_id = world
        .checkpoint("vm-steady")
        .expect("checkpoint should succeed");

    // mutate both world topology and vm heap after the checkpoint
    world
        .spawn_runtime(Vec::new(), &options, vm_engine())
        .expect("second runtime should spawn in world");
    allocate_vm_heap_cell(&world, runtime_id);
    assert_eq!(world.runtime_ids().len(), 2);
    assert_eq!(vm_heap_cell_count(&world, runtime_id), 2);

    // rewind back to the captured point
    world
        .rewind(checkpoint_id)
        .expect("rewind should restore the checkpoint");

    // the world and vm heap should both return to the checkpoint state
    assert_eq!(world.runtime_ids(), vec![runtime_id]);
    assert_eq!(vm_heap_cell_count(&world, runtime_id), 1);
}

/// Ensures attached resources remain an explicit checkpoint barrier.
#[test]
fn test_world_checkpoint_rejects_attached_resources() {
    // build one runtime and attach one resource to its primary agent
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let runtime_id = world
        .spawn_runtime(Vec::new(), &options, vm_engine())
        .expect("runtime should spawn in world");

    world
        .with_runtime_mut(runtime_id, |runtime| {
            let agent_id = runtime.primary_agent_id();
            let agent = runtime
                .agent_mut(agent_id)
                .expect("runtime should keep its primary agent");

            let _ = agent
                .resources
                .insert(&world, ResourceEntry::new(ResourceKind::Timer), None);

            Ok(())
        })
        .expect("resource should attach");

    // checkpointing should fail loudly for resources without capture support
    let error = world
        .checkpoint("blocked")
        .expect_err("checkpoint should fail");
    let message = error.to_string();
    assert!(
        message.contains("does not support capture"),
        "unexpected checkpoint error: {message}"
    );
}

/// Ensures high-volume observation stays separate from causal trace.
#[test]
fn test_world_observe_records_control_and_resource_events() {
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let runtime_id = world
        .spawn_runtime(Vec::new(), &options, vm_engine())
        .expect("runtime should spawn in world");

    world
        .with_runtime_mut(runtime_id, |runtime| {
            let agent_id = runtime.primary_agent_id();
            let agent = runtime
                .agent_mut(agent_id)
                .expect("runtime should keep its primary agent");
            let resource_id =
                agent
                    .resources
                    .insert(&world, ResourceEntry::new(ResourceKind::Timer), None);
            let _ = agent.resources.remove(&world, resource_id, None);

            Ok(())
        })
        .expect("resource lifecycle should succeed");

    let records = world.observe().records_after(None);
    assert!(
        records
            .iter()
            .any(|record| matches!(record.event, ObserveEvent::Control { .. })),
        "expected one control observation event"
    );
    assert!(
        records.iter().any(|record| {
            matches!(
                record.event,
                ObserveEvent::Resource {
                    is_attach: true,
                    ..
                }
            )
        }),
        "expected one resource attach observation event"
    );
    assert!(
        records.iter().any(|record| {
            matches!(
                record.event,
                ObserveEvent::Resource {
                    is_attach: false,
                    ..
                }
            )
        }),
        "expected one resource detach observation event"
    );
}

/// Ensures forked worlds restore from the checkpoint and diverge independently.
#[test]
fn test_world_fork_isolates_vm_runtime_state() {
    // build one vm-backed runtime and capture a baseline checkpoint
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let runtime_id = world
        .spawn_runtime(Vec::new(), &options, vm_engine())
        .expect("runtime should spawn in world");

    allocate_vm_heap_cell(&world, runtime_id);
    let checkpoint_id = world
        .checkpoint("baseline")
        .expect("checkpoint should succeed");

    // fork one child world from that baseline
    let child = world
        .fork(checkpoint_id, "child")
        .expect("fork should succeed");

    // parent and child should start from the same captured heap state
    assert_eq!(vm_heap_cell_count(&world, runtime_id), 1);
    assert_eq!(vm_heap_cell_count(&child, runtime_id), 1);

    // mutate parent and child independently after the fork
    allocate_vm_heap_cell(&world, runtime_id);
    allocate_vm_heap_cell(&world, runtime_id);
    allocate_vm_heap_cell(&child, runtime_id);

    // both worlds should diverge without affecting each other
    assert_eq!(vm_heap_cell_count(&world, runtime_id), 3);
    assert_eq!(vm_heap_cell_count(&child, runtime_id), 2);
}

/// Ensures serialized world snapshots preserve lineage metadata and restore state.
#[test]
fn test_world_snapshot_roundtrip_restores_lineage_and_state() {
    // build one vm-backed runtime and capture one checkpoint
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let runtime_id = world
        .spawn_runtime(Vec::new(), &options, vm_engine())
        .expect("runtime should spawn in world");

    allocate_vm_heap_cell(&world, runtime_id);
    let checkpoint_id = world
        .checkpoint("baseline")
        .expect("checkpoint should succeed");
    let checkpoint = world
        .checkpoint_info(checkpoint_id)
        .expect("checkpoint metadata should exist");
    let revision = world
        .revision_info(checkpoint.revision_id)
        .expect("checkpoint revision should exist");
    let image_id = revision.image_id;

    // encode one serialized snapshot from that checkpoint image
    let snapshot = world.snapshot(image_id).expect("snapshot should build");
    let bytes = snapshot.encode().expect("snapshot should encode");
    let snapshot = crate::runtime::Snapshot::decode(&bytes).expect("snapshot should decode");

    // mutate the world after the snapshot
    allocate_vm_heap_cell(&world, runtime_id);
    world
        .spawn_runtime(Vec::new(), &options, vm_engine())
        .expect("second runtime should spawn in world");
    assert_eq!(world.runtime_ids().len(), 2);
    assert_eq!(vm_heap_cell_count(&world, runtime_id), 2);

    // restore from the serialized snapshot
    world
        .restore_snapshot(&snapshot, None)
        .expect("snapshot should restore");

    // the snapshot should restore both runtime state and lineage metadata
    assert_eq!(world.runtime_ids(), vec![runtime_id]);
    assert_eq!(vm_heap_cell_count(&world, runtime_id), 1);
    assert_eq!(world.checkpoint_ids(), vec![checkpoint_id]);
    assert_eq!(world.revision_id(), checkpoint.revision_id);

    // rebuilding one fresh world from the same snapshot should preserve the same lineage
    let restored_world =
        World::from_snapshot(&snapshot, None).expect("snapshot should rebuild world");
    assert_eq!(restored_world.branch_id(), world.branch_id());
    assert_eq!(restored_world.runtime_ids(), vec![runtime_id]);
    assert_eq!(vm_heap_cell_count(&restored_world, runtime_id), 1);
    assert_eq!(restored_world.checkpoint_ids(), vec![checkpoint_id]);
    assert_eq!(restored_world.revision_id(), checkpoint.revision_id);
}

/// Ensures hibernation snapshots preserve pending suspendable runtime state.
#[test]
fn test_world_hibernate_snapshot_roundtrip_preserves_pending_state() {
    // build one runtime with pending ingress but no suspended continuations
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let runtime_id = world
        .spawn_runtime(Vec::new(), &options, vm_engine())
        .expect("runtime should spawn in world");
    world
        .with_runtime_mut(runtime_id, |runtime| {
            let agent_id = runtime.primary_agent_id();
            let agent = runtime
                .agent_mut(agent_id)
                .expect("primary agent should exist");
            agent.event_loop.enqueue_events(vec![PollerEvent {
                resource_id: ResourceId(71),
                source: PollerEventSource::Io,
                mask: PollerEventMask::READABLE,
                flags: PollerEventFlags::NONE,
                token: PollerToken(811),
                payload: PollerEventPayload::Io { data: 3 },
            }]);

            Ok(())
        })
        .expect("event enqueue should succeed");

    let snapshot = world
        .hibernate_snapshot()
        .expect("hibernate snapshot should succeed");

    // rebuilding one world from that snapshot should preserve the queued event
    let restored_world =
        World::from_snapshot(&snapshot, None).expect("snapshot should rebuild world");
    let runtime_id = restored_world.runtime_ids()[0];
    let next = restored_world
        .with_runtime_mut(runtime_id, |runtime| {
            let agent_id = runtime.primary_agent_id();
            let agent = runtime
                .agent_mut(agent_id)
                .expect("primary agent should exist");

            agent.event_loop.next_runnable(0, 0)
        })
        .expect("queued state should be inspectable");

    assert!(matches!(next, Some(Runnable::PollerEvent(_))));
}

/// Ensures worlds track unique runtime identities for explicitly spawned runtimes.
#[test]
fn test_world_spawn_runtime_tracks_identity() {
    // create one shared world and two runtimes
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let runtime_a = world
        .spawn_runtime(Vec::new(), &options, TestEngine::default())
        .expect("runtime should spawn in world");
    let runtime_b = world
        .spawn_runtime(Vec::new(), &options, TestEngine::default())
        .expect("runtime should spawn in world");

    // both runtimes should keep unique identities in one shared world
    assert_ne!(runtime_a, runtime_b);
    assert_eq!(world.runtime_ids().len(), 2);
}

/// Ensures shared worlds expose one shared control state across agents.
#[test]
fn test_world_shared_commands_affect_detached_agents() {
    // create one shared world with two agents
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let _ = Agent::new_in_world(
        Vec::new(),
        &options,
        &world,
        Box::new(TestEngine::default()),
    )
    .expect("agent should construct in world");
    let _ = Agent::new_in_world(
        Vec::new(),
        &options,
        &world,
        Box::new(TestEngine::default()),
    )
    .expect("agent should construct in world");

    // install one rule through one world command
    world
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

    // both agents should observe the same world policy view
    assert_eq!(world.policy().rules.len(), 1);
}

/// Ensures worlds expose runtime identity only for explicitly spawned runtimes.
#[test]
fn test_world_spawn_runtime_registers_identity() {
    // create one shared world and one runtime in that world
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let runtime_id = world
        .spawn_runtime(Vec::new(), &options, TestEngine::default())
        .expect("runtime should spawn in world");

    // one runtime registration should appear in world topology
    assert_eq!(world.runtime_ids(), vec![runtime_id]);
    assert_eq!(world.runtime_ids().len(), 1);
}

/// Ensures live world policy updates affect binding checks for existing agents.
#[test]
fn test_agent_world_control_update_refreshes_policy() {
    // create one agent in one shared world
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let agent = Agent::new_in_world(
        Vec::new(),
        &options,
        &world,
        Box::new(TestEngine::default()),
    )
    .expect("agent should construct in world");
    let host = Host::from_runtime_options(&options, agent.runtime_id);
    let descriptor = BindingDescriptor::pure("destack.test.live.policy", "()");

    // baseline policy should allow the call
    let baseline_call_context =
        BindingCallContext::new(&agent, agent.event_loop.as_ref(), &host, &world);
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
    let refreshed_call_context =
        BindingCallContext::new(&agent, agent.event_loop.as_ref(), &host, &world);
    let refreshed_result = refreshed_call_context.on_before_binding(descriptor);
    assert!(refreshed_result.is_err());
}

/// Ensures live world policy updates affect hook plans for existing agents.
#[test]
fn test_agent_world_control_update_refreshes_hooks() {
    // create one agent in one shared world
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let agent = Agent::new_in_world(
        Vec::new(),
        &options,
        &world,
        Box::new(TestEngine::default()),
    )
    .expect("agent should construct in world");
    let host = Host::from_runtime_options(&options, agent.runtime_id);
    let descriptor = BindingDescriptor::pure("destack.test.live.hooks", "()");

    // install one hook-bearing fault rule in the shared world
    world
        .set_policy(Policy {
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
        })
        .expect("policy update should succeed");

    // firing the matching hook should enqueue one unapplied policy decision
    let call_context = BindingCallContext::new(&agent, agent.event_loop.as_ref(), &host, &world);
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
    let mut agent_a = Agent::new_in_world(
        Vec::new(),
        &options_a,
        &world,
        Box::new(TestEngine::default()),
    )
    .expect("agent should construct in world");
    let mut agent_b = Agent::new_in_world(
        Vec::new(),
        &options_b,
        &world,
        Box::new(TestEngine::default()),
    )
    .expect("agent should construct in world");
    let host_a = Host::from_runtime_options(&options_a, agent_a.runtime_id);
    let host_b = Host::from_runtime_options(&options_b, agent_b.runtime_id);

    // install one scheduler hook rule scoped to agent_a
    world
        .set_policy(Policy {
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
        })
        .expect("policy update should succeed");

    // apply control updates on both agents
    let _ = agent_a
        .tick(&world, &host_a)
        .expect("tick should refresh policy state");
    let _ = agent_b
        .tick(&world, &host_b)
        .expect("tick should refresh policy state");

    // fire the same hook on both agents
    agent_a.hooks.on_scheduler_dequeue(&world);
    agent_b.hooks.on_scheduler_dequeue(&world);

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
    let agent = Agent::new_in_world(
        Vec::new(),
        &options,
        &world,
        Box::new(TestEngine::default()),
    )
    .expect("agent should construct in world");
    let host = Host::from_runtime_options(&options, agent.runtime_id);
    let descriptor = BindingDescriptor::pure("destack.test.program.policy", "()");

    // install one deny rule through one world command
    world
        .install_rule(Rule {
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
        })
        .expect("policy command should apply");

    // the installed rule should deny matching calls
    let call_context = BindingCallContext::new(&agent, agent.event_loop.as_ref(), &host, &world);
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
        .define_entity_kind(WorldEntityKindDefinition {
            kind: "app.node".into(),
            labels: std::collections::BTreeMap::new(),
            supported_faults: Default::default(),
        })
        .expect("entity kind should define");
    world
        .define_edge_kind(WorldEdgeKindDefinition {
            kind: "app.link".into(),
            labels: std::collections::BTreeMap::new(),
            supported_faults: Default::default(),
        })
        .expect("edge kind should define");

    // insert two entities and one connecting edge
    world
        .upsert_entity(WorldEntity {
            id: "node-a".into(),
            kind: "app.node".into(),
            labels: std::collections::BTreeMap::new(),
        })
        .expect("first entity should upsert");
    world
        .upsert_entity(WorldEntity {
            id: "node-b".into(),
            kind: "app.node".into(),
            labels: std::collections::BTreeMap::new(),
        })
        .expect("second entity should upsert");
    world
        .upsert_edge(WorldEdge {
            id: "link-a-b".into(),
            kind: "app.link".into(),
            from: "node-a".into(),
            to: "node-b".into(),
            labels: std::collections::BTreeMap::new(),
        })
        .expect("edge should upsert");

    // removing one entity should also remove incident edges
    world
        .remove_entity("node-a".into())
        .expect("entity removal should succeed");
    assert!(!world.edges().contains_key("link-a-b"));
}

/// Ensures one failed topology command does not roll back prior successful commands.
#[test]
fn test_world_topology_command_failure_does_not_revert_prior_commands() {
    // create one world
    let world = Arc::new(World::default());

    // apply one valid command first
    world
        .define_entity_kind(WorldEntityKindDefinition {
            kind: "app.atomic.node".into(),
            labels: std::collections::BTreeMap::new(),
            supported_faults: Default::default(),
        })
        .expect("entity kind should define");

    // apply one invalid command next
    let result = world.upsert_entity(WorldEntity {
        id: "bad-node".into(),
        kind: "app.missing.kind".into(),
        labels: std::collections::BTreeMap::new(),
    });

    // the failing command should not mutate topology, prior command stays committed
    let entity_kinds = world.entity_kinds();
    let entities = world.entities();
    assert!(result.is_err());
    assert!(entity_kinds.contains_key("app.atomic.node"));
    assert!(!entities.contains_key("bad-node"));
}

/// Ensures resource attach and detach operations synchronize into world topology and resource state.
#[test]
fn test_world_resource_lifecycle_updates_topology() {
    // create one agent and insert one resource
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let agent = Agent::new_in_world(
        Vec::new(),
        &options,
        &world,
        Box::new(TestEngine::default()),
    )
    .expect("agent should construct in world");
    let resource_id = agent.resources.insert(
        &world,
        ResourceEntry::new(ResourceKind::Timer).with_label("test-timer"),
        None,
    );

    // verify world resource payload and topology metadata exist
    let resources = world.resources();
    let world_resource_id = crate::runtime::WorldResourceId::new(agent.id, resource_id);
    let world_resource = resources
        .get(&world_resource_id)
        .expect("resource should exist in world resource state");
    let resource_entity_id = world_resource_id.entity_id();
    let resource_edge_id = world_resource_id.ownership_edge_id();
    let entities = world.entities();
    let edges = world.edges();
    assert_eq!(world_resource.kind.as_str(), ResourceKind::Timer.kind_id());
    assert_eq!(world_resource.label.as_deref(), Some("test-timer"));
    assert!(entities.contains_key(&resource_entity_id));
    assert!(edges.contains_key(&resource_edge_id));

    // remove the resource and verify both payload and topology metadata disappear
    let removed = agent.resources.remove(&world, resource_id, None);
    assert!(removed.is_some());
    let resources = world.resources();
    let entities = world.entities();
    let edges = world.edges();
    assert!(!resources.contains_key(&world_resource_id));
    assert!(!entities.contains_key(&resource_entity_id));
    assert!(!edges.contains_key(&resource_edge_id));
}

/// Ensures explicit world agent removal clears selector metadata and topology ownership.
#[test]
fn test_world_remove_agent_cleans_topology() {
    // create one world and one detached agent
    let world = Arc::new(World::default());
    let options = RuntimeOptions::default();
    let agent = Agent::new_in_world(
        Vec::new(),
        &options,
        &world,
        Box::new(TestEngine::default()),
    )
    .expect("agent should construct in world");
    let agent_id = agent.id;
    let host = Host::from_runtime_options(&options, agent.runtime_id);
    let descriptor = BindingDescriptor::pure("destack.test.removed.agent", "()");

    // binding checks should work before removal
    let before_context = BindingCallContext::new(&agent, agent.event_loop.as_ref(), &host, &world);
    let before_result = before_context.on_before_binding(descriptor);
    assert!(before_result.is_ok());

    // removing the agent should clear its selector metadata
    world
        .remove_agent(agent_id)
        .expect("agent removal should succeed");

    let entities = world.entities();
    assert!(!entities.contains_key(&agent_id.entity_id()));

    let after_context = BindingCallContext::new(&agent, agent.event_loop.as_ref(), &host, &world);
    let after_result = after_context.on_before_binding(descriptor);
    assert!(after_result.is_err());
}

/// Ensures failed world commands do not append replay events.
#[test]
fn test_world_apply_record_failure_does_not_append_replay_events() {
    // create one record-mode world
    let options = RuntimeOptions {
        execution: ExecutionMode::Record,
        ..RuntimeOptions::default()
    };
    let world = World::from_options(&options).expect("world should construct");

    // apply one successful command first
    world
        .define_entity_kind(WorldEntityKindDefinition {
            kind: "app.record.atomic.node".into(),
            labels: Default::default(),
            supported_faults: Default::default(),
        })
        .expect("define entity kind should succeed");
    let sequence_after_success = world.trace().log().next_sequence().get();
    assert_eq!(sequence_after_success, 1);

    // apply one failing command after that
    let result = world.upsert_entity(WorldEntity {
        id: "missing-kind-node".into(),
        kind: "app.record.atomic.missing".into(),
        labels: Default::default(),
    });
    assert!(result.is_err());

    // replay log should not advance after failure
    let next_sequence = world.trace().log().next_sequence().get();
    assert_eq!(next_sequence, sequence_after_success);
}

/// Ensures spawned agents inherit world-scoped runtime options.
#[test]
fn test_runtime_spawn_agent_aligns_world_scoped_options() {
    // create one runtime with one shared world
    let options = RuntimeOptions::default();
    let world = World::from_options(&options).expect("world should construct");
    let runtime_id = world
        .spawn_runtime(Vec::new(), &options, TestEngine::default())
        .expect("runtime builds");

    // request one conflicting option set for spawn
    let spawn_options = RuntimeOptions {
        execution: ExecutionMode::Replay,
        access: RuntimeAccess::Deny,
        world: RuntimeWorld::Simulation,
        random: destack_workspace::RandomOptions {
            mode: RandomMode::Host,
            ..Default::default()
        },
        time: destack_workspace::TimeOptions {
            mode: TimeMode::Host,
            ..Default::default()
        },
        ..RuntimeOptions::default()
    };

    // spawned agent should keep runtime world-scoped settings
    world
        .with_runtime_mut(runtime_id, |runtime| {
            let spawned_agent_id = runtime
                .spawn_agent_with_options(&world, &spawn_options, Box::new(TestEngine::default()))
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

            Ok(())
        })
        .expect("runtime should exist");
}

/// Ensures deterministic worlds reject secure randomness bindings by default.
#[test]
fn test_world_deterministic_mode_rejects_secure_randomness() {
    // construct one deterministic-random world
    let mut options = RuntimeOptions::default();
    options.random.mode = RandomMode::Deterministic;
    let world = World::from_options(&options).expect("world should construct");

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

    let world = World::from_options(&options).expect("world should construct");
    let agent = Agent::new_in_world(
        Vec::new(),
        &options,
        &world,
        Box::new(TestEngine::default()),
    )
    .expect("agent should construct");
    let policy = agent.bindings.policy().read();

    assert!(policy.is_capability_requirements_enforced());
    assert!(policy.capabilities().contains_name("fs.read"));
    assert!(policy.capabilities().contains_name("net.connect"));
    assert_eq!(policy.capabilities().len(), 2);
}

/// Ensures simulation deadlines publish the earliest scheduled event.
#[test]
fn test_world_next_simulation_deadline_returns_earliest_deadline() {
    // create one world and schedule several simulated events
    let world = World::default();
    {
        let mut simulation = world.write_simulation();
        simulation.schedule_event(WorldInstant::new(9_000));
        simulation.schedule_event(WorldInstant::new(5_000));
        simulation.schedule_event(WorldInstant::new(7_000));
        simulation.schedule_event(WorldInstant::new(6_000));
    }

    // the world should expose the earliest published deadline
    assert_eq!(
        world.next_simulation_deadline(),
        Some(WorldInstant::new(5_000))
    );
}
