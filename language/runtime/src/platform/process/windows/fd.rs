#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
use crate::platform::process::core as core_process;
use crate::platform::resource::{ResourceFinalizer, ResourceId};
use crate::platform::{PlatformError, PlatformErrorCode, core as core_platform};

use crate::runtime::BindingCallContext;

use super::wait;

use crate::platform::process::{
    ProcessFdFlags, ProcessFdSignalFlags, ProcessId, ProcessWaitExitedStatus, ProcessWaitFlags,
    ProcessWaitRunningStatus, ProcessWaitStatus, Signal, SignalEvent, SignalFdFlags,
};
use crate::platform::resource;

/// Finalizer that closes one Windows process handle.
#[derive(Debug)]
struct ProcessHandleFinalizer {
    /// Raw process handle to close.
    handle: windows_sys::Win32::Foundation::HANDLE,
}

impl ProcessHandleFinalizer {
    /// Create a process handle finalizer from one raw handle.
    fn new(handle: windows_sys::Win32::Foundation::HANDLE) -> Self {
        Self { handle }
    }
}

impl ResourceFinalizer for ProcessHandleFinalizer {
    /// Close the process handle when the resource is finalized.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

/// Finalizer that closes one duplicated Windows stdio handle.
#[derive(Debug)]
struct StdioHandleFinalizer {
    /// Raw stdio handle to close.
    handle: windows_sys::Win32::Foundation::HANDLE,
}

impl StdioHandleFinalizer {
    /// Create a stdio handle finalizer from one raw handle.
    fn new(handle: windows_sys::Win32::Foundation::HANDLE) -> Self {
        Self { handle }
    }
}

impl ResourceFinalizer for StdioHandleFinalizer {
    /// Close the duplicated stdio handle when the resource is finalized.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

/// Duplicate one stdio handle into the current process handle table.
fn duplicate_stdio_handle(
    source: windows_sys::Win32::Foundation::HANDLE,
    syscall: &str,
) -> RuntimeResult<windows_sys::Win32::Foundation::HANDLE> {
    use windows_sys::Win32::Foundation::{DUPLICATE_SAME_ACCESS, DuplicateHandle};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    let process = unsafe { GetCurrentProcess() };
    let mut duplicated = 0;
    let rc = unsafe {
        DuplicateHandle(
            process,
            source,
            process,
            &mut duplicated,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    };
    if rc == 0 || duplicated == 0 {
        let error = core_platform::last_error_code() as u32;
        return Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::Io),
            None,
            Some(error as i32),
            Some(syscall.to_string()),
            None,
            format!("{syscall} failed while duplicating stdio handle"),
        ))
        .boxed());
    }

    Ok(duplicated)
}

/// Resolve and duplicate one standard stream handle.
fn open_stdio_handle(
    std_handle: u32,
    handle_name: &str,
) -> RuntimeResult<windows_sys::Win32::Foundation::HANDLE> {
    use windows_sys::Win32::Foundation::{ERROR_INVALID_HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Console::GetStdHandle;

    let handle = unsafe { GetStdHandle(std_handle) };
    if handle == 0 {
        return Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoNotFound),
            None,
            None,
            Some("GetStdHandle".to_string()),
            None,
            format!("{handle_name} is not configured for this process"),
        ))
        .boxed());
    }
    if handle == INVALID_HANDLE_VALUE {
        let error = core_platform::last_error_code() as u32;
        let code = if error == ERROR_INVALID_HANDLE {
            PlatformErrorCode::IoNotFound
        } else {
            PlatformErrorCode::Io
        };
        return Err(RuntimeError::from(PlatformError::io_with(
            Some(code),
            None,
            Some(error as i32),
            Some("GetStdHandle".to_string()),
            None,
            format!("failed to resolve {handle_name}"),
        ))
        .boxed());
    }

    duplicate_stdio_handle(handle, "DuplicateHandle")
}

