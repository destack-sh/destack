use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc::{Receiver, Sender, channel, sync_channel};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::thread::{JoinHandle, ThreadId};

use parking_lot::Mutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::{self as core_platform};
use crate::runtime::thread::{
    ExecutionAffinity, ExecutionMode, ExecutionPolicy, ExecutionScope, start_with_policy,
};

use super::super::windows::initialize_windows_winrt_mta;
use super::state::{ExecutorFailure, ExecutorFailureKind, ExecutorState};

/// One bootstrap or dispatch command for one service-thread executor.
enum ServiceThreadCommand<S> {
    /// Execute one callback against the service state.
    Run(Box<dyn FnOnce(&mut S) + Send + 'static>),
    /// Shut down the service thread.
    Shutdown,
}

/// One thread-bootstrap guard.
pub(crate) enum ServiceThreadGuard {
    /// No bootstrap teardown is required.
    None,
    /// Uninitialize one owned Windows multithreaded apartment.
    WindowsMta,
}

impl Drop for ServiceThreadGuard {
    /// Tear down one owned service-thread bootstrap.
    fn drop(&mut self) {
        #[cfg(windows)]
        if matches!(self, ServiceThreadGuard::WindowsMta) {
            unsafe {
                windows::Win32::System::WinRT::RoUninitialize();
            }
        }
    }
}

/// One service-thread executor for one runtime-owned service.
pub(crate) struct ServiceThreadExecutor<S> {
    /// Logical service name for diagnostics.
    name: String,
    /// Command sender for one live service thread.
    sender: Sender<ServiceThreadCommand<S>>,
    /// Thread identity for re-entrancy guards.
    thread_id: Arc<OnceLock<ThreadId>>,
    /// Shared executor lifecycle state.
    state: Arc<ExecutorState>,
    /// Join handle for deterministic shutdown.
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl<S> ServiceThreadExecutor<S> {
    /// Spawn one service thread and bootstrap its state.
    pub(crate) fn spawn(
        name: &str,
        policy: ExecutionPolicy,
        build: impl FnOnce() -> RuntimeResult<S> + Send + 'static,
    ) -> RuntimeResult<Self>
    where
        S: 'static,
    {
        // policy
        validate_thread_policy(policy)?;

        let (sender, receiver) = channel::<ServiceThreadCommand<S>>();
        let (ready_tx, ready_rx) = sync_channel::<RuntimeResult<()>>(1);
        let thread_name = name.to_string();
        let thread_id = Arc::new(OnceLock::new());
        let bootstrap_thread_id = thread_id.clone();
        let state = ExecutorState::shared();
        let bootstrap_state = state.clone();
        let panic_state = state.clone();
        let panic_name = thread_name.clone();

        // service thread
        let handle = start_with_policy(
            thread_name.clone(),
            "platform.service.spawn",
            policy,
            move || {
                let result = panic::catch_unwind(AssertUnwindSafe(|| {
                    Self::service_thread_main(
                        &thread_name,
                        policy.affinity,
                        build,
                        receiver,
                        ready_tx,
                        bootstrap_thread_id,
                        bootstrap_state,
                    );
                }));

                if result.is_err() {
                    panic_state.mark_failed(ExecutorFailure::new(
                        ExecutorFailureKind::ThreadPanic,
                        format!("service thread {panic_name} panicked unexpectedly"),
                    ));
                }
            },
        )?;

        // wait for the bootstrap result before publishing the handle
        ready_rx.recv().map_err(|error| {
            core_platform::io_operation_error(
                "platform.service.spawn",
                None,
                format!("failed to receive service bootstrap status for {name}: {error}"),
            )
        })??;

        Ok(Self {
            name: name.to_string(),
            sender,
            thread_id,
            state,
            handle: Mutex::new(Some(handle)),
        })
    }

    /// Bootstrap one service thread and run its command loop.
    fn service_thread_main(
        name: &str,
        affinity: Option<ExecutionAffinity>,
        build: impl FnOnce() -> RuntimeResult<S>,
        receiver: Receiver<ServiceThreadCommand<S>>,
        ready_tx: std::sync::mpsc::SyncSender<RuntimeResult<()>>,
        thread_id: Arc<OnceLock<ThreadId>>,
        state: Arc<ExecutorState>,
    ) where
        S: 'static,
    {
        // record the thread identity before bootstrap
        if thread_id.set(thread::current().id()).is_err() {
            state.mark_failed(ExecutorFailure::new(
                ExecutorFailureKind::BootstrapPanic,
                format!("service thread {name} thread id was already initialized"),
            ));

            let error = core_platform::io_operation_error(
                "platform.service.spawn",
                None,
                format!("service thread {name} thread id was already initialized"),
            );
            let _bootstrap_error_delivered = ready_tx.send(Err(error)).is_ok();
            return;
        }

        // initialize bootstrap state
        let bootstrap_result = panic::catch_unwind(AssertUnwindSafe(|| {
            let thread_guard = Self::initialize_service_thread(name, affinity)?;
            let service_state = build()?;

            Ok::<_, Box<RuntimeError>>((thread_guard, service_state))
        }));

        let (_thread_guard, mut service_state) = match bootstrap_result {
            Ok(Ok(result)) => {
                if ready_tx.send(Ok(())).is_err() {
                    state.mark_stopped();
                    return;
                }

                result
            }
            Ok(Err(error)) => {
                if ready_tx.send(Err(error)).is_err() {
                    state.mark_stopped();
                    return;
                }

                state.mark_stopped();
                return;
            }
            Err(_) => {
                state.mark_failed(ExecutorFailure::new(
                    ExecutorFailureKind::BootstrapPanic,
                    format!("service thread {name} panicked during bootstrap"),
                ));

                let error = core_platform::io_operation_error(
                    "platform.service.spawn",
                    None,
                    format!("service thread {name} panicked during bootstrap"),
                );
                let _bootstrap_error_delivered = ready_tx.send(Err(error)).is_ok();
                return;
            }
        };

        // run the blocking command loop until shutdown
        loop {
            let message = match receiver.recv() {
                Ok(message) => message,
                Err(_) => {
                    state.mark_stopped();
                    break;
                }
            };

            match message {
                ServiceThreadCommand::Run(callback) => {
                    let result =
                        panic::catch_unwind(AssertUnwindSafe(|| callback(&mut service_state)));
                    if result.is_err() {
                        state.mark_failed(ExecutorFailure::new(
                            ExecutorFailureKind::CallbackPanic,
                            format!("service thread {name} panicked while running one callback"),
                        ));
                        break;
                    }
                }
                ServiceThreadCommand::Shutdown => {
                    state.mark_stopped();
                    break;
                }
            }
        }
    }

    /// Initialize one service thread.
    fn initialize_service_thread(
        name: &str,
        affinity: Option<ExecutionAffinity>,
    ) -> RuntimeResult<ServiceThreadGuard> {
        // windows bootstrap
        #[cfg(windows)]
        if matches!(affinity, Some(ExecutionAffinity::WindowsMta)) {
            return initialize_windows_winrt_mta(name);
        }

        // unsupported bootstrap
        #[cfg(not(windows))]
        if matches!(affinity, Some(ExecutionAffinity::WindowsMta)) {
            return Err(core_platform::io_operation_error(
                "platform.service.bootstrap",
                None,
                format!("windows MTA bootstrap is unavailable on this target for {name}"),
            ));
        }

        Ok(ServiceThreadGuard::None)
    }

    /// Execute one callback on the service thread.
    pub(crate) fn call<R>(
        &self,
        operation: &'static str,
        callback: impl FnOnce(&mut S) -> RuntimeResult<R> + Send + 'static,
    ) -> RuntimeResult<R>
    where
        R: Send + 'static,
    {
        // reject calls once the executor is no longer available
        if let Some(error) = self
            .state
            .unavailable_error(operation, &self.name, "service executor")
        {
            return Err(error);
        }

        // reject synchronous self-calls to avoid deadlock
        if self
            .thread_id
            .get()
            .is_some_and(|thread_id| *thread_id == thread::current().id())
        {
            return Err(core_platform::io_operation_error(
                operation,
                None,
                format!(
                    "service {} cannot synchronously call itself from its own thread",
                    self.name
                ),
            ));
        }

        let (result_tx, result_rx) = sync_channel::<RuntimeResult<R>>(1);
        let name = self.name.clone();

        // send one blocking service call to the executor thread
        self.sender
            .send(ServiceThreadCommand::Run(Box::new(move |state| {
                let result = callback(state);
                let _result_delivered = result_tx.send(result).is_ok();
            })))
            .map_err(|error| {
                if let Some(error) =
                    self.state
                        .unavailable_error(operation, &name, "service executor")
                {
                    return error;
                }

                core_platform::io_operation_error(
                    operation,
                    None,
                    format!("failed to dispatch to service thread {name}: {error}"),
                )
            })?;

        // wait for the service-thread result
        result_rx.recv().map_err(|error| {
            if let Some(error) = self
                .state
                .unavailable_error(operation, &name, "service executor")
            {
                return error;
            }

            core_platform::io_operation_error(
                operation,
                None,
                format!("failed to receive service result from {name}: {error}"),
            )
        })?
    }
}

impl<S> Drop for ServiceThreadExecutor<S> {
    /// Shut down one service thread.
    fn drop(&mut self) {
        let _ = self.sender.send(ServiceThreadCommand::Shutdown);

        // avoid joining from the dedicated service thread itself
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
                format!("service thread {} panicked during shutdown", self.name),
            ));
            return;
        }

        self.state.mark_stopped();
    }
}

/// Validate one process-scoped thread runtime policy.
fn validate_thread_policy(policy: ExecutionPolicy) -> RuntimeResult<()> {
    if policy.scope != ExecutionScope::Process {
        return Err(RuntimeError::Internal {
            message: "service thread executor requires process scope".to_string(),
        }
        .boxed());
    }

    if policy.mode != ExecutionMode::Thread {
        return Err(RuntimeError::Internal {
            message: "service thread executor requires thread execution".to_string(),
        }
        .boxed());
    }

    Ok(())
}
