use tspp_program as program;
use tspp_repository::RuntimeOptions;

use crate::binding::BindingTable;
use crate::machine::native::{Loader, Platform};
use crate::tests::{TestProgram, TestWorker};
use crate::worker::{Request, RunnableProgress, WorkerRunOutcome};

/// Retain, inspect, and resume the same task at one runtime poll.
#[test]
fn test_inspect_polled_task() {
    let program = TestProgram::mir(
        r#"
export function task(v0: int32): int32 {
entry(v0: int32):
    poll
    v1: int32 = 1
    v2: int32 = add v0, v1
    return v2
}
"#,
    )
    .build();
    let mut worker = TestWorker::bytecode(&RuntimeOptions::default(), program, BindingTable::new());
    let task_id = worker.enqueue_task("task", 41);
    worker.request(Request::Pause);

    // retain the active task at the exact poll point
    let stopped = worker
        .run_task()
        .expect("inspection should retain the active task");
    let WorkerRunOutcome::Stopped {
        fiber_id,
        reason: program::StopReason::Pause { point },
    } = stopped
    else {
        panic!("inspection should stop at the runtime poll");
    };
    assert_eq!(fiber_id, program::FiberId::new(0, 1));
    assert_eq!(point.operation, 1);

    // resume the same task without selecting another runnable
    let resumed = worker
        .resume()
        .expect("retained task should resume to completion");
    assert_eq!(
        resumed,
        WorkerRunOutcome::Progressed {
            progress: RunnableProgress::Task { task_id },
        }
    );
}

/// Retain native execution at a poll and resume its canonical frame through bytecode.
#[test]
fn test_inspect_native_task() {
    let program = TestProgram::mir(
        r#"
export function task(v0: int32): int32 {
entry(v0: int32):
    poll
    v1: int32 = 1
    v2: int32 = add v0, v1
    return v2
}
"#,
    )
    .compile_native()
    .build();
    let code = Platform
        .load(&program)
        .expect("native inspection program should load");
    let mut worker = TestWorker::native(
        &RuntimeOptions::default(),
        program,
        BindingTable::new(),
        code,
    );
    let task_id = worker.enqueue_task("task", 41);
    worker.request(Request::Pause);

    // capture the native stack value under the exact canonical frame layout
    let stopped = worker
        .run_task()
        .expect("native inspection should retain the active task");
    let WorkerRunOutcome::Stopped {
        fiber_id,
        reason: program::StopReason::Pause { point },
    } = stopped
    else {
        panic!("native inspection should stop at the runtime poll");
    };
    assert_eq!(fiber_id, program::FiberId::new(0, 1));
    assert_eq!(point.operation, 1);

    // restore the native frame through bytecode and finish the same task
    let resumed = worker
        .resume()
        .expect("retained native task should resume through bytecode");
    assert_eq!(
        resumed,
        WorkerRunOutcome::Progressed {
            progress: RunnableProgress::Task { task_id },
        }
    );
}

/// Retain a native breakpoint and resume after its operation through bytecode.
#[test]
fn test_inspect_native_instruction() {
    let program = TestProgram::mir(
        r#"
export function task(v0: int32): int32 {
entry(v0: int32):
    breakpoint
    return v0
}
"#,
    )
    .compile_native()
    .build();
    let code = Platform
        .load(&program)
        .expect("native breakpoint program should load");
    let mut worker = TestWorker::native(
        &RuntimeOptions::default(),
        program,
        BindingTable::new(),
        code,
    );
    let task_id = worker.enqueue_task("task", 41);

    // retain the breakpoint operation while capturing its following frame state
    let stopped = worker
        .run_task()
        .expect("native breakpoint should retain the active task");
    let WorkerRunOutcome::Stopped {
        fiber_id,
        reason: program::StopReason::Instruction { point },
    } = stopped
    else {
        panic!("native breakpoint should stop at its instruction");
    };
    assert_eq!(fiber_id, program::FiberId::new(0, 1));
    assert_eq!(point.operation, 0);

    // resume from the following operation without executing the breakpoint again
    let resumed = worker
        .resume()
        .expect("native breakpoint should resume through bytecode");
    assert_eq!(
        resumed,
        WorkerRunOutcome::Progressed {
            progress: RunnableProgress::Task { task_id },
        }
    );
}