/// Register one duplicated stdio handle as a file handle resource.
fn register_stdio_handle(
    binding: &BindingCallContext,
    out: *mut resource::FileHandle,
    std_handle: u32,
    handle_name: &str,
    label: &str,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let duplicated = open_stdio_handle(std_handle, handle_name)?;
    let entry = resource::ResourceEntry::new(resource::ResourceKind::File)
        .with_label(label)
        .with_handle(duplicated as _)
        .with_finalizer(StdioHandleFinalizer::new(duplicated));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        *out = resource::FileHandle(resource_id);
    }

    Ok(())
}

/// Open one process handle for process-fd operations.
fn open_process_fd_handle(pid: ProcessId) -> RuntimeResult<windows_sys::Win32::Foundation::HANDLE> {
    use windows_sys::Win32::Foundation::ERROR_INVALID_PARAMETER;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
    };

    if pid.0 == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "pid",
            "process id must be greater than zero",
        ))
        .boxed());
    }

    let process_handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE | PROCESS_TERMINATE,
            0,
            pid.0,
        )
    };
    if process_handle == 0 {
        let error = core_platform::last_error_code() as u32;
        let code = if error == ERROR_INVALID_PARAMETER {
            PlatformErrorCode::ProcessNotFound
        } else {
            PlatformErrorCode::ProcessPermissionDenied
        };
        return Err(RuntimeError::from(PlatformError::process_with(
            Some(code),
            Some(error.to_string()),
            None,
            None,
            Some("OpenProcess".to_string()),
            format!("failed to open process {} for process-fd", pid.0),
        ))
        .boxed());
    }

    Ok(process_handle)
}

/// Open one standard input stream handle.
///
/// Open one handle for the current process standard input stream.
/// The returned handle can be used with file-handle read and close operations.
///
/// # Platform
/// Unix and Windows.
/// Uses dup(2) from descriptor 0 on Unix and DuplicateHandle from GetStdHandle(STD_INPUT_HANDLE) on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.stdio`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_stdio_stdin(
    binding: &BindingCallContext,
    out: *mut resource::FileHandle,
) -> RuntimeResult<()> {
    register_stdio_handle(
        binding,
        out,
        windows_sys::Win32::System::Console::STD_INPUT_HANDLE,
        "stdin",
        "process.stdio.stdin",
    )
}

/// Open one standard output stream handle.
///
/// Open one handle for the current process standard output stream.
/// The returned handle can be used with file-handle write, sync, and close operations.
///
/// # Platform
/// Unix and Windows.
/// Uses dup(2) from descriptor 1 on Unix and DuplicateHandle from GetStdHandle(STD_OUTPUT_HANDLE) on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.stdio`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_stdio_stdout(
    binding: &BindingCallContext,
    out: *mut resource::FileHandle,
) -> RuntimeResult<()> {
    register_stdio_handle(
        binding,
        out,
        windows_sys::Win32::System::Console::STD_OUTPUT_HANDLE,
        "stdout",
        "process.stdio.stdout",
    )
}

/// Open one standard error stream handle.
///
/// Open one handle for the current process standard error stream.
/// The returned handle can be used with file-handle write, sync, and close operations.
///
/// # Platform
/// Unix and Windows.
/// Uses dup(2) from descriptor 2 on Unix and DuplicateHandle from GetStdHandle(STD_ERROR_HANDLE) on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.stdio`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_stdio_stderr(
    binding: &BindingCallContext,
    out: *mut resource::FileHandle,
) -> RuntimeResult<()> {
    register_stdio_handle(
        binding,
        out,
        windows_sys::Win32::System::Console::STD_ERROR_HANDLE,
        "stderr",
        "process.stdio.stderr",
    )
}

/// Resolve a process-fd handle into process id and raw process handle payload.
fn resolve_process_fd(
    binding: &BindingCallContext,
    handle: resource::ProcessFdHandle,
) -> RuntimeResult<(ProcessId, windows_sys::Win32::Foundation::HANDLE)> {
    let resolved = binding.worker().resources.with_entry(handle.0, |entry| {
        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<core_process::ProcessFdBinding>())
            .map(|binding| {
                (
                    binding.pid,
                    entry.handle().map(|raw_handle| raw_handle as _),
                )
            })
    });

    let Some((pid, process_handle)) = resolved.flatten() else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown process fd handle",
        ))
        .boxed());
    };
    let Some(process_handle) = process_handle else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "process fd handle is missing a raw process handle",
        ))
        .boxed());
    };

    Ok((pid, process_handle))
}

