#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::{bindings_generated as bindings, core as core_process};
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, NativeStringSlice, PlatformError, PlatformErrorCode,
    core as core_platform,
};

use crate::runtime::BindingCallContext;
use bindings::*;

use crate::platform::process::{
    ExecAtFlags, GroupId, ProcessCpuSet, ProcessFdAction, ProcessFdFlags, ProcessFdSignalFlags,
    ProcessGroupIds, ProcessId, ProcessLimit, ProcessLimitResource, ProcessNamespaceKind,
    ProcessSchedulerConfig, ProcessSchedulerPolicy, ProcessSpawnOptions, ProcessStdio,
    ProcessUnshareFlags, ProcessUserIds, ProcessWaitExitedStatus, ProcessWaitFlags,
    ProcessWaitRunningStatus, ProcessWaitSignaledStatus, ProcessWaitStatus, Signal, SignalEvent,
    SignalFdFlags, SignalMaskHow, SyscallFilterFlags, UserId,
};
use crate::platform::{fs, resource};

/// Resolve a process handle into its process id payload.
fn resolve_spawned_process_handle(
    context: &BindingCallContext,
    handle: resource::ProcessHandle,
) -> RuntimeResult<(ProcessId, Option<windows_sys::Win32::Foundation::HANDLE>)> {
    resource::require_payload_with::<core_process::SpawnedProcess, _>(
        context,
        handle.0,
        resource::ResourceKind::Process,
        None,
        "handle",
        "process",
        |process, entry| (process.pid, entry.handle().map(|handle| handle as _)),
    )
}

/// Return true when a wait status is terminal for a spawned process.
fn is_terminal_wait_status(status: &ProcessWaitStatus) -> bool {
    matches!(
        status,
        ProcessWaitStatus::ProcessWaitExitedStatus(_)
            | ProcessWaitStatus::ProcessWaitSignaledStatus(_)
    )
}

/// Wait one process handle with an explicit timeout in milliseconds.
fn wait_process_handle_with_timeout(
    pid: ProcessId,
    process_handle: windows_sys::Win32::Foundation::HANDLE,
    timeout_ms: u32,
) -> RuntimeResult<ProcessWaitStatus> {
    use windows_sys::Win32::Foundation::STILL_ACTIVE;
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};

    let wait_status = unsafe { WaitForSingleObject(process_handle, timeout_ms) };
    match core_platform::decode_wait_for_single_object_status(wait_status, "WaitForSingleObject")? {
        core_platform::WaitStatus::TimedOut => Err(core_platform::io_would_block(
            "WaitForSingleObject",
            format!("wait timed out for pid {}", pid.0),
        )),
        core_platform::WaitStatus::Signaled => {
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
                        kind: "running".into(),
                        pid,
                    },
                ));
            }

            Ok(ProcessWaitStatus::ProcessWaitExitedStatus(
                ProcessWaitExitedStatus {
                    kind: "exited".into(),
                    pid,
                    exit_code: exit_code as i32,
                },
            ))
        }
        core_platform::WaitStatus::Abandoned => Err(core_platform::io_error("WaitForSingleObject")),
    }
}

/// Wait one process handle using process wait flags.
fn wait_process_handle_with_flags(
    pid: ProcessId,
    process_handle: windows_sys::Win32::Foundation::HANDLE,
    flags: ProcessWaitFlags,
) -> RuntimeResult<ProcessWaitStatus> {
    use windows_sys::Win32::System::Threading::INFINITE;

    if flags.0 & !PROCESS_WAIT_FLAG_NOHANG != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "unsupported process wait flags",
        ))
        .boxed());
    }

    let timeout_ms = if flags.0 & PROCESS_WAIT_FLAG_NOHANG != 0 {
        0
    } else {
        INFINITE
    };
    wait_process_handle_with_timeout(pid, process_handle, timeout_ms)
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
    let (process_id, process_handle) = resolve_spawned_process_handle(context, handle)?;
    let status = match process_handle {
        Some(process_handle) => wait_process_handle_with_flags(
            process_id,
            process_handle,
            ProcessWaitFlags(PROCESS_WAIT_FLAG_NOHANG),
        )?,
        None => process_wait_pid(process_id.0, PROCESS_WAIT_FLAG_NOHANG)?,
    };

    if is_terminal_wait_status(&status) {
        let _ = context
            .runtime()
            .resources
            .remove_and_finalize(handle.0, Some(context.engine()));
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
    let (process_id, process_handle) = resolve_spawned_process_handle(context, handle)?;
    let status = match process_handle {
        Some(process_handle) => wait_process_handle_with_flags(process_id, process_handle, flags)?,
        None => process_wait_pid(process_id.0, flags.0)?,
    };

    if is_terminal_wait_status(&status) {
        let _ = context
            .runtime()
            .resources
            .remove_and_finalize(handle.0, Some(context.engine()));
    }

    unsafe {
        *out = status;
    }

    Ok(())
}

/// Nonblocking wait flag used by process wait bindings.
pub(crate) const PROCESS_WAIT_FLAG_NOHANG: u32 = 0x0000_0001;

/// Wait for one process state transition.
fn process_wait_pid(pid: u32, flags: u32) -> RuntimeResult<ProcessWaitStatus> {
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_INVALID_PARAMETER, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, INFINITE, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        WaitForSingleObject,
    };

    let pid = core_process::process_pid_to_windows_target(pid, "pid")?;
    if flags & !PROCESS_WAIT_FLAG_NOHANG != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "unsupported process wait flags",
        ))
        .boxed());
    }

    const PROCESS_SYNCHRONIZE: u32 = 0x0010_0000;
    let process = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
            0,
            pid,
        )
    };
    if process == 0 {
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
            format!("failed to open process {pid} for wait"),
        ))
        .boxed());
    }

    let timeout = if flags & PROCESS_WAIT_FLAG_NOHANG != 0 {
        0
    } else {
        INFINITE
    };
    let wait_status = unsafe { WaitForSingleObject(process, timeout) };
    let status = match core_platform::decode_wait_for_single_object_status(
        wait_status,
        "WaitForSingleObject",
    )? {
        core_platform::WaitStatus::TimedOut => Err(core_platform::io_would_block(
            "WaitForSingleObject",
            format!("wait would block for pid {pid}"),
        )),
        core_platform::WaitStatus::Signaled => {
            let mut exit_code = 0_u32;
            let rc = unsafe { GetExitCodeProcess(process, &mut exit_code) };
            if rc == 0 {
                let error = core_platform::last_error_code();
                Err(RuntimeError::from(PlatformError::io(format!(
                    "failed to read process exit code: {error}",
                )))
                .boxed())
            } else if exit_code == STILL_ACTIVE as u32 {
                Ok(ProcessWaitStatus::ProcessWaitRunningStatus(
                    ProcessWaitRunningStatus {
                        kind: "running".into(),
                        pid: ProcessId(pid),
                    },
                ))
            } else {
                Ok(ProcessWaitStatus::ProcessWaitExitedStatus(
                    ProcessWaitExitedStatus {
                        kind: "exited".into(),
                        pid: ProcessId(pid),
                        exit_code: exit_code as i32,
                    },
                ))
            }
        }
        core_platform::WaitStatus::Abandoned => Err(core_platform::io_error("WaitForSingleObject")),
    };

    unsafe {
        CloseHandle(process);
    }

    status
}
