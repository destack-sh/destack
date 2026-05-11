use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc, OnceLock};
use std::thread;
use std::thread::{JoinHandle, ThreadId};

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::runtime::thread::{ExecutionMode, ExecutionPolicy, ExecutionScope};

use crate::runtime::service::executor::state::{
    ExecutorFailure, ExecutorFailureKind, ExecutorState,
};
use crate::runtime::thread::start_with_policy;

/// One owned worker loop shutdown callback.
pub(crate) type WorkerLoopShutdown = Box<dyn FnOnce() + Send + 'static>;

/// One owned worker loop body.
pub(crate) type WorkerLoopRun = Box<dyn FnOnce() -> RuntimeResult<()> + Send + 'static>;

/// One owned worker loop.
pub(crate) struct WorkerLoop {
    /// Logical worker name for diagnostics.
    name: String,
    /// Shared executor lifecycle state.
    state: Arc<ExecutorState>,
    /// Worker thread identity for shutdown guards.
    thread_id: Arc<OnceLock<ThreadId>>,
    /// Deferred shutdown callback invoked before join.
    shutdown: Mutex<Option<WorkerLoopShutdown>>,
    /// Joined worker loop thread.
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl WorkerLoop {
    /// Open one owned worker loop.
    pub(crate) fn open(
        name: &str,
        operation: &'static str,
        policy: ExecutionPolicy,
        build: impl FnOnce() -> RuntimeResult<(WorkerLoopShutdown, WorkerLoopRun)> + Send + 'static,
    ) -> RuntimeResult<Self> {
        // policy
        if !matches!(
            policy.scope,
            ExecutionScope::Process | ExecutionScope::Resource
        ) {
            return Err(core_platform::invalid_argument(
                "execution.scope",
                "worker loops must be process-scoped or resource-scoped",
            ));
        }

        if policy.mode != ExecutionMode::Loop {
            return Err(core_platform::invalid_argument(
                "execution.mode",
                "worker loops require loop execution",
            ));
        }

        let (ready_tx, ready_rx) = sync_channel::<RuntimeResult<WorkerLoopShutdown>>(1);
        let thread_name = name.to_string();
        let state = ExecutorState::shared();
        let thread_id = Arc::new(OnceLock::new());
        let thread_state = state.clone();
        let panic_state = state.clone();
        let loop_thread_id = thread_id.clone();
        let panic_name = thread_name.clone();

        // worker loop thread
        let handle = start_with_policy(thread_name.clone(), operation, policy, move || {
            let result = panic::catch_unwind(AssertUnwindSafe(|| {
                worker_loop_main(
                    &thread_name,
                    operation,
                    build,
                    ready_tx,
                    loop_thread_id,
                    thread_state,
                );
            }));

            if result.is_err() {
                panic_state.mark_failed(ExecutorFailure::new(
                    ExecutorFailureKind::ThreadPanic,
                    format!("worker loop {panic_name} panicked unexpectedly"),
                ));
            }
        })?;

        // wait for the loop bootstrap result before publishing the handle
        let shutdown = ready_rx.recv().map_err(|error| {
            core_platform::io_operation_error(
                operation,
                None,
                format!("failed to receive worker loop bootstrap status for {name}: {error}"),
            )
        })??;

        Ok(Self {
            name: name.to_string(),
            state,
            thread_id,
            shutdown: Mutex::new(Some(shutdown)),
            handle: Mutex::new(Some(handle)),
        })
    }
}

impl Drop for WorkerLoop {
    /// Shut down the worker loop and join its thread.
    fn drop(&mut self) {
        // request shutdown before joining
        if let Some(shutdown) = self.shutdown.lock().take() {
            shutdown();
        }

        let Some(handle) = self.handle.lock().take() else {
            return;
        };

        // avoid joining from the worker loop thread itself
        if self
            .thread_id
            .get()
            .is_some_and(|thread_id| *thread_id == thread::current().id())
        {
            return;
        }

        if handle.join().is_err() {
            self.state.mark_failed(ExecutorFailure::new(
                ExecutorFailureKind::ShutdownPanic,
                format!("worker loop {} panicked during shutdown", self.name),
            ));
        }
    }
}

/// Bootstrap one worker loop and run it until shutdown.
fn worker_loop_main(
    name: &str,
    operation: &'static str,
    build: impl FnOnce() -> RuntimeResult<(WorkerLoopShutdown, WorkerLoopRun)>,
    ready_tx: std::sync::mpsc::SyncSender<RuntimeResult<WorkerLoopShutdown>>,
    thread_id: Arc<OnceLock<ThreadId>>,
    state: Arc<ExecutorState>,
) {
    // record the thread identity before bootstrap
    if thread_id.set(thread::current().id()).is_err() {
        state.mark_failed(ExecutorFailure::new(
            ExecutorFailureKind::BootstrapPanic,
            format!("worker loop {name} thread id was already initialized"),
        ));

        let error = core_platform::io_operation_error(
            operation,
            None,
            format!("worker loop {name} thread id was already initialized"),
        );
        let _bootstrap_error_delivered = ready_tx.send(Err(error)).is_ok();
        return;
    }

    // initialize the worker loop on its owning thread
    let bootstrap_result = panic::catch_unwind(AssertUnwindSafe(build));

    let run = match bootstrap_result {
        Ok(Ok((shutdown, run))) => {
            if ready_tx.send(Ok(shutdown)).is_err() {
                state.mark_stopped();
                return;
            }

            run
        }
        Ok(Err(error)) => {
            if ready_tx.send(Err(error)).is_err() {
                state.mark_stopped();
            }

            state.mark_stopped();
            return;
        }
        Err(_) => {
            state.mark_failed(ExecutorFailure::new(
                ExecutorFailureKind::BootstrapPanic,
                format!("worker loop {name} panicked during bootstrap"),
            ));

            let error = core_platform::io_operation_error(
                operation,
                None,
                format!("worker loop {name} panicked during bootstrap"),
            );
            let _bootstrap_error_delivered = ready_tx.send(Err(error)).is_ok();
            return;
        }
    };

    // run the worker loop until shutdown or failure
    let run_result = panic::catch_unwind(AssertUnwindSafe(run));
    match run_result {
        Ok(Ok(())) => {
            state.mark_stopped();
        }
        Ok(Err(error)) => {
            state.mark_failed(ExecutorFailure::new(
                ExecutorFailureKind::ThreadFailed,
                format!("worker loop {name} failed: {error}"),
            ));
        }
        Err(_) => {
            state.mark_failed(ExecutorFailure::new(
                ExecutorFailureKind::ThreadPanic,
                format!("worker loop {name} panicked while running"),
            ));
        }
    }
}