/// Wait one process handle with an explicit timeout in milliseconds.
fn wait_process_handle_with_timeout(
    binding: &BindingCallContext,
    pid: ProcessId,
    process_handle: windows_sys::Win32::Foundation::HANDLE,
    timeout_ms: u32,
) -> RuntimeResult<ProcessWaitStatus> {
    use windows_sys::Win32::Foundation::{STILL_ACTIVE, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT};
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};

    let wait_status = unsafe { WaitForSingleObject(process_handle, timeout_ms) };
    match wait_status {
        WAIT_TIMEOUT => Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoWouldBlock),
            None,
            None,
            Some("WaitForSingleObject".to_string()),
            None,
            format!("wait timed out for pid {}", pid.0),
        ))
        .boxed()),
        WAIT_OBJECT_0 => {
            let mut exit_code = 0_u32;
            let read_exit_code = unsafe { GetExitCodeProcess(process_handle, &mut exit_code) };
            if read_exit_code == 0 {
                let error = core_platform::last_error_code();
                return Err(RuntimeError::from(PlatformError::io(format!(
                    "failed to read process exit code: {error}",
                )))
                .boxed());
            }

            if exit_code == STILL_ACTIVE as u32 {
                return Ok(ProcessWaitStatus::ProcessWaitRunningStatus(
                    ProcessWaitRunningStatus {
                        kind: binding.store_string("running"),
                        pid,
                    },
                ));
            }

            Ok(ProcessWaitStatus::ProcessWaitExitedStatus(
                ProcessWaitExitedStatus {
                    kind: binding.store_string("exited"),
                    pid,
                    exit_code: exit_code as i32,
                },
            ))
        }
        WAIT_FAILED => {
            let error = core_platform::last_error_code();
            Err(RuntimeError::from(PlatformError::io(format!(
                "wait failed for pid {}: {error}",
                pid.0
            )))
            .boxed())
        }
        _ => Err(RuntimeError::from(PlatformError::io("wait returned unexpected result")).boxed()),
    }
}

/// Wait one process handle using process wait flags.
fn wait_process_handle_with_flags(
    binding: &BindingCallContext,
    pid: ProcessId,
    process_handle: windows_sys::Win32::Foundation::HANDLE,
    flags: ProcessWaitFlags,
) -> RuntimeResult<ProcessWaitStatus> {
    use windows_sys::Win32::System::Threading::INFINITE;

    if flags.0 & !wait::PROCESS_WAIT_FLAG_NOHANG != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "unsupported process wait flags",
        ))
        .boxed());
    }

    let timeout_ms = if flags.0 & wait::PROCESS_WAIT_FLAG_NOHANG != 0 {
        0
    } else {
        INFINITE
    };
    wait_process_handle_with_timeout(binding, pid, process_handle, timeout_ms)
}

/// Wait one process handle with a nanosecond timeout.
fn wait_process_handle_with_timeout_ns(
    binding: &BindingCallContext,
    pid: ProcessId,
    process_handle: windows_sys::Win32::Foundation::HANDLE,
    timeout_ns: u64,
) -> RuntimeResult<ProcessWaitStatus> {
    if timeout_ns == 0 {
        return wait_process_handle_with_flags(
            binding,
            pid,
            process_handle,
            ProcessWaitFlags(wait::PROCESS_WAIT_FLAG_NOHANG),
        );
    }

    let timeout_ms = (timeout_ns / 1_000_000).max(1);
    let timeout_ms = timeout_ms.min(u32::MAX as u64) as u32;
    wait_process_handle_with_timeout(binding, pid, process_handle, timeout_ms)
}

