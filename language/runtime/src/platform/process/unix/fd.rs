#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
use crate::platform::process::core as core_process;
use crate::platform::{PlatformError, core as core_platform};

use crate::runtime::BindingCallContext;

#[cfg(target_os = "linux")]
use super::signals;
#[cfg(target_os = "linux")]
use super::wait;

use crate::platform::process::{
    ProcessFdFlags, ProcessFdSignalFlags, ProcessId, ProcessWaitStatus, Signal, SignalEvent,
    SignalFdFlags,
};
use crate::platform::resource;

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

/// Read the current thread errno value.
#[cfg(target_os = "linux")]
fn last_errno() -> i32 {
    std::io::Error::last_os_error()
        .raw_os_error()
        .unwrap_or(libc::EINVAL)
}

/// Build one `ioWouldBlock` error for a descriptor wait timeout.
#[cfg(target_os = "linux")]
fn would_block_error(operation: &str, message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(crate::platform::diagnostic::PlatformErrorCode::IoWouldBlock),
        None,
        Some(libc::EWOULDBLOCK),
        Some(operation.to_string()),
        None,
        message,
    ))
    .boxed()
}

/// Convert one requested signal slice into a host signal set.
#[cfg(target_os = "linux")]
fn signal_set_from_slice(signals: &[Signal]) -> RuntimeResult<libc::sigset_t> {
    let mut signal_set = unsafe { std::mem::zeroed::<libc::sigset_t>() };
    let empty_result = unsafe { libc::sigemptyset(&mut signal_set) };
    if empty_result != 0 {
        return Err(core_process::process_errno_error(
            last_errno(),
            "sigemptyset",
            "failed to initialize signal set",
        ));
    }

    for signal in signals {
        if signal.0 == 0 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "signals",
                "signal id 0 is invalid",
            ))
            .boxed());
        }

        let add_result = unsafe { libc::sigaddset(&mut signal_set, signal.0 as libc::c_int) };
        if add_result != 0 {
            return Err(core_process::process_errno_error(
                last_errno(),
                "sigaddset",
                "failed to build signal set",
            ));
        }
    }

    Ok(signal_set)
}

/// Open one linux pidfd for the target process id.
#[cfg(target_os = "linux")]
fn open_pidfd(pid: ProcessId, flags: ProcessFdFlags) -> RuntimeResult<i32> {
    let pid = core_process::process_pid_to_unix_target(pid.0, "pid")?;
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, flags.0 as libc::c_uint) as i32 };
    if fd < 0 {
        let errno = last_errno();
        if errno == libc::ENOSYS {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.process.fd.processFdOpen",
            ))
            .boxed());
        }

        return Err(core_process::process_errno_error(
            errno,
            "pidfd_open",
            format!("failed to open process fd for pid {}", pid as u32),
        ));
    }

    Ok(fd)
}

/// Send one signal through a linux pidfd.
#[cfg(target_os = "linux")]
fn send_pidfd_signal(fd: i32, signal: Signal, flags: ProcessFdSignalFlags) -> RuntimeResult<()> {
    let result = unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            fd,
            signal.0 as libc::c_int,
            std::ptr::null::<libc::siginfo_t>(),
            flags.0 as libc::c_uint,
        ) as i32
    };
    if result == 0 {
        return Ok(());
    }

    let errno = last_errno();
    if errno == libc::ENOSYS {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.fd.processFdSendSignal",
        ))
        .boxed());
    }

    Err(core_process::process_errno_error(
        errno,
        "pidfd_send_signal",
        "failed to send signal through process fd",
    ))
}

/// Poll one descriptor for process-exit readiness.
#[cfg(target_os = "linux")]
fn poll_descriptor(fd: i32, timeout_ms: i32, operation: &str) -> RuntimeResult<()> {
    let mut poll_fd = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    let result = unsafe { libc::poll(&mut poll_fd, 1, timeout_ms) };
    if result > 0 {
        return Ok(());
    }
    if result == 0 {
        return Err(would_block_error(operation, "process fd wait would block"));
    }

    Err(core_process::process_errno_error(
        last_errno(),
        "poll",
        "failed to poll process fd",
    ))
}

/// Open one linux signalfd for the provided mask.
#[cfg(target_os = "linux")]
fn open_signalfd(signals: &[Signal], flags: SignalFdFlags) -> RuntimeResult<i32> {
    if flags.0 != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "signal fd flags are not supported",
        ))
        .boxed());
    }

    signals::ensure_signals_blocked(signals)?;

    let signal_set = signal_set_from_slice(signals)?;
    let fd = unsafe { libc::signalfd(-1, &signal_set, libc::SFD_CLOEXEC) };
    if fd >= 0 {
        return Ok(fd);
    }

    let errno = last_errno();
    if errno == libc::ENOSYS {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.fd.signalFdOpen",
        ))
        .boxed());
    }

    Err(core_process::process_errno_error(
        errno,
        "signalfd",
        "failed to open signal fd",
    ))
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
    binding: &BindingCallContext,
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

