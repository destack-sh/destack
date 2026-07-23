use destack_mir::Space;
use destack_program::{MemoryAccess, StopReason, StopSet, WatchSet, Word};

use super::{TestMachine, TestProgram};

/// Stop before one selected operation and continue it exactly once.
#[test]
fn test_stop_at_breakpoint() {
    let stop = TestMachine::breakpoint(0, 0, 7);
    let reason = stop.reason;
    let stops = StopSet::new(vec![stop]);
    let mut machine = TestMachine::parse(
        r#"
type Value

export function add(r0: int32, r1: int32): int32 {
    slot s0: Value = r0[1]
    slot s1: Value = r1[1]

    r2: int32 = int.add r0, r1
    return r2
}
"#,
        TestProgram::new(),
    );

    // stop before the selected addition
    let (continuation, stopped) =
        machine.run_to_stop("add", &[Word::int32(4), Word::int32(6)], Some(&stops), None);
    assert_eq!(stopped, reason);

    // skip the retained breakpoint once and execute the selected addition
    let value =
        machine.continue_to_completion(continuation, Some(&stops), None, stopped.resume_skip());
    assert_eq!(value, vec![Word::int32(10)]);
}

/// Stop after an explicit breakpoint instruction and continue at its successor.
#[test]
fn test_execute_breakpoint() {
    let mut machine = TestMachine::parse(
        r#"
type Value

export function pause(r0: int32): int32 {
    slot s0: Value = r0[1]

    breakpoint
    return r0
}
"#,
        TestProgram::new(),
    );

    let (continuation, reason) = machine.run_to_stop("pause", &[Word::int32(13)], None, None);
    assert_eq!(
        reason,
        StopReason::Instruction {
            point: TestMachine::point(0, 0),
        }
    );

    let value = machine.continue_to_completion(continuation, None, None, None);
    assert_eq!(value, vec![Word::int32(13)]);
}

/// Stop after one selected write while preserving the completed mutation.
#[test]
fn test_stop_at_watchpoint() {
    let point = TestMachine::point(0, 1);
    let site = TestMachine::memory(0, 1, MemoryAccess::Write, Space::Static);
    let watch = TestMachine::watchpoint(0, 1, 11, MemoryAccess::Write);
    let watchpoint_id = watch.watchpoint_id;
    let watches = WatchSet::new(vec![watch]);
    let mut machine = TestMachine::parse(
        r#"
type State

local global state: State = zero

export function update(r0: int32): int32 {
    slot s0: State = r1[1]

    r1: pointer = global.address state
    store.int32 r1, r0
    r2: int32 = load.int32 r1
    return r2
}
"#,
        TestProgram::new().memory([site]),
    );

    // execute the selected write before retaining the continuation
    let (continuation, reason) =
        machine.run_to_stop("update", &[Word::int32(29)], None, Some(&watches));
    assert_eq!(
        reason,
        StopReason::Watchpoint {
            watchpoint_id,
            point,
        }
    );

    // observe the completed write after continuing at the next operation
    let value = machine.continue_to_completion(continuation, None, Some(&watches), None);
    assert_eq!(value, vec![Word::int32(29)]);
}
