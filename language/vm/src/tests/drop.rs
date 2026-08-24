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

/// Select one unique allocation destructor from its heap metadata.
#[test]
fn test_execute_indirect_drop() {
    let allocation = TestProgram::value_allocation(1, 0, Space::Local, 1);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    load.int32 r1, r0
    global.address r2, g0
    store.int32 r2, r1
    return
}

function f1 {
    new.zeroed r1, a0
    store.int32 r1, r0
    drop r1
    free r1
    global.address r2, g0
    load.int32 r3, r2
    return r3
}
"#,
        TestProgram::words()
            .local_global()
            .allocations([allocation])
            .destructor(1, Storage::LocalHeap, 0)
            .frame(1, 2, [(RegisterSpan::new(RegisterId(1), 1), 1)]),
    );

    let value = machine.complete(1, &[Word::int32(59)]);

    assert_eq!(value, vec![Word::int32(59)]);
}

/// Select one shared allocation destructor from its heap metadata.
#[test]
fn test_execute_shared_indirect_drop() {
    let allocation = TestProgram::value_allocation(1, 0, Space::Shared, 1);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    load.int32 r1, r0
    global.address r2, g0
    store.int32 r2, r1
    return
}

function f1 {
    new.zeroed r1, a0
    store.int32 r1, r0
    drop r1
    free r1
    global.address r2, g0
    load.int32 r3, r2
    return r3
}
"#,
        TestProgram::words()
            .shared_global()
            .allocations([allocation])
            .destructor(1, Storage::SharedHeap, 0)
            .frame(1, 2, [(RegisterSpan::new(RegisterId(1), 1), 1)]),
    );

    let value = machine.complete(1, &[Word::int32(61)]);

    assert_eq!(value, vec![Word::int32(61)]);
}
