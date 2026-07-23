use destack_mir::Space;
use destack_program::{Profile, ProfileOptions, Word};

use super::{TestMachine, TestProgram};

/// Allocate local storage and access it through its stable heap reference.
#[test]
fn test_execute_local_allocation() {
    let site = TestMachine::value_allocation(0, 0, Space::Local, 0);
    let mut machine = TestMachine::parse(
        r#"
type Value

export function allocate(r0: int32): int32 {
    r1: ref<managed, space(local)> = new.local.managed.zeroed Value
    r2: pointer = reference.pointer r1
    store.int32 r2, r0
    r3: int32 = load.int32 r2
    return r3
}
"#,
        TestProgram::new().allocations([site]),
    );
    let mut profile = Profile::new(machine.program(), ProfileOptions::STANDARD);

    let value = machine.complete_profiled("allocate", &[Word::int32(53)], &mut profile);

    assert_eq!(value, vec![Word::int32(53)]);
    assert_eq!(profile.allocations[0].count, 1);
    assert_eq!(profile.allocations[0].bytes, Word::BYTE_LEN as u64);
}

/// Enter the linked concrete destructor and resume the caller after it returns.
#[test]
fn test_execute_drop() {
    let mut machine = TestMachine::parse(
        r#"
type State
type Point

local global dropped: State = zero

function Point.destruct(r0: pointer): void {
    r1: int32 = load.int32 r0
    r2: pointer = global.address dropped
    store.int32 r2, r1
    return
}

export function destroy(r0: int32): int32 {
    drop r0: Point
    r1: pointer = global.address dropped
    r2: int32 = load.int32 r1
    return r2
}
"#,
        TestProgram::new(),
    );

    let value = machine.complete("destroy", &[Word::int32(53)]);

    assert_eq!(value, vec![Word::int32(53)]);
}

/// Allocate shared storage and access it through its stable heap reference.
#[test]
fn test_execute_shared_allocation() {
    let site = TestMachine::value_allocation(0, 0, Space::Shared, 0);
    let mut machine = TestMachine::parse(
        r#"
type Value

export function allocate(r0: int32): int32 {
    r1: ref<managed, space(shared)> = new.shared.managed.zeroed Value
    r2: pointer = reference.pointer r1
    store.int32 r2, r0
    r3: int32 = load.int32 r2
    return r3
}
"#,
        TestProgram::new().allocations([site]),
    );

    let value = machine.complete("allocate", &[Word::int32(61)]);

    assert_eq!(value, vec![Word::int32(61)]);
}

/// Execute pinning, barriers, and explicit unique release through heap ownership.
#[test]
fn test_execute_reference_lifetime() {
    let managed = TestMachine::value_allocation(0, 0, Space::Local, 0);
    let unique = TestMachine::value_allocation(0, 4, Space::Local, 0);
    let mut machine = TestMachine::parse(
        r#"
type Value

export function maintain(r0: uint64, r1: uint64): boolean {
    r2: ref<managed, space(local)> = new.local.managed.zeroed Value
    pin r2
    barrier r2, r0, r1
    unpin r2
    r3: ref<unique, space(local)> = new.local.unique.zeroed Value
    free r3
    r4: boolean = true
    return r4
}
"#,
        TestProgram::new().allocations([managed, unique]),
    );

    let value = machine.complete(
        "maintain",
        &[Word::uint64(0), Word::uint64(Word::BYTE_LEN as u64)],
    );

    assert_eq!(value, vec![Word::boolean(true)]);
}
