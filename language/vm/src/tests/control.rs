use destack_program::{TypeId, Word};

use super::{TestMachine, TestProgram};
use crate::{ErrorReason, Panic, Trap};

/// Execute constants, arithmetic, branches, calls, and returns from linked bytecode.
#[test]
fn test_execute_scalar_call_control_flow() {
    let mut machine = TestMachine::parse(
        r#"
function double(r0: int32): int32 {
    r1: int32 = int.add r0, r0
    return r1
}

export function calculate(r0: int32, r1: int32, r2: boolean): int32 {
    r3: int32 = int.add r0, r1
    branch r2, l0, l1

l0:
    r4: int32 = call double(r3)
    return r4

l1:
    return r3
}
"#,
        TestProgram::new(),
    );

    // take the direct return edge
    let value = machine.complete(
        "calculate",
        &[Word::int32(3), Word::int32(4), Word::boolean(false)],
    );
    assert_eq!(value, vec![Word::int32(7)]);

    // take the call edge and transfer its result back into the caller
    let value = machine.complete(
        "calculate",
        &[Word::int32(3), Word::int32(4), Word::boolean(true)],
    );
    assert_eq!(value, vec![Word::int32(14)]);
}

/// Follow checked overflow and fused scalar comparison edges.
#[test]
fn test_execute_checked_control_flow() {
    let mut machine = TestMachine::parse(
        r#"
export function classify(r0: int8, r1: int8): int8 {
    check.add.overflow.int8 r0, r1 else l2
    branch.lt.int8 r0, r1 => l0, l1

l0:
    r2: int8 = 1
    return r2

l1:
    r2: int8 = 2
    return r2

l2:
    r2: int8 = 3
    return r2
}
"#,
        TestProgram::new(),
    );

    // follow the successful less-than edge
    let value = machine.complete("classify", &[Word::int8(10), Word::int8(20)]);
    assert_eq!(value, vec![Word::int8(1)]);

    // follow the successful greater-than-or-equal edge
    let value = machine.complete("classify", &[Word::int8(20), Word::int8(10)]);
    assert_eq!(value, vec![Word::int8(2)]);

    // follow the overflow failure edge
    let value = machine.complete("classify", &[Word::int8(100), Word::int8(100)]);
    assert_eq!(value, vec![Word::int8(3)]);
}

/// Reuse one frame across deep direct tail recursion.
#[test]
fn test_execute_tail_call() {
    let mut machine = TestMachine::parse(
        r#"
export function countdown(r0: int32): int32 {
    r1: int32 = 0
    r2: boolean = equal r0, r1
    branch r2, l0, l1

l0:
    return r0

l1:
    r3: int32 = 1
    r4: int32 = int.sub r0, r3
    tail.call countdown(r4)
}
"#,
        TestProgram::new(),
    );

    // exceed the ordinary test frame limit without growing the call stack
    let value = machine.complete("countdown", &[Word::int32(1_000)]);

    assert_eq!(value, vec![Word::int32(0)]);
}

/// Preserve a typed panic payload across invoke cleanup and outward unwind.
#[test]
fn test_execute_panic_unwind() {
    let mut machine = TestMachine::parse(
        r#"
type Failure

function fail(r0: ref<managed, space(local)>): void {
    panic r0: Failure
}

export function run(r0: ref<managed, space(local)>): void {
    invoke fail(r0) => l0 | l1

l0:
    return

l1:
    unwind.resume
}
"#,
        TestProgram::new(),
    );
    let payload = Word::from_bits(0x1234);

    let error = machine
        .run("run", &[payload], None, None, None)
        .expect_err("panic should cross the machine boundary");

    assert_eq!(
        error.reason,
        ErrorReason::Panic(Panic::new(TypeId(0), vec![payload]))
    );
}

/// Abort when cleanup starts a second panic during unwind.
#[test]
fn test_abort_double_panic() {
    let mut machine = TestMachine::parse(
        r#"
type Failure

function fail(r0: ref<managed, space(local)>): void {
    panic r0: Failure
}

export function run(r0: ref<managed, space(local)>): void {
    invoke fail(r0) => l0 | l1

l0:
    return

l1:
    panic r0: Failure
}
"#,
        TestProgram::new(),
    );

    let error = machine
        .run("run", &[Word::from_bits(0x1234)], None, None, None)
        .expect_err("double panic should abort");

    assert_eq!(error.reason, ErrorReason::Trap(Trap::Abort));
}
