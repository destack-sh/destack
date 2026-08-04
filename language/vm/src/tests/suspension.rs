use destack_bytecode::{RegisterId, RegisterSpan};
use destack_heap::{HeapReference, Root};
use destack_mir::{ReferenceKind, Space, Storage};
use destack_program::{CoroutineKind, FunctionId, Outcome, TypeId, Waiter, Word};

use super::{RuntimeCall, TestMachine, TestProgram};
use crate::ErrorReason;

/// Await one asynchronous value and resume through its fulfillment edge.
#[test]
fn test_execute_await() {
    let site = TestProgram::await_site(0, 0, 1, 2, 3, 0, 0);
    let program = TestProgram::words()
        .signature(0, [0], 0)
        .signature(1, [], 0)
        .coroutine(0, CoroutineKind::ASYNC)
        .frame(0, 0, [])
        .suspensions([site]);
    let mut machine = TestMachine::parse(
        r#"
function wait {
    await r1, f1, r0 => b0 | b1 | b2

b0:
    return r1

b1:
    return

b2:
    unwind.resume
}

function f1 {
    return
}
"#,
        program,
    );

    // park on the exact asynchronous value
    let (park, awaitable, continuation) = machine.run_to_await(0, &[Word::int32(7)]);
    assert_eq!(park, FunctionId(1));
    assert_eq!(awaitable, vec![Word::int32(7)]);
    assert!(machine.continuation_bytes(&continuation).is_empty());

    // deliver fulfillment into the await result register
    let value = machine.resume_to_completion(continuation, &[Word::int32(11)]);
    assert_eq!(value, vec![Word::int32(11)]);

    // cancellation enters cleanup without manufacturing a result value
    let (_, _, continuation) = machine.run_to_await(0, &[Word::int32(7)]);
    let outcome = machine
        .cancel(continuation)
        .expect("await cancellation should execute");
    assert!(matches!(outcome, Outcome::Cancelled));
}

/// Route waiter settlement and cancellation through the runtime activation.
#[test]
fn test_execute_waiter() {
    let mut machine = TestMachine::parse(
        r#"
function settle {
    waiter.queue r2, r0, r1, t0
    return
}

function cancel {
    waiter.cancel r1, r0
    return
}
"#,
        TestProgram::words()
            .signature(0, [0, 0], 1)
            .signature(1, [0], 1),
    );
    let waiter = Waiter::new(7, 3);

    machine.complete(0, &[Word::uint64(waiter.bits()), Word::int32(11)]);
    let value = machine.value(TypeId(0), [Word::int32(11)]);
    assert_eq!(
        machine.take_runtime_calls(),
        vec![RuntimeCall::Queue { waiter, value }]
    );

    machine.complete(1, &[Word::uint64(waiter.bits())]);
    assert_eq!(
        machine.take_runtime_calls(),
        vec![RuntimeCall::CancelWaiter(waiter)]
    );
}

/// Retain a captured closure environment as one ready continuation root.
#[test]
fn test_trace_ready_continuation_environment() {
    let program = TestProgram::words()
        .signature(0, [], 1)
        .environment(0, 0)
        .coroutine(0, CoroutineKind::GENERATOR)
        .signature(1, [0], 1)
        .reference(0, 1, ReferenceKind::Managed, Storage::Heap(Space::Local));
    let mut machine = TestMachine::parse(
        r#"
function generate {
    return
}

function owner {
    continuation.new r1, generate, r0
    return r1
}
"#,
        program,
    );
    let reference = HeapReference::new(37);

    machine.complete(1, &[Word::from_bits(reference.bits() as u64)]);

    assert_eq!(machine.roots(), vec![Root::HeapReference(reference)]);
}

