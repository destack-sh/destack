#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::{bindings_generated as bindings, core as core_process};
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, NativeStringSlice, PlatformError, PlatformErrorCode,
};

use crate::runtime::BindingCallContext;
use bindings::*;

use crate::platform::process::{
    ExecAtFlags, GroupId, ProcessCpuSet, ProcessFdAction, ProcessFdFlags, ProcessFdSignalFlags,
    ProcessGroupIds, ProcessId, ProcessLimit, ProcessLimitResource, ProcessNamespaceKind,
    ProcessSchedulerConfig, ProcessSchedulerPolicy, ProcessSpawnOptions, ProcessStdio,
    ProcessUnshareFlags, ProcessUserIds, ProcessWaitContinuedStatus, ProcessWaitExitedStatus,
    ProcessWaitFlags, ProcessWaitRunningStatus, ProcessWaitSignaledStatus, ProcessWaitStatus,
    ProcessWaitStoppedStatus, Signal, SignalEvent, SignalFdFlags, SignalMaskHow,
    SyscallFilterFlags, UserId,
};
use crate::platform::{fs, resource};
use std::time::{Duration, Instant};

/// Resolve a process handle into its process id payload.
fn resolve_spawned_process_handle(
    context: &BindingCallContext,
    handle: resource::ProcessHandle,
) -> RuntimeResult<ProcessId> {
    let resolved = context.runtime().resources.with_entry(handle.0, |entry| {
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
    _context: &BindingCallContext,
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
    context: &BindingCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let process_id = resolve_spawned_process_handle(context, handle)?;
    let status = process_wait_pid(process_id.0, PROCESS_WAIT_FLAG_NOHANG)?;

    if is_terminal_wait_status(&status) {
        let _ = context.runtime().resources.remove_and_finalize(handle.0);
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
    context: &BindingCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessHandle,
    flags: ProcessWaitFlags,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let process_id = resolve_spawned_process_handle(context, handle)?;
    let status = process_wait_pid(process_id.0, flags.0)?;

    if is_terminal_wait_status(&status) {
        let _ = context.runtime().resources.remove_and_finalize(handle.0);
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

/// Wait for one process state transition with a timeout.
pub(super) fn process_wait_pid_timeout(
    pid: u32,
    timeout_ns: u64,
) -> RuntimeResult<ProcessWaitStatus> {
    if timeout_ns == 0 {
        return process_wait_pid(pid, PROCESS_WAIT_FLAG_NOHANG);
    }

    let timeout = Duration::from_nanos(timeout_ns);
    let deadline = Instant::now().checked_add(timeout).ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "timeoutns",
            "timeout is too large",
        ))
        .boxed()
    })?;

    let mut sleep_duration = Duration::from_micros(100);
    let max_sleep_duration = Duration::from_millis(10);
    loop {
        match process_wait_pid(pid, PROCESS_WAIT_FLAG_NOHANG) {
            Ok(status) => return Ok(status),
            Err(error) => {
                if !is_would_block_error(&error) {
                    return Err(error);
                }

                if Instant::now() >= deadline {
                    return Err(error);
                }
            }
        }

        let now = Instant::now();
        let remaining = deadline.saturating_duration_since(now);
        let sleep = remaining.min(sleep_duration);
        if !sleep.is_zero() {
            sleep_for_duration(sleep);
        }
        sleep_duration = (sleep_duration * 2).min(max_sleep_duration);
    }
}

/// Sleep for one duration using host nanosleep semantics.
fn sleep_for_duration(duration: Duration) {
    if duration.is_zero() {
        return;
    }

    let max_seconds = libc::time_t::MAX as u64;
    let requested_seconds = duration.as_secs().min(max_seconds);
    let mut requested = libc::timespec {
        tv_sec: requested_seconds as libc::time_t,
        tv_nsec: duration.subsec_nanos() as libc::c_long,
    };

    loop {
        let mut remaining = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let rc = unsafe { libc::nanosleep(&requested, &mut remaining) };
        if rc == 0 {
            return;
        }

        let errno = std::io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::EINVAL);
        if errno != libc::EINTR {
            return;
        }

        requested = remaining;
    }
}

/// Return true when a runtime error maps to `ioWouldBlock`.
fn is_would_block_error(error: &RuntimeError) -> bool {
    let Some(platform_error) = error.platform_error() else {
        return false;
    };

    platform_error.code == PlatformErrorCode::IoWouldBlock
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
