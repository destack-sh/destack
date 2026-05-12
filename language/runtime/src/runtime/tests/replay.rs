use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::diagnostic::RuntimeError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{
    PlatformError, ResourceBacking, ResourceCapture, ResourceId, ResourcePortability,
};
use crate::runtime::WorkerId;
use crate::runtime::binding::{
    BindingAffinity, BindingDescriptor, BindingEffect, BindingId, BindingProvider,
    BindingReplayKind, BindingReplayPayload, RuntimeAccess,
};
use crate::runtime::engine::Entry;
use crate::runtime::policy::{Rule, RuleAction, RuleId, RuntimeSelector};
use crate::runtime::random::RandomStreamId;
use crate::runtime::time::Instant;
use crate::runtime::trace::{
    EntropyKind, EntropySubject, EnvironmentConfig, Trace, TraceError, TraceHeader,
};
use crate::runtime::world::{
    Command, Entity, EntityDefinition, EntityKind, RuntimeId, WorldResource, WorldResourceId,
};
use destack_vm as vm;
use destack_workspace::config::ExecutionMode;
use serde::{Deserialize, Serialize};

/// Build one replay entropy subject for tests.
fn test_entropy_subject(binding_name: &'static str) -> EntropySubject {
    EntropySubject {
        runtime_id: RuntimeId(1),
        worker_id: WorkerId(1),
        binding_id: BindingId::from_name(binding_name),
        engine: None,
        task_id: None,
        microtask_id: None,
    }
}

/// Replay payload fixture for generic binding-call replay tests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct BindingErrorReplayPayload {
    /// Replayed result payload.
    result: Result<u64, TraceError>,
}

/// Build one explicit trace header for replay tests.
fn test_trace_header() -> TraceHeader {
    TraceHeader::new(EnvironmentConfig::default())
}

/// Recordable binding calls replay in order.
#[test]
fn test_record_replay_binding_call() {
    // setup a recordable binding descriptor
    let descriptor = BindingDescriptor::new(
        "destack.test.call",
        "test() -> u64",
        BindingEffect::ExternalRecordable,
        BindingReplayKind::BindingCall,
        BindingReplayPayload::Results,
        &[],
        BindingProvider::Host,
        BindingAffinity::None,
    );

    // record a binding call
    let record_state = Trace::new(ExecutionMode::Record, test_trace_header());
    record_state
        .record_binding_call(descriptor, &[1, 2, 3])
        .expect("record binding call");

    // replay the binding call from the same log
    let replay_state = Trace::from_log(ExecutionMode::Replay, record_state.log().clone());
    let call = replay_state
        .next_binding_call(descriptor)
        .expect("read binding call");

    // verify the payload matches
    assert_eq!(call.payload, vec![1, 2, 3]);
}

/// Binding-call replay preserves runtime error variants.
#[test]
fn test_replay_binding_call_runtime_error_roundtrip() {
    // setup one recordable binding descriptor
    let descriptor = BindingDescriptor::new(
        "destack.test.binding.error",
        "test() -> u64",
        BindingEffect::ExternalRecordable,
        BindingReplayKind::BindingCall,
        BindingReplayPayload::Results,
        &[],
        BindingProvider::Host,
        BindingAffinity::None,
    );

    // record one failing binding call
    let record_state = Trace::new(ExecutionMode::Record, test_trace_header());
    let record_error = record_state
        .run_binding_without_context(
            descriptor,
            BindingReplayPayload::Results,
            || {
                Err(RuntimeError::PolicyViolation {
                    name: "destack.test.binding.error".to_string(),
                }
                .boxed())
            },
            |result| {
                let payload = match result {
                    Ok(value) => BindingErrorReplayPayload { result: Ok(*value) },
                    Err(error) => BindingErrorReplayPayload {
                        result: Err(TraceError::from(error.as_ref())),
                    },
                };
                Ok(Some(payload))
            },
            |payload| match payload.result {
                Ok(value) => Ok(value),
                Err(error) => Err(Box::<RuntimeError>::from(error)),
            },
        )
        .expect_err("record binding call error");

    // replay the same binding call from the recorded log
    let replay_state = Trace::from_log(ExecutionMode::Replay, record_state.log().clone());
    let replay_error = replay_state
        .run_binding_without_context(
            descriptor,
            BindingReplayPayload::Results,
            || Ok(123),
            |_result| unreachable!("replay should not encode payloads"),
            |payload: BindingErrorReplayPayload| match payload.result {
                Ok(value) => Ok(value),
                Err(error) => Err(Box::<RuntimeError>::from(error)),
            },
        )
        .expect_err("replay binding call error");

    // verify replay preserved the runtime variant
    match (record_error.as_ref(), replay_error.as_ref()) {
        (
            RuntimeError::PolicyViolation {
                name: recorded_name,
            },
            RuntimeError::PolicyViolation {
                name: replayed_name,
            },
        ) => assert_eq!(replayed_name, recorded_name),
        _ => panic!("replay did not preserve policy violation variant"),
    }
}

