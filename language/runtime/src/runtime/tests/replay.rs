use crate::platform::ResourceId;
use crate::runtime::AgentId;
use crate::runtime::bindings::{
    BindingDescriptor, BindingReplayKind, BindingReplayPayload, BindingReplayPolicy,
};
use crate::runtime::policy::{Effect, Rule, RuleId};
use crate::runtime::replay::{ReplayController, ReplayHeader};
use crate::runtime::world::{
    WorldCommand, WorldEntity, WorldEntityKind, WorldEntityKindDefinition, WorldResource,
    WorldResourceId,
};
use destack_workspace::{ExecutionMode, RuntimeAccess, RuntimeSelector};

/// Recordable binding calls replay in order.
#[test]
fn test_record_replay_binding_call() {
    // setup a recordable binding descriptor
    let descriptor = BindingDescriptor::external(
        "destack.test.call",
        "test() -> u64",
        BindingReplayPolicy::Recordable,
        BindingReplayKind::BindingCall,
    );

    // record a binding call
    let record_state = ReplayController::new(
        ExecutionMode::Record,
        BindingReplayPayload::Results,
        ReplayHeader::default(),
    );
    record_state
        .record_binding_call(descriptor, &[1, 2, 3])
        .expect("record binding call");

    // replay the binding call from the same log
    let replay_state = ReplayController::from_log(
        ExecutionMode::Replay,
        BindingReplayPayload::Results,
        record_state.log().clone(),
    );
    let call = replay_state
        .next_binding_call(descriptor)
        .expect("read binding call");

    // verify the payload matches
    assert_eq!(call.payload, vec![1, 2, 3]);
}

/// Stream allocation replays deterministically.
#[test]
fn test_record_replay_random_stream() {
    // record a stream allocation
    let record_state = ReplayController::new(
        ExecutionMode::Record,
        BindingReplayPayload::Results,
        ReplayHeader::default(),
    );
    let stream_id = record_state
        .run_random_stream(|| Ok(42))
        .expect("record random stream");
    assert_eq!(stream_id, 42);

    // replay the stream allocation
    let replay_state = ReplayController::from_log(
        ExecutionMode::Replay,
        BindingReplayPayload::Results,
        record_state.log().clone(),
    );
    let replayed = replay_state
        .run_random_stream(|| Ok(7))
        .expect("replay random stream");

    // verify the replayed value matches the recorded one
    assert_eq!(replayed, 42);
}

/// Policy commands replay in the same order they were recorded.
#[test]
fn test_record_replay_policy_command() {
    // build one policy command payload
    let command = WorldCommand::InstallRule {
        rule: Rule {
            id: RuleId("test.runtime.policy.command".to_string()),
            enabled: true,
            when: Some(RuntimeSelector {
                binding: Some("destack.test.policy.command".to_string()),
                ..RuntimeSelector::default()
            }),
            action: Effect::SetAccess {
                access: RuntimeAccess::Deny,
            },
            trigger: None,
        },
    };

    // record the command to the replay log
    let record_state = ReplayController::new(
        ExecutionMode::Record,
        BindingReplayPayload::Results,
        ReplayHeader::default(),
    );
    record_state
        .record_world_command(&command)
        .expect("record world command");

    // replay the command from the same log
    let replay_state = ReplayController::from_log(
        ExecutionMode::Replay,
        BindingReplayPayload::Results,
        record_state.log().clone(),
    );
    let replayed = replay_state
        .next_world_command()
        .expect("replay world command");

    // verify the replayed command matches
    assert_eq!(replayed, command);
}

/// Topology-style world commands replay in the same order they were recorded.
#[test]
fn test_record_replay_topology_world_command() {
    // build one world command payload
    let command = WorldCommand::DefineEntityKind {
        kind: WorldEntityKindDefinition {
            kind: "test.entity".into(),
            labels: Default::default(),
            supported_faults: Default::default(),
        },
    };

    // record the command to the replay log
    let record_state = ReplayController::new(
        ExecutionMode::Record,
        BindingReplayPayload::Results,
        ReplayHeader::default(),
    );
    record_state
        .record_world_command(&command)
        .expect("record world command");

    // replay the command from the same log
    let replay_state = ReplayController::from_log(
        ExecutionMode::Replay,
        BindingReplayPayload::Results,
        record_state.log().clone(),
    );
    let replayed = replay_state
        .next_world_command()
        .expect("replay world command");

    // verify the replayed command matches
    assert_eq!(replayed, command);
}

/// World commands replay in the same order they were recorded.
#[test]
fn test_record_replay_world_commands() {
    // build two world command payloads
    let commands = vec![
        WorldCommand::DefineEntityKind {
            kind: WorldEntityKindDefinition {
                kind: "test.program.entity".into(),
                labels: Default::default(),
                supported_faults: Default::default(),
            },
        },
        WorldCommand::UpsertEntity {
            entity: WorldEntity {
                id: "test.program.entity.1".into(),
                kind: "test.program.entity".into(),
                labels: Default::default(),
            },
        },
    ];

    // record the commands to the replay log
    let record_state = ReplayController::new(
        ExecutionMode::Record,
        BindingReplayPayload::Results,
        ReplayHeader::default(),
    );
    for command in &commands {
        record_state
            .record_world_command(command)
            .expect("record world command");
    }

    // replay the commands from the same log
    let replay_state = ReplayController::from_log(
        ExecutionMode::Replay,
        BindingReplayPayload::Results,
        record_state.log().clone(),
    );
    let mut replayed = Vec::new();
    for _ in &commands {
        let command = replay_state
            .next_world_command()
            .expect("replay world command");
        replayed.push(command);
    }

    // verify the replayed commands match
    assert_eq!(replayed, commands);
}

/// Resource lifecycle replays through world commands.
#[test]
fn test_record_replay_resource_world_command() {
    // build one resource creation command
    let command = WorldCommand::CreateResource {
        resource: WorldResource::new(
            WorldResourceId::new(AgentId(42), ResourceId(7)),
            WorldEntityKind::from("resource.timer"),
            Some("test-timer".to_string()),
        ),
    };

    // record the command to the replay log
    let record_state = ReplayController::new(
        ExecutionMode::Record,
        BindingReplayPayload::Results,
        ReplayHeader::default(),
    );
    record_state
        .record_world_command(&command)
        .expect("record world command");

    // replay the command from the same log
    let replay_state = ReplayController::from_log(
        ExecutionMode::Replay,
        BindingReplayPayload::Results,
        record_state.log().clone(),
    );
    let replayed = replay_state
        .next_world_command()
        .expect("replay world command");

    // verify the replayed command matches
    assert_eq!(replayed, command);

    // there should be no additional events after replay
    let trailing = replay_state
        .next_event()
        .expect("read trailing replay event");
    assert!(trailing.is_none());
}
