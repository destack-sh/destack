use destack_bytecode::{RegisterId, RegisterSpan};
use destack_mir::{Space, Storage};
use destack_program::Word;

use super::{TestMachine, TestProgram};

/// Enter one statically selected frame destructor and resume its caller.
#[test]
fn test_execute_static_drop() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    load.int32 r1, r0
    global.address r3, g0
    store.int32 r3, r1
    return
}

function f1 {
    drop r0, f0
    global.address r3, g0
    load.int32 r2, r3
    return r2
}
"#,
        TestProgram::words()
            .local_global()
            .destructor(1, Storage::Frame, 0)
            .frame(1, 0, [(RegisterSpan::new(RegisterId(0), 1), 1)]),
    );

    let value = machine.complete(1, &[Word::int32(53)]);

    assert_eq!(value, vec![Word::int32(53)]);
}

/// Select a unique dynamic payload destructor through its concrete type.
#[test]
fn test_execute_dynamic_drop() {
    let allocation = TestProgram::value_allocation(1, 0, Space::Local, 1);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    constant.int32 r1, 53
    global.address r2, g0
    store.int32 r2, r1
    return
}

function f1 {
    new.zeroed r0, a0
    dynamic.bind r1:r2, r0, d0
    drop.dynamic r1:r2: ref<unique, local>
    global.address r3, g0
    load.int32 r4, r3
    return r4
}
"#,
        TestProgram::words()
            .local_global()
            .allocations([allocation])
            .dynamic_table(1, 0, [])
            .destructor(1, Storage::Heap(Space::Local), 0)
            .frame(1, 2, []),
    );

    let value = machine.complete(1, &[]);

    assert_eq!(value, vec![Word::int32(53)]);
}

/// Select a unique function environment destructor through its bound function.
#[test]
fn test_execute_function_drop() {
    let allocation = TestProgram::value_allocation(2, 0, Space::Local, 1);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    constant.int32 r1, 61
    global.address r2, g0
    store.int32 r2, r1
    return
}

function f1 {
    return
}

function f2 {
    new.zeroed r0, a0
    function.bind r1:r2, f1, r0
    drop.function r1:r2: ref<unique, local>
    global.address r3, g0
    load.int32 r4, r3
    return r4
}
"#,
        TestProgram::words()
            .local_global()
            .allocations([allocation])
            .environment(1, 1)
            .destructor(1, Storage::Heap(Space::Local), 0)
            .frame(2, 2, []),
    );

    let value = machine.complete(2, &[]);

    assert_eq!(value, vec![Word::int32(61)]);
}
