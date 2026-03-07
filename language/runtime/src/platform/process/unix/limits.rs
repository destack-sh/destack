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
    _binding: &BindingCallContext,
    out: *mut ProcessLimit,
    resource: ProcessLimitResource,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let mut raw_limit = unsafe { std::mem::zeroed::<libc::rlimit>() };
    let result = unsafe { libc::getrlimit(resource.0 as _, &mut raw_limit) };
    if result != 0 {
        return Err(core_process::process_last_error(
            "getrlimit",
            format!("failed to get limit for resource {}", resource.0),
        ));
    }

    let value = ProcessLimit {
        soft: raw_limit.rlim_cur,
        hard: raw_limit.rlim_max,
    };
    unsafe {
        *out = value;
    }

    Ok(())
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
    _binding: &BindingCallContext,
    resource: ProcessLimitResource,
    limit: ProcessLimit,
) -> RuntimeResult<()> {
    if limit.soft > limit.hard {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "limit.soft",
            "soft limit must be less than or equal to hard limit",
        ))
        .boxed());
    }

    let raw_limit = libc::rlimit {
        rlim_cur: limit.soft as libc::rlim_t,
        rlim_max: limit.hard as libc::rlim_t,
    };

    let result = unsafe { libc::setrlimit(resource.0 as _, &raw_limit) };
    if result != 0 {
        return Err(core_process::process_last_error(
            "setrlimit",
            format!("failed to set limit for resource {}", resource.0),
        ));
    }

    Ok(())
}
