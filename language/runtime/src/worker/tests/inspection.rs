use std::sync::Arc;

use destack_native as native;
use destack_native::abi;
use destack_program as program;
use destack_repository::RuntimeOptions;

use crate::binding::BindingTable;
use crate::machine::native::{Call, Code, Function, Image, ModuleTable};
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
    v2: int32 = int.add v0, v1
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
        reason: program::StopReason::Pause { point },
    } = stopped
    else {
        panic!("inspection should stop at the runtime poll");
    };
    assert_eq!(point.operation, 1);

    // resume the same task without selecting another runnable
    let resumed = worker
        .continue_stop()
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
    let value = native::FrameValueBuilder::new().locations([native::FrameLocation::new(
        native::FrameSource::Stack,
        0,
        0,
        4,
    )]);
    let frame = native::FrameMapBuilder::new(0, 0, 0, 0, 1).values([value]);
    let map = native::CodeMapBuilder::new().frames([frame]);
    let program = TestProgram::mir(
        r#"
export function task(v0: int32): int32 {
entry(v0: int32):
    poll
    v1: int32 = 1
    v2: int32 = int.add v0, v1
    return v2
}
"#,
    )
    .native(map)
    .build();
    let task = program
        .function_id_by_name("task")
        .expect("native test task should link");
    let frames = program
        .native()
        .expect("native test program should retain native code")
        .map;
    let mut code = Code::new(
        Image::resident(),
        Arc::new(ModuleTable::empty()),
        frames,
        Vec::new(),
    );
    code.set_function(Function::new(task, native_poll_task));
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
        reason: program::StopReason::Pause { point },
    } = stopped
    else {
        panic!("native inspection should stop at the runtime poll");
    };
    assert_eq!(point.operation, 1);

    // restore the native frame through bytecode and finish the same task
    let resumed = worker
        .continue_stop()
        .expect("retained native task should resume through bytecode");
    assert_eq!(
        resumed,
        WorkerRunOutcome::Progressed {
            progress: RunnableProgress::Task { task_id },
        }
    );
}

/// Poll one resident native frame whose first argument remains live on the stack.
unsafe extern "C" fn native_poll_task(
    activation: *mut abi::Activation,
    arguments: *const u64,
    _result: *mut u64,
) -> abi::ExitCode {
    // SAFETY: the runtime supplies one int32 argument in a complete ABI word
    let value = unsafe { arguments.read() };
    let anchor = std::ptr::from_ref(&value).cast::<u8>();
    // SAFETY: frame map zero describes the live stack word anchored above
    let status = unsafe { (Call::poll_entry())(activation, 0, anchor) };
    if status == abi::RuntimeStatus::Continue.code() {
        abi::ExitKind::Completed.code()
    } else {
        // SAFETY: the runtime activation owns one live exit record
        unsafe { (*(*activation).exit).kind }
    }
}
