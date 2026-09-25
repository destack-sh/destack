use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use tspp_native::abi;
use tspp_program as program;
use tspp_repository::Environment;
use tspp_repository::config::ExecutionMode;
use tspp_vm as vm;

use crate::binding::{Binding, ReplayPayload};
use crate::diagnostic::{BindingError, HostErrorCode, RuntimeError, RuntimeResult};
use crate::host::{HostError, ResourceId};
use crate::machine::{Entry, native};
use crate::runtime::RuntimeId;
use crate::tests::TestProgram;
use crate::worker::{Activation, RunnableScope, WorkerId};
use crate::world::policy::{ActionSelector, Rule};
use crate::world::random::RandomStreamId;
use crate::world::time::Instant;
use crate::world::trace::{EntropySubject, EntrypointCall, TraceHeader, TraceLog, TraceResult};
use crate::world::{Entity, EntityDefinition, EntityKind, Mutation};

/// Build one replay entropy subject for tests.
fn test_entropy_subject(binding_name: &'static str) -> EntropySubject {
    EntropySubject {
        runtime_id: RuntimeId(1),
        worker_id: WorkerId(1),
        binding_id: program::BindingId::from_name(binding_name),
        scope: RunnableScope::empty(),
    }
}

/// Replay payload fixture for generic binding-call replay tests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct BindingErrorReplayPayload {
    /// Replayed result payload.
    result: TraceResult<u64>,
}

/// Build one explicit trace header for replay tests.
fn test_trace_header() -> TraceHeader {
    TraceHeader::new(Environment::default())
}

/// Build one runtime binding for trace tests.
fn test_binding(name: &'static str) -> Binding {
    Binding::new(
        program::BindingId::from_static_name(name),
        ReplayPayload::Results,
        test_binding_call,
    )
}

/// Provide the inert implementation required by one registered binding.
fn test_binding_call(
    _activation: &mut Activation<'_>,
    _memory: program::Memory<'_>,
    _context: program::Context,
    _fiber_id: Option<program::FiberId>,
    _declaration: &program::Binding,
    _arguments: &[program::Word],
    _result: &mut [program::Word],
) -> RuntimeResult<()> {
    Ok(())
}

/// Recordable binding calls replay in order.
#[test]
fn test_record_replay_binding_call() {
    let name = "tspp.test.call";
    let binding = test_binding(name);

    // record a binding call
    let record_state = TraceLog::new(ExecutionMode::Record, test_trace_header());
    record_state
        .record_binding_call(binding, name, &[1, 2, 3])
        .expect("record binding call");

    // replay the binding call from the same log
    let replay_state = TraceLog::from_store(ExecutionMode::Replay, record_state.store().clone());
    let call = replay_state
        .next_binding_call(binding, name)
        .expect("read binding call");

    // verify the encoded bytes match
    assert_eq!(call.bytes, vec![1, 2, 3]);
}

/// Binding-call replay preserves runtime error variants.
#[test]
fn test_replay_binding_call_runtime_error_roundtrip() {
    let name = "tspp.test.binding.error";
    let binding = test_binding(name);

    // record one failing binding call
    let record_state = TraceLog::new(ExecutionMode::Record, test_trace_header());
    let record_error = record_state
        .run_binding_without_context(
            binding,
            name,
            ReplayPayload::Results,
            || Err(RuntimeError::policy_violation("tspp.test.binding.error".to_string()).boxed()),
            |result| {
                let payload = match result {
                    Ok(value) => BindingErrorReplayPayload { result: Ok(*value) },
                    Err(error) => BindingErrorReplayPayload {
                        result: Err(error.clone()),
                    },
                };
                Ok(Some(payload))
            },
            |payload| match payload.result {
                Ok(value) => Ok(value),
                Err(error) => Err(error),
            },
        )
        .expect_err("record binding call error");

    // replay the same binding call from the recorded log
    let replay_state = TraceLog::from_store(ExecutionMode::Replay, record_state.store().clone());
    let replay_error = replay_state
        .run_binding_without_context(
            binding,
            name,
            ReplayPayload::Results,
            || Ok(123),
            |_result| unreachable!("replay should not encode payloads"),
            |payload: BindingErrorReplayPayload| match payload.result {
                Ok(value) => Ok(value),
                Err(error) => Err(error),
            },
        )
        .expect_err("replay binding call error");

    // verify replay preserved the runtime variant
    match (record_error.as_ref(), replay_error.as_ref()) {
        (
            RuntimeError::Binding {
                name: recorded_name,
                reason: BindingError::PolicyViolation,
            },
            RuntimeError::Binding {
                name: replayed_name,
                reason: BindingError::PolicyViolation,
            },
        ) => assert_eq!(replayed_name, recorded_name),
        _ => panic!("replay did not preserve policy violation variant"),
    }
}

