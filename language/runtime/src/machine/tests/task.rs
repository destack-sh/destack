use destack_repository::RuntimeOptions;

use crate::binding::BindingTable;
use crate::tests::{TestProgram, TestWorker, TestWorld};
use crate::worker::WorkerRunOutcome;
use crate::world::{FrameSource, RunOutcome, View};

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
    let mut worker = TestWorker::bytecode(
        &RuntimeOptions::default(),
        program.build(),
        BindingTable::new(),
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
    let mut worker = TestWorker::bytecode(
        &RuntimeOptions::default(),
        program.build(),
        BindingTable::new(),
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
    let mut worker = TestWorker::bytecode(
        &RuntimeOptions::default(),
        program.build(),
        BindingTable::new(),
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
