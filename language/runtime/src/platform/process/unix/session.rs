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
    context: &RuntimeCallContext,
    out: *mut ProcessId,
    pid: ProcessId,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SESSION_GETPGID)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = ProcessId(core_process::process_getpgid(pid.0)?);
    unsafe {
        *out = value;
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
    context: &RuntimeCallContext,
    pid: ProcessId,
    pgid: ProcessId,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SESSION_SETPGID)?;
    core_process::process_setpgid(pid.0, pgid.0)
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
    context: &RuntimeCallContext,
    out: *mut ProcessId,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SESSION_SETSID)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = core_process::process_setsid()?;
    unsafe {
        *out = value;
    }

    Ok(())
}