/// Resolve a process-fd handle into its process id payload.
#[cfg(target_os = "linux")]
fn resolve_process_fd(
    binding: &BindingCallContext,
    handle: resource::ProcessFdHandle,
) -> RuntimeResult<(ProcessId, i32)> {
    let resolved = binding.worker().resources.with_entry(handle.0, |entry| {
        let pid = entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<core_process::ProcessFdBinding>())
            .map(|binding| binding.pid)?;
        let fd = entry.fd()?;

        Some((pid, fd))
    });

    resolved.flatten().ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown process fd handle",
        ))
        .boxed()
    })
}

/// Resolve a signal-fd handle into its signal mask payload.
#[cfg(target_os = "linux")]
fn resolve_signal_fd(
    binding: &BindingCallContext,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<(Vec<Signal>, i32)> {
    let resolved = binding.worker().resources.with_entry(handle.0, |entry| {
        let signals = entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<core_process::SignalFdBinding>())
            .map(|binding| binding.signals.clone())?;
        let fd = entry.fd()?;

        Some((signals, fd))
    });

    resolved.flatten().ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown signal fd handle",
        ))
        .boxed()
    })
}

/// Replace the signal mask payload for one signal-fd handle.
#[cfg(target_os = "linux")]
fn update_signal_fd(
    binding: &BindingCallContext,
    handle: resource::SignalFdHandle,
    signals: Vec<Signal>,
) -> RuntimeResult<()> {
    let updated = binding
        .worker()
        .resources
        .with_entry_mut(handle.0, |entry| {
            entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<core_process::SignalFdBinding>())
                .map(|binding| {
                    binding.signals = signals;
                })
        });

    if updated.flatten().is_none() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown signal fd handle",
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

