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
/// Delete an environment variable by UTF-8 name.
///
/// Remove one key from the process environment block.
/// Missing keys are handled according to host environment semantics.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses unsetenv(3) on Unix and SetEnvironmentVariableW with null value on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `env.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_env_delete(
    context: &RuntimeCallContext,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ENV_DELETE)?;
    let name = unsafe { name.as_str()? };
    core_process::process_env_delete(name)
}

/// Delete an environment variable by raw byte name.
///
/// Remove one key from the environment block without UTF-8 normalization.
/// This is intended for byte-level Unix-style environment access.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses unsetenv(3)-style byte keys on Unix and runtime transcoding on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `env.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_env_delete_bytes(
    context: &RuntimeCallContext,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ENV_DELETE_BYTES)?;
    let name = unsafe { name.as_slice()? };
    core_process::process_env_delete_bytes(name)
}

/// Read an environment variable by UTF-8 name.
///
/// Resolve one key from the process environment block and decode it as a runtime string.
/// Missing keys and invalid entries are surfaced as platform errors.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses getenv(3) on Unix and GetEnvironmentVariableW on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `env.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_env_get(
    context: &RuntimeCallContext,
    out: *mut NativeStringRef,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ENV_GET)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let name_value = unsafe { name.as_str()? };
    let value = core_process::process_env_get(name_value)?
        .ok_or_else(|| core_process::missing_env_error(name_value.to_string()))?;

    unsafe {
        *out = context.store_string(&value);
    }

    Ok(())
}

/// Read an environment variable by raw byte name.
///
/// Resolve one key from the process environment block without UTF-8 normalization.
/// This is intended for byte-level Unix-style environment access.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses getenv(3)-style byte keys on Unix and runtime transcoding on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `env.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_env_get_bytes(
    context: &RuntimeCallContext,
    out: *mut NativeArray<u8>,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ENV_GET_BYTES)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let name_bytes = unsafe { name.as_slice()? };
    let value = core_process::process_env_get_bytes(name_bytes)?.ok_or_else(|| {
        core_process::missing_env_error(String::from_utf8_lossy(name_bytes).to_string())
    })?;

    unsafe {
        *out = context.store_array(value);
    }

    Ok(())
}

/// Set an environment variable by UTF-8 name and value.
///
/// Insert or replace one key-value pair in the process environment block.
/// Persistence and inheritance semantics follow host process-spawn rules.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses setenv(3) on Unix and SetEnvironmentVariableW on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `env.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_env_set(
    context: &RuntimeCallContext,
    name: NativeStringRef,
    argument_value: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ENV_SET)?;
    let name = unsafe { name.as_str()? };
    let value = unsafe { argument_value.as_str()? };
    core_process::process_env_set(name, value)
}

/// Set an environment variable by raw byte name and value.
///
/// Insert or replace one key-value pair in the environment block without UTF-8 normalization.
/// This is intended for byte-level Unix-style environment access.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses setenv(3)-style byte keys on Unix and runtime transcoding on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `env.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_env_set_bytes(
    context: &RuntimeCallContext,
    name: NativeSlice<u8>,
    argument_value: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ENV_SET_BYTES)?;
    let name = unsafe { name.as_slice()? };
    let value = unsafe { argument_value.as_slice()? };
    core_process::process_env_set_bytes(name, value)
}
