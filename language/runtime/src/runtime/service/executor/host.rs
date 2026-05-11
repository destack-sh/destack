use std::sync::OnceLock;
use std::sync::atomic::AtomicBool;
use std::thread::ThreadId;

use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "macos")]
use crate::host::os::apple::ingress::r#loop::is_process_main_context;
use crate::platform::core::{self as core_platform};
use crate::runtime::thread::ExecutionAffinity;

#[cfg(target_os = "macos")]
use super::super::unix::call_process_main_thread;
#[cfg(windows)]
use super::super::windows::{
    WINDOWS_HOST_LOOP_SERVICE_MESSAGE_ID, WindowsLoopQueue, register_windows_loop_queue,
    windows_loop_queue,
};
#[cfg(windows)]
use std::sync::Arc;
#[cfg(windows)]
use std::sync::mpsc::sync_channel;

/// One host-loop-bound executor for one host-affine platform service.
pub(crate) struct HostExecutor {
    /// Logical service name for diagnostics.
    pub(crate) name: String,
    /// The required host loop for this service.
    pub(crate) host_loop: ExecutionAffinity,
    /// The bound host-loop thread for this service.
    pub(crate) thread_id: OnceLock<ThreadId>,
    /// Whether the host loop has been bound yet.
    pub(crate) is_bound: AtomicBool,
    /// Queued callbacks for one windows message loop.
    #[cfg(windows)]
    pub(in super::super) windows_queue: Arc<WindowsLoopQueue>,
    /// The bound windows thread id for one windows message loop.
    #[cfg(windows)]
    pub(in super::super) windows_thread_id: OnceLock<u32>,
}

impl HostExecutor {
    /// Create one host-loop-bound executor.
    pub(crate) fn new(name: &str, host_loop: ExecutionAffinity) -> RuntimeResult<Self> {
        validate_host_loop_affinity(host_loop)?;

        Ok(Self {
            name: name.to_string(),
            host_loop,
            thread_id: OnceLock::new(),
            is_bound: AtomicBool::new(false),
            #[cfg(windows)]
            windows_queue: windows_loop_queue(),
            #[cfg(windows)]
            windows_thread_id: OnceLock::new(),
        })
    }

    /// Execute one callback on the configured host loop, marshalling when needed.
    pub(crate) fn call_loop<R>(
        &self,
        operation: &'static str,
        _callback: impl FnOnce() -> RuntimeResult<R> + Send + 'static,
    ) -> RuntimeResult<R>
    where
        R: Send + 'static,
    {
        // dispatch through the configured host loop
        match self.host_loop {
            #[cfg(target_os = "macos")]
            ExecutionAffinity::MainThread => call_process_main_thread(operation, self, _callback),

            #[cfg(windows)]
            ExecutionAffinity::WindowsMessageLoop => {
                self.call_windows_message_loop(operation, _callback)
            }

            #[cfg(windows)]
            ExecutionAffinity::WindowsMta => Err(core_platform::io_operation_error(
                operation,
                None,
                format!(
                    "host-loop service {} cannot use windows MTA affinity",
                    self.name
                ),
            )),
        }
    }

    /// Return whether the current thread is the bound host-loop thread.
    #[cfg(windows)]
    pub(crate) fn is_current_bound_thread(&self) -> bool {
        self.thread_id
            .get()
            .is_some_and(|thread_id| *thread_id == std::thread::current().id())
    }