/// Ensure one signal-fd handle resolves to a signal-fd payload.
fn ensure_signal_fd_handle(
    binding: &BindingCallContext,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    let is_signal_fd = binding.worker().resources.with_entry(handle.0, |entry| {
        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<core_process::SignalFdBinding>())
            .is_some()
    });

    if is_signal_fd != Some(true) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown signal fd handle",
        ))
        .boxed());
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
    binding: &BindingCallContext,
    out: *mut resource::FileHandle,
) -> RuntimeResult<()> {
    register_stdio_fd(binding, out, libc::STDIN_FILENO, "process.stdio.stdin")
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
    register_stdio_fd(binding, out, libc::STDOUT_FILENO, "process.stdio.stdout")
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
    register_stdio_fd(binding, out, libc::STDERR_FILENO, "process.stdio.stderr")
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

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, out, pid);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.fd.processFdOpen",
        ))
        .boxed())
    }

    #[cfg(target_os = "linux")]
    {
        let fd = open_pidfd(pid, flags)?;
        let entry = resource::ResourceEntry::new(resource::ResourceKind::ProcessFd)
            .with_label("process.fd")
            .with_fd(fd)
            .with_payload(core_process::ProcessFdBinding { pid })
            .with_finalizer(StdioFdFinalizer { fd });
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

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle, signal);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.fd.processFdSendSignal",
        ))
        .boxed())
    }

    #[cfg(target_os = "linux")]
    {
        let (_process_id, fd) = resolve_process_fd(binding, handle)?;
        send_pidfd_signal(fd, signal, flags)
    }
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
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.fd.processFdTryWait",
        ))
        .boxed())
    }

    #[cfg(target_os = "linux")]
    {
        let (process_id, fd) = resolve_process_fd(binding, handle)?;
        poll_descriptor(fd, 0, "destack.process.fd.processFdTryWait")?;

        let status = wait::process_wait_pid(process_id.0, wait::PROCESS_WAIT_FLAG_NOHANG)?;
        unsafe {
            *out = status;
        }

        Ok(())
    }
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
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle, timeoutns);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.fd.processFdWait",
        ))
        .boxed())
    }

    #[cfg(target_os = "linux")]
    {
        let (process_id, fd) = resolve_process_fd(binding, handle)?;
        let timeout_ms = if timeoutns == u64::MAX {
            -1
        } else {
            let timeout_ms = timeoutns / 1_000_000;
            i32::try_from(timeout_ms).unwrap_or(i32::MAX)
        };
        poll_descriptor(fd, timeout_ms, "destack.process.fd.processFdWait")?;

        let status = wait::process_wait_pid(process_id.0, 0)?;
        unsafe {
            *out = status;
        }

        Ok(())
    }
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
    binding: &BindingCallContext,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    ensure_signal_fd_handle(binding, handle)?;

    let removed = binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    );
    if !removed {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown signal fd handle",
        ))
        .boxed());
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
    binding: &BindingCallContext,
    out: *mut resource::SignalFdHandle,
    signals: NativeSlice<Signal>,
    flags: SignalFdFlags,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, signals, flags);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.fd.signalFdOpen",
        ))
        .boxed())
    }

    #[cfg(target_os = "linux")]
    {
        let signals = unsafe { signals.as_slice()? }.to_vec();
        let fd = open_signalfd(&signals, flags)?;
        let entry = resource::ResourceEntry::new(resource::ResourceKind::SignalFd)
            .with_label("process.signal.fd")
            .with_fd(fd)
            .with_payload(core_process::SignalFdBinding { signals })
            .with_finalizer(StdioFdFinalizer { fd });
        let resource_id =
            binding
                .worker()
                .resources
                .insert(&binding.world(), entry, Some(binding.engine()));

        unsafe {
            *out = resource::SignalFdHandle(resource_id);
        }

        Ok(())
    }
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
    binding: &BindingCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.fd.signalFdRead",
        ))
        .boxed())
    }

    #[cfg(target_os = "linux")]
    {
        let (_signals, fd) = resolve_signal_fd(binding, handle)?;
        let mut signal_info = unsafe { std::mem::zeroed::<libc::signalfd_siginfo>() };
        let read = unsafe {
            libc::read(
                fd,
                &mut signal_info as *mut libc::signalfd_siginfo as *mut libc::c_void,
                std::mem::size_of::<libc::signalfd_siginfo>(),
            )
        };
        if read < 0 {
            return Err(core_process::process_errno_error(
                last_errno(),
                "read",
                "failed to read signal fd",
            ));
        }
        if read as usize != std::mem::size_of::<libc::signalfd_siginfo>() {
            return Err(RuntimeError::from(PlatformError::io(
                "signal fd read returned a truncated payload",
            ))
            .boxed());
        }

        let event = SignalEvent {
            signal: Signal(signal_info.ssi_signo),
            pid: ProcessId(signal_info.ssi_pid),
        };
        unsafe {
            *out = event;
        }

        Ok(())
    }
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
    binding: &BindingCallContext,
    handle: resource::SignalFdHandle,
    signals: NativeSlice<Signal>,
) -> RuntimeResult<()> {
    let signals = unsafe { signals.as_slice()? }.to_vec();

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle, signals);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.fd.signalFdSetMask",
        ))
        .boxed())
    }

    #[cfg(target_os = "linux")]
    {
        let (_current_signals, fd) = resolve_signal_fd(binding, handle)?;
        signals::ensure_signals_blocked(&signals)?;
        let signal_set = signal_set_from_slice(&signals)?;
        let result = unsafe { libc::signalfd(fd, &signal_set, 0) };
        if result < 0 {
            return Err(core_process::process_errno_error(
                last_errno(),
                "signalfd",
                "failed to update signal fd mask",
            ));
        }

        update_signal_fd(binding, handle, signals)
    }
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
    binding: &BindingCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.fd.signalFdTryRead",
        ))
        .boxed())
    }

    #[cfg(target_os = "linux")]
    {
        let (_signals, fd) = resolve_signal_fd(binding, handle)?;
        poll_descriptor(fd, 0, "destack.process.fd.signalFdTryRead")?;

        let mut signal_info = unsafe { std::mem::zeroed::<libc::signalfd_siginfo>() };
        let read = unsafe {
            libc::read(
                fd,
                &mut signal_info as *mut libc::signalfd_siginfo as *mut libc::c_void,
                std::mem::size_of::<libc::signalfd_siginfo>(),
            )
        };
        if read < 0 {
            return Err(core_process::process_errno_error(
                last_errno(),
                "read",
                "failed to poll signal fd",
            ));
        }
        if read as usize != std::mem::size_of::<libc::signalfd_siginfo>() {
            return Err(RuntimeError::from(PlatformError::io(
                "signal fd poll returned a truncated payload",
            ))
            .boxed());
        }

        let event = SignalEvent {
            signal: Signal(signal_info.ssi_signo),
            pid: ProcessId(signal_info.ssi_pid),
        };
        unsafe {
            *out = event;
        }

        Ok(())
    }
}
