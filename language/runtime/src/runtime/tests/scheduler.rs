use crate::runtime::engine::{EngineContinuation, NativeContinuation, RuntimeValue};
use crate::runtime::scheduler::{Task, TaskId, TaskStatus};

use super::tests::{TestEngine, test_runtime};

/// Executes one queued task in one tick.
#[test]
fn test_tick_once_executes_one_task() {
    // create runtime state with one queued task
    let mut runtime = test_runtime();
    runtime.event_loop.enqueue_task(Task {
        id: TaskId::new(7),
        runnable: EngineContinuation::Native(NativeContinuation::new(1)),
        resume_value: RuntimeValue::VOID,
        status: TaskStatus::Ready,
        priority: 0,
    });

    // execute one tick and verify one resume
    let mut engine = TestEngine::default();
    let progressed = runtime
        .tick_once(&mut engine)
        .expect("tick should execute one runnable");
    assert!(progressed, "tick should report progress");
    assert_eq!(engine.resume_calls, 1, "one task should be resumed once");
}

/// Drains queued tasks and re-yields until idle.
#[test]
fn test_tick_until_idle_drains_yielded_tasks() {
    // create runtime state with one queued task
    let mut runtime = test_runtime();
    runtime.event_loop.enqueue_task(Task {
        id: TaskId::new(11),
        runnable: EngineContinuation::Native(NativeContinuation::new(9)),
        resume_value: RuntimeValue::VOID,
        status: TaskStatus::Ready,
        priority: 0,
    });

    // run ticks until the queue is drained
    let mut engine = TestEngine::default();
    runtime
        .tick_until_idle(&mut engine)
        .expect("tick until idle should drain yielded tasks");

    // verify the yielded continuation was resumed and then completed
    assert_eq!(engine.resume_calls, 2, "yielded task should resume twice");
    assert!(
        !runtime.event_loop.has_pending_work(),
        "event loop should be idle after draining tasks"
    );
}
