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
    new.zeroed r1, a0
    store.int32 r1, r0
    load.int32 r3, r1
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

/// Allocate shared storage and access it through its stable heap reference.
#[test]
fn test_execute_shared_allocation() {
    let site = TestProgram::value_allocation(0, 0, Space::Shared, 0);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    new.zeroed r1, a0
    store.int32 r1, r0
    load.int32 r3, r1
    return r3
}
"#,
        TestProgram::words().allocations([site]),
    );

    let value = machine.complete(0, &[Word::int32(61)]);

    assert_eq!(value, vec![Word::int32(61)]);
}

/// Execute barriers and explicit unique release through heap ownership.
#[test]
fn test_execute_reference_operations() {
    let managed = TestProgram::value_allocation(0, 0, Space::Local, 0);
    let unique = TestProgram::value_allocation(0, 4, Space::Local, 0);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    new.zeroed r2, a0
    barrier r2, r0, r1: ref<managed, local>
    new.zeroed r3, a1
    release r3
    constant.boolean r4, true
    return r4
}
"#,
        TestProgram::words().allocations([managed, unique]),
    );

    let value = machine.complete(0, &[Word::uint64(0), Word::uint64(Word::BYTE_LEN as u64)]);

    assert_eq!(value, vec![Word::boolean(true)]);
}
