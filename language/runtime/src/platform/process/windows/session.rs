#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::bindings_generated as bindings;
use crate::platform::{NativeArray, PlatformError};
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
    _pid: ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.session.getpgid",
    ))
    .boxed())
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
    let _ = (pid, pgid);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.session.setpgid",
    ))
    .boxed())
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

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.session.setsid",
    ))
    .boxed())
}