/// Send one signal through one process handle.
fn send_signal_process_handle(
    pid: ProcessId,
    process_handle: windows_sys::Win32::Foundation::HANDLE,
    signal: Signal,
) -> RuntimeResult<()> {
    use windows_sys::Win32::Foundation::ERROR_ACCESS_DENIED;
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, TerminateProcess};

    if signal.0 == 0 {
        let mut exit_code = 0_u32;
        let read_exit_code = unsafe { GetExitCodeProcess(process_handle, &mut exit_code) };
        if read_exit_code == 0 {
            let error = core_platform::last_error_code() as u32;
            let code = if error == ERROR_ACCESS_DENIED {
                PlatformErrorCode::ProcessPermissionDenied
            } else {
                PlatformErrorCode::Process
            };
            return Err(RuntimeError::from(PlatformError::process_with(
                Some(code),
                Some(error.to_string()),
                None,
                None,
                Some("GetExitCodeProcess".to_string()),
                format!("failed to query process {}", pid.0),
            ))
            .boxed());
        }

        return Ok(());
    }

    if signal.0 != 1 && signal.0 != 2 && signal.0 != 9 && signal.0 != 15 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "signal",
            format!("unsupported signal {} on windows", signal.0),
        ))
        .boxed());
    }

    let terminate_process =
        unsafe { TerminateProcess(process_handle, 128_u32.saturating_add(signal.0)) };
    if terminate_process == 0 {
        let error = core_platform::last_error_code() as u32;
        let code = if error == ERROR_ACCESS_DENIED {
            PlatformErrorCode::ProcessPermissionDenied
        } else {
            PlatformErrorCode::Process
        };
        return Err(RuntimeError::from(PlatformError::process_with(
            Some(code),
            Some(error.to_string()),
            None,
            None,
            Some("TerminateProcess".to_string()),
            format!("failed to terminate process {}", pid.0),
        ))
        .boxed());
    }

    Ok(())
}

/// Ensure one process-fd handle resolves to a process-fd payload.
fn ensure_process_fd_handle(
    binding: &BindingCallContext,
    handle: resource::ProcessFdHandle,
) -> RuntimeResult<()> {
    let is_process_fd = binding.worker().resources.with_entry(handle.0, |entry| {
        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<core_process::ProcessFdBinding>())
            .is_some()
    });

    if is_process_fd != Some(true) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown process fd handle",
        ))
        .boxed());
    }

    Ok(())
}

/// Close one process descriptor.
///
/// Close one host process descriptor and release the kernel object reference.
/// Closing semantics follow host descriptor teardown behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses close(2) on Unix and CloseHandle on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.handle`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_process_fd_close(
    binding: &BindingCallContext,
    handle: resource::ProcessFdHandle,
) -> RuntimeResult<()> {
    ensure_process_fd_handle(binding, handle)?;

    let removed = binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    );
    if !removed {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown process fd handle",
        ))
        .boxed());
    }

    Ok(())
}

/// Open one process descriptor for the target process id.
///
/// Open one host process descriptor that can be used for wait and signal operations without pid reuse races.
/// Descriptor semantics follow pidfd on Linux and host-equivalent process-handle semantics on other targets.
///
/// # Platform
/// Unix and Windows.
/// Uses pidfd_open(2) on Linux and process handle duplication on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.handle`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_process_fd_open(
    binding: &BindingCallContext,
    out: *mut resource::ProcessFdHandle,
    pid: ProcessId,
    flags: ProcessFdFlags,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    if flags.0 != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "process fd flags are not supported",
        ))
        .boxed());
    }

    let process_handle = open_process_fd_handle(pid)?;
    let entry = resource::ResourceEntry::new(resource::ResourceKind::ProcessFd)
        .with_label("process.fd")
        .with_payload(core_process::ProcessFdBinding { pid })
        .with_handle(process_handle as _)
        .with_finalizer(ProcessHandleFinalizer::new(process_handle));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        *out = resource::ProcessFdHandle(resource_id);
    }

    Ok(())
}

