use std::sync::mpsc::{Receiver, Sender, channel, sync_channel};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::thread::ThreadId;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};

use super::super::affinity::ServiceThreadBootstrap;
#[cfg(windows)]
use super::super::windows::initialize_windows_winrt_mta;

/// One bootstrap or dispatch command for one dedicated-thread executor.
#[allow(dead_code)]
enum DedicatedThreadCommand<S> {
    /// Execute one callback against the service state.
    Run(Box<dyn FnOnce(&mut S) + Send + 'static>),
    /// Shut down the service thread.
    Shutdown,
}

/// One thread-bootstrap guard.
#[allow(dead_code)]
pub(crate) enum ServiceThreadGuard {
    /// No thread teardown work is needed.
    None,
    /// Uninitialize one owned Windows multithreaded apartment.
    #[cfg(windows)]
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

/// One dedicated-thread executor for one host-affine platform service.
#[allow(dead_code)]
pub(crate) struct DedicatedThreadExecutor<S> {
    /// Logical service name for diagnostics.
    name: String,
    /// Command sender for one live service thread.
    sender: Sender<DedicatedThreadCommand<S>>,
    /// Thread identity for re-entrancy guards.
    thread_id: Arc<OnceLock<ThreadId>>,
}

impl<S> DedicatedThreadExecutor<S> {
    /// Spawn one dedicated service thread and bootstrap its state.
    pub(crate) fn spawn(
        name: &str,
        bootstrap: ServiceThreadBootstrap,
        build: impl FnOnce() -> RuntimeResult<S> + Send + 'static,
    ) -> RuntimeResult<Self>
    where
        S: 'static,
    {
        let (sender, receiver) = channel::<DedicatedThreadCommand<S>>();
        let (ready_tx, ready_rx) = sync_channel::<RuntimeResult<()>>(1);
        let thread_name = name.to_string();
        let thread_id = Arc::new(OnceLock::new());
        let bootstrap_thread_id = thread_id.clone();

        // dedicated service thread
        thread::Builder::new()
            .name(thread_name.clone())
            .spawn(move || {
                service_thread_main(
                    &thread_name,
                    bootstrap,
                    build,
                    receiver,
                    ready_tx,
                    bootstrap_thread_id,
                )
            })
            .map_err(|error| {
                core_platform::io_operation_error(
                    "platform.service.spawn",
                    None,
                    format!("failed to spawn service thread {name}: {error}"),
                )
            })?;

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
        })
    }

    /// Execute one callback on the dedicated service thread.
    pub(crate) fn call<R>(
        &self,
        operation: &'static str,
        callback: impl FnOnce(&mut S) -> RuntimeResult<R> + Send + 'static,
    ) -> RuntimeResult<R>
    where
        R: Send + 'static,
    {
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
            .send(DedicatedThreadCommand::Run(Box::new(move |state| {
                let result = callback(state);
                if result_tx.send(result).is_err() {
                    return;
                }
            })))
            .map_err(|error| {
                core_platform::io_operation_error(
                    operation,
                    None,
                    format!("failed to dispatch to service thread {name}: {error}"),
                )
            })?;

        // wait for the service-thread result
        result_rx.recv().map_err(|error| {
            core_platform::io_operation_error(
                operation,
                None,
                format!("failed to receive service result from {name}: {error}"),
            )
        })?
    }
}

impl<S> Drop for DedicatedThreadExecutor<S> {
    /// Shut down one dedicated service thread.
    fn drop(&mut self) {
        drop(self.sender.send(DedicatedThreadCommand::Shutdown));
    }
}

/// Bootstrap one service thread and run its command loop.
#[allow(dead_code)]
fn service_thread_main<S>(
    name: &str,
    bootstrap: ServiceThreadBootstrap,
    build: impl FnOnce() -> RuntimeResult<S>,
    receiver: Receiver<DedicatedThreadCommand<S>>,
    ready_tx: std::sync::mpsc::SyncSender<RuntimeResult<()>>,
    thread_id: Arc<OnceLock<ThreadId>>,
) where
    S: 'static,
{
    // record the thread identity before bootstrap
    let _thread_id_set_result = thread_id.set(thread::current().id());

    // initialize the service thread before building state
    let thread_guard = match initialize_service_thread(name, bootstrap) {
        Ok(guard) => guard,
        Err(error) => {
            if ready_tx.send(Err(error)).is_err() {
                return;
            }
            return;
        }
    };

    let mut state = match build() {
        Ok(state) => {
            if ready_tx.send(Ok(())).is_err() {
                return;
            }
            state
        }
        Err(error) => {
            if ready_tx.send(Err(error)).is_err() {
                return;
            }
            return;
        }
    };

    let _thread_guard = thread_guard;

    // run the blocking command loop until shutdown
    while let Ok(message) = receiver.recv() {
        match message {
            DedicatedThreadCommand::Run(callback) => callback(&mut state),
            DedicatedThreadCommand::Shutdown => break,
        }
    }
}

/// Initialize one dedicated service thread.
#[allow(dead_code)]
fn initialize_service_thread(
    #[cfg_attr(not(windows), allow(unused_variables))] name: &str,
    bootstrap: ServiceThreadBootstrap,
) -> RuntimeResult<ServiceThreadGuard> {
    match bootstrap {
        ServiceThreadBootstrap::None => Ok(ServiceThreadGuard::None),

        #[cfg(windows)]
        ServiceThreadBootstrap::WindowsMta => initialize_windows_winrt_mta(name),
    }
}
