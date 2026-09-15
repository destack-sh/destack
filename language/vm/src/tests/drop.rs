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

/// Destroy an unretained allocation's value through its heap metadata before freeing it.
#[test]
fn test_execute_release_runs_the_allocation_destructor() {
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
    release r1
    global.address r2, g0
    load.int32 r3, r2
    return r3
}
"#,
        TestProgram::words()
            .local_global()
            .allocations([allocation])
            .destructor(1, Storage::Heap(Space::Local), 0)
            .frame(1, 2, [(RegisterSpan::new(RegisterId(1), 1), 1)]),
    );

    let value = machine.complete(1, &[Word::int32(59)]);

    assert_eq!(value, vec![Word::int32(59)]);
}

/// Destroy a shared allocation's value through its heap metadata before freeing it.
#[test]
fn test_execute_shared_release_runs_the_allocation_destructor() {
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
    release r1
    global.address r2, g0
    load.int32 r3, r2
    return r3
}
"#,
        TestProgram::words()
            .shared_global()
            .allocations([allocation])
            .destructor(1, Storage::Heap(Space::Shared), 0)
            .frame(1, 2, [(RegisterSpan::new(RegisterId(1), 1), 1)]),
    );

    let value = machine.complete(1, &[Word::int32(61)]);

    assert_eq!(value, vec![Word::int32(61)]);
}

/// Destroy every value of a released repeated allocation from the last to the first.
#[test]
fn test_execute_release_runs_the_destructor_of_each_repeated_value() {
    let allocation = TestProgram::slice_allocation(1, 0, Space::Local, 1);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    global.address r1, g0
    load.int32 r2, r1
    constant.int32 r3, 1
    add.int32 r4, r2, r3
    store.int32 r1, r4
    return
}

function f1 {
    new.slice.uninit r1:r2, a0, r0
    release r1
    global.address r3, g0
    load.int32 r4, r3
    return r4
}
"#,
        TestProgram::words()
            .local_global()
            .allocations([allocation])
            .destructor(1, Storage::Heap(Space::Local), 0)
            .frame(1, 1, [(RegisterSpan::new(RegisterId(1), 1), 1)]),
    );

    let value = machine.complete(1, &[Word::uint64(3)]);

    assert_eq!(value, vec![Word::int32(3)]);
}

/// Free an allocation whose values moved out without running its destructor.
#[test]
fn test_execute_free_skips_the_allocation_destructor() {
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
    free r1
    global.address r2, g0
    load.int32 r3, r2
    return r3
}
"#,
        TestProgram::words()
            .local_global()
            .allocations([allocation])
            .destructor(1, Storage::Heap(Space::Local), 0),
    );

    let value = machine.complete(1, &[Word::int32(59)]);

    assert_eq!(value, vec![Word::int32(0)]);
}