/// Stream allocation replays deterministically.
#[test]
fn test_record_replay_random_stream() {
    // record a stream allocation
    let record_state = TraceLog::new(ExecutionMode::Record, test_trace_header());
    let subject = test_entropy_subject("tspp.test.random.stream");
    let stream_id = record_state
        .run_random_stream(subject, || {}, || Ok(42))
        .expect("record random stream");
    assert_eq!(stream_id, 42);

    // replay the stream allocation
    let replay_state = TraceLog::from_store(ExecutionMode::Replay, record_state.store().clone());
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
    let record_state = TraceLog::new(ExecutionMode::Record, test_trace_header());
    let subject = test_entropy_subject("tspp.test.time.wall");
    let recorded = record_state
        .run_time_wall_read(subject, || {}, || Ok(123))
        .expect("record time read");
    assert_eq!(recorded, 123);

    // replay the wall clock read and observe one replay-side hook
    let replay_state = TraceLog::from_store(ExecutionMode::Replay, record_state.store().clone());
    let hook_count = Arc::new(AtomicU64::new(0));
    let hook_count_2 = Arc::clone(&hook_count);
    let replayed = replay_state
        .run_time_wall_read(
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
    let record_state = TraceLog::new(ExecutionMode::Record, test_trace_header());
    let subject = test_entropy_subject("tspp.test.random.u64");
    let recorded = record_state
        .run_random_u64(subject, stream_id, || {}, || Ok(777))
        .expect("record random u64");
    assert_eq!(recorded, 777);

    // replay the sample and observe one replay-side hook
    let replay_state = TraceLog::from_store(ExecutionMode::Replay, record_state.store().clone());
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

/// Entropy replay preserves host errors for deterministic time paths.
#[test]
fn test_replay_entropy_host_error_roundtrip() {
    // record one time read that fails with one host error
    let record_state = TraceLog::new(ExecutionMode::Record, test_trace_header());
    let subject = test_entropy_subject("tspp.test.time.wall.error");
    let record_error = record_state
        .run_time_wall_read(
            subject,
            || {},
            || Err(RuntimeError::from(HostError::time(None, "clock unavailable")).boxed()),
        )
        .expect_err("record time read error");
    let record_host_error = record_error.host_error().expect("record host error");
    assert_eq!(record_host_error.code, HostErrorCode::Time);

    // replay the same error from the recorded trace fact
    let replay_state = TraceLog::from_store(ExecutionMode::Replay, record_state.store().clone());
    let replay_error = replay_state
        .run_time_wall_read(subject, || {}, || Ok(1))
        .expect_err("replay time read error");
    let replay_host_error = replay_error.host_error().expect("replay host error");

    // verify replay preserved host error fields
    assert_eq!(replay_host_error.code, record_host_error.code);
    assert_eq!(replay_host_error.message, record_host_error.message);
}

/// Entropy replay preserves non-platform runtime error variants.
#[test]
fn test_replay_entropy_runtime_error_roundtrip() {
    // record one random read that fails with one runtime policy error
    let record_state = TraceLog::new(ExecutionMode::Record, test_trace_header());
    let subject = test_entropy_subject("tspp.test.random.u64.error");
    let stream_id = RandomStreamId::new(17);
    let record_error = record_state
        .run_random_u64(
            subject,
            stream_id,
            || {},
            || {
                Err(
                    RuntimeError::policy_violation("tspp.test.random.u64.error".to_string())
                        .boxed(),
                )
            },
        )
        .expect_err("record random error");

    // replay the same error from the recorded trace fact
    let replay_state = TraceLog::from_store(ExecutionMode::Replay, record_state.store().clone());
    let replay_error = replay_state
        .run_random_u64(subject, stream_id, || {}, || Ok(123))
        .expect_err("replay random error");

    // verify replay preserved the runtime variant
    match (record_error.as_ref(), replay_error.as_ref()) {
        (
            RuntimeError::Binding {
                name: recorded_name,
                reason: BindingError::PolicyViolation,
            },
            RuntimeError::Binding {
                name: replayed_name,
                reason: BindingError::PolicyViolation,
            },
        ) => assert_eq!(replayed_name, recorded_name),
        _ => panic!("replay did not preserve policy violation variant"),
    }
}

/// Entropy replay preserves VM error variants for deterministic paths.
#[test]
fn test_replay_entropy_vm_error_roundtrip() {
    // record one random read that fails with one VM panic
    let record_state = TraceLog::new(ExecutionMode::Record, test_trace_header());
    let subject = test_entropy_subject("tspp.test.random.vm.error");
    let stream_id = RandomStreamId::new(23);
    let record_error = record_state
        .run_random_u64(
            subject,
            stream_id,
            || {},
            || Err(RuntimeError::Vm(Box::new(vm::Error::panic(vm::Panic::empty()))).boxed()),
        )
        .expect_err("record vm error");

    // replay the same vm error from the recorded trace fact
    let replay_state = TraceLog::from_store(ExecutionMode::Replay, record_state.store().clone());
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

/// Entropy replay preserves native error variants for deterministic paths.
#[test]
fn test_replay_entropy_native_error_roundtrip() {
    // record one random read that fails with one native trap
    let record_state = TraceLog::new(ExecutionMode::Record, test_trace_header());
    let subject = test_entropy_subject("tspp.test.random.native.error");
    let stream_id = RandomStreamId::new(29);
    let error = native::Error::Trapped {
        trap: abi::Trap::Bounds,
    };
    let record_error = record_state
        .run_random_u64(
            subject,
            stream_id,
            || {},
            || Err(RuntimeError::Native(Box::new(error)).boxed()),
        )
        .expect_err("record native error");

    // replay the same native error from the recorded trace entry
    let replay_state = TraceLog::from_store(ExecutionMode::Replay, record_state.store().clone());
    let replay_error = replay_state
        .run_random_u64(subject, stream_id, || {}, || Ok(321))
        .expect_err("replay native error");

    // verify replay preserved the native error variant and payload
    match (record_error.as_ref(), replay_error.as_ref()) {
        (RuntimeError::Native(recorded), RuntimeError::Native(replayed)) => {
            assert_eq!(replayed.as_ref(), recorded.as_ref());
        }
        _ => panic!("replay did not preserve native error variant"),
    }
}

/// World mutations replay in the same order they were recorded.
#[test]
fn test_record_replay_mutations() {
    // build representative mutation payloads
    let mutations = vec![
        Mutation::AddRule {
            rule: Rule::deny(
                "test.runtime.policy.mutation",
                ActionSelector {
                    binding: Some("tspp.test.policy.mutation".to_string()),
                    ..ActionSelector::default()
                },
            ),
        },
        Mutation::DefineEntityKind {
            kind: EntityDefinition {
                kind: "test.program.entity".into(),
                labels: Default::default(),
            },
        },
        Mutation::UpsertEntity {
            entity: Entity::new("test.program.entity.1", "test.program.entity"),
        },
        Mutation::AddResource {
            resource_id: ResourceId::new(WorkerId(42), 7),
            entity: Entity::new("runtime.resource.42.7", EntityKind::from("host.time.timer"))
                .named("test-timer"),
        },
    ];

    // record the mutations to the replay log
    let record_state = TraceLog::new(ExecutionMode::Record, test_trace_header());
    for mutation in &mutations {
        record_state
            .record_mutation(mutation.clone())
            .expect("record world mutation");
    }

    // replay the mutations from the same log
    let replay_state = TraceLog::from_store(ExecutionMode::Replay, record_state.store().clone());
    let mut replayed = Vec::new();
    for _ in &mutations {
        let mutation = replay_state.next_mutation().expect("replay mutation");
        replayed.push(mutation);
    }

    // verify the replayed mutations match
    assert_eq!(replayed, mutations);

    // replay should consume exactly the recorded mutation stream
    let trailing = replay_state
        .next_entry()
        .expect("read trailing replay entry");
    assert!(trailing.is_none());
}

/// Mixed entrypoint and world mutations replay in the same order they were recorded.
#[test]
fn test_record_replay_entrypoint_and_world_mutation() {
    let program = TestProgram::mir(
        r#"
export function entry(v0: int32): void {
entry(v0: int32):
    return
}
"#,
    )
    .build();
    let function = program
        .function_id_by_name("entry")
        .expect("trace test function should exist");
    let parameters = program
        .function_parameters(function)
        .expect("trace test function should have a signature");
    let argument = program
        .value(parameters[0], [program::Word::int32(11)])
        .expect("trace test value should match its Program type");
    let invocation = EntrypointCall {
        runtime_id: RuntimeId(7),
        entry: Entry::new("entry"),
        args: vec![argument],
    };
    let mutation = Mutation::RemoveRuntime {
        runtime_id: RuntimeId(9),
    };

    let record_state = TraceLog::new(ExecutionMode::Record, test_trace_header());
    record_state
        .record_entrypoint(invocation.clone())
        .expect("record entrypoint");
    record_state
        .record_mutation(mutation.clone())
        .expect("record world mutation");

    let replay_state = TraceLog::from_store(ExecutionMode::Replay, record_state.store().clone());
    let replayed_invocation = replay_state.next_entrypoint().expect("replay entrypoint");
    let replayed_mutation = replay_state.next_mutation().expect("replay mutation");

    assert_eq!(replayed_invocation, invocation);
    assert_eq!(replayed_mutation, mutation);
}

/// Runtime time advances replay in the same order they were recorded.
#[test]
fn test_record_replay_time_advance() {
    // record one virtual time advance
    let record_state = TraceLog::new(ExecutionMode::Record, test_trace_header());
    record_state
        .record_time_advance(Instant::new(123_456))
        .expect("record time advance");

    // replay the same virtual time advance
    let replay_state = TraceLog::from_store(ExecutionMode::Replay, record_state.store().clone());
    let replayed = replay_state
        .next_time_advance()
        .expect("replay time advance");

    // verify the replayed time advance matches
    assert_eq!(replayed, Instant::new(123_456));
}

/// Replay time-advance resolution rejects mismatched deadlines.
#[test]
fn test_resolve_time_advance_rejects_mismatch() {
    // record one virtual time advance
    let record_state = TraceLog::new(ExecutionMode::Record, test_trace_header());
    record_state
        .record_time_advance(Instant::new(123_456))
        .expect("record time advance");

    // replaying with a different deadline must fail loudly
    let replay_state = TraceLog::from_store(ExecutionMode::Replay, record_state.store().clone());
    let error = replay_state
        .resolve_time_advance(Instant::new(123_457))
        .expect_err("time advance mismatch should fail");
    assert!(
        error.message().contains("time"),
        "time mismatch should identify the replay channel"
    );
}

/// Replay rejects time advances that move virtual time backwards.
#[test]
fn test_replay_rejects_backward_time_advance() {
    // record one forward and one backward time advance
    let record_state = TraceLog::new(ExecutionMode::Record, test_trace_header());
    record_state
        .record_time_advance(Instant::new(50))
        .expect("record time advance");
    record_state
        .record_time_advance(Instant::new(40))
        .expect("record time advance");

    // replay should reject the backward move on the second advance
    let replay_state = TraceLog::from_store(ExecutionMode::Replay, record_state.store().clone());
    let _ = replay_state
        .next_time_advance()
        .expect("first time advance should replay");
    let error = replay_state
        .next_time_advance()
        .expect_err("backward time advance should fail");

    // verify validator reports a time mismatch
    assert!(
        error.message().contains("time"),
        "backward time advances should fail on the time channel"
    );
}
