use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::thread::{JoinHandle, ThreadId};
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::runtime::thread::{ExecutionPolicy, start_with_policy};

use super::state::{ExecutorFailure, ExecutorFailureKind, ExecutorState};

/// One fallback wait used when no periodic task is currently registered.
const PERIODIC_IDLE_WAIT: Duration = Duration::from_secs(60);
/// One callback invoked by the periodic executor.
type PeriodicCallback = dyn Fn() -> RuntimeResult<()> + Send + Sync + 'static;

/// One command for the periodic executor thread.
enum PeriodicExecutorCommand {
    /// Register one periodic callback.
    Register {
        /// Stable task id.
        task_id: u64,
        /// Requested callback interval.
        interval: Duration,
        /// Callback invoked on each due tick.
        callback: Arc<PeriodicCallback>,
    },
    /// Remove one registered callback.
    Unregister {
        /// Stable task id.
        task_id: u64,
    },
    /// Shut down the executor thread.
    Shutdown,
}

/// One registered periodic callback.
struct PeriodicTask {
    /// Requested callback interval.
    interval: Duration,
    /// Next deadline for this callback.
    next_run_at: Instant,
    /// Callback invoked on each due tick.
    callback: Arc<PeriodicCallback>,
}

/// One shared periodic service executor.
struct PeriodicExecutor {
    /// Logical executor name for diagnostics.
    name: String,
    /// Command sender for the executor thread.
    sender: Sender<PeriodicExecutorCommand>,
    /// Next stable task id.
    next_task_id: AtomicU64,
    /// Thread identity for re-entrancy guards.
    thread_id: Arc<OnceLock<ThreadId>>,
    /// Shared executor lifecycle state.
    state: Arc<ExecutorState>,
    /// Join handle for deterministic shutdown.
    handle: Mutex<Option<JoinHandle<()>>>,
}

/// One owned registration in the periodic executor.
pub(crate) struct PeriodicTaskHandle {
    /// Executor retained for this task registration lifetime.
    _executor: Arc<PeriodicExecutor>,
    /// Stable task id.
    task_id: u64,
    /// Command sender for task teardown.
    sender: Sender<PeriodicExecutorCommand>,
}

impl std::fmt::Debug for PeriodicTaskHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PeriodicTaskHandle")
            .field("task_id", &self.task_id)
            .finish()
    }
}

impl PeriodicExecutor {
    /// Open one periodic executor thread.
    fn open(name: &str, policy: ExecutionPolicy) -> RuntimeResult<Arc<Self>> {
        let (sender, receiver) = channel::<PeriodicExecutorCommand>();
        let thread_name = name.to_string();
        let thread_id = Arc::new(OnceLock::new());
        let executor_thread_id = thread_id.clone();
        let state = ExecutorState::shared();
        let executor_state = state.clone();
        let panic_state = state.clone();
        let panic_name = thread_name.clone();

        // executor thread
        let handle = start_with_policy(
            thread_name.clone(),
            "platform.service.spawn",
            policy,
            move || {
                let result = panic::catch_unwind(AssertUnwindSafe(|| {
                    Self::periodic_executor_main(
                        &thread_name,
                        receiver,
                        executor_thread_id,
                        executor_state,
                    );
                }));

                if result.is_err() {
                    panic_state.mark_failed(ExecutorFailure::new(
                        ExecutorFailureKind::ThreadPanic,
                        format!("periodic executor {panic_name} panicked unexpectedly"),
                    ));
                }
            },
        )?;

        Ok(Arc::new(Self {
            name: name.to_string(),
            sender,
            next_task_id: AtomicU64::new(1),
            thread_id,
            state,
            handle: Mutex::new(Some(handle)),
        }))
    }

    /// Run the periodic executor command loop.
    fn periodic_executor_main(
        name: &str,
        receiver: Receiver<PeriodicExecutorCommand>,
        thread_id: Arc<OnceLock<ThreadId>>,
        state: Arc<ExecutorState>,
    ) {
        if thread_id.set(thread::current().id()).is_err() {
            state.mark_failed(ExecutorFailure::new(
                ExecutorFailureKind::BootstrapPanic,
                format!("periodic executor {name} thread id was already initialized"),
            ));
            return;
        }

        let mut tasks = HashMap::<u64, PeriodicTask>::new();
        loop {
            // wait for one command or the next due callback deadline
            let timeout = Self::next_wait_duration(&tasks);
            let command = receiver.recv_timeout(timeout);
            match command {
                Ok(PeriodicExecutorCommand::Register {
                    task_id,
                    interval,
                    callback,
                }) => {
                    tasks.insert(
                        task_id,
                        PeriodicTask {
                            interval,
                            next_run_at: Instant::now(),
                            callback,
                        },
                    );
                }
                Ok(PeriodicExecutorCommand::Unregister { task_id }) => {
                    tasks.remove(&task_id);
                }
                Ok(PeriodicExecutorCommand::Shutdown) => {
                    state.mark_stopped();
                    break;
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    state.mark_stopped();
                    break;
                }
            }

            // run one due callback batch
            if let Err(failure) = Self::run_due_tasks(name, &mut tasks) {
                state.mark_failed(failure);
                break;
            }
        }
    }

