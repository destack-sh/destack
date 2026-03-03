#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::{bindings_generated as bindings, core as core_process};
use crate::platform::{NativeArray, PlatformError, PlatformErrorCode, core as core_platform};
use crate::runtime::{NativeSlice, NativeStringRef, NativeStringSlice};

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
/// Read process CPU affinity.
///
/// Read the active CPU affinity mask for the target process identifier.
/// Returned CPUs reflect host scheduler topology visibility.
///
/// # Platform
/// Unix and Windows.
/// Uses sched_getaffinity(2) on Unix and GetProcessAffinityMask on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.affinity`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_get_affinity(
    context: &BindingCallContext,
    out: *mut ProcessCpuSet,
    pid: ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, pid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.getAffinity",
    ))
    .boxed())
}

/// Read a process priority value.
///
/// Read the scheduler priority value for the target process identifier.
/// Priority ranges and classes are host-defined.
///
/// # Platform
/// Unix and Windows.
/// Uses getpriority(2) on Unix and GetPriorityClass plus thread priority mapping on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.priority`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_get_priority(
    _context: &BindingCallContext,
    out: *mut i32,
    pid: ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_INVALID_PARAMETER};
    use windows_sys::Win32::System::Threading::{
        GetPriorityClass, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    let pid = core_process::process_pid_to_windows_target(pid.0, "pid")?;
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
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
            format!("failed to open process {pid} for priority read"),
        ))
        .boxed());
    }

    let class = unsafe { GetPriorityClass(process) };
    let value = if class == 0 {
        let error = core_platform::last_error_code();
        unsafe {
            CloseHandle(process);
        }
        return Err(RuntimeError::from(PlatformError::io(format!(
            "failed to read process priority class: {error}",
        )))
        .boxed());
    } else {
        windows_priority_class_to_nice(class)?
    };

    unsafe {
        CloseHandle(process);
    }

    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read scheduler policy and priority for a process.
///
/// Read one process scheduler policy class and its priority details.
/// Returned policy availability and numeric ranges are host-defined.
///
/// # Platform
/// Unix and Windows.
/// Uses sched_getscheduler plus sched_getparam on Unix and process scheduling class mapping on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.scheduler`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_get_scheduler(
    context: &BindingCallContext,
    out: *mut ProcessSchedulerConfig,
    pid: ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, pid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.getScheduler",
    ))
    .boxed())
}

/// Set process CPU affinity.
///
/// Set the CPU affinity mask for the target process identifier.
/// Invalid CPU sets and privilege violations are rejected by the host scheduler.
///
/// # Platform
/// Unix and Windows.
/// Uses sched_setaffinity(2) on Unix and SetProcessAffinityMask on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.affinity`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_set_affinity(
    context: &BindingCallContext,
    pid: ProcessId,
    cpus: ProcessCpuSet,
) -> RuntimeResult<()> {
    let cpus = unsafe { cpus.cpus.as_slice()? };
    let _ = (context, pid, cpus);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.setAffinity",
    ))
    .boxed())
}

/// Set a process priority value.
///
/// Set the scheduler priority value for the target process identifier.
/// Privilege checks and clamping are enforced by the host scheduler.
///
/// # Platform
/// Unix and Windows.
/// Uses setpriority(2) on Unix and SetPriorityClass or SetThreadPriority on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.priority`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_set_priority(
    _context: &BindingCallContext,
    pid: ProcessId,
    priority: i32,
) -> RuntimeResult<()> {
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_INVALID_PARAMETER};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_SET_INFORMATION, SetPriorityClass,
    };

    let pid = core_process::process_pid_to_windows_target(pid.0, "pid")?;
    let process = unsafe { OpenProcess(PROCESS_SET_INFORMATION, 0, pid) };
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
            format!("failed to open process {pid} for priority update"),
        ))
        .boxed());
    }

    let priority_class = windows_nice_to_priority_class(priority);
    let result = unsafe { SetPriorityClass(process, priority_class) };
    let status = if result == 0 {
        let error = core_platform::last_error_code();
        Err(RuntimeError::from(PlatformError::io(format!(
            "failed to set process priority class: {error}",
        )))
        .boxed())
    } else {
        Ok(())
    };

    unsafe {
        CloseHandle(process);
    }

    status
}

/// Set scheduler policy and priority for a process.
///
/// Set one process scheduler policy class with explicit priority and flags.
/// Privilege checks and policy-specific clamping are enforced by the host scheduler.
///
/// # Platform
/// Unix and Windows.
/// Uses sched_setscheduler plus sched_setparam on Unix and process scheduling class mapping on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.scheduler`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_set_scheduler(
    context: &BindingCallContext,
    pid: ProcessId,
    config: ProcessSchedulerConfig,
) -> RuntimeResult<()> {
    let _ = (context, pid, config);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.setScheduler",
    ))
    .boxed())
}

/// Yield the current thread to the scheduler.
///
/// Yield one scheduler timeslice voluntarily from the current execution context.
/// Yield ordering and wakeup behavior follow host scheduler semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses sched_yield(2) on Unix and SwitchToThread or Sleep(0) on Windows.
///
/// # Errors
/// Returns processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.scheduler`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_yield_now(_context: &BindingCallContext) -> RuntimeResult<()> {
    use windows_sys::Win32::System::Threading::{Sleep, SwitchToThread};

    let switched = unsafe { SwitchToThread() };
    if switched == 0 {
        unsafe { Sleep(0) };
    }

    Ok(())
}

/// Map a Windows process priority class to a unix-style nice value.
fn windows_priority_class_to_nice(class: u32) -> RuntimeResult<i32> {
    use windows_sys::Win32::System::Threading::{
        ABOVE_NORMAL_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS,
        IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, REALTIME_PRIORITY_CLASS,
    };

    if class == REALTIME_PRIORITY_CLASS {
        return Ok(-20);
    }
    if class == HIGH_PRIORITY_CLASS {
        return Ok(-10);
    }
    if class == ABOVE_NORMAL_PRIORITY_CLASS {
        return Ok(-5);
    }
    if class == NORMAL_PRIORITY_CLASS {
        return Ok(0);
    }
    if class == BELOW_NORMAL_PRIORITY_CLASS {
        return Ok(10);
    }
    if class == IDLE_PRIORITY_CLASS {
        return Ok(19);
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "priorityClass",
        format!("unsupported priority class {class}"),
    ))
    .boxed())
}

/// Map a unix-style nice value to a Windows process priority class.
fn windows_nice_to_priority_class(priority: i32) -> u32 {
    use windows_sys::Win32::System::Threading::{
        ABOVE_NORMAL_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS,
        IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, REALTIME_PRIORITY_CLASS,
    };

    if priority <= -15 {
        return REALTIME_PRIORITY_CLASS;
    }
    if priority <= -8 {
        return HIGH_PRIORITY_CLASS;
    }
    if priority <= -3 {
        return ABOVE_NORMAL_PRIORITY_CLASS;
    }
    if priority <= 4 {
        return NORMAL_PRIORITY_CLASS;
    }
    if priority <= 10 {
        return BELOW_NORMAL_PRIORITY_CLASS;
    }

    IDLE_PRIORITY_CLASS
}