/// Stream allocation replays deterministically.
#[test]
fn test_record_replay_random_stream() {
    // record a stream allocation
    let record_state = Trace::new(ExecutionMode::Record, test_trace_header());
    let subject = test_entropy_subject("destack.test.random.stream");
    let stream_id = record_state
        .run_random_stream(subject, || {}, || Ok(42))
        .expect("record random stream");
    assert_eq!(stream_id, 42);

    // replay the stream allocation
    let replay_state = Trace::from_log(ExecutionMode::Replay, record_state.log().clone());
    let replayed = replay_state
        .run_random_stream(subject, || {}, || Ok(7))
        .expect("replay random stream");

    // verify the replayed value matches the recorded one
    assert_eq!(replayed, 42);
}

/// Time replay executes one replay-side read hook callback.
#[test]
fn test_replay_time_read_executes_replay_hook() {
    // record one wall clock read
    let record_state = Trace::new(ExecutionMode::Record, test_trace_header());
    let subject = test_entropy_subject("destack.test.time.wall");
    let recorded = record_state
        .run_time_read(EntropyKind::TimeReadWall, subject, || {}, || Ok(123))
        .expect("record time read");
    assert_eq!(recorded, 123);

    // replay the wall clock read and observe one replay-side hook
    let replay_state = Trace::from_log(ExecutionMode::Replay, record_state.log().clone());
    let hook_count = Arc::new(AtomicU64::new(0));
    let hook_count_2 = Arc::clone(&hook_count);
    let replayed = replay_state
        .run_time_read(
            EntropyKind::TimeReadWall,
            subject,
            move || {
                let _ = hook_count_2.fetch_add(1, Ordering::Relaxed);
            },
            || panic!("replay should not execute host time call"),
        )
        .expect("replay time read");

    // verify the replayed sample and replay-hook callback count
    assert_eq!(replayed, 123);
    assert_eq!(hook_count.load(Ordering::Relaxed), 1);
}

/// Random replay executes one replay-side read hook callback.
#[test]
fn test_replay_random_u64_executes_replay_hook() {
    // record one stream-scoped random sample
    let stream_id = RandomStreamId::new(99);
    let record_state = Trace::new(ExecutionMode::Record, test_trace_header());
    let subject = test_entropy_subject("destack.test.random.u64");
    let recorded = record_state
        .run_random_u64(subject, stream_id, || {}, || Ok(777))
        .expect("record random u64");
    assert_eq!(recorded, 777);

    // replay the sample and observe one replay-side hook
    let replay_state = Trace::from_log(ExecutionMode::Replay, record_state.log().clone());
    let hook_count = Arc::new(AtomicU64::new(0));
    let hook_count_2 = Arc::clone(&hook_count);
    let replayed = replay_state
        .run_random_u64(
            subject,
            stream_id,
            move || {
                let _ = hook_count_2.fetch_add(1, Ordering::Relaxed);
            },
            || panic!("replay should not execute host random call"),
        )
        .expect("replay random u64");

    // verify the replayed sample and replay-hook callback count
    assert_eq!(replayed, 777);
    assert_eq!(hook_count.load(Ordering::Relaxed), 1);
}

/// Entropy replay preserves platform errors for deterministic time paths.
#[test]
fn test_replay_entropy_platform_error_roundtrip() {
    // record one time read that fails with one platform error
    let record_state = Trace::new(ExecutionMode::Record, test_trace_header());
    let subject = test_entropy_subject("destack.test.time.wall.error");
    let record_error = record_state
        .run_time_read(
            EntropyKind::TimeReadWall,
            subject,
            || {},
            || Err(RuntimeError::from(PlatformError::time(None, "clock unavailable")).boxed()),
        )
        .expect_err("record time read error");
    let record_platform_error = record_error
        .platform_error()
        .expect("record platform error");
    assert_eq!(record_platform_error.code, PlatformErrorCode::Time);

    // replay the same error from the recorded entropy event
    let replay_state = Trace::from_log(ExecutionMode::Replay, record_state.log().clone());
    let replay_error = replay_state
        .run_time_read(EntropyKind::TimeReadWall, subject, || {}, || Ok(1))
        .expect_err("replay time read error");
    let replay_platform_error = replay_error
        .platform_error()
        .expect("replay platform error");

    // verify replay preserved platform error fields
    assert_eq!(replay_platform_error.code, record_platform_error.code);
    assert_eq!(replay_platform_error.message, record_platform_error.message);
}

