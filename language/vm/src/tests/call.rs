use crate::diagnostic::{Error, ImportError, Trap};
use crate::tests::{
    TestMachine, assert_runtime_error_matches, run_mir, run_mir_expect, run_mir_expect_error,
};
use destack_mir::{Space, TraceMap, Type};
use destack_program::{CallDispatch, DynamicTableId, Value};

/// function.address produces a function pointer for call.indirect.
#[test]
fn test_function_addr_indirect_call() {
    let mir = r#"
function add(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}

function caller(v0: int32): int32 {
entry(v0: int32):
    v1: fn(int32) => int32 = function.address add
    v2: int32 = call.indirect v1(v0): (int32) => int32
    return v2
}
"#;
    run_mir_expect(mir, "caller", &[Value::int32(21)], Value::int32(42));
}

/// Dynamic values carry their payload and resolve calls through one witness table.
#[test]
fn test_dynamic_bind_projects_and_dispatches() {
    let mir = r#"
type Reader {
    read: fn(ref<ReaderImpl, managed, readonly>) => int32;
}

type ReaderImpl {
    value: int32;
}

function ReaderImpl.read(v0: ref<ReaderImpl, managed, readonly>): int32 {
entry(v0: ref<ReaderImpl, managed, readonly>):
    v1: ReaderImpl = load v0
    v2: int32 = field.get v1, 0
    return v2
}

function run(): int32 {
entry:
    v0: int32 = 42
    v1: ReaderImpl = aggregate (v0)
    v2: ref<ReaderImpl, managed, readonly> = new.zeroed ReaderImpl
    store v2, v1
    v3: dynamic<Reader> = dynamic.bind v2, ReaderImpl
    v4: ref<void, managed, readonly> = dynamic.payload v3
    v5: int32 = call.dynamic v3, Reader, 0(v4): (ref<void, managed, readonly>) => int32
    return v5
}

function concreteType(): typeId {
entry:
    v0: ref<ReaderImpl, managed, readonly> = new.zeroed ReaderImpl
    v1: dynamic<Reader> = dynamic.bind v0, ReaderImpl
    v2: typeId = dynamic.type v1
    return v2
}

function invokeRun(): int32 {
entry:
    v0: int32 = 43
    v1: ReaderImpl = aggregate (v0)
    v2: ref<ReaderImpl, managed, readonly> = new.zeroed ReaderImpl
    store v2, v1
    v3: dynamic<Reader> = dynamic.bind v2, ReaderImpl
    v4: ref<void, managed, readonly> = dynamic.payload v3
    invoke.dynamic v3, Reader, 0(v4): (ref<void, managed, readonly>) => int32 => completed | cleanup
completed(v5: int32):
    return v5
cleanup:
    unwind.resume
}

function tailRun(): int32 {
entry:
    v0: int32 = 44
    v1: ReaderImpl = aggregate (v0)
    v2: ref<ReaderImpl, managed, readonly> = new.zeroed ReaderImpl
    store v2, v1
    v3: dynamic<Reader> = dynamic.bind v2, ReaderImpl
    v4: ref<void, managed, readonly> = dynamic.payload v3
    tail.call.dynamic v3, Reader, 0(v4): (ref<void, managed, readonly>) => int32
}

function dropDynamic(): int32 {
entry:
    v0: ref<ReaderImpl, managed, readonly> = new.zeroed ReaderImpl
    v1: dynamic<Reader> = dynamic.bind v0, ReaderImpl
    drop v1
    v2: int32 = 45
    return v2
}
"#;
    let mut machine = TestMachine::dynamic(mir, "ReaderImpl", "Reader", &["ReaderImpl.read"]);

    // record local payload space across every dynamic call mode
    let program = &machine.machine.program;
    let dynamic_calls = program
        .sites()
        .calls(program.sections())
        .iter()
        .filter(|site| site.dispatch == CallDispatch::Dynamic)
        .collect::<Vec<_>>();
    assert_eq!(dynamic_calls.len(), 3);
    assert!(
        dynamic_calls
            .iter()
            .all(|site| site.space.get() == Some(Space::Local))
    );

    // keep each dynamic payload visible to local frame tracing
    let dynamic_types = machine
        .tree
        .iter_nodes::<Type>()
        .filter_map(|(ty, value)| matches!(value, Type::Dynamic { .. }).then_some(ty))
        .map(|ty| machine.program_type(ty))
        .collect::<Vec<_>>();
    for dynamic_type in dynamic_types {
        let layout = program
            .layout(dynamic_type)
            .expect("dynamic layout should exist");
        let trace = program
            .trace_map(layout.trace)
            .expect("dynamic trace map should decode");
        let TraceMap::Fixed {
            local_offsets,
            shared_offsets,
            frame_offsets,
        } = trace
        else {
            panic!("dynamic layout should use one fixed trace map");
        };
        assert_eq!(local_offsets.as_ref(), &[0]);
        assert!(shared_offsets.is_empty());
        assert!(frame_offsets.is_empty());
    }

    // dispatch through the witness while forwarding the projected payload
    let output = machine
        .run_function_by_name("run", &[])
        .expect("dynamic call should complete");
    assert_eq!(output, Value::int32(42));

    // recover concrete identity from the same witness table
    let concrete = machine
        .machine
        .program
        .dynamic_table(DynamicTableId(0))
        .expect("dynamic table should be linked")
        .concrete;
    let output = machine
        .run_function_by_name("concreteType", &[])
        .expect("dynamic type projection should complete");
    assert_eq!(output, Value::uint32(concrete.0));

    // preserve dynamic dispatch across invocation and tail-call transfers
    let output = machine
        .run_function_by_name("invokeRun", &[])
        .expect("dynamic invocation should complete");
    assert_eq!(output, Value::int32(43));
    let output = machine
        .run_function_by_name("tailRun", &[])
        .expect("dynamic tail call should complete");
    assert_eq!(output, Value::int32(44));

    // clear the dynamic carrier without routing through a call target
    let output = machine
        .run_function_by_name("dropDynamic", &[])
        .expect("dynamic drop should complete");
    assert_eq!(output, Value::int32(45));
}

