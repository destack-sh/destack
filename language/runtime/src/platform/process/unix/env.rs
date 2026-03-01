#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::{bindings_generated as bindings, core as core_process};
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, NativeStringSlice, PlatformError,
};

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
use std::ffi::CStr;

/// Delete an environment variable by UTF-8 name.
///
/// Remove one key from the process environment block.
/// Missing keys are handled according to host environment semantics.
///
/// # Platform
/// Unix and Windows.
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
    _context: &BindingCallContext,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let name = unsafe { name.as_str()? };

    let name =
        core_process::cstring_from_str(name, "name", "environment variable contains nul byte")?;

    let rc = unsafe { libc::unsetenv(name.as_ptr()) };
    if rc != 0 {
        return Err(RuntimeError::from(PlatformError::io("failed to delete environment")).boxed());
    }

    Ok(())
}

/// Delete an environment variable by raw byte name.
///
/// Remove one key from the environment block without UTF-8 normalization.
/// This is intended for byte-level Unix-style environment access.
///
/// # Platform
/// Unix and Windows.
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
    _context: &BindingCallContext,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let name = unsafe { name.as_slice()? };

    let name =
        core_process::cstring_from_bytes(name, "name", "environment variable contains nul byte")?;

    let rc = unsafe { libc::unsetenv(name.as_ptr()) };
    if rc != 0 {
        return Err(RuntimeError::from(PlatformError::io("failed to delete environment")).boxed());
    }

    Ok(())
}

/// Read an environment variable by UTF-8 name.
///
/// Resolve one key from the process environment block and decode it as a runtime string.
/// Missing keys and invalid entries are surfaced as platform errors.
///
/// # Platform
/// Unix and Windows.
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
    context: &BindingCallContext,
    out: *mut NativeStringRef,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let name_value = unsafe { name.as_str()? };

    let name = core_process::cstring_from_str(
        name_value,
        "name",
        "environment variable contains nul byte",
    )?;
    let value = unsafe { libc::getenv(name.as_ptr()) };
    let value = if value.is_null() {
        None
    } else {
        let value = unsafe { CStr::from_ptr(value) };
        Some(String::from_utf8_lossy(value.to_bytes()).to_string())
    };
    let value = value.ok_or_else(|| core_process::missing_env_error(name_value.to_string()))?;

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
/// Unix and Windows.
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
    context: &BindingCallContext,
    out: *mut NativeArray<u8>,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let name_bytes = unsafe { name.as_slice()? };

    let name = core_process::cstring_from_bytes(
        name_bytes,
        "name",
        "environment variable contains nul byte",
    )?;
    let value = unsafe { libc::getenv(name.as_ptr()) };
    let value = if value.is_null() {
        None
    } else {
        let value = unsafe { CStr::from_ptr(value) };
        Some(value.to_bytes().to_vec())
    };
    let value = value.ok_or_else(|| {
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
/// Unix and Windows.
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
    _context: &BindingCallContext,
    name: NativeStringRef,
    argument_value: NativeStringRef,
) -> RuntimeResult<()> {
    let name = unsafe { name.as_str()? };
    let value = unsafe { argument_value.as_str()? };

    let name =
        core_process::cstring_from_str(name, "name", "environment variable contains nul byte")?;
    let value =
        core_process::cstring_from_str(value, "value", "environment variable contains nul byte")?;

    let rc = unsafe { libc::setenv(name.as_ptr(), value.as_ptr(), 1) };
    if rc != 0 {
        return Err(RuntimeError::from(PlatformError::io("failed to set environment")).boxed());
    }

    Ok(())
}

/// Set an environment variable by raw byte name and value.
///
/// Insert or replace one key-value pair in the environment block without UTF-8 normalization.
/// This is intended for byte-level Unix-style environment access.
///
/// # Platform
/// Unix and Windows.
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
    _context: &BindingCallContext,
    name: NativeSlice<u8>,
    argument_value: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let name = unsafe { name.as_slice()? };
    let value = unsafe { argument_value.as_slice()? };
    let name =
        core_process::cstring_from_bytes(name, "name", "environment variable contains nul byte")?;
    let value =
        core_process::cstring_from_bytes(value, "value", "environment variable contains nul byte")?;

    let rc = unsafe { libc::setenv(name.as_ptr(), value.as_ptr(), 1) };
    if rc != 0 {
        return Err(RuntimeError::from(PlatformError::io("failed to set environment")).boxed());
    }

    Ok(())
}