/// Entropy replay preserves non-platform runtime error variants.
#[test]
fn test_replay_entropy_runtime_error_roundtrip() {
    // record one random read that fails with one runtime policy error
    let record_state = Trace::new(ExecutionMode::Record, test_trace_header());
    let subject = test_entropy_subject("destack.test.random.u64.error");
    let stream_id = RandomStreamId::new(17);
    let record_error = record_state
        .run_random_u64(
            subject,
            stream_id,
            || {},
            || {
                Err(RuntimeError::PolicyViolation {
                    name: "destack.test.random.u64.error".to_string(),
                }
                .boxed())
            },
        )
        .expect_err("record random error");

    // replay the same error from the recorded entropy event
    let replay_state = Trace::from_log(ExecutionMode::Replay, record_state.log().clone());
    let replay_error = replay_state
        .run_random_u64(subject, stream_id, || {}, || Ok(123))
        .expect_err("replay random error");

    // verify replay preserved the runtime variant
    match (record_error.as_ref(), replay_error.as_ref()) {
        (
            RuntimeError::PolicyViolation {
                name: recorded_name,
            },
            RuntimeError::PolicyViolation {
                name: replayed_name,
            },
        ) => assert_eq!(replayed_name, recorded_name),
        _ => panic!("replay did not preserve policy violation variant"),
    }
}

/// Entropy replay preserves VM error variants for deterministic paths.
#[test]
fn test_replay_entropy_vm_error_roundtrip() {
    // record one random read that fails with one VM panic
    let record_state = Trace::new(ExecutionMode::Record, test_trace_header());
    let subject = test_entropy_subject("destack.test.random.vm.error");
    let stream_id = RandomStreamId::new(23);
    let record_error = record_state
        .run_random_u64(
            subject,
            stream_id,
            || {},
            || {
                Err(RuntimeError::Vm(Box::new(vm::Error::Panic {
                    message: "dst vm panic".to_string(),
                }))
                .boxed())
            },
        )
        .expect_err("record vm error");

    // replay the same vm error from the recorded entropy event
    let replay_state = Trace::from_log(ExecutionMode::Replay, record_state.log().clone());
    let replay_error = replay_state
        .run_random_u64(subject, stream_id, || {}, || Ok(321))
        .expect_err("replay vm error");

    // verify replay preserved the vm error variant and payload
    match (record_error.as_ref(), replay_error.as_ref()) {
        (RuntimeError::Vm(recorded), RuntimeError::Vm(replayed)) => {
            assert_eq!(replayed.as_ref(), recorded.as_ref());
        }
        _ => panic!("replay did not preserve vm error variant"),
    }
}

/// Policy commands replay in the same order they were recorded.
#[test]
fn test_record_replay_policy_command() {
    // build one policy command payload
    let command = Command::InstallRule {
        rule: Rule {
            id: RuleId("test.runtime.policy.command".to_string()),
            enabled: true,
            when: Some(RuntimeSelector {
                binding: Some("destack.test.policy.command".to_string()),
                ..RuntimeSelector::default()
            }),
            action: RuleAction::SetAccess {
                access: RuntimeAccess::Deny,
            },
            trigger: None,
        },
    };

    // record the command to the replay log
    let record_state = Trace::new(ExecutionMode::Record, test_trace_header());
    record_state
        .record_command(command.clone())
        .expect("record world command");

    // replay the command from the same log
    let replay_state = Trace::from_log(ExecutionMode::Replay, record_state.log().clone());
    let replayed = replay_state.next_command().expect("replay command");

    // verify the replayed command matches
    assert_eq!(replayed, command);
}

/// Topology commands replay in the same order they were recorded.
#[test]
fn test_record_replay_topology_command() {
    // build one topology command payload
    let command = Command::DefineEntityKind {
        kind: EntityDefinition {
            kind: "test.entity".into(),
            labels: Default::default(),
            supported_faults: Default::default(),
        },
    };

    // record the command to the replay log
    let record_state = Trace::new(ExecutionMode::Record, test_trace_header());
    record_state
        .record_command(command.clone())
        .expect("record world command");

    // replay the command from the same log
    let replay_state = Trace::from_log(ExecutionMode::Replay, record_state.log().clone());
    let replayed = replay_state.next_command().expect("replay command");

    // verify the replayed command matches
    assert_eq!(replayed, command);
}

