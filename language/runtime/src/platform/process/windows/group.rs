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
/// Read one control-group resource limit.
///
/// Read one controller limit value from one control-group path.
/// Resource selector interpretation follows host controller semantics.
///
/// # Platform
/// Linux.
/// Uses cgroup controller files in v2 hierarchies.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.cgroup`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_process_cgroup_get_limit(
    context: &RuntimeCallContext,
    out: *mut ProcessLimit,
    path: NativeStringRef,
    resource: ProcessLimitResource,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_GROUP_CGROUP_GET_LIMIT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let path = unsafe { path.as_str()? };
    let value = core_process::process_cgroup_get_limit(path, resource.0)?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Join one control group.
///
/// Attach the current process to one control-group path.
/// Group hierarchy and controller behavior follow host kernel rules.
///
/// # Platform
/// Linux.
/// Uses cgroup v2 control files.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.cgroup`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_process_cgroup_join(
    context: &RuntimeCallContext,
    path: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_GROUP_CGROUP_JOIN)?;
    let path = unsafe { path.as_str()? };
    core_process::process_cgroup_join(path)
}

/// Write one control-group resource limit.
///
/// Write one controller limit value to one control-group path.
/// Controller validation and privilege checks are host-enforced.
///
/// # Platform
/// Linux.
/// Uses cgroup controller files in v2 hierarchies.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.cgroup`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_process_cgroup_set_limit(
    context: &RuntimeCallContext,
    path: NativeStringRef,
    resource: ProcessLimitResource,
    limit: ProcessLimit,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_GROUP_CGROUP_SET_LIMIT)?;
    let path = unsafe { path.as_str()? };
    core_process::process_cgroup_set_limit(path, resource.0, limit)
}

/// Assign processes to one Windows job object.
///
/// Attach one or more target processes to one named job object.
/// Job object lifetime and inheritance follow host process-manager semantics.
///
/// # Platform
/// Windows.
/// Uses job object assignment APIs.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.cgroup`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_process_job_assign(
    context: &RuntimeCallContext,
    name: NativeStringRef,
    pids: NativeSlice<ProcessId>,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_GROUP_JOB_ASSIGN)?;
    let name = unsafe { name.as_str()? };
    let pids = unsafe { pids.as_slice()? };
    core_process::process_job_assign(name, pids)
}

/// Set one Windows job object resource limit.
///
/// Write one resource limit entry to one named job object.
/// Resource selector interpretation follows host job object semantics.
///
/// # Platform
/// Windows.
/// Uses job object limit APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.cgroup`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_process_job_set_limit(
    context: &RuntimeCallContext,
    name: NativeStringRef,
    resource: ProcessLimitResource,
    limit: ProcessLimit,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_GROUP_JOB_SET_LIMIT)?;
    let name = unsafe { name.as_str()? };
    core_process::process_job_set_limit(name, resource.0, limit)
}
