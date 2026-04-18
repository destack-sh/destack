#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::core as core_process;
use crate::platform::{PlatformError, PlatformErrorCode};

use crate::runtime::BindingCallContext;

use crate::platform::process::{
    ProcessId, ProcessWaitContinuedStatus, ProcessWaitExitedStatus, ProcessWaitFlags,
    ProcessWaitRunningStatus, ProcessWaitSignaledStatus, ProcessWaitStatus,
    ProcessWaitStoppedStatus, Signal,
};
use crate::platform::resource;

/// Resolve a process handle into its process id payload.
fn resolve_spawned_process_handle(
    binding: &BindingCallContext,
    handle: resource::ProcessHandle,
) -> RuntimeResult<ProcessId> {
    let resolved = binding.worker().resources.with_entry(handle.0, |entry| {
        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<core_process::SpawnedProcess>())
            .map(|process| process.pid)
    });

    resolved.flatten().ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown process handle",
        ))
        .boxed()
    })
}

/// Return true when a wait status is terminal for a spawned process.
fn is_terminal_wait_status(status: &ProcessWaitStatus) -> bool {
    matches!(
        status,
        ProcessWaitStatus::ProcessWaitExitedStatus(_)
            | ProcessWaitStatus::ProcessWaitSignaledStatus(_)
    )
}
/// Wait for a process identifier.
///
/// Wait for one state transition for the specified process identifier.
/// Identifier matching and visibility follow host process table and job-control rules.
///
/// # Platform
/// Unix and Windows.
/// Uses waitpid or waitid on Unix and process-handle wait translation on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.wait`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_wait_pid(
    _binding: &BindingCallContext,
    out: *mut ProcessWaitStatus,
    pid: ProcessId,
    flags: ProcessWaitFlags,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = process_wait_pid(pid.0, flags.0)?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Poll a child process handle without blocking.
///
/// Poll one child for a state transition and return immediately when no transition is pending.
/// Pending absence is reported through `ioWouldBlock` without sleeping.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking waitpid or waitid on Unix and zero-timeout wait on Windows.
///
/// # Errors
/// Returns processNotFound, processPermissionDenied, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.wait`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_try_wait(
    binding: &BindingCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let process_id = resolve_spawned_process_handle(binding, handle)?;
    let status = process_wait_pid(process_id.0, PROCESS_WAIT_FLAG_NOHANG)?;

    if is_terminal_wait_status(&status) {
        let _ = binding.worker().resources.remove_and_finalize(
            &binding.world(),
            handle.0,
            Some(binding.engine()),
        );
    }

    unsafe {
        *out = status;
    }

    Ok(())
}

/// Wait for a child process handle.
///
/// Wait for one child state transition and return a normalized wait status payload.
/// Blocking and state-filter behavior is controlled by wait flags and host wait semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses waitpid or waitid on Unix and WaitForSingleObject plus status queries on Windows.
///
/// # Errors
/// Returns processNotFound, processPermissionDenied, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.wait`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_wait(
    binding: &BindingCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessHandle,
    flags: ProcessWaitFlags,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let process_id = resolve_spawned_process_handle(binding, handle)?;
    let status = process_wait_pid(process_id.0, flags.0)?;

    if is_terminal_wait_status(&status) {
        let _ = binding.worker().resources.remove_and_finalize(
            &binding.world(),
            handle.0,
            Some(binding.engine()),
        );
    }

    unsafe {
        *out = status;
    }

    Ok(())
}

/// Nonblocking wait flag used by process wait bindings.
pub(crate) const PROCESS_WAIT_FLAG_NOHANG: u32 = libc::WNOHANG as u32;

