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

/// Resolve a process handle into its process id payload.
fn resolve_spawned_process_handle(
    context: &RuntimeCallContext,
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
    status.kind == ProcessWaitKind::Exited || status.kind == ProcessWaitKind::Signaled
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
    context: &RuntimeCallContext,
    out: *mut ProcessWaitStatus,
    pid: ProcessId,
    flags: ProcessWaitFlags,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_WAIT_PID)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = core_process::process_wait_pid(pid.0, flags.0)?;
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
    context: &RuntimeCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessHandle,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_WAIT_TRY_WAIT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let process_id = resolve_spawned_process_handle(context, handle)?;
    let status =
        core_process::process_wait_pid(process_id.0, core_process::PROCESS_WAIT_FLAG_NOHANG)?;

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
    context: &RuntimeCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessHandle,
    flags: ProcessWaitFlags,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_WAIT_WAIT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let process_id = resolve_spawned_process_handle(context, handle)?;
    let status = core_process::process_wait_pid(process_id.0, flags.0)?;

    if is_terminal_wait_status(&status) {
        let _ = context.runtime().resources.remove_and_finalize(handle.0);
    }

    unsafe {
        *out = status;
    }

    Ok(())
}
