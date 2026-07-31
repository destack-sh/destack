use destack_program::{CoroutineKind, FramePoint, Task, TaskOutcome, TypeId, Waiter, Word};

use super::{RuntimeCall, TestMachine, TestProgram};
use crate::ErrorReason;

/// Start one task eagerly and settle its result before returning to the caller.
#[test]
fn test_execute_eager_task() {
    let program = TestProgram::words()
        .signature(0, [0], 0)
        .signature(1, [0], 0)
        .coroutine(0, CoroutineKind::ASYNC);
    let mut machine = TestMachine::parse(
        r#"
function body {
    return r0
}

function owner {
    continuation.new r1, body, r0
    task.start r2, r1
    return r2
}
"#,
        program,
    );

    let result = machine.complete(1, &[Word::int32(37)]);
    let task = Task::from_bits(result[0].bits());
    let value = machine.value(TypeId(0), [Word::int32(37)]);
    let calls = machine.take_runtime_calls();

    assert_eq!(result, vec![Word::from_bits(task.bits())]);
    assert_eq!(
        calls,
        vec![
            RuntimeCall::Start(task),
            RuntimeCall::IsCancelled(task),
            RuntimeCall::Finish {
                task,
                outcome: TaskOutcome::Completed(value),
            },
        ]
    );
}

/// Suspend an eager task without suspending its caller.
#[test]
fn test_execute_task_await() {
    let suspension = TestProgram::await_site(1, 0, 1, 2, 3, 0, 0);
    let program = TestProgram::words()
        .signature(0, [0, 0], 1)
        .signature(1, [0], 0)
        .signature(2, [0], 0)
        .coroutine(1, CoroutineKind::ASYNC)
        .frame(1, 0, [])
        .suspensions([suspension]);
    let mut machine = TestMachine::parse(
        r#"
function park {
    waiter.queue r2, r1, r0, t0
    return
}

function body {
    await r1, park, r0 => b0 | b1 | b2

b0:
    return r1

b1:
    return

b2:
    unwind.resume
}

function owner {
    continuation.new r1, body, r0
    task.start r2, r1
    return r2
}
"#,
        program,
    );

    let result = machine.complete(2, &[Word::int32(41)]);
    let task = Task::from_bits(result[0].bits());
    let waiter = Waiter::new(0, 1);
    let calls = machine.take_runtime_calls();

    assert_eq!(result, vec![Word::from_bits(task.bits())]);
    assert_eq!(calls.len(), 4);
    assert_eq!(calls[0], RuntimeCall::Start(task));
    assert_eq!(calls[1], RuntimeCall::IsCancelled(task));
    let RuntimeCall::Suspend {
        task: suspended_task,
        waiter: suspended_waiter,
        continuation,
    } = &calls[2]
    else {
        panic!("task should suspend through the runtime boundary");
    };
    assert_eq!((*suspended_task, *suspended_waiter), (task, waiter));
    let state = continuation
        .innermost()
        .expect("suspended task should retain one frame");
    assert_eq!(
        machine.program().frame_point(state.state()),
        Some(FramePoint::operation(TestProgram::point(1, 0)))
    );
    assert_eq!(
        calls[3],
        RuntimeCall::Queue {
            waiter,
            value: machine.value(TypeId(0), [Word::int32(41)]),
        }
    );
}

/// Route task result, cancellation, parking, and detachment through the runtime ABI.
#[test]
fn test_execute_task_operations() {
    let mut machine = TestMachine::parse(
        r#"
function operations {
    task.resolve r2, r0, t0
    task.park r2, r1
    task.resolve r3, r0, t0
    task.cancel r3
    task.detach r3
    return
}
"#,
        TestProgram::words().signature(0, [0, 0], 1),
    );
    let waiter = Waiter::new(5, 2);

    machine.complete(0, &[Word::int32(43), Word::uint64(waiter.bits())]);
    let parked = Task::new(0, 1);
    let detached = Task::new(1, 1);
    let first = machine.value(TypeId(0), [Word::int32(43)]);
    let second = machine.value(TypeId(0), [Word::int32(43)]);
    let calls = machine.take_runtime_calls();

    assert_eq!(
        calls,
        vec![
            RuntimeCall::Resolve {
                task: parked,
                value: first,
            },
            RuntimeCall::Park {
                task: parked,
                waiter,
            },
            RuntimeCall::Resolve {
                task: detached,
                value: second,
            },
            RuntimeCall::CancelTask(detached),
            RuntimeCall::Detach(detached),
        ]
    );
}

/// Cancel one eager task before propagating its panic through the caller.
#[test]
fn test_cancel_task_on_panic() {
    let program = TestProgram::words()
        .signature(0, [], 0)
        .signature(1, [], 0)
        .coroutine(0, CoroutineKind::ASYNC);
    let mut machine = TestMachine::parse(
        r#"
function body {
    panic
}

function owner {
    continuation.new r0, body, _
    task.start r1, r0
    return
}
"#,
        program,
    );

    let error = machine
        .run(1, &[], None, None, None)
        .expect_err("task panic should propagate");
    let task = Task::new(0, 1);
    let calls = machine.take_runtime_calls();

    assert!(matches!(error.reason(), ErrorReason::Panic(_)));
    assert_eq!(
        calls,
        vec![
            RuntimeCall::Start(task),
            RuntimeCall::Finish {
                task,
                outcome: TaskOutcome::Cancelled,
            },
        ]
    );
}
