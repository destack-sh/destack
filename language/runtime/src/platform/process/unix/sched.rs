#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::{bindings_generated as bindings, core as core_process};
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, NativeStringSlice, PlatformError,
};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::process::{
    ExecAtFlags, GroupId, ProcessCpuSet, ProcessFdAction, ProcessFdActionKind, ProcessFdFlags,
    ProcessFdSignalFlags, ProcessGroupIds, ProcessId, ProcessLimit, ProcessLimitResource,
    ProcessNamespaceKind, ProcessSchedulerConfig, ProcessSchedulerPolicy, ProcessSpawnOptions,
    ProcessStdio, ProcessStdioKind, ProcessUnshareFlags, ProcessUserIds, ProcessWaitFlags,
    ProcessWaitKind, ProcessWaitStatus, Signal, SignalEvent, SignalFdFlags, SignalMaskHow,
    SyscallFilterFlags, UserId,
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
    context: &RuntimeCallContext,
    out: *mut ProcessCpuSet,
    pid: ProcessId,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SCHED_GET_AFFINITY)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let cpus = core_process::process_get_affinity(pid.0)?;
    let value = ProcessCpuSet {
        cpus: context.store_array(cpus),
    };
    unsafe {
        *out = value;
    }

    Ok(())
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
    context: &RuntimeCallContext,
    out: *mut i32,
    pid: ProcessId,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SCHED_GET_PRIORITY)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = core_process::process_get_priority(pid.0)?;
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
    context: &RuntimeCallContext,
    out: *mut ProcessSchedulerConfig,
    pid: ProcessId,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SCHED_GET_SCHEDULER)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = core_process::process_get_scheduler(pid.0)?;
    unsafe {
        *out = value;
    }

    Ok(())
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
    context: &RuntimeCallContext,
    pid: ProcessId,
    cpus: ProcessCpuSet,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SCHED_SET_AFFINITY)?;
    let cpus = unsafe { cpus.cpus.as_slice()? };
    core_process::process_set_affinity(pid.0, cpus)
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
    context: &RuntimeCallContext,
    pid: ProcessId,
    priority: i32,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SCHED_SET_PRIORITY)?;
    core_process::process_set_priority(pid.0, priority)
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
    context: &RuntimeCallContext,
    pid: ProcessId,
    config: ProcessSchedulerConfig,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SCHED_SET_SCHEDULER)?;
    core_process::process_set_scheduler(pid.0, config)
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
pub(crate) unsafe fn destack_process_yield_now(context: &RuntimeCallContext) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SCHED_YIELD_NOW)?;
    core_process::process_yield_now()
}
