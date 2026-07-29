use destack_program as program;
use destack_program::native::{NativeContext, NativeExitCode, NativeExitKind, NativeRuntimeStatus};
use destack_repository::RuntimeOptions;
use destack_vm as vm;

use crate::binding::{Binding, BindingTable, ReplayPayload};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::family_name;
use crate::machine::Engine;
use crate::machine::native::{Call, Code, Image};
use crate::tests::{TestProgram, TestWorker, TestWorld};
use crate::worker::{Activation, WorkerRunOutcome};
use crate::world::{FrameSource, RunOutcome, View};

/// Execute one Program implementation attached to a binding identity.
#[test]
fn test_execute_binding_definition() {
    let program = TestProgram::mir(
        r#"
@binding("runtime.touch", { provider: "runtime", effect: "pure", affinity: "worker" })
export function task(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = int.add v0, v1
    return v2
}
"#,
    )
    .build();
    let mut worker = TestWorker::build(
        &RuntimeOptions::default(),
        program,
        BindingTable::new(),
        Engine::vm(vm::MachineLimits::test()),
    );

    let value = worker
        .run_entrypoint("task", 41)
        .expect("program binding implementation should execute");

    assert_eq!(value.words(), &[program::Word::int32(42)]);
}

/// Execute one linked binding through the bytecode machine and worker runtime table.
#[test]
fn test_execute_vm_binding() {
    let program = touch_program();
    let mut worker = TestWorker::build(
        &RuntimeOptions::default(),
        program,
        bindings(),
        Engine::vm(vm::MachineLimits::test()),
    );

    let value = worker
        .run_entrypoint("task", 41)
        .expect("runtime binding should execute");
    assert_eq!(value.words(), &[program::Word::int32(42)]);

    // preserve the exact runtime failure outside language panic and unwind
    let error = worker
        .run_entrypoint("task", 13)
        .expect_err("runtime binding failure should propagate");
    assert_eq!(
        *error,
        RuntimeError::Internal {
            message: "runtime.touch rejected 13".to_string(),
        }
    );
}

/// Execute one binding selected by the active target family.
#[test]
fn test_execute_binding_family() {
    let source = format!(
        r#"
@binding("runtime.touch", {{ provider: "runtime", effect: "pure", affinity: "worker", families: ["{}"], hosts: ["unavailable"] }})
external function touch(int32): int32

export function task(v0: int32): int32 {{
entry(v0: int32):
    v1: int32 = call touch(v0): (int32) => int32
    return v1
}}
"#,
        family_name()
    );
    let program = TestProgram::mir(&source).build();
    let mut worker = TestWorker::build(
        &RuntimeOptions::default(),
        program,
        bindings(),
        Engine::vm(vm::MachineLimits::test()),
    );

    let value = worker
        .run_entrypoint("task", 41)
        .expect("target family should select runtime binding");

    assert_eq!(value.words(), &[program::Word::int32(42)]);
}

/// Execute one linked binding through native code and the same worker runtime table.
#[test]
fn test_execute_native_binding() {
    let program = touch_program();
    let task = program
        .function_id_by_name("task")
        .expect("native test task should link");
    let [binding] = program.bindings() else {
        panic!("native test should link one runtime binding");
    };
    assert_eq!(binding.function, program::FunctionId(1));
    let mut code = Code::new(Image::resident(), Vec::new());
    code.set_entry(task, native_task);
    let mut worker = TestWorker::build(
        &RuntimeOptions::default(),
        program,
        bindings(),
        Engine::native(code),
    );

    let value = worker
        .run_entrypoint("task", 41)
        .expect("native runtime binding should execute");
    assert_eq!(value.words(), &[program::Word::int32(42)]);

    // preserve the same runtime failure across the native ABI callback
    let error = worker
        .run_entrypoint("task", 13)
        .expect_err("native runtime binding failure should propagate");
    assert_eq!(
        *error,
        RuntimeError::Internal {
            message: "runtime.touch rejected 13".to_string(),
        }
    );
}

