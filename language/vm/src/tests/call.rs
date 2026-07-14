use crate::diagnostic::{Error, ImportError, Trap};
use crate::tests::{
    TestMachine, assert_runtime_error_matches, run_mir, run_mir_expect, run_mir_expect_error,
};
use destack_program::{DynamicTableId, Value};

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
    v3: ref<void, managed, readonly> = cast.bit v2 -> ref<void, managed, readonly>
    v4: dynamic<Reader> = dynamic.bind v3, ReaderImpl
    v5: ref<ReaderImpl, managed, readonly> = dynamic.payload v4
    v6: int32 = call.dynamic v4, Reader, 0(v5): (ref<ReaderImpl, managed, readonly>) => int32
    return v6
}

function concreteType(): typeId {
entry:
    v0: ref<ReaderImpl, managed, readonly> = new.zeroed ReaderImpl
    v1: ref<void, managed, readonly> = cast.bit v0 -> ref<void, managed, readonly>
    v2: dynamic<Reader> = dynamic.bind v1, ReaderImpl
    v3: typeId = dynamic.type v2
    return v3
}

function invokeRun(): int32 {
entry:
    v0: int32 = 43
    v1: ReaderImpl = aggregate (v0)
    v2: ref<ReaderImpl, managed, readonly> = new.zeroed ReaderImpl
    store v2, v1
    v3: ref<void, managed, readonly> = cast.bit v2 -> ref<void, managed, readonly>
    v4: dynamic<Reader> = dynamic.bind v3, ReaderImpl
    v5: ref<ReaderImpl, managed, readonly> = dynamic.payload v4
    invoke.dynamic v4, Reader, 0(v5): (ref<ReaderImpl, managed, readonly>) => int32 => completed | cleanup
completed(v6: int32):
    return v6
cleanup:
    unwind.resume
}

function tailRun(): int32 {
entry:
    v0: int32 = 44
    v1: ReaderImpl = aggregate (v0)
    v2: ref<ReaderImpl, managed, readonly> = new.zeroed ReaderImpl
    store v2, v1
    v3: ref<void, managed, readonly> = cast.bit v2 -> ref<void, managed, readonly>
    v4: dynamic<Reader> = dynamic.bind v3, ReaderImpl
    v5: ref<ReaderImpl, managed, readonly> = dynamic.payload v4
    tail.call.dynamic v4, Reader, 0(v5): (ref<ReaderImpl, managed, readonly>) => int32
}

function dropDynamic(): int32 {
entry:
    v0: ref<ReaderImpl, managed, readonly> = new.zeroed ReaderImpl
    v1: ref<void, managed, readonly> = cast.bit v0 -> ref<void, managed, readonly>
    v2: dynamic<Reader> = dynamic.bind v1, ReaderImpl
    drop v2
    v3: int32 = 45
    return v3
}
"#;
    let mut machine = TestMachine::dynamic(mir, "ReaderImpl", "Reader", &["ReaderImpl.read"]);

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
