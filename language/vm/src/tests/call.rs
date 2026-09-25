use tspp_mir::Space;
use tspp_program::{Memory, Word};

use super::{RuntimeCall, TestMachine, TestProgram};
use crate::Result;

/// Return 42 from one exact word argument.
fn touch(_memory: Memory<'_>, arguments: &[Word], result: &mut [Word]) -> Result<()> {
    assert_eq!(arguments, [Word::int32(41)]);
    assert_eq!(result.len(), 1);
    result[0] = Word::int32(42);

    Ok(())
}

/// Execute the Program implementation attached to one binding identity.
#[test]
fn test_execute_binding_definition() {
    let program = TestProgram::words().binding(0, "runtime.touch");
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    constant.int32 r0, 42
    return r0
}
"#,
        program,
    );

    let value = machine.complete(0, &[]);

    assert_eq!(value, vec![Word::int32(42)]);
}

/// Execute one runtime binding from root, ordinary, and tail call paths.
#[test]
fn test_execute_runtime_binding() {
    let program = TestProgram::words()
        .signature(0, [0], 0)
        .binding(0, "runtime.touch");
    let mut machine = TestMachine::parse(
        r#"
external function f0

function f1 {
    constant.int32 r0, 41
    call r1, f0(r0)
    return r1
}

function f2 {
    constant.int32 r0, 41
    tail.call f0(r0)
}

function f3 {
    constant.int32 r0, 41
    invoke r1, f0(r0) => b0 | b1

b0:
    return r1

b1:
    unreachable
}
"#,
        program,
    );
    machine.bind("runtime.touch", touch);

    // execute each call path through the same registered implementation
    let root = machine.complete(0, &[Word::int32(41)]);
    let ordinary = machine.complete(1, &[]);
    let tail = machine.complete(2, &[]);
    let invoke = machine.complete(3, &[]);
    assert_eq!(root, vec![Word::int32(42)]);
    assert_eq!(ordinary, vec![Word::int32(42)]);
    assert_eq!(tail, vec![Word::int32(42)]);
    assert_eq!(invoke, vec![Word::int32(42)]);

    // preserve exact flattened arguments at each runtime boundary
    let calls = machine.take_runtime_calls();
    assert_eq!(calls.len(), 4);
    assert!(calls.iter().all(|call| matches!(
        call,
        RuntimeCall::Binding { arguments, .. } if arguments == &[Word::int32(41)]
    )));
}

/// Dispatch through the table id initialized in one virtual object.
#[test]
fn test_execute_virtual_call() {
    let allocation = TestProgram::virtual_allocation(2, 0, Space::Local, 1, 1);
    let program = TestProgram::words()
        .allocations([allocation])
        .virtual_table(0, [0])
        .virtual_table(1, [1])
        .virtual_object(1, 16, 12);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    constant.int32 r1, 7
    return r1
}

function f1 {
    constant.int32 r1, 42
    return r1
}

function f2 {
    new.zeroed r0, a0
    call.virtual r1, r0: ref<managed, local>[12, 0](r0)
    return r1
}
"#,
        program,
    );

    let value = machine.complete(2, &[]);

    assert_eq!(value, vec![Word::int32(42)]);
}