/// Drop passes one live frame value to its destructor.
#[test]
fn test_execute_drop_calls_destructor() {
    let mir = r#"
type Item {
    value: int32;
}

function Item.drop(v0: ref<Item, borrowed, exclusive>): void {
entry(v0: ref<Item, borrowed, exclusive>):
    v1: Item = load v0
    v2: int32 = field.get v1, 0
    v3: int32 = 42
    v4: boolean = int.eq v2, v3
    branch v4, matched, unmatched
matched:
    trap.abort
unmatched:
    return
}

function test(): void {
entry:
    v0: int32 = 42
    v1: Item = aggregate (v0)
    drop v1
    return
}
"#;
    let mut machine = TestMachine::with_drop(mir, "Item", "Item.drop");

    // the abort proves the destructor received the live value address
    machine
        .run_function_by_name("test", &[])
        .expect_err("drop function should abort");
}

/// Invoke enters its normal successor with the returned value.
#[test]
fn test_invoke_enters_normal_successor() {
    let mir = r#"
function double(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}

function caller(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 10int32
    invoke double(v0) => b1(v1) | cleanup
b1(v2: int32, v3: int32):
    v4: int32 = int.add v2, v3
    return v4
cleanup:
    unwind.resume
}"#;

    run_mir_expect(mir, "caller", &[Value::int32(8)], Value::int32(26));
}

/// Invoke enters its unwind successor when the callee panics.
#[test]
fn test_invoke_enters_unwind_successor() {
    let mir = r#"
function fail(): void {
entry:
    panic
}

function caller(): void {
entry:
    invoke fail() => completed | cleanup
completed:
    return
cleanup:
    trap.abort
}
"#;
    let expected = Error::Trap {
        reason: Trap::Abort,
    };

    run_mir_expect_error(mir, "caller", &[], expected);
}

/// Unwind resume propagates the original panic through nested invokes.
#[test]
fn test_unwind_resume_propagates_through_invoke() {
    let mir = r#"
function fail(): void {
entry:
    panic
}

function middle(): void {
entry:
    invoke fail() => completed | cleanup
completed:
    return
cleanup:
    unwind.resume
}

function caller(): void {
entry:
    invoke middle() => completed | cleanup
completed:
    return
cleanup:
    unwind.resume
}
"#;
    let expected = Error::panic("panic");

    run_mir_expect_error(mir, "caller", &[], expected);
}

/// A second panic during cleanup aborts the machine.
#[test]
fn test_panic_during_unwind_aborts() {
    let mir = r#"
function fail(): void {
entry:
    panic
}

function caller(): void {
entry:
    invoke fail() => completed | cleanup
completed:
    return
cleanup:
    panic
}
"#;
    let expected = Error::Trap {
        reason: Trap::Abort,
    };

    run_mir_expect_error(mir, "caller", &[], expected);
}

/// Binding calls fail until runtime dispatch is installed.
#[test]
fn test_call_binding_requires_runtime_dispatch() {
    let mir = r#"
@binding("touch")
external function touch(): void

function caller(): int32 {
b0:
    invoke touch() => b1 | cleanup
b1:
    v0: int32 = 7int32
    return v0
cleanup:
    unwind.resume
}"#;
    let result = run_mir(mir, "caller", &[]);

    assert_runtime_error_matches!(
        result,
        Error::Import {
            ref name,
            reason: ImportError::Forbidden,
        } if name == "touch",
    );
}