/// Send one signal through a process descriptor.
///
/// Deliver one signal using a stable process descriptor rather than a numeric pid.
/// Delivery semantics follow pidfd_send_signal on Linux and host-equivalent process-signal APIs on other targets.
///
/// # Platform
/// Unix and Windows.
/// Uses pidfd_send_signal(2) on Linux and process-handle control APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.send`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_process_fd_send_signal(
    binding: &BindingCallContext,
    handle: resource::ProcessFdHandle,
    signal: Signal,
    flags: ProcessFdSignalFlags,
) -> RuntimeResult<()> {
    if flags.0 != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "process fd signal flags are not supported",
        ))
        .boxed());
    }

    let (process_id, process_handle) = resolve_process_fd(binding, handle)?;
    send_signal_process_handle(process_id, process_handle, signal)
}

/// Poll one process descriptor state transition without blocking.
///
/// Poll one process descriptor for state transition readiness and return immediately when no transition is pending.
/// Non-ready state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking poll over pidfd on Linux and zero-timeout process wait on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.wait`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_process_fd_try_wait(
    binding: &BindingCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessFdHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let (process_id, process_handle) = resolve_process_fd(binding, handle)?;
    let status = wait_process_handle_with_flags(
        binding,
        process_id,
        process_handle,
        ProcessWaitFlags(wait::PROCESS_WAIT_FLAG_NOHANG),
    )?;
    unsafe {
        *out = status;
    }

    Ok(())
}

/// Wait for one process descriptor state transition.
///
/// Wait for one child-state transition associated with the process descriptor.
/// Wait semantics follow pollable pidfd readiness on Linux and host process wait APIs on other targets.
///
/// # Platform
/// Unix and Windows.
/// Uses poll or waitid over pidfd on Linux and WaitForSingleObject plus status queries on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.wait`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_process_fd_wait(
    binding: &BindingCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessFdHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let (process_id, process_handle) = resolve_process_fd(binding, handle)?;
    let status =
        wait_process_handle_with_timeout_ns(binding, process_id, process_handle, timeoutns)?;
    unsafe {
        *out = status;
    }

    Ok(())
}

/// Close one signal descriptor.
///
/// Close one descriptor-backed signal queue and release host resources.
/// Close semantics follow host descriptor teardown behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses close(2) on Unix and CloseHandle on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_signal_fd_close(
    _binding: &BindingCallContext,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdClose",
    ))
    .boxed())
}

/// Open one signal descriptor for the provided signal mask.
///
/// Open one descriptor-backed signal queue that can be polled and read like other fd resources.
/// Signal mask semantics follow signalfd on Linux and host-equivalent runtime adapters on other targets.
///
/// # Platform
/// Unix and Windows.
/// Uses signalfd(2) on Linux and host-equivalent process-signal descriptor adapters elsewhere.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_signal_fd_open(
    _binding: &BindingCallContext,
    out: *mut resource::SignalFdHandle,
    signals: NativeSlice<Signal>,
    flags: SignalFdFlags,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    if flags.0 != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "signal fd flags are not supported",
        ))
        .boxed());
    }

    let _ = signals;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdOpen",
    ))
    .boxed())
}

/// Read one queued signal event from a signal descriptor.
///
/// Read the next queued signal payload from one descriptor-backed signal queue.
/// Queue ordering and coalescing behavior follow host signal queue semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses read(2) over signalfd on Linux and host-equivalent signal descriptor reads on other targets.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_signal_fd_read(
    _binding: &BindingCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdRead",
    ))
    .boxed())
}

/// Replace the active signal mask for one signal descriptor.
///
/// Replace the descriptor signal mask with one explicit signal-set value.
/// Mask transitions are atomic under host signal-descriptor APIs.
///
/// # Platform
/// Unix and Windows.
/// Uses signalfd mask update semantics on Linux and host-equivalent signal descriptor mask updates elsewhere.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_signal_fd_set_mask(
    _binding: &BindingCallContext,
    handle: resource::SignalFdHandle,
    signals: NativeSlice<Signal>,
) -> RuntimeResult<()> {
    let _ = (handle, signals);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdSetMask",
    ))
    .boxed())
}

/// Poll one queued signal event from a signal descriptor without blocking.
///
/// Poll one descriptor-backed signal queue for one signal event and return immediately when empty.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking reads over signalfd on Linux and host-equivalent signal descriptor polling elsewhere.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_signal_fd_try_read(
    _binding: &BindingCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdTryRead",
    ))
    .boxed())
}
