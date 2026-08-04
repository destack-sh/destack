use destack_bytecode::{RegisterId, RegisterSpan};
use destack_mir::{GlobalStorage, ReferenceKind, Space, Storage};
use destack_program::{CoroutineKind, MemoryAccess, Poll, StopReason, StopSet, WatchSet, Word};

use super::{RuntimeCall, TestMachine, TestProgram};

/// Pause at one runtime poll with exact roots and continue at its successor.
#[test]
fn test_pause_at_poll() {
    let allocation = TestProgram::value_allocation(0, 0, Space::Local, 1);
    let program = TestProgram::words()
        .allocations([allocation])
        .reference(1, 0, ReferenceKind::Managed, Storage::Heap(Space::Local))
        .frame(0, 2, [(RegisterSpan::new(RegisterId(0), 1), 1)]);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    new.zeroed r0, a0: ref<managed, local>
    poll
    return r0
}
"#,
        program,
    );
    machine.request_poll(Poll::Pause);

    // expose the live allocation to the runtime and retain the poll successor
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
            root_count: 1,
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
    int.add r2, r0, r1: int32
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
    let site = TestProgram::memory_site(
        0,
        1,
        MemoryAccess::Write,
        Some(Storage::Global(GlobalStorage::Local)),
    );
    let watch = TestProgram::watchpoint(0, 1, 11, MemoryAccess::Write);
    let watchpoint_id = watch.watchpoint_id;
    let watches = WatchSet::new(vec![watch]);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    global.address.local r15, g0
    store r15, r0: int32
    load r2, r15: int32
    return r2
}
"#,
        TestProgram::words().local_global().memory([site]).frame(
            0,
            2,
            [(RegisterSpan::new(RegisterId(15), 1), 0)],
        ),
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
        .reference(1, 0, ReferenceKind::Borrowed, Storage::Frame)
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
    load r1, r0: int32
    return r1
}
"#,
        program,
    );

    // capture both physical frames before the callee dereferences its caller
    let stopped = machine.run_to_stop(0, &[Word::int32(41)], Some(&stops), None);
    assert_eq!(stopped, reason);
    let (image, memory) = machine.capture();
    assert_eq!(image.frames().len(), 2);

    // restore into a distinct virtual memory map and follow the frame address
    machine.restore(image, memory);
    let value = machine.continue_to_completion(Some(&stops), None, stopped.resume_skip());
    assert_eq!(value, vec![Word::int32(41)]);
}

/// Restore a stopped destructor with its retained continuation value.
#[test]
fn test_restore_continuation_destruction() {
    let program = TestProgram::words()
        .signature(0, [1], 0)
        .signature(1, [0], 0)
        .signature(2, [0], 0)
        .coroutine(1, CoroutineKind::GENERATOR)
        .reference(1, 0, ReferenceKind::Borrowed, Storage::Frame)
        .frame(0, 1, [(RegisterSpan::new(RegisterId(0), 1), 1)])
        .frame(2, 1, [])
        .local_global()
        .destructor(0, Storage::Frame, 0);
    let mut machine = TestMachine::parse(
        r#"
function destroy {
    breakpoint
    load r1, r0: int32
    global.address.local r15, g0
    store r15, r1: int32
    return
}

function generate {
    unreachable
}

function owner {
    continuation.new r1, generate, r0
    continuation.destroy r1
    global.address.local r15, g0
    load r3, r15: int32
    return r3
}
"#,
        program,
    );

    // stop while the destructor points into retained continuation storage
    machine.run_to_stop(2, &[Word::int32(47)], None, None);
    let (image, memory) = machine.capture();
    assert_eq!(image.frames().len(), 3);

    // restore the owner, retained frame, and destructor into a fresh memory map
    machine.restore(image, memory);
    let value = machine.continue_to_completion(None, None, None);
    assert_eq!(value, vec![Word::int32(47)]);
}

/// Expose managed references retained by stopped physical frames as mutable roots.
#[test]
fn test_visit_stopped_roots() {
    let allocation = TestProgram::value_allocation(0, 0, Space::Local, 1);
    let program = TestProgram::words()
        .allocations([allocation])
        .reference(1, 0, ReferenceKind::Managed, Storage::Heap(Space::Local))
        .frame(0, 2, [(RegisterSpan::new(RegisterId(0), 1), 1)]);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    new.zeroed r0, a0: ref<managed, local>
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
