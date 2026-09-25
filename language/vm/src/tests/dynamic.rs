use tspp_bytecode::{RegisterId, RegisterSpan};
use tspp_mir::{Space, Storage};
use tspp_program::{DynamicEntry, FunctionId, MemoryAccess, StopReason, TypeId, WatchSet, Word};

use super::{TestMachine, TestProgram};

/// Bind, inspect, and dispatch one erased dynamic value.
#[test]
fn test_execute_dynamic_value() {
    let program = TestProgram::words().dynamic_table(1, 0, []).dynamic_table(
        3,
        2,
        [DynamicEntry::function(FunctionId(0))],
    );
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    constant.int32 r1, 41
    return r1
}

function f1 {
    dynamic.bind r1:r2, r0, d1
    extract r3, r1:r2, 0:8
    dynamic.type r4, r1:r2
    call.dynamic r5, r1:r2[0](r0)
    return r3:r5
}
"#,
        program,
    );

    let payload = Word::from_bits(0x2400);
    let concrete = Word::uint32(u32::from(TypeId(3)));
    let value = machine.complete(1, &[payload]);

    assert_eq!(value, vec![payload, concrete, Word::int32(41)]);
}

/// Read one concrete field through an erased dynamic value.
#[test]
fn test_execute_dynamic_read() {
    let write = TestProgram::memory_site(
        0,
        1,
        MemoryAccess::Write,
        Some(Storage::Static(Space::Local)),
    );
    let read = TestProgram::memory_site(
        0,
        3,
        MemoryAccess::Read,
        Some(Storage::Static(Space::Local)),
    );
    let watch = TestProgram::watchpoint(0, 3, 19, MemoryAccess::Read);
    let watchpoint_id = watch.watchpoint_id;
    let watches = WatchSet::new(vec![watch]);
    let program = TestProgram::words()
        .local_global()
        .dynamic_table(3, 2, [DynamicEntry::field_offset(0)])
        .memory([write, read])
        .frame(0, 4, [(RegisterSpan::new(RegisterId(4), 1), 0)]);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    global.address r1, g0
    store.int32 r1, r0
    dynamic.bind r2:r3, r1, d0
    dynamic.read r4, r2:r3[0], 4
    return r4
}
"#,
        program,
    );

    // stop after reading the selected dynamic field
    let reason = machine.run_to_stop(0, &[Word::int32(41)], None, Some(&watches));
    assert_eq!(
        reason,
        StopReason::Watchpoint {
            watchpoint_id,
            point: TestProgram::point(0, 3),
        }
    );

    // return the retained field value after resuming
    let value = machine.continue_to_completion(None, Some(&watches), None);
    assert_eq!(value, vec![Word::int32(41)]);
}
