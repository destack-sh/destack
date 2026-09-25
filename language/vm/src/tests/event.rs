use tspp_mir::{Space, Storage};
use tspp_program::{
    BindingEvent, BindingId, EdgeSite, Event, EventKind, FrameEvent, FunctionId, GlobalId,
    GlobalLocation, Memory, MemoryAccess, MemoryRange, TypeId, Word,
};

use super::{TestMachine, TestProgram};
use crate::Result;

/// Return from one test binding without producing a value.
fn return_nothing(_memory: Memory<'_>, _arguments: &[Word], _result: &mut [Word]) -> Result<()> {
    Ok(())
}

/// Publish exact point, call, and frame transitions in execution order.
#[test]
fn test_observe_control_flow() {
    let call = TestProgram::call(1, 0, 1, 0);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    return
}

function f1 {
    call _, f0()
    return
}
"#,
        TestProgram::words().calls([call]),
    );
    machine.select_events([EventKind::Point, EventKind::Call, EventKind::Frame]);

    let value = machine.complete(1, &[]);

    assert_eq!(value, Vec::<Word>::new());
    assert_eq!(
        machine.take_events(),
        vec![
            Event::Frame {
                event: FrameEvent::Enter,
                function: FunctionId(1),
            },
            Event::Point {
                point: TestProgram::point(1, 0),
            },
            Event::Call {
                site: call,
                function: FunctionId(0),
            },
            Event::Frame {
                event: FrameEvent::Enter,
                function: FunctionId(0),
            },
            Event::Point {
                point: TestProgram::point(0, 0),
            },
            Event::Frame {
                event: FrameEvent::Exit,
                function: FunctionId(0),
            },
            Event::Point {
                point: TestProgram::point(1, 1),
            },
            Event::Frame {
                event: FrameEvent::Exit,
                function: FunctionId(1),
            },
        ]
    );
}

/// Publish exact allocation and memory effects after successful execution.
#[test]
fn test_observe_memory() {
    let allocation = TestProgram::value_allocation(0, 0, Space::Local, 0);
    let store =
        TestProgram::memory_site(0, 1, MemoryAccess::Write, Some(Storage::Heap(Space::Local)));
    let load =
        TestProgram::memory_site(0, 2, MemoryAccess::Read, Some(Storage::Heap(Space::Local)));
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    new.zeroed r1, a0
    store.int32 r1, r0
    load.int32 r2, r1
    return r1
}
"#,
        TestProgram::words()
            .allocations([allocation])
            .memory([store, load]),
    );
    machine.select_events([EventKind::Allocation, EventKind::Memory]);

    let value = machine.complete(0, &[Word::int32(53)]);
    let allocation_range = MemoryRange::local_heap(value[0].bits(), Word::BYTE_LEN as u64);
    let access_range = MemoryRange::local_heap(value[0].bits(), u32::BITS as u64 / u8::BITS as u64);

    assert_eq!(
        machine.take_events(),
        vec![
            Event::Allocation {
                site: allocation,
                range: allocation_range,
            },
            Event::Memory {
                site: store,
                access: MemoryAccess::Write,
                range: Some(access_range),
            },
            Event::Memory {
                site: load,
                access: MemoryAccess::Read,
                range: Some(access_range),
            },
        ]
    );
}

/// Pair multiple physical accesses with their exact ordered Program sites.
#[test]
fn test_observe_memory_sites() {
    let storage = Some(Storage::Static(Space::Local));
    let source = TestProgram::memory_site(0, 3, MemoryAccess::Read, storage);
    let target = TestProgram::memory_site(0, 3, MemoryAccess::Write, storage);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    global.address r0, g0
    address.add r1, r0, 4
    constant.uint64 r2, 4
    memory.copy r1, r0, r2
    return
}
"#,
        TestProgram::words().local_global().memory([source, target]),
    );
    machine.select_events([EventKind::Memory]);

    let value = machine.complete(0, &[]);

    assert_eq!(value, Vec::<Word>::new());
    assert_eq!(
        machine.take_events(),
        vec![
            Event::Memory {
                site: source,
                access: MemoryAccess::Read,
                range: Some(MemoryRange::global(
                    GlobalId(0),
                    GlobalLocation::LocalStatic,
                    0,
                    4,
                )),
            },
            Event::Memory {
                site: target,
                access: MemoryAccess::Write,
                range: Some(MemoryRange::global(
                    GlobalId(0),
                    GlobalLocation::LocalStatic,
                    4,
                    4,
                )),
            },
        ]
    );
}

