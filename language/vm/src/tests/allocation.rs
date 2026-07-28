use destack_bytecode::{RegisterId, RegisterSpan};
use destack_mir::Space;
use destack_program::{Profile, ProfileOptions, Word};

use super::{TestMachine, TestProgram};

/// Allocate local storage and access it through its stable heap reference.
#[test]
fn test_execute_local_allocation() {
    let site = TestProgram::value_allocation(0, 0, Space::Local, 0);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    new.local.managed.zeroed r1, a0
    pointer.local r2, r1
    store.int32 r2, r0
    load.int32 r3, r2
    return r3
}
"#,
        TestProgram::words().allocations([site]),
    );
    let mut profile = Profile::new(machine.program(), ProfileOptions::STANDARD);

    let value = machine.complete_profiled(0, &[Word::int32(53)], &mut profile);

    assert_eq!(value, vec![Word::int32(53)]);
    assert_eq!(profile.allocations[0].count, 1);
    assert_eq!(profile.allocations[0].bytes, Word::BYTE_LEN as u64);
}

/// Enter the linked concrete destructor and resume the caller after it returns.
#[test]
fn test_execute_drop() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    pointer.frame r4, r0
    load.int32 r1, r4
    global.address r3, g0
    pointer.global r2, r3
    store.int32 r2, r1
    return
}

function f1 {
    drop r0, f0
    global.address r3, g0
    pointer.global r1, r3
    load.int32 r2, r1
    return r2
}
"#,
        TestProgram::words().local_global().drop(1, 0).frame(
            1,
            0,
            [(RegisterSpan::new(RegisterId(0), 1), 1)],
        ),
    );

    let value = machine.complete(1, &[Word::int32(53)]);

    assert_eq!(value, vec![Word::int32(53)]);
}

/// Allocate shared storage and access it through its stable heap reference.
#[test]
fn test_execute_shared_allocation() {
    let site = TestProgram::value_allocation(0, 0, Space::Shared, 0);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    new.shared.managed.zeroed r1, a0
    pointer.shared r2, r1
    store.int32 r2, r0
    load.int32 r3, r2
    return r3
}
"#,
        TestProgram::words().allocations([site]),
    );

    let value = machine.complete(0, &[Word::int32(61)]);

    assert_eq!(value, vec![Word::int32(61)]);
}

/// Execute pinning, barriers, and explicit unique release through heap ownership.
#[test]
fn test_execute_reference_lifetime() {
    let managed = TestProgram::value_allocation(0, 0, Space::Local, 0);
    let unique = TestProgram::value_allocation(0, 4, Space::Local, 0);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    new.local.managed.zeroed r2, a0
    pin.local.managed r2
    barrier.local.managed r2, r0, r1
    unpin.local.managed r2
    new.local.unique.zeroed r3, a1
    free.local.unique r3
    constant.boolean r4, true
    return r4
}
"#,
        TestProgram::words().allocations([managed, unique]),
    );

    let value = machine.complete(0, &[Word::uint64(0), Word::uint64(Word::BYTE_LEN as u64)]);

    assert_eq!(value, vec![Word::boolean(true)]);
}