    /// Bind or validate the current host loop for this service.
    #[cfg(target_os = "macos")]
    pub(crate) fn ensure_host_loop(&self, operation: &'static str) -> RuntimeResult<()> {
        // validate platform-specific host-loop requirements first
        self.validate_host_loop(operation)?;

        let current_thread_id = std::thread::current().id();

        // bind the first observed host-loop thread
        if !self.is_bound.load(std::sync::atomic::Ordering::Acquire)
            && self.thread_id.set(current_thread_id).is_ok()
        {
            self.is_bound
                .store(true, std::sync::atomic::Ordering::Release);

            return Ok(());
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
    #[cfg(target_os = "macos")]
    fn validate_host_loop(&self, operation: &'static str) -> RuntimeResult<()> {
        match self.host_loop {
            ExecutionAffinity::MainThread => {
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
        }
    }
}

/// Validate one host-loop-compatible runtime affinity.
fn validate_host_loop_affinity(host_loop: ExecutionAffinity) -> RuntimeResult<()> {
    match host_loop {
        #[cfg(target_os = "macos")]
        ExecutionAffinity::MainThread => {}

        #[cfg(windows)]
        ExecutionAffinity::WindowsMessageLoop => {}

        #[cfg(windows)]
        ExecutionAffinity::WindowsMta => {
            return Err(core_platform::invalid_argument(
                "execution.affinity",
                "host-loop executor cannot use windows MTA affinity",
            ));
        }
    }

    Ok(())
}

#[cfg(windows)]
impl HostExecutor {
    /// Execute one callback on the bound windows message loop.
    fn call_windows_message_loop<R>(
        &self,
        operation: &'static str,
        callback: impl FnOnce() -> RuntimeResult<R> + Send + 'static,
    ) -> RuntimeResult<R>
    where
        R: Send + 'static,
    {
        // run directly when the current thread already owns the host loop
        if self.try_bind_windows_message_loop() || self.is_current_bound_thread() {
            return callback();
        }

        let Some(thread_id) = self.windows_thread_id.get().copied() else {
            return Err(core_platform::io_operation_error(
                operation,
                None,
                format!(
                    "loop service {} has no bound windows message loop thread",
                    self.name
                ),
            ));
        };

        let (result_tx, result_rx) = sync_channel::<RuntimeResult<R>>(1);

        // queue one callback for the bound windows message loop
        self.windows_queue.enqueue(Box::new(move || {
            let _result_delivered = result_tx.send(callback()).is_ok();
        }));

        // wake the bound message loop thread to service the callback
        let dispatched = unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                thread_id,
                WINDOWS_HOST_LOOP_SERVICE_MESSAGE_ID,
                0,
                0,
            )
        };
        if dispatched == 0 {
            return Err(core_platform::io_operation_error(
                operation,
                None,
                format!(
                    "failed to post loop callback to windows thread {thread_id}: {}",
                    core_platform::last_error_code()
                ),
            ));
        }

        // wait for the windows loop callback result
        result_rx.recv().map_err(|error| {
            core_platform::io_operation_error(
                operation,
                None,
                format!(
                    "failed to receive windows loop callback result for {}: {error}",
                    self.name
                ),
            )
        })?
    }

    /// Bind the current windows message loop thread on first use.
    pub(crate) fn try_bind_windows_message_loop(&self) -> bool {
        if self.is_current_bound_thread() {
            return true;
        }

        if self.is_bound.load(std::sync::atomic::Ordering::Acquire) {
            return false;
        }

        let current_thread_id = std::thread::current().id();
        let current_windows_thread_id =
            unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() };
        let mut message =
            unsafe { std::mem::zeroed::<windows_sys::Win32::UI::WindowsAndMessaging::MSG>() };

        // create the thread message queue before other threads post callbacks to it
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::PeekMessageW(
                &mut message,
                0,
                0,
                0,
                windows_sys::Win32::UI::WindowsAndMessaging::PM_NOREMOVE,
            );
        }

        // bind the first observed windows message loop thread
        if self.thread_id.set(current_thread_id).is_err() {
            return self.is_current_bound_thread();
        }

        if self
            .windows_thread_id
            .set(current_windows_thread_id)
            .is_err()
        {
            return self
                .windows_thread_id
                .get()
                .copied()
                .is_some_and(|thread_id| thread_id == current_windows_thread_id);
        }

        register_windows_loop_queue(current_windows_thread_id, &self.windows_queue);
        self.is_bound
            .store(true, std::sync::atomic::Ordering::Release);

        true
    }
}