/// Wait for one child process state transition.
pub(super) fn process_wait_pid(pid: u32, flags: u32) -> RuntimeResult<ProcessWaitStatus> {
    let pid = core_process::process_pid_to_unix_target(pid, "pid")?;
    let options = decode_wait_flags(flags)?;

    let mut raw_status: libc::c_int = 0;
    let waited = unsafe { libc::waitpid(pid, &mut raw_status, options) };

    if waited == 0 {
        return Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoWouldBlock),
            None,
            Some(libc::EWOULDBLOCK),
            Some("waitpid".to_string()),
            None,
            format!("waitpid would block for pid {}", pid as u32),
        ))
        .boxed());
    }

    if waited < 0 {
        return Err(waitpid_error(pid as u32, flags));
    }

    Ok(wait_status_from_raw(waited, raw_status))
}

/// Decode public wait flags into host waitpid flags.
fn decode_wait_flags(flags: u32) -> RuntimeResult<libc::c_int> {
    let supported = (libc::WNOHANG | libc::WUNTRACED | libc::WCONTINUED) as u32;
    if flags & !supported != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "unsupported process wait flags",
        ))
        .boxed());
    }

    Ok(flags as libc::c_int)
}

/// Build a process wait error from the current errno state.
fn waitpid_error(pid: u32, flags: u32) -> Box<RuntimeError> {
    let error = std::io::Error::last_os_error();
    let errno = error.raw_os_error().unwrap_or(libc::EINVAL);

    let code = match errno {
        libc::ECHILD => PlatformErrorCode::ProcessNotFound,
        libc::EACCES | libc::EPERM => PlatformErrorCode::ProcessPermissionDenied,
        libc::EINTR => PlatformErrorCode::IoInterrupted,
        value if value == libc::EAGAIN || value == libc::EWOULDBLOCK => {
            PlatformErrorCode::IoWouldBlock
        }
        _ => PlatformErrorCode::ProcessWaitFailed,
    };

    if code == PlatformErrorCode::IoInterrupted || code == PlatformErrorCode::IoWouldBlock {
        return RuntimeError::from(PlatformError::io_with(
            Some(code),
            None,
            Some(errno),
            Some("waitpid".to_string()),
            None,
            format!("waitpid failed for pid {pid} with flags {flags}"),
        ))
        .boxed();
    }

    RuntimeError::from(PlatformError::process_with(
        Some(code),
        Some(errno.to_string()),
        None,
        None,
        Some("waitpid".to_string()),
        format!("waitpid failed for pid {pid} with flags {flags}"),
    ))
    .boxed()
}

/// Decode a host wait status into the platform wait payload.
fn wait_status_from_raw(waited: libc::pid_t, raw_status: libc::c_int) -> ProcessWaitStatus {
    if libc::WIFEXITED(raw_status) {
        return ProcessWaitStatus::ProcessWaitExitedStatus(ProcessWaitExitedStatus {
            kind: "exited".into(),
            pid: ProcessId(waited as u32),
            exit_code: libc::WEXITSTATUS(raw_status),
        });
    }

    if libc::WIFSIGNALED(raw_status) {
        return ProcessWaitStatus::ProcessWaitSignaledStatus(ProcessWaitSignaledStatus {
            kind: "signaled".into(),
            pid: ProcessId(waited as u32),
            signal: Signal(libc::WTERMSIG(raw_status) as u32),
            core_dumped: libc::WCOREDUMP(raw_status),
        });
    }

    if libc::WIFSTOPPED(raw_status) {
        return ProcessWaitStatus::ProcessWaitStoppedStatus(ProcessWaitStoppedStatus {
            kind: "stopped".into(),
            pid: ProcessId(waited as u32),
            signal: Signal(libc::WSTOPSIG(raw_status) as u32),
        });
    }

    if libc::WIFCONTINUED(raw_status) {
        return ProcessWaitStatus::ProcessWaitContinuedStatus(ProcessWaitContinuedStatus {
            kind: "continued".into(),
            pid: ProcessId(waited as u32),
        });
    }

    ProcessWaitStatus::ProcessWaitRunningStatus(ProcessWaitRunningStatus {
        kind: "running".into(),
        pid: ProcessId(waited as u32),
    })
}
