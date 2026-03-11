#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::process_error_code_from_errno;
use crate::platform::{PlatformError, PlatformErrorCode};

use crate::runtime::BindingCallContext;

use crate::platform::process::ProcessId;

/// Build one process-domain runtime error from errno.
fn session_errno_error(errno: i32, syscall: &str, message: impl Into<String>) -> Box<RuntimeError> {
    let code = process_error_code_from_errno(errno).unwrap_or(PlatformErrorCode::Process);
    RuntimeError::from(PlatformError::process_with(
        Some(code),
        Some(errno.to_string()),
        None,
        None,
        Some(syscall.to_string()),
        message.into(),
    ))
    .boxed()
}

/// Validate one process id for unix session syscalls.
fn session_pid_to_unix_target(pid: ProcessId, field: &str) -> RuntimeResult<libc::pid_t> {
    if pid.0 == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "process id must be greater than zero",
        ))
        .boxed());
    }
    if pid.0 > libc::pid_t::MAX as u32 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "process id exceeds host pid range",
        ))
        .boxed());
    }

    Ok(pid.0 as libc::pid_t)
}

/// Read a process group id.
///
/// Read the process-group identifier currently assigned to the target process.
/// Visibility and lookup behavior follow host process table rules.
///
/// # Platform
/// Unix and Windows.
/// Uses getpgid(2) on Unix and runtime emulation or `notSupported` on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.session`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_getpgid(
    _binding: &BindingCallContext,
    out: *mut ProcessId,
    pid: ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let pid = session_pid_to_unix_target(pid, "pid")?;
    let value = unsafe { libc::getpgid(pid) };
    if value < 0 {
        let errno = std::io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::EINVAL);
        return Err(session_errno_error(
            errno,
            "getpgid",
            format!("failed to read process group for pid {}", pid as u32),
        ));
    }

    unsafe {
        *out = ProcessId(value as u32);
    }

    Ok(())
}

/// Set a process group id for a process.
///
/// Move the target process into the requested process group identifier.
/// Cross-session moves and permission checks follow host kernel job-control rules.
///
/// # Platform
/// Unix and Windows.
/// Uses setpgid(2) on Unix and runtime emulation or `notSupported` on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.session`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_setpgid(
    _binding: &BindingCallContext,
    pid: ProcessId,
    pgid: ProcessId,
) -> RuntimeResult<()> {
    let pid = session_pid_to_unix_target(pid, "pid")?;
    let pgid = session_pid_to_unix_target(pgid, "pgid")?;
    let result = unsafe { libc::setpgid(pid, pgid) };
    if result != 0 {
        let errno = std::io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::EINVAL);
        return Err(session_errno_error(
            errno,
            "setpgid",
            format!(
                "failed to set process group {} for pid {}",
                pgid as u32, pid as u32
            ),
        ));
    }

    Ok(())
}

/// Create a new session and return the new session leader id.
///
/// Create a new session boundary and make the caller its session leader.
/// Session and controlling-terminal semantics follow host job-control rules.
///
/// # Platform
/// Unix and Windows.
/// Uses setsid(2) on Unix and runtime emulation or `notSupported` on Windows.
///
/// # Errors
/// Returns processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.session`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_setsid(
    _binding: &BindingCallContext,
    out: *mut ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let value = unsafe { libc::setsid() };
    if value < 0 {
        let errno = std::io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::EPERM);
        return Err(session_errno_error(
            errno,
            "setsid",
            "failed to create a new session",
        ));
    }

    unsafe {
        *out = ProcessId(value as u32);
    }

    Ok(())
}