/// World commands replay in the same order they were recorded.
#[test]
fn test_record_replay_world_commands() {
    // build two command payloads
    let commands = vec![
        Command::DefineEntityKind {
            kind: EntityDefinition {
                kind: "test.program.entity".into(),
                labels: Default::default(),
                supported_faults: Default::default(),
            },
        },
        Command::UpsertEntity {
            entity: Entity {
                id: "test.program.entity.1".into(),
                kind: "test.program.entity".into(),
                labels: Default::default(),
            },
        },
    ];

    // record the commands to the replay log
    let record_state = Trace::new(ExecutionMode::Record, test_trace_header());
    for command in &commands {
        record_state
            .record_command(command.clone())
            .expect("record world command");
    }

    // replay the commands from the same log
    let replay_state = Trace::from_log(ExecutionMode::Replay, record_state.log().clone());
    let mut replayed = Vec::new();
    for _ in &commands {
        let command = replay_state.next_command().expect("replay command");
        replayed.push(command);
    }

    // verify the replayed commands match
    assert_eq!(replayed, commands);
}

/// Mixed world commands replay in the same order they were recorded.
#[test]
fn test_record_replay_world_commands_mixed() {
    let commands = vec![
        Command::Tick,
        Command::RunEntrypoint {
            runtime_id: RuntimeId(7),
            entry: Entry::new("test.entry"),
            args: vec![destack_vm::Value::Int {
                value: 11,
                width: 32,
            }],
        },
        Command::RemoveRuntime {
            runtime_id: RuntimeId(9),
        },
    ];

    let record_state = Trace::new(ExecutionMode::Record, test_trace_header());
    for command in &commands {
        record_state
            .record_command(command.clone())
            .expect("record world command");
    }

    let replay_state = Trace::from_log(ExecutionMode::Replay, record_state.log().clone());
    let mut replayed = Vec::new();
    for _ in &commands {
        replayed.push(replay_state.next_command().expect("replay command"));
    }

    assert_eq!(replayed, commands);
}

/// Resource lifecycle replays through world commands.
#[test]
fn test_record_replay_resource_command() {
    // build one resource creation command
    let command = Command::CreateResource {
        resource: WorldResource::new(
            WorldResourceId::new(WorkerId(42), ResourceId(7)),
            EntityKind::from("resource.timer"),
            Some("test-timer".to_string()),
            ResourceBacking::Virtual,
            ResourceCapture::State,
            ResourcePortability::Portable,
        ),
    };

    // record the command to the replay log
    let record_state = Trace::new(ExecutionMode::Record, test_trace_header());
    record_state
        .record_command(command.clone())
        .expect("record world command");

    // replay the command from the same log
    let replay_state = Trace::from_log(ExecutionMode::Replay, record_state.log().clone());
    let replayed = replay_state.next_command().expect("replay command");

    // verify the replayed command matches
    assert_eq!(replayed, command);

    // there should be no additional events after replay
    let trailing = replay_state
        .next_event()
        .expect("read trailing replay event");
    assert!(trailing.is_none());
}

/// Runtime tick advances replay in the same order they were recorded.
#[test]
fn test_record_replay_tick() {
    // record one virtual time advance
    let record_state = Trace::new(ExecutionMode::Record, test_trace_header());
    record_state
        .record_time_advance(Instant::new(123_456))
        .expect("record tick");

    // replay the same virtual time advance
    let replay_state = Trace::from_log(ExecutionMode::Replay, record_state.log().clone());
    let replayed = replay_state
        .next_time_advance()
        .expect("replay time advance");

    // verify the replayed tick matches
    assert_eq!(replayed, Instant::new(123_456));
}

/// Replay tick resolution rejects mismatched deadlines.
#[test]
fn test_resolve_tick_rejects_mismatch() {
    // record one virtual time advance
    let record_state = Trace::new(ExecutionMode::Record, test_trace_header());
    record_state
        .record_time_advance(Instant::new(123_456))
        .expect("record tick");

    // replaying with a different deadline must fail loudly
    let replay_state = Trace::from_log(ExecutionMode::Replay, record_state.log().clone());
    let error = replay_state
        .resolve_time_advance(Instant::new(123_457))
        .expect_err("tick mismatch should fail");
    assert!(
        error.message().contains("time"),
        "time mismatch should identify the replay channel"
    );
}

/// Replay rejects ticks that move virtual time backwards.
#[test]
fn test_replay_rejects_backward_tick() {
    // record one forward tick and one backward tick in one log
    let record_state = Trace::new(ExecutionMode::Record, test_trace_header());
    record_state
        .record_time_advance(Instant::new(50))
        .expect("record tick");
    record_state
        .record_time_advance(Instant::new(40))
        .expect("record tick");

    // replay should reject the backward move on the second tick
    let replay_state = Trace::from_log(ExecutionMode::Replay, record_state.log().clone());
    let _ = replay_state
        .next_time_advance()
        .expect("first time advance should replay");
    let error = replay_state
        .next_time_advance()
        .expect_err("backward tick should fail");

    // verify validator reports a tick mismatch
    assert!(
        error.message().contains("time"),
        "backward time advances should fail on the time channel"
    );
}
