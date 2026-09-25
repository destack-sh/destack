use tspp_bytecode::{RegisterId, RegisterSpan};
use tspp_mir::{Reference, Space, Storage};
use tspp_program::{MemoryAccess, Poll, StopReason, StopSet, WatchSet, Word};

use super::{RuntimeCall, TestMachine, TestProgram};

/// Pause at one runtime poll with exact roots and continue at its successor.
#[test]
fn test_pause_at_poll() {
    let allocation = TestProgram::value_allocation(0, 0, Space::Local, 1);
    let program = TestProgram::words()
        .allocations([allocation])
        .reference(
            1,
            0,
            Reference::Managed(Space::Local),
            Storage::Heap(Space::Local),
        )
        .frame(0, 2, [(RegisterSpan::new(RegisterId(0), 1), 1)]);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    new.zeroed r0, a0
    poll
    return r0
}
"#,
        program,
    );
    machine.request_poll(Poll::Pause);

    // pause at the poll and retain the poll successor
    let reason = machine.run_to_stop(0, &[], None, None);
    assert_eq!(
        reason,
        StopReason::Pause {
            point: TestProgram::point(0, 1),
        }
    );
    assert_eq!(
        machine.take_runtime_calls(),
        vec![RuntimeCall::Poll {
            action: Poll::Pause,
        }]
    );

    // continue from the successor without polling a second time
    let [result] = machine
        .continue_to_completion(None, None, None)
        .try_into()
        .expect("poll function should return one word");
    assert!(!result.is_nullish());
}

/// Stop before one selected operation and continue it exactly once.
#[test]
fn test_stop_at_breakpoint() {
    let stop = TestProgram::breakpoint(0, 0, 7);
    let reason = stop.reason;
    let stops = StopSet::new(vec![stop]);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    add.int32 r2, r0, r1
    return r2
}
"#,
        TestProgram::words().frame(
            0,
            0,
            [
                (RegisterSpan::new(RegisterId(0), 1), 0),
                (RegisterSpan::new(RegisterId(1), 1), 0),
            ],
        ),
    );

    // stop before the selected addition
    let stopped = machine.run_to_stop(0, &[Word::int32(4), Word::int32(6)], Some(&stops), None);
    assert_eq!(stopped, reason);

    // skip the retained breakpoint once and execute the selected addition
    let value = machine.continue_to_completion(Some(&stops), None, stopped.resume_skip());
    assert_eq!(value, vec![Word::int32(10)]);
}

/// Stop after an explicit breakpoint instruction and continue at its successor.
#[test]
fn test_execute_breakpoint() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    breakpoint
    return r0
}
"#,
        TestProgram::words().frame(0, 1, [(RegisterSpan::new(RegisterId(0), 1), 0)]),
    );

    let reason = machine.run_to_stop(0, &[Word::int32(13)], None, None);
    assert_eq!(
        reason,
        StopReason::Instruction {
            point: TestProgram::point(0, 0),
        }
    );

    let value = machine.continue_to_completion(None, None, None);
    assert_eq!(value, vec![Word::int32(13)]);
}

/// Stop after one selected write while preserving the completed mutation.
#[test]
fn test_stop_at_watchpoint() {
    let point = TestProgram::point(0, 1);
    let store = TestProgram::memory_site(
        0,
        1,
        MemoryAccess::Write,
        Some(Storage::Static(Space::Local)),
    );
    let load = TestProgram::memory_site(
        0,
        2,
        MemoryAccess::Read,
        Some(Storage::Static(Space::Local)),
    );
    let watch = TestProgram::watchpoint(0, 1, 11, MemoryAccess::Write);
    let watchpoint_id = watch.watchpoint_id;
    let watches = WatchSet::new(vec![watch]);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    global.address r15, g0
    store.int32 r15, r0
    load.int32 r2, r15
    return r2
}
"#,
        TestProgram::words()
            .local_global()
            .memory([store, load])
            .frame(0, 2, [(RegisterSpan::new(RegisterId(15), 1), 0)]),
    );

    // execute the selected write before retaining the machine
    let reason = machine.run_to_stop(0, &[Word::int32(29)], None, Some(&watches));
    assert_eq!(
        reason,
        StopReason::Watchpoint {
            watchpoint_id,
            point,
        }
    );

    // observe the completed write after continuing at the next operation
    let value = machine.continue_to_completion(None, Some(&watches), None);
    assert_eq!(value, vec![Word::int32(29)]);
}

/// Restore a stopped nested call stack while preserving cross-frame addresses.
#[test]
fn test_restore_nested_stop() {
    let stop = TestProgram::breakpoint(1, 0, 17);
    let call = TestProgram::call(0, 1, 2, 1);
    let reason = stop.reason;
    let stops = StopSet::new(vec![stop]);
    let program = TestProgram::words()
        .reference(1, 0, Reference::Borrowed, Storage::Frame)
        .frame(
            0,
            1,
            [
                (RegisterSpan::new(RegisterId(0), 1), 0),
                (RegisterSpan::new(RegisterId(1), 1), 1),
            ],
        )
        .frame(1, 0, [(RegisterSpan::new(RegisterId(0), 1), 1)]);
    let program = program.calls([call]);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    frame.address r1, r0
    call r2, f1(r1)
    return r2
}

function f1 {
    load.int32 r1, r0
    return r1
}
"#,
        program,
    );

    // capture both physical frames before the callee dereferences its caller
    let stopped = machine.run_to_stop(0, &[Word::int32(41)], Some(&stops), None);
    assert_eq!(stopped, reason);
    let (fiber, memory) = machine.fork_fiber();
    assert_eq!(fiber.frame_count(), 2);

    // restore into a distinct virtual memory map and follow the frame address
    machine.adopt_fiber(fiber, memory);
    let value = machine.continue_to_completion(Some(&stops), None, stopped.resume_skip());
    assert_eq!(value, vec![Word::int32(41)]);
}

/// Expose managed references retained by stopped physical frames as mutable roots.
#[test]
fn test_visit_stopped_roots() {
    let allocation = TestProgram::value_allocation(0, 0, Space::Local, 1);
    let program = TestProgram::words()
        .allocations([allocation])
        .reference(
            1,
            0,
            Reference::Managed(Space::Local),
            Storage::Heap(Space::Local),
        )
        .frame(0, 2, [(RegisterSpan::new(RegisterId(0), 1), 1)]);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    new.zeroed r0, a0
    breakpoint
    return r0
}
"#,
        program,
    );

    // stop with the sole allocation reachable only from the retained frame
    machine.run_to_stop(0, &[], None, None);
    let roots = machine.roots();

    assert_eq!(roots.len(), 1);
    assert!(!roots[0].is_nullish());
}