/// Yield generator values and resume independent continuations.
#[test]
fn test_execute_yield() {
    let site = TestProgram::yield_site(0, 1, 2, 4, 5, 0, 0, 0);
    let program = TestProgram::words()
        .signature(0, [0], 0)
        .coroutine(0, CoroutineKind::GENERATOR)
        .frame(0, 1, [(RegisterSpan::new(RegisterId(1), 1), 0)])
        .suspensions([site]);
    let mut machine = TestMachine::parse(
        r#"
function generate {
    constant r1, 5: int32
    yield r2, r4, r0 => b0 | b1 | b2

b0:
    int.add r3, r1, r2: int32
    return r3

b1:
    return r4

b2:
    unwind.resume
}
"#,
        program,
    );

    // retain the yielded value and two independent generator states
    let (value, first) = machine.run_to_yield(0, &[Word::int32(13)]);
    assert_eq!(value, vec![Word::int32(13)]);
    assert_eq!(
        machine.continuation_bytes(&first),
        Word::int32(5).to_bytes()
    );
    let (_, second) = machine.run_to_yield(0, &[Word::int32(13)]);

    // resume each continuation with its captured value and an independent next value
    let value = machine.resume_to_completion(first, &[Word::int32(17)]);
    assert_eq!(value, vec![Word::int32(22)]);

    let value = machine.resume_to_completion(second, &[Word::int32(19)]);
    assert_eq!(value, vec![Word::int32(24)]);

    // enter the explicit completion edge with its independent result value
    let (_, continuation) = machine.run_to_yield(0, &[Word::int32(13)]);
    let value = machine.complete_to_completion(continuation, &[Word::int32(29)]);
    assert_eq!(value, vec![Word::int32(29)]);
}

/// Complete one ready generator without entering its body.
#[test]
fn test_complete_ready_continuation() {
    let site = TestProgram::continuation_site(2, 1, 2, 4, Some(8));
    let program = TestProgram::words()
        .signature(1, [0], 0)
        .signature(2, [0, 0], 0)
        .coroutine(1, CoroutineKind::GENERATOR)
        .frame(2, 1, [])
        .continuations([site])
        .local_global()
        .destructor(0, Storage::Frame, 0);
    let mut machine = TestMachine::parse(
        r#"
function destroy {
    load r1, r0: int32
    global.address.local r15, g0
    store r15, r1: int32
    return
}

function generate {
    unreachable
}

function owner {
    continuation.new r2, generate, r0
    continuation.complete r3, r4, r5, r2, r1 => b0 | b1 | b2

b0:
    continuation.destroy r4
    unreachable

b1:
    global.address.local r15, g0
    load r7, r15: int32
    int.add r8, r5, r7: int32
    return r8

b2:
    unwind.resume
}
"#,
        program,
    );

    let value = machine.complete(2, &[Word::int32(7), Word::int32(11)]);

    assert_eq!(value, vec![Word::int32(18)]);
}

/// Destroy values captured by one ready continuation in linked drop code.
#[test]
fn test_destroy_ready_continuation() {
    let program = TestProgram::words()
        .signature(1, [0], 0)
        .signature(2, [0], 0)
        .coroutine(1, CoroutineKind::GENERATOR)
        .frame(2, 1, [])
        .local_global()
        .destructor(0, Storage::Frame, 0);
    let mut machine = TestMachine::parse(
        r#"
function destroy {
    load r1, r0: int32
    global.address.local r15, g0
    store r15, r1: int32
    return
}

function generate {
    unreachable
}

function owner {
    continuation.new r1, generate, r0
    continuation.destroy r1
    global.address.local r15, g0
    load r3, r15: int32
    return r3
}
"#,
        program,
    );

    let value = machine.complete(2, &[Word::int32(37)]);

    assert_eq!(value, vec![Word::int32(37)]);
}

