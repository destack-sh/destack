use tspp_program::{TypeId, Word};

use super::{TestMachine, TestProgram};
use crate::{ErrorReason, Panic, ResourceError, Trap};

/// Execute constants, arithmetic, branches, calls, and returns from linked bytecode.
#[test]
fn test_execute_scalar_call_control_flow() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    poll
    add.int32 r1, r0, r0
    return r1
}

function f1 {
    add.int32 r3, r0, r1
    branch r2 => b0 | b1

b0:
    call r4, f0(r3)
    return r4

b1:
    return r3
}
"#,
        TestProgram::words(),
    );

    // take the direct return edge
    let value = machine.complete(1, &[Word::int32(3), Word::int32(4), Word::boolean(false)]);
    assert_eq!(value, vec![Word::int32(7)]);

    // take the call edge and transfer its result back into the caller
    let value = machine.complete(1, &[Word::int32(3), Word::int32(4), Word::boolean(true)]);
    assert_eq!(value, vec![Word::int32(14)]);
}

/// Follow checked overflow and fused scalar comparison edges.
#[test]
fn test_execute_checked_control_flow() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    check.add.overflow.int8 r0, r1 | b2
    branch.lt.int8 r0, r1 => b0 | b1
b0:
    constant.int8 r2, 1
    return r2

b1:
    constant.int8 r2, 2
    return r2

b2:
    constant.int8 r2, 3
    return r2
}
"#,
        TestProgram::words(),
    );

    // follow the successful less-than edge
    let value = machine.complete(0, &[Word::int8(10), Word::int8(20)]);
    assert_eq!(value, vec![Word::int8(1)]);

    // follow the successful greater-than-or-equal edge
    let value = machine.complete(0, &[Word::int8(20), Word::int8(10)]);
    assert_eq!(value, vec![Word::int8(2)]);

    // follow the overflow failure edge
    let value = machine.complete(0, &[Word::int8(100), Word::int8(100)]);
    assert_eq!(value, vec![Word::int8(3)]);
}

/// Reuse one frame across deep direct tail recursion.
#[test]
fn test_execute_tail_call() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    constant.int32 r1, 0
    equal r2, r0, r1
    branch r2 => b0 | b1

b0:
    return r0

b1:
    constant.int32 r3, 1
    sub.int32 r4, r0, r3
    tail.call f0(r4)
}
"#,
        TestProgram::words(),
    );

    // exceed the ordinary test frame limit without growing the call stack
    let value = machine.complete(0, &[Word::int32(1_000)]);

    assert_eq!(value, vec![Word::int32(0)]);
}

/// Preserve a typed panic payload across invoke cleanup and outward unwind.
#[test]
fn test_execute_panic_unwind() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    panic r0, t0
}

function f1 {
    invoke _, f0(r0) => b0 | b1

b0:
    return

b1:
    unwind.resume
}
"#,
        TestProgram::words(),
    );
    let payload = Word::from_bits(0x1234);

    let error = machine
        .run(1, &[payload], None, None, None)
        .expect_err("panic should cross the machine boundary");

    assert_eq!(
        error.reason(),
        &ErrorReason::Panic(Panic::new(TypeId(0), vec![payload]))
    );
}

/// Abort when cleanup starts a second panic during unwind.
#[test]
fn test_abort_double_panic() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    panic r0, t0
}

function f1 {
    invoke _, f0(r0) => b0 | b1

b0:
    return

b1:
    panic r0, t0
}
"#,
        TestProgram::words(),
    );

    let error = machine
        .run(1, &[Word::from_bits(0x1234)], None, None, None)
        .expect_err("double panic should abort");

    assert_eq!(error.reason(), &ErrorReason::Trap(Trap::Abort));
}

/// Clear failed physical execution before accepting another entry.
#[test]
fn test_clear_failed_execution() {
    let mut machine = TestMachine::parse(
        r#"
function loop {
b0:
    jump b0
}

function next {
    constant.int32 r0, 7
    return r0
}
"#,
        TestProgram::words(),
    );

    // exhaust the current activation through one terminal VM error
    let error = machine
        .run(0, &[], None, None, None)
        .expect_err("unbounded loop should exhaust its instruction budget");
    assert_eq!(
        error.reason(),
        &ErrorReason::Resource(ResourceError::InstructionLimitExceeded)
    );

    // execute an unrelated entry through the cleared physical machine
    let value = machine.complete(1, &[]);
    assert_eq!(value, vec![Word::int32(7)]);
}
