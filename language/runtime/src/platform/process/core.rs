use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
#[cfg(unix)]
use crate::platform::PlatformErrorCode;
#[cfg(unix)]
use crate::platform::diagnostic::process_error_code_from_errno;
use crate::platform::process::ProcessId;
#[cfg(unix)]
use crate::platform::{process::Signal, resource};
#[cfg(unix)]
use crate::runtime::BindingCallContext;
#[cfg(unix)]
use std::ffi::CString;

/// Build an error for a missing environment variable.
pub(crate) fn missing_env_error(name: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument_value(
        "name",
        format!("environment variable not found: {}", name.into()),
    ))
    .boxed()
}

/// Validate that a string argument does not contain nul bytes.
pub(crate) fn ensure_no_nul_str(
    value: &str,
    field: &str,
    message: &'static str,
) -> RuntimeResult<()> {
    if value.contains('\0') {
        return Err(
            RuntimeError::from(PlatformError::invalid_argument_value(field, message)).boxed(),
        );
    }

    Ok(())
}

/// Validate that a byte argument does not contain nul bytes.
pub(crate) fn ensure_no_nul_bytes(
    value: &[u8],
    field: &str,
    message: &'static str,
) -> RuntimeResult<()> {
    if value.contains(&0) {
        return Err(
            RuntimeError::from(PlatformError::invalid_argument_value(field, message)).boxed(),
        );
    }

    Ok(())
}

/// Build a unix CString for one string argument after nul validation.
#[cfg(unix)]
pub(crate) fn cstring_from_str(
    value: &str,
    field: &str,
    message: &'static str,
) -> RuntimeResult<CString> {
    ensure_no_nul_str(value, field, message)?;
    CString::new(value).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(field, message)).boxed()
    })
}

/// Build a unix CString for one byte argument after nul validation.
#[cfg(unix)]
pub(crate) fn cstring_from_bytes(
    value: &[u8],
    field: &str,
    message: &'static str,
) -> RuntimeResult<CString> {
    ensure_no_nul_bytes(value, field, message)?;
    CString::new(value).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(field, message)).boxed()
    })
}

/// Build a process-domain error from the current errno state.
#[cfg(unix)]
pub(crate) fn process_last_error(syscall: &str, message: impl Into<String>) -> Box<RuntimeError> {
    let errno = std::io::Error::last_os_error()
        .raw_os_error()
        .unwrap_or(libc::EINVAL);
    process_errno_error(errno, syscall, message)
}

/// Build a process-domain error from a specific errno value.
#[cfg(unix)]
pub(crate) fn process_errno_error(
    errno: i32,
    syscall: &str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    let message = message.into();
    let mapped = match errno {
        libc::EINTR => PlatformErrorCode::IoInterrupted,
        value if value == libc::EAGAIN || value == libc::EWOULDBLOCK => {
            PlatformErrorCode::IoWouldBlock
        }
        _ => process_error_code_from_errno(errno).unwrap_or(PlatformErrorCode::Process),
    };

    if mapped == PlatformErrorCode::IoWouldBlock || mapped == PlatformErrorCode::IoInterrupted {
        return RuntimeError::from(PlatformError::io_with(
            Some(mapped),
            None,
            Some(errno),
            Some(syscall.to_string()),
            None,
            message,
        ))
        .boxed();
    }

    RuntimeError::from(PlatformError::process_with(
        Some(mapped),
        Some(errno.to_string()),
        None,
        None,
        Some(syscall.to_string()),
        message,
    ))
    .boxed()
}

/// Validate one process identifier for syscalls that target a single process.
#[cfg(unix)]
pub(crate) fn process_pid_to_unix_target(pid: u32, field: &str) -> RuntimeResult<libc::pid_t> {
    if pid == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "process id must be greater than zero",
        ))
        .boxed());
    }
    if pid > libc::pid_t::MAX as u32 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "process id exceeds host pid range",
        ))
        .boxed());
    }

    Ok(pid as libc::pid_t)
}

/// Validate one process identifier for windows process-targeted syscalls.
#[cfg(windows)]
pub(crate) fn process_pid_to_windows_target(pid: u32, field: &str) -> RuntimeResult<u32> {
    if pid == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "process id must be greater than zero",
        ))
        .boxed());
    }

    Ok(pid)
}

/// Resolve a signal subscription handle into its signal set.
#[cfg(unix)]
pub(crate) fn resolve_signal_subscription(
    binding: &BindingCallContext,
    handle: resource::SignalHandle,
) -> RuntimeResult<Vec<Signal>> {
    let resolved = binding.worker().resources.with_entry(handle.0, |entry| {
        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<SignalSubscription>())
            .map(|subscription| subscription.signals.clone())
    });

    resolved.flatten().ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown signal subscription handle",
        ))
        .boxed()
    })
}

/// Subscription payload stored for signal handles.
#[cfg(unix)]
#[derive(Debug, Clone)]
pub(crate) struct SignalSubscription {
    /// Signals associated with this subscription.
    pub signals: Vec<Signal>,
}

/// Process payload stored for spawned process handles.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SpawnedProcess {
    /// Process id associated with the handle.
    pub pid: ProcessId,
}

/// Process payload stored for process-fd style handles.
#[cfg_attr(
    all(unix, not(target_os = "linux"), not(target_os = "android")),
    allow(dead_code)
)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProcessFdBinding {
    /// Process id associated with the descriptor handle.
    #[cfg_attr(target_os = "android", allow(dead_code))]
    pub pid: ProcessId,
}

/// Process payload stored for signal-fd style handles.
#[cfg_attr(
    all(unix, not(target_os = "linux"), not(target_os = "android")),
    allow(dead_code)
)]
#[cfg(unix)]
#[derive(Debug, Clone)]
pub(crate) struct SignalFdBinding {
    /// Signal mask associated with the descriptor handle.
    #[cfg_attr(target_os = "android", allow(dead_code))]
    pub signals: Vec<Signal>,
}
