use std::cell::Cell;

use destack_program as program;
use destack_repository::RuntimeOptions;

use crate::binding::{Binding, BindingTable, ReplayPayload};
use crate::diagnostic::RuntimeResult;
use crate::tests::{TestProgram, TestWorker};
use crate::worker::{Activation, WorkerRunOutcome};

/// Fiber binding declarations shared by every fiber integration program.
const FIBER_BINDINGS: &str = r#"
@binding("destack.fiber.current", { provider: "runtime", effect: "deterministic", affinity: "worker" })
external function fiberCurrent(): uint64

@binding("destack.fiber.wake", { provider: "runtime", effect: "deterministic", affinity: "worker" })
external function fiberWake(uint64, int32): int32

@binding("destack.fiber.park", { provider: "runtime", effect: "deterministic", affinity: "worker", park: true })
external function fiberPark(): int32
"#;

/// Settle one park in place through a wake buffered before it.
#[test]
fn test_park_consumes_buffered_wake() {
    let source = format!(
        r#"{FIBER_BINDINGS}
export function task(v0: int32): int32 {{
entry(v0: int32):
    v1: uint64 = call fiberCurrent(): () => uint64
    v2: int32 = call fiberWake(v1, v0): (uint64, int32) => int32
    v3: int32 = call fiberPark(): () => int32
    return v3
}}
"#
    );
    let program = TestProgram::mir(&source).build();
    let mut worker = TestWorker::bytecode(
        &RuntimeOptions::default(),
        program,
        BindingTable::new().with_fiber_bindings(),
    );

    let value = worker
        .run_entrypoint("task", 47)
        .expect("buffered wake should settle the park in place");
    assert_eq!(value.words(), &[program::Word::int32(47)]);
    assert!(!worker.has_pending_work());
}

/// Park one fiber and resume it through a wake from another task.
#[test]
fn test_wake_resumes_parked_fiber() {
    let source = format!(
        r#"{FIBER_BINDINGS}
@binding("test.fiber.stash", {{ provider: "runtime", effect: "deterministic", affinity: "worker" }})
external function stash(uint64): int32

@binding("test.fiber.take", {{ provider: "runtime", effect: "deterministic", affinity: "worker" }})
external function take(): uint64

@binding("test.observe", {{ provider: "runtime", effect: "deterministic", affinity: "worker" }})
external function observe(int32): int32

export function sleeper(v0: int32): int32 {{
entry(v0: int32):
    v1: uint64 = call fiberCurrent(): () => uint64
    v2: int32 = call stash(v1): (uint64) => int32
    v3: int32 = call fiberPark(): () => int32
    v4: int32 = call observe(v3): (int32) => int32
    return v3
}}

export function waker(v0: int32): int32 {{
entry(v0: int32):
    v1: uint64 = call take(): () => uint64
    v2: int32 = call fiberWake(v1, v0): (uint64, int32) => int32
    return v0
}}
"#
    );
    let program = TestProgram::mir(&source).build();
    let mut worker = TestWorker::bytecode(&RuntimeOptions::default(), program, bindings());
    OBSERVED.set(0);

    // park the sleeper across its runnable
    worker.enqueue_task("sleeper", 1);
    assert!(worker.run());
    assert!(worker.has_pending_work());
    assert_eq!(OBSERVED.get(), 0);

    // deliver the wake from a second task
    worker.enqueue_task("waker", 99);
    assert!(worker.run());

    // the queued wake microtask resumes the sleeper with the delivered value
    let outcome = worker
        .run_microtask()
        .expect("wake resumption should execute");
    assert!(matches!(outcome, WorkerRunOutcome::Progressed { .. }));
    assert_eq!(OBSERVED.get(), 99);
    assert!(!worker.has_pending_work());
}

/// Run one spawned thunk on its own fresh fiber.
#[test]
fn test_spawn_runs_thunk_on_fresh_fiber() {
    let source = format!(
        r#"{FIBER_BINDINGS}
@binding("destack.fiber.spawn", {{ provider: "runtime", effect: "deterministic", affinity: "worker" }})
external function fiberSpawn(fn() => void): int32

@binding("test.observe", {{ provider: "runtime", effect: "deterministic", affinity: "worker" }})
external function observe(int32): int32

function thunk(): void {{
entry:
    v0: int32 = 73
    v1: int32 = call observe(v0): (int32) => int32
    return
}}

export function task(v0: int32): int32 {{
entry(v0: int32):
    v1: fn() => void = function.address thunk
    v2: int32 = call fiberSpawn(v1): (fn() => void) => int32
    return v0
}}
"#
    );
    let program = TestProgram::mir(&source).build();
    let mut worker = TestWorker::bytecode(&RuntimeOptions::default(), program, bindings());
    OBSERVED.set(0);

    // spawning queues the thunk without running it inline
    worker.enqueue_task("task", 1);
    assert!(worker.run());
    assert_eq!(OBSERVED.get(), 0);
    assert!(worker.has_pending_work());

    // the spawned fiber runs as its own task
    assert!(worker.run());
    assert_eq!(OBSERVED.get(), 73);
    assert!(!worker.has_pending_work());
}

