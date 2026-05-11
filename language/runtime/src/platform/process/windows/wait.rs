#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::core as core_process;
use crate::platform::{PlatformError, PlatformErrorCode, core as core_platform};

use crate::runtime::BindingCallContext;

use crate::platform::process::{
    ProcessId, ProcessWaitExitedStatus, ProcessWaitFlags, ProcessWaitRunningStatus,
    ProcessWaitStatus,
};
use crate::platform::resource;

/// Resolve a process handle into its process id payload.
fn resolve_spawned_process_handle(
    binding: &BindingCallContext,
    handle: resource::ProcessHandle,
) -> RuntimeResult<(ProcessId, Option<windows_sys::Win32::Foundation::HANDLE>)> {
    let resolved = binding.worker().resources.with_entry(handle.0, |entry| {
        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<core_process::SpawnedProcess>())
            .map(|process| (process.pid, entry.handle().map(|handle| handle as _)))
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
    matches!(status, ProcessWaitStatus::ProcessWaitExitedStatus(_))
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
    wait_process_handle_with_timeout(binding, pid, process_handle, timeout_ms)
}

/// Wait for a process identifier.
pub(crate) unsafe fn destack_process_wait_pid(
    binding: &BindingCallContext,
    out: *mut ProcessWaitStatus,
    pid: ProcessId,
    flags: ProcessWaitFlags,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = process_wait_pid(binding, pid.0, flags.0)?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Poll a child process handle without blocking.
pub(crate) unsafe fn destack_process_try_wait(
    binding: &BindingCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let (process_id, process_handle) = resolve_spawned_process_handle(binding, handle)?;
    let status = match process_handle {
        Some(process_handle) => wait_process_handle_with_flags(
            binding,
            process_id,
            process_handle,
            ProcessWaitFlags(PROCESS_WAIT_FLAG_NOHANG),
        )?,
        None => process_wait_pid(binding, process_id.0, PROCESS_WAIT_FLAG_NOHANG)?,
    };

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
pub(crate) unsafe fn destack_process_wait(
    binding: &BindingCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessHandle,
    flags: ProcessWaitFlags,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let (process_id, process_handle) = resolve_spawned_process_handle(binding, handle)?;
    let status = match process_handle {
        Some(process_handle) => {
            wait_process_handle_with_flags(binding, process_id, process_handle, flags)?
        }
        None => process_wait_pid(binding, process_id.0, flags.0)?,
    };

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
pub(crate) const PROCESS_WAIT_FLAG_NOHANG: u32 = 0x0000_0001;

/// Wait for one process state transition.
fn process_wait_pid(
    binding: &BindingCallContext,
    pid: u32,
    flags: u32,
) -> RuntimeResult<ProcessWaitStatus> {
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_INVALID_PARAMETER, STILL_ACTIVE, WAIT_FAILED, WAIT_OBJECT_0,
        WAIT_TIMEOUT,
    };
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
    let status = match wait_status {
        WAIT_TIMEOUT => Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoWouldBlock),
            None,
            None,
            Some("WaitForSingleObject".to_string()),
            None,
            format!("wait would block for pid {pid}"),
        ))
        .boxed()),
        WAIT_OBJECT_0 => {
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
                        kind: binding.store_string("running"),
                        pid: ProcessId(pid),
                    },
                ))
            } else {
                Ok(ProcessWaitStatus::ProcessWaitExitedStatus(
                    ProcessWaitExitedStatus {
                        kind: binding.store_string("exited"),
                        pid: ProcessId(pid),
                        exit_code: exit_code as i32,
                    },
                ))
            }
        }
        WAIT_FAILED => {
            let error = core_platform::last_error_code();
            Err(RuntimeError::from(PlatformError::io(format!(
                "wait failed for pid {pid}: {error}",
            )))
            .boxed())
        }
        _ => Err(RuntimeError::from(PlatformError::io("wait returned unexpected result")).boxed()),
    };

    unsafe {
        CloseHandle(process);
    }

    status
}
