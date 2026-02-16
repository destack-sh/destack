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
/// Read a process resource limit.
///
/// Read soft and hard limits for one host resource selector.
/// Resource selector interpretation follows host kernel limit tables.
///
/// # Platform
/// Unix and Windows.
/// Uses getrlimit or prlimit on Unix and job-object limit queries on Windows where available.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.run`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_get_limit(
    context: &RuntimeCallContext,
    out: *mut ProcessLimit,
    resource: ProcessLimitResource,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, resource);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.limits.getLimit",
    ))
    .boxed())
}

/// Set a process resource limit.
///
/// Update soft and hard limits for one host resource selector.
/// Privilege checks and hard-limit rules are enforced by the host kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses setrlimit or prlimit on Unix and job-object limit updates on Windows where available.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.run`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_set_limit(
    context: &RuntimeCallContext,
    resource: ProcessLimitResource,
    limit: ProcessLimit,
) -> RuntimeResult<()> {
    let _ = (context, resource, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.limits.setLimit",
    ))
    .boxed())
}