    /// Return the wait duration until the next periodic task is due.
    fn next_wait_duration(tasks: &HashMap<u64, PeriodicTask>) -> Duration {
        let now = Instant::now();
        let Some(next_run_at) = tasks.values().map(|task| task.next_run_at).min() else {
            return PERIODIC_IDLE_WAIT;
        };

        next_run_at.saturating_duration_since(now)
    }

    /// Run all callbacks that are due at the current instant.
    fn run_due_tasks(
        name: &str,
        tasks: &mut HashMap<u64, PeriodicTask>,
    ) -> Result<(), ExecutorFailure> {
        let now = Instant::now();
        let mut due_callbacks = Vec::new();

        // collect one due callback batch and advance their deadlines
        for task in tasks.values_mut() {
            if task.next_run_at > now {
                continue;
            }

            due_callbacks.push(task.callback.clone());

            let mut next_run_at = task.next_run_at;
            while next_run_at <= now {
                next_run_at += task.interval;
            }

            task.next_run_at = next_run_at;
        }

        // run callbacks outside the task table mutation pass
        for callback in due_callbacks {
            let callback_result = panic::catch_unwind(AssertUnwindSafe(|| callback()));
            match callback_result {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    return Err(ExecutorFailure::new(
                        ExecutorFailureKind::CallbackFailed,
                        format!("periodic executor {name} callback failed: {error}"),
                    ));
                }
                Err(_) => {
                    return Err(ExecutorFailure::new(
                        ExecutorFailureKind::CallbackPanic,
                        format!("periodic executor {name} callback panicked"),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Register one periodic callback.
    pub(crate) fn register(
        self: &Arc<Self>,
        interval: Duration,
        callback: impl Fn() -> RuntimeResult<()> + Send + Sync + 'static,
    ) -> RuntimeResult<PeriodicTaskHandle> {
        // reject registration after one terminal executor transition
        if let Some(error) = self.state.unavailable_error(
            "platform.service.periodic.register",
            &self.name,
            "periodic executor",
        ) {
            return Err(error);
        }

        // reject synchronous self-registration from the executor thread
        if self
            .thread_id
            .get()
            .is_some_and(|thread_id| *thread_id == thread::current().id())
        {
            return Err(core_platform::io_operation_error(
                "platform.service.periodic.register",
                None,
                format!(
                    "periodic executor {} cannot synchronously register from its own thread",
                    self.name
                ),
            ));
        }

        let task_id = self.next_task_id.fetch_add(1, Ordering::Relaxed);
        let callback = Arc::new(callback);

        // register one new periodic callback
        self.sender
            .send(PeriodicExecutorCommand::Register {
                task_id,
                interval: interval.max(Duration::from_nanos(1)),
                callback,
            })
            .map_err(|error| {
                if let Some(error) = self.state.unavailable_error(
                    "platform.service.periodic.register",
                    &self.name,
                    "periodic executor",
                ) {
                    return error;
                }

                core_platform::io_operation_error(
                    "platform.service.periodic.register",
                    None,
                    format!(
                        "failed to register periodic callback on {}: {error}",
                        self.name
                    ),
                )
            })?;

        Ok(PeriodicTaskHandle {
            _executor: Arc::clone(self),
            task_id,
            sender: self.sender.clone(),
        })
    }
}

impl Drop for PeriodicExecutor {
    /// Shut down the periodic executor thread.
    fn drop(&mut self) {
        let _ = self.sender.send(PeriodicExecutorCommand::Shutdown);

        // avoid joining from the periodic executor thread itself
        let is_current_thread = self
            .thread_id
            .get()
            .is_some_and(|thread_id| *thread_id == thread::current().id());
        if is_current_thread {
            return;
        }

        // wait for thread teardown when the join handle is still owned
        let Some(handle) = self.handle.get_mut().take() else {
            return;
        };

        if handle.join().is_err() {
            self.state.mark_failed(ExecutorFailure::new(
                ExecutorFailureKind::ShutdownPanic,
                format!("periodic executor {} panicked during shutdown", self.name),
            ));
            return;
        }

        self.state.mark_stopped();
    }
}

impl Drop for PeriodicTaskHandle {
    /// Unregister one periodic callback.
    fn drop(&mut self) {
        let _ = self.sender.send(PeriodicExecutorCommand::Unregister {
            task_id: self.task_id,
        });
    }
}

/// Open one named periodic task on one dedicated periodic worker.
pub(crate) fn open_periodic_task(
    name: &str,
    policy: ExecutionPolicy,
    interval: Duration,
    callback: impl Fn() -> RuntimeResult<()> + Send + Sync + 'static,
) -> RuntimeResult<PeriodicTaskHandle> {
    let executor = PeriodicExecutor::open(name, policy)?;

    executor.register(interval, callback)
}