/// Cancel one awaited continuation through its runtime waiter.
#[test]
fn test_cancel_await() {
    let program = TestProgram::mir(
        r#"
function cancel(v0: int32, v1: waiter<int32>): void {
entry(v0: int32, v1: waiter<int32>):
    v2: boolean = waiter.cancel v1
    v3: boolean = waiter.queue v1, v0
    return
}

export async function task(v0: int32): int32 {
entry(v0: int32):
    await cancel(v0) => resumed | cancelled | unwind

resumed(v1: int32):
    return v1

cancelled:
    return

unwind:
    unwind.resume
}
"#,
    );
    let mut worker = TestWorker::build(
        &RuntimeOptions::default(),
        program.build(),
        BindingTable::new(),
        Engine::vm(vm::MachineLimits::test()),
    );
    worker.enqueue_task("task", 7);

    // park the task and queue its cancellation from the concrete awaitable
    assert!(worker.run());
    assert!(worker.has_pending_work());

    // run cancellation cleanup and exhaust the event loop
    let outcome = worker
        .run_microtask()
        .expect("cancellation cleanup should execute");
    assert!(matches!(outcome, WorkerRunOutcome::Progressed { .. }));
    assert!(!worker.has_pending_work());
    assert!(!worker.run());
}

/// Resume and settle one eagerly started task through a runtime waiter.
#[test]
fn test_run_eager_task() {
    let program = TestProgram::mir(
        r#"
type Task = newtype<uint64>;

function park(v0: int32, v1: waiter<int32>): void {
entry(v0: int32, v1: waiter<int32>):
    v2: boolean = waiter.queue v1, v0
    v3: boolean = waiter.cancel v1
    return
}

async function body(v0: int32): int32 {
entry(v0: int32):
    await park(v0) => resumed | cancelled | unwind

resumed(v1: int32):
    return v1

cancelled:
    return

unwind:
    unwind.resume
}

export function task(v0: int32): void {
entry(v0: int32):
    v1: continuation<void, never, int32> = continuation.new body(v0)
    v2: Task = task.start v1
    task.detach v2
    return
}
"#,
    );
    let mut runtime = TestWorld::build(&RuntimeOptions::default(), program);
    let worker_id = runtime.default_worker_id();
    runtime.enqueue_task(worker_id, "task", 47);

    // start the task, suspend its body, and queue its retained continuation
    assert_eq!(runtime.run_task(), RunOutcome::Progressed);

    // inspect the queued continuation through the captured world image
    let view = runtime
        .world_mut()
        .view(View::Now)
        .expect("suspended task world should be inspectable");
    let frames = view
        .worker_frames(worker_id)
        .expect("suspended task frames should be inspectable");
    assert_eq!(frames.len(), 1);
    assert!(matches!(frames[0].source, FrameSource::Microtask { .. }));
    assert_eq!(frames[0].bytes(), 47_i32.to_le_bytes());

    // resume the body, settle the detached task, and release its runtime state
    assert_eq!(runtime.run_microtask(), RunOutcome::Progressed);
    assert_eq!(runtime.run_task(), RunOutcome::Idle);
}

/// Await one already completed task through the real task and waiter tables.
#[test]
fn test_await_resolved_task() {
    let program = TestProgram::mir(
        r#"
type Task = newtype<uint64>;

function park(v0: Task, v1: waiter<int32>): void {
entry(v0: Task, v1: waiter<int32>):
    task.park v0, v1
    return
}

export async function task(v0: int32): int32 {
entry(v0: int32):
    v1: Task = task.resolve v0
    await park(v1) => resumed | cancelled | unwind

resumed(v2: int32):
    return v2

cancelled:
    return

unwind:
    unwind.resume
}
"#,
    );
    let mut worker = TestWorker::build(
        &RuntimeOptions::default(),
        program.build(),
        BindingTable::new(),
        Engine::vm(vm::MachineLimits::test()),
    );
    worker.enqueue_task("task", 59);

    // park the await continuation on the retained completed task value
    assert!(worker.run());
    assert!(worker.has_pending_work());

    // deliver the retained value exactly once and release the task handle
    let outcome = worker
        .run_microtask()
        .expect("completed task waiter should resume");
    assert!(matches!(outcome, WorkerRunOutcome::Progressed { .. }));
    assert!(!worker.has_pending_work());
}