/// Destroy values retained by one yielded continuation in linked drop code.
#[test]
fn test_destroy_suspended_continuation() {
    let suspension = TestProgram::yield_site(1, 0, 1, 2, 3, 0, 0, 0);
    let continuation = TestProgram::continuation_site(2, 1, 2, 6, Some(7));
    let program = TestProgram::words()
        .signature(1, [0], 0)
        .signature(2, [0, 0], 0)
        .coroutine(1, CoroutineKind::GENERATOR)
        .frame(1, 0, [(RegisterSpan::new(RegisterId(0), 1), 0)])
        .frame(2, 2, [])
        .suspensions([suspension])
        .continuations([continuation])
        .local_global()
        .destructor(0, Storage::Frame, 0);
    let mut machine = TestMachine::parse(
        r#"
function destroy {
    load r1, r0: int32
    global.address.local r15, g0
    store r15, r1: int32
    return
}

function generate {
    yield r1, r2, r0 => b0 | b1 | b2

b0:
    return r0

b1:
    return r2

b2:
    unwind.resume
}

function owner {
    continuation.new r2, generate, r0
    continuation.resume r3, r4, r5, r2, r1 => b0 | b1 | b2

b0:
    continuation.destroy r4
    global.address.local r15, g0
    load r7, r15: int32
    return r7

b1:
    return r5

b2:
    unwind.resume
}
"#,
        program,
    );

    let value = machine.complete(2, &[Word::int32(41), Word::int32(11)]);

    assert_eq!(value, vec![Word::int32(41)]);
}

/// Preserve one pending resume while an async generator awaits.
#[test]
fn test_resume_async_generator() {
    let yield_site = TestProgram::yield_site(0, 0, 1, 2, 3, 0, 0, 0);
    let await_site = TestProgram::await_site(0, 1, 4, 3, 3, 0, 0);
    let first_resume = TestProgram::continuation_site(2, 1, 2, 3, Some(4));
    let second_resume = TestProgram::continuation_site(2, 2, 5, 6, Some(4));
    let program = TestProgram::words()
        .signature(0, [0], 0)
        .signature(1, [0], 0)
        .signature(2, [0, 0], 0)
        .coroutine(0, CoroutineKind::ASYNC_GENERATOR)
        .coroutine(2, CoroutineKind::ASYNC)
        .frame(0, 0, [])
        .frame(0, 1, [])
        .frame(2, 2, [])
        .suspensions([yield_site, await_site])
        .continuations([first_resume, second_resume]);
    let mut machine = TestMachine::parse(
        r#"
function generate {
    yield r1, r3, r0 => b0 | b1 | b2

b0:
    await r2, park, r1 => b3 | b2 | b2

b1:
    return r3

b2:
    unwind.resume

b3:
    return r2
}

function park {
    return r0
}

function owner {
    continuation.new r2, generate, r0
    continuation.resume r3, r4, r5, r2, r1 => b0 | b1 | b2

b0:
    continuation.resume r6, r7, r8, r4, r1 => b3 | b4 | b2

b1:
    return r5

b2:
    unwind.resume

b3:
    return r6

b4:
    return r8
}
"#,
        program,
    );

    // retain the owner Resume while the resumed generator awaits
    let (park, awaitable, continuation) =
        machine.run_to_await(2, &[Word::int32(7), Word::int32(9)]);
    assert_eq!(park, FunctionId(1));
    assert_eq!(awaitable, vec![Word::int32(9)]);

    // restore the pending Resume and return through its completion edge
    let value = machine.resume_to_completion(continuation, &[Word::int32(11)]);
    assert_eq!(value, vec![Word::int32(11)]);
}

/// Reject coroutine bodies entered through synchronous bytecode calls.
#[test]
fn test_reject_coroutine_call() {
    let program = TestProgram::words()
        .signature(0, [], 0)
        .signature(1, [], 0)
        .coroutine(0, CoroutineKind::ASYNC);
    let mut machine = TestMachine::parse(
        r#"
function wait {
    return
}

function caller {
    call _, wait()
    return
}
"#,
        program,
    );

    let error = machine
        .run(1, &[], None, None, None)
        .expect_err("synchronous calls must reject coroutine bodies");

    assert!(matches!(error.reason(), ErrorReason::Instruction(_)));
}
