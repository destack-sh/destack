use tspp_repository::RuntimeOptions;

use crate::binding::BindingTable;
use crate::diagnostic::{RuntimeError, RuntimeFailure};
use crate::tests::{TestProgram, TestWorker};
use crate::worker::Request;

/// Terminate one active task at a runtime poll and release its retained execution.
#[test]
fn test_terminate_polled_task() {
    let program = TestProgram::mir(
        r#"
export function task(v0: int32): void {
entry(v0: int32):
    poll
    return
}
"#,
    )
    .build();
    let mut worker = TestWorker::bytecode(&RuntimeOptions::default(), program, BindingTable::new());
    worker.enqueue_task("task", 0);
    worker.request(Request::Terminate);

    // terminate the active task and release its retained activation
    let error = worker
        .run_task()
        .expect_err("termination should end the active task");
    assert_eq!(
        *error,
        RuntimeError::Runtime {
            reason: RuntimeFailure::Terminated,
        }
    );
    assert!(!worker.has_pending_work());
}