/// Cancel one eagerly started task through its asynchronous cleanup edge.
#[test]
fn test_cancel_eager_task() {
    let program = TestProgram::mir(
        r#"
type Task = newtype<uint64>;

function park(v0: int32, v1: waiter<int32>): void {
entry(v0: int32, v1: waiter<int32>):
    return
}

async function body(v0: int32): int32 {
entry(v0: int32):
    await park(v0) => resumed | cancelled | unwind

resumed(v1: int32):
    return v1

cancelled:
    return

unwind:
    unwind.resume
}

export function task(v0: int32): void {
entry(v0: int32):
    v1: continuation<void, never, int32> = continuation.new body(v0)
    v2: Task = task.start v1
    task.cancel v2
    task.detach v2
    return
}
"#,
    );
    let mut worker = TestWorker::build(
        &RuntimeOptions::default(),
        program.build(),
        BindingTable::new(),
        Engine::vm(vm::MachineLimits::test()),
    );
    worker.enqueue_task("task", 53);

    // suspend the task and queue cancellation before its owner returns
    assert!(worker.run());
    assert!(worker.has_pending_work());

    // run cancellation cleanup and release detached task state
    let outcome = worker
        .run_microtask()
        .expect("task cancellation cleanup should execute");
    assert!(matches!(outcome, WorkerRunOutcome::Progressed { .. }));
    assert!(!worker.has_pending_work());
}

/// Return 42 from one exact int32 argument.
fn touch(
    _activation: &mut Activation<'_>,
    _memory: program::Memory<'_>,
    arguments: &[program::Word],
    result: &mut [program::Word],
) -> RuntimeResult<()> {
    assert_eq!(arguments.len(), 1);
    assert_eq!(result.len(), 1);

    // reject one value to exercise exact runtime error propagation
    if arguments[0] != program::Word::int32(41) {
        return Err(RuntimeError::Internal {
            message: format!("runtime.touch rejected {}", arguments[0].bits()),
        }
        .boxed());
    }
    result[0] = program::Word::int32(42);

    Ok(())
}

/// Build the runtime binding table shared by machine integration tests.
fn bindings() -> BindingTable {
    let mut bindings = BindingTable::new();
    bindings.upsert(Binding::new(
        program::BindingId::from_static_name("runtime.touch"),
        ReplayPayload::Results,
        touch,
    ));

    bindings
}

/// Build the external runtime binding program shared by machine tests.
fn touch_program() -> program::Program {
    TestProgram::mir(
        r#"
@binding("runtime.touch", { provider: "runtime", effect: "pure", affinity: "worker" })
external function touch(int32): int32

export function task(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call touch(v0): (int32) => int32
    return v1
}
"#,
    )
    .build()
}

/// Call the runtime binding imported by one resident native entry.
unsafe extern "C" fn native_task(
    context: *mut NativeContext,
    arguments: *const program::Word,
    result: *mut program::Word,
) -> NativeExitCode {
    // SAFETY: the runtime supplies one active native context and exact ABI value ranges
    let status = unsafe {
        (Call::binding_entry())(context, program::FunctionId(1), arguments, 1, result, 1)
    };
    if status == NativeRuntimeStatus::Continue.code() {
        NativeExitKind::Completed.code()
    } else {
        // SAFETY: the runtime context owns one live exit record
        unsafe { (*(*context).exit).kind }
    }
}