/// Split one detached thunk at its park and resume it independently.
#[test]
fn test_detach_splits_at_park() {
    let source = format!(
        r#"{FIBER_BINDINGS}
@binding("test.fiber.stash", {{ provider: "runtime", effect: "deterministic", affinity: "worker" }})
external function stash(uint64): int32

@binding("test.fiber.take", {{ provider: "runtime", effect: "deterministic", affinity: "worker" }})
external function take(): uint64

@binding("test.observe", {{ provider: "runtime", effect: "deterministic", affinity: "worker" }})
external function observe(int32): int32

function thunk(): void {{
entry:
    v0: uint64 = call fiberCurrent(): () => uint64
    v1: int32 = call stash(v0): (uint64) => int32
    v2: int32 = call fiberPark(): () => int32
    v3: int32 = call observe(v2): (int32) => int32
    return
}}

export function task(v0: int32): int32 {{
entry(v0: int32):
    v1: fn() => void = function.address thunk
    call.detach v1
    v2: int32 = 7
    v3: int32 = call observe(v2): (int32) => int32
    return v0
}}

export function waker(v0: int32): int32 {{
entry(v0: int32):
    v1: uint64 = call take(): () => uint64
    v2: int32 = call fiberWake(v1, v0): (uint64, int32) => int32
    return v0
}}
"#
    );
    let program = TestProgram::mir(&source).build();
    let mut worker = TestWorker::bytecode(&RuntimeOptions::default(), program, bindings());
    OBSERVED.set(0);

    // the thunk runs inline to its park, then the caller continues past the boundary
    worker.enqueue_task("task", 1);
    assert!(worker.run());
    assert_eq!(OBSERVED.get(), 7);
    assert!(worker.has_pending_work());

    // waking the stashed identity resumes only the split-off execution
    worker.enqueue_task("waker", 99);
    assert!(worker.run());
    let outcome = worker
        .run_microtask()
        .expect("split-off resumption should execute");
    assert!(matches!(outcome, WorkerRunOutcome::Progressed { .. }));
    assert_eq!(OBSERVED.get(), 99);
    assert!(!worker.has_pending_work());
}

thread_local! {
    /// Fiber handle carried between test tasks.
    static STASHED: Cell<u64> = const { Cell::new(0) };
    /// Value observed by resumed test code.
    static OBSERVED: Cell<i64> = const { Cell::new(0) };
}

/// Build the fiber and observation bindings used by these tests.
fn bindings() -> BindingTable {
    let mut bindings = BindingTable::new().with_fiber_bindings();
    bindings.upsert(Binding::new(
        program::BindingId::from_static_name("test.fiber.stash"),
        ReplayPayload::ArgumentsAndResults,
        stash,
    ));
    bindings.upsert(Binding::new(
        program::BindingId::from_static_name("test.fiber.take"),
        ReplayPayload::Results,
        take,
    ));
    bindings.upsert(Binding::new(
        program::BindingId::from_static_name("test.observe"),
        ReplayPayload::ArgumentsAndResults,
        observe,
    ));

    bindings
}

/// Retain one fiber handle for a later test task.
fn stash(
    _activation: &mut Activation<'_>,
    _memory: program::Memory<'_>,
    _context: program::Context,
    _fiber_id: Option<program::FiberId>,
    _declaration: &program::Binding,
    arguments: &[program::Word],
    _result: &mut [program::Word],
) -> RuntimeResult<()> {
    STASHED.set(arguments[0].bits());

    Ok(())
}

/// Return the retained fiber handle.
fn take(
    _activation: &mut Activation<'_>,
    _memory: program::Memory<'_>,
    _context: program::Context,
    _fiber_id: Option<program::FiberId>,
    _declaration: &program::Binding,
    _arguments: &[program::Word],
    result: &mut [program::Word],
) -> RuntimeResult<()> {
    result[0] = program::Word::from_bits(STASHED.get());

    Ok(())
}

/// Record one observed value.
fn observe(
    _activation: &mut Activation<'_>,
    _memory: program::Memory<'_>,
    _context: program::Context,
    _fiber_id: Option<program::FiberId>,
    _declaration: &program::Binding,
    arguments: &[program::Word],
    _result: &mut [program::Word],
) -> RuntimeResult<()> {
    OBSERVED.set(arguments[0].bits() as i64);

    Ok(())
}
