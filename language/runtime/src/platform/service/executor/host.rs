use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::ThreadId;

use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "macos")]
use crate::host::apple::message::is_process_main_context;
use crate::platform::core::{self as core_platform};

use super::super::affinity::ServiceHostLoop;
#[cfg(target_os = "macos")]
use super::super::unix::call_process_main_thread;
#[cfg(windows)]
use super::super::windows::{
    call_process_windows_message_loop, register_windows_loop_queue, try_bind_windows_message_loop,
    windows_loop_queue,
};
#[cfg(windows)]
use std::sync::Arc;

/// One host-loop-bound executor for one host-affine platform service.
pub(crate) struct HostLoopExecutor {
    /// Logical service name for diagnostics.
    pub(crate) name: String,
    /// The required host loop for this service.
    pub(crate) host_loop: ServiceHostLoop,
    /// The bound host-loop thread for this service.
    pub(crate) thread_id: OnceLock<ThreadId>,
    /// Whether the host loop has been bound yet.
    pub(crate) is_bound: AtomicBool,
    /// Queued callbacks for one windows message loop.
    #[cfg(windows)]
    pub(crate) windows_queue: Arc<super::super::windows::WindowsLoopQueue>,
    /// The bound windows thread id for one windows message loop.
    #[cfg(windows)]
    pub(crate) windows_thread_id: OnceLock<u32>,
}

impl HostLoopExecutor {
    /// Create one host-loop-bound executor.
    pub(crate) fn new(name: &str, host_loop: ServiceHostLoop) -> Self {
        Self {
            name: name.to_string(),
            host_loop,
            thread_id: OnceLock::new(),
            is_bound: AtomicBool::new(false),
            #[cfg(windows)]
            windows_queue: windows_loop_queue(),
            #[cfg(windows)]
            windows_thread_id: OnceLock::new(),
        }
    }

    /// Execute one callback on the configured host loop, marshalling when needed.
    pub(crate) fn call_loop<R>(
        &self,
        operation: &'static str,
        callback: impl FnOnce() -> RuntimeResult<R> + Send + 'static,
    ) -> RuntimeResult<R>
    where
        R: Send + 'static,
    {
        match self.host_loop {
            #[cfg(target_os = "macos")]
            ServiceHostLoop::MainThread => call_process_main_thread(operation, self, callback),

            #[cfg(windows)]
            ServiceHostLoop::WindowsMessageLoop => {
                call_process_windows_message_loop(operation, self, callback)
            }
        }
    }

    /// Return whether the current thread is the bound host-loop thread.
    #[cfg(windows)]
    pub(crate) fn is_current_bound_thread(&self) -> bool {
        self.thread_id
            .get()
            .is_some_and(|thread_id| *thread_id == thread::current().id())
    }

    /// Bind or validate the current host loop for this service.
    pub(crate) fn ensure_host_loop(&self, operation: &'static str) -> RuntimeResult<()> {
        // validate platform-specific host-loop requirements first
        self.validate_host_loop(operation)?;

        #[cfg(windows)]
        {
            if self.host_loop == ServiceHostLoop::WindowsMessageLoop
                && self.try_bind_windows_message_loop()
            {
                return Ok(());
            }
        }

        let current_thread_id = thread::current().id();

        // bind the first observed host-loop thread
        if !self.is_bound.load(Ordering::Acquire) {
            if self.thread_id.set(current_thread_id).is_ok() {
                self.is_bound.store(true, Ordering::Release);

                #[cfg(windows)]
                {
                    if self.host_loop == ServiceHostLoop::WindowsMessageLoop {
                        register_windows_loop_queue(self);
                    }
                }

                return Ok(());
            }
        }

        // require all later calls to stay on the bound host-loop thread
        let Some(bound_thread_id) = self.thread_id.get() else {
            return Err(core_platform::io_operation_error(
                operation,
                None,
                format!("host-loop service {} has no bound thread", self.name),
            ));
        };

        if *bound_thread_id == current_thread_id {
            return Ok(());
        }

        Err(core_platform::io_operation_error(
            operation,
            None,
            format!(
                "host-loop service {} must run on its bound {:?} thread",
                self.name, self.host_loop
            ),
        ))
    }

    /// Validate one platform-specific host-loop requirement.
    fn validate_host_loop(&self, operation: &'static str) -> RuntimeResult<()> {
        match self.host_loop {
            #[cfg(target_os = "macos")]
            ServiceHostLoop::MainThread => {
                if is_process_main_context() {
                    Ok(())
                } else {
                    Err(core_platform::io_operation_error(
                        operation,
                        None,
                        format!(
                            "host-loop service {} must run on the process main thread",
                            self.name
                        ),
                    ))
                }
            }

            #[cfg(windows)]
            ServiceHostLoop::WindowsMessageLoop => Ok(()),
        }
    }
}

#[cfg(windows)]
impl HostLoopExecutor {
    /// Bind the current windows message loop thread on first use.
    pub(crate) fn try_bind_windows_message_loop(&self) -> bool {
        try_bind_windows_message_loop(self)
    }
}
