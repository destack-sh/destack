#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::{bindings_generated as bindings, core as core_process};
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, NativeStringSlice, PlatformError,
    core as core_platform,
};

use crate::runtime::BindingCallContext;
use bindings::*;

use crate::platform::process::{
    ExecAtFlags, GroupId, ProcessCpuSet, ProcessFdAction, ProcessFdFlags, ProcessFdSignalFlags,
    ProcessGroupIds, ProcessId, ProcessLimit, ProcessLimitResource, ProcessNamespaceKind,
    ProcessSchedulerConfig, ProcessSchedulerPolicy, ProcessSpawnOptions, ProcessStdio,
    ProcessUnshareFlags, ProcessUserIds, ProcessWaitFlags, ProcessWaitStatus, Signal, SignalEvent,
    SignalFdFlags, SignalMaskHow, SyscallFilterFlags, UserId,
};
use crate::platform::{fs, resource};

/// Finalizer that closes one duplicated stdio file descriptor.
#[derive(Debug)]
struct StdioFdFinalizer {
    /// File descriptor to close.
    fd: libc::c_int,
}

impl resource::ResourceFinalizer for StdioFdFinalizer {
    /// Close the duplicated descriptor when the resource is finalized.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

/// Duplicate one stdio descriptor for the current process.
fn duplicate_stdio_fd(fd: libc::c_int) -> RuntimeResult<libc::c_int> {
    // duplicate the descriptor
    let duplicated = unsafe { libc::dup(fd) };
    if duplicated < 0 {
        return Err(core_platform::io_error("dup", None));
    }

    // set close on exec on the duplicated descriptor
    let rc = unsafe { libc::fcntl(duplicated, libc::F_SETFD, libc::FD_CLOEXEC) };
    if rc < 0 {
        let error = core_platform::io_error("fcntl", None);
        unsafe {
            libc::close(duplicated);
        }
        return Err(error);
    }

    Ok(duplicated)
}

/// Register one duplicated stdio descriptor as a file handle.
fn register_stdio_fd(
    context: &BindingCallContext,
    out: *mut resource::FileHandle,
    fd: libc::c_int,
    label: &str,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let duplicated = duplicate_stdio_fd(fd)?;
    let entry = resource::ResourceEntry::new(resource::ResourceKind::File)
        .with_label(label)
        .with_fd(duplicated)
        .with_finalizer(StdioFdFinalizer { fd: duplicated });
    let resource_id = context
        .runtime()
        .resources
        .insert(entry, Some(context.engine()));

    unsafe {
        *out = resource::FileHandle(resource_id);
    }

    Ok(())
}

/// Resolve a process-fd handle into its process id payload.
fn resolve_process_fd(
    context: &BindingCallContext,
    handle: resource::ProcessFdHandle,
) -> RuntimeResult<ProcessId> {
    resource::require_payload::<core_process::ProcessFdBinding>(
        context,
        handle.0,
        resource::ResourceKind::ProcessFd,
        None,
        "handle",
        "process fd",
    )
    .map(|binding| binding.pid)
}

/// Resolve a signal-fd handle into its signal mask payload.
fn resolve_signal_fd(
    context: &BindingCallContext,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<Vec<Signal>> {
    resource::require_payload::<core_process::SignalFdBinding>(
        context,
        handle.0,
        resource::ResourceKind::SignalFd,
        None,
        "handle",
        "signal fd",
    )
    .map(|binding| binding.signals)
}

/// Replace the signal mask payload for one signal-fd handle.
fn update_signal_fd(
    context: &BindingCallContext,
    handle: resource::SignalFdHandle,
    signals: Vec<Signal>,
) -> RuntimeResult<()> {
    let updated = resource::with_entry_mut(
        context,
        handle.0,
        resource::ResourceKind::SignalFd,
        None,
        |entry| {
            entry
                .payload_mut::<core_process::SignalFdBinding>()
                .map(|binding| {
                    binding.signals = signals;
                })
        },
    );

    if updated.flatten().is_none() {
        return Err(core_platform::unknown_handle("handle", "signal fd"));
    }

    Ok(())
}

/// Ensure one process-fd handle resolves to a process-fd payload.
fn ensure_process_fd_handle(
    context: &BindingCallContext,
    handle: resource::ProcessFdHandle,
) -> RuntimeResult<()> {
    let is_process_fd = resource::with_payload::<core_process::ProcessFdBinding, _>(
        context,
        handle.0,
        resource::ResourceKind::ProcessFd,
        None,
        |_binding, _entry| true,
    );

    if is_process_fd != Some(true) {
        return Err(core_platform::unknown_handle("handle", "process fd"));
    }

    Ok(())
}

/// Ensure one signal-fd handle resolves to a signal-fd payload.
fn ensure_signal_fd_handle(
    context: &BindingCallContext,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    let is_signal_fd = resource::with_payload::<core_process::SignalFdBinding, _>(
        context,
        handle.0,
        resource::ResourceKind::SignalFd,
        None,
        |_binding, _entry| true,
    );

    if is_signal_fd != Some(true) {
        return Err(core_platform::unknown_handle("handle", "signal fd"));
    }

    Ok(())
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
    context: &BindingCallContext,
    out: *mut resource::FileHandle,
) -> RuntimeResult<()> {
    register_stdio_fd(context, out, libc::STDIN_FILENO, "process.stdio.stdin")
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
    context: &BindingCallContext,
    out: *mut resource::FileHandle,
) -> RuntimeResult<()> {
    register_stdio_fd(context, out, libc::STDOUT_FILENO, "process.stdio.stdout")
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
    context: &BindingCallContext,
    out: *mut resource::FileHandle,
) -> RuntimeResult<()> {
    register_stdio_fd(context, out, libc::STDERR_FILENO, "process.stdio.stderr")
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
    context: &BindingCallContext,
    handle: resource::ProcessFdHandle,
) -> RuntimeResult<()> {
    ensure_process_fd_handle(context, handle)?;

    let removed = context
        .runtime()
        .resources
        .remove_and_finalize(handle.0, Some(context.engine()));
    if !removed {
        return Err(core_platform::unknown_handle("handle", "process fd"));
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
    context: &BindingCallContext,
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

    super::signals::process_kill(pid.0, 0)?;
    let entry = resource::ResourceEntry::new(resource::ResourceKind::ProcessFd)
        .with_label("process.fd")
        .with_payload(core_process::ProcessFdBinding { pid });
    let resource_id = context
        .runtime()
        .resources
        .insert(entry, Some(context.engine()));

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
    context: &BindingCallContext,
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

    let process_id = resolve_process_fd(context, handle)?;
    super::signals::process_kill(process_id.0, signal.0)
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
    context: &BindingCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessFdHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let process_id = resolve_process_fd(context, handle)?;
    let status =
        super::wait::process_wait_pid(process_id.0, super::wait::PROCESS_WAIT_FLAG_NOHANG)?;
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
    context: &BindingCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessFdHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let process_id = resolve_process_fd(context, handle)?;
    let status = super::wait::process_wait_pid_timeout(context, process_id.0, timeoutns)?;
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
    context: &BindingCallContext,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    ensure_signal_fd_handle(context, handle)?;

    let removed = context
        .runtime()
        .resources
        .remove_and_finalize(handle.0, Some(context.engine()));
    if !removed {
        return Err(core_platform::unknown_handle("handle", "signal fd"));
    }

    Ok(())
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
    context: &BindingCallContext,
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

    let signals = unsafe { signals.as_slice()? }.to_vec();
    let entry = resource::ResourceEntry::new(resource::ResourceKind::SignalFd)
        .with_label("process.signal.fd")
        .with_payload(core_process::SignalFdBinding { signals });
    let resource_id = context
        .runtime()
        .resources
        .insert(entry, Some(context.engine()));

    unsafe {
        *out = resource::SignalFdHandle(resource_id);
    }

    Ok(())
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
    context: &BindingCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let signals = resolve_signal_fd(context, handle)?;
    let event = super::signals::process_signal_wait(&signals)?;
    unsafe {
        *out = event;
    }

    Ok(())
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
    context: &BindingCallContext,
    handle: resource::SignalFdHandle,
    signals: NativeSlice<Signal>,
) -> RuntimeResult<()> {
    let signals = unsafe { signals.as_slice()? }.to_vec();
    update_signal_fd(context, handle, signals)
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
    context: &BindingCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let signals = resolve_signal_fd(context, handle)?;
    let event = super::signals::process_signal_try_wait(&signals)?;
    unsafe {
        *out = event;
    }

    Ok(())
}