/// Publish the exact selected control-flow edge.
#[test]
fn test_observe_edge() {
    let taken = EdgeSite {
        source: TestProgram::point(0, 0),
        target: TestProgram::point(0, 1),
    };
    let skipped = EdgeSite {
        source: TestProgram::point(0, 0),
        target: TestProgram::point(0, 3),
    };
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    branch r0 => b0 | b1

b0:
    constant.int32 r1, 1
    return r1

b1:
    constant.int32 r1, 2
    return r1
}
"#,
        TestProgram::words().edges([taken, skipped]),
    );
    machine.select_events([EventKind::Edge]);

    let value = machine.complete(0, &[Word::boolean(true)]);

    assert_eq!(value, vec![Word::int32(1)]);
    assert_eq!(machine.take_events(), vec![Event::Edge { site: taken }]);
}

/// Publish binding calls through the same ordered Program event path.
#[test]
fn test_observe_binding() {
    let call = TestProgram::call(1, 0, 1, 0);
    let binding_id = BindingId::from_static_name("runtime.return_nothing");
    let mut machine = TestMachine::parse(
        r#"
external function f0

function f1 {
    call _, f0()
    return
}
"#,
        TestProgram::words()
            .binding(0, "runtime.return_nothing")
            .calls([call]),
    );
    machine.bind("runtime.return_nothing", return_nothing);
    machine.select_events([EventKind::Call, EventKind::Binding]);

    let value = machine.complete(1, &[]);

    assert_eq!(value, Vec::<Word>::new());
    assert_eq!(
        machine.take_events(),
        vec![
            Event::Call {
                site: call,
                function: FunctionId(0),
            },
            Event::Binding {
                event: BindingEvent::Enter,
                binding_id,
            },
            Event::Binding {
                event: BindingEvent::Exit,
                binding_id,
            },
        ]
    );
}

/// Publish frame exit and entry when a tail call replaces its caller.
#[test]
fn test_observe_tail_call_frames() {
    let call = TestProgram::tail_call(0, 0, 1);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    tail.call f1()
}

function f1 {
    return
}
"#,
        TestProgram::words().calls([call]),
    );
    machine.select_events([EventKind::Frame]);

    let value = machine.complete(0, &[]);

    assert_eq!(value, Vec::<Word>::new());
    assert_eq!(
        machine.take_events(),
        vec![
            Event::Frame {
                event: FrameEvent::Enter,
                function: FunctionId(0),
            },
            Event::Frame {
                event: FrameEvent::Exit,
                function: FunctionId(0),
            },
            Event::Frame {
                event: FrameEvent::Enter,
                function: FunctionId(1),
            },
            Event::Frame {
                event: FrameEvent::Exit,
                function: FunctionId(1),
            },
        ]
    );
}

/// Publish every frame exit while a panic unwinds through its caller.
#[test]
fn test_observe_panic_and_unwind() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    panic r0, t0
}

function f1 {
    call _, f0(r0)
    return
}
"#,
        TestProgram::words(),
    );
    machine.select_events([EventKind::Frame, EventKind::Panic]);

    machine
        .run(1, &[Word::from_bits(0x1234)], None, None, None)
        .expect_err("panic should cross the machine boundary");

    assert_eq!(
        machine.take_events(),
        vec![
            Event::Frame {
                event: FrameEvent::Enter,
                function: FunctionId(1),
            },
            Event::Frame {
                event: FrameEvent::Enter,
                function: FunctionId(0),
            },
            Event::Panic {
                point: TestProgram::point(0, 0),
                ty: Some(TypeId(0)),
            },
            Event::Frame {
                event: FrameEvent::Exit,
                function: FunctionId(0),
            },
            Event::Frame {
                event: FrameEvent::Exit,
                function: FunctionId(1),
            },
        ]
    );
}
