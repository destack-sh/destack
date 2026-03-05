#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::{bindings_generated as bindings, core as core_process};
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
    binding: &BindingCallContext,
    out: *mut ProcessLimit,
    resource: ProcessLimitResource,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, resource);
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
    binding: &BindingCallContext,
    resource: ProcessLimitResource,
    limit: ProcessLimit,
) -> RuntimeResult<()> {
    let _ = (binding, resource, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.limits.setLimit",
    ))
    .boxed())
}
