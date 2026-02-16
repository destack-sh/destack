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

use crate::platform::fs::core as core_fs;
use crate::platform::process::{
    ExecAtFlags, GroupId, ProcessCpuSet, ProcessFdAction, ProcessFdActionKind, ProcessFdFlags,
    ProcessFdSignalFlags, ProcessGroupIds, ProcessId, ProcessLimit, ProcessLimitResource,
    ProcessNamespaceKind, ProcessSchedulerConfig, ProcessSchedulerPolicy, ProcessSpawnOptions,
    ProcessStdio, ProcessStdioKind, ProcessUnshareFlags, ProcessUserIds, ProcessWaitFlags,
    ProcessWaitKind, ProcessWaitStatus, Signal, SignalEvent, SignalFdFlags, SignalMaskHow,
    SyscallFilterFlags, UserId,
};
use crate::platform::{fs, resource};

/// Decode a native string slice into owned UTF-8 strings.
unsafe fn decode_native_strings(slice: NativeStringSlice) -> RuntimeResult<Vec<String>> {
    let values = unsafe { slice.as_slice()? };
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        let value = unsafe { value.as_str()? };
        decoded.push(value.to_string());
    }

    Ok(decoded)
}

/// Resolve a directory handle into a unix descriptor.
fn resolve_directory_fd(
    context: &RuntimeCallContext,
    handle: resource::DirectoryHandle,
) -> RuntimeResult<i32> {
    let resolved = context.runtime().resources.with_entry(handle.0, |entry| {
        if entry.kind != crate::platform::resource::ResourceKind::Directory {
            return None;
        }
        entry.fd()
    });

    resolved.flatten().ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "directory",
            "unknown directory handle",
        ))
        .boxed()
    })
}

/// Resolve a file handle into a unix descriptor.
fn resolve_file_fd(
    context: &RuntimeCallContext,
    handle: resource::FileHandle,
) -> RuntimeResult<i32> {
    let resolved = context.runtime().resources.with_entry(handle.0, |entry| {
        if entry.kind != crate::platform::resource::ResourceKind::File {
            return None;
        }
        entry.fd()
    });

    resolved.flatten().ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "executable",
            "unknown file handle",
        ))
        .boxed()
    })
}
/// Replace the current process image with a command path.
///
/// Replace the current process image in-place by executing the given command path.
/// This call does not return on success and preserves host exec semantics for inherited descriptors.
///
/// # Platform
/// Unix and Windows.
/// Uses execve(2) on Unix and process-replacement emulation with CreateProcessW plus exit on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.exec`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_exec(
    context: &RuntimeCallContext,
    command: fs::OsPath,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_EXEC_EXEC)?;
    let command = core_fs::os_path_to_utf8_string(command, "command")?;
    let arguments = unsafe { decode_native_strings(arguments)? };
    let environment = unsafe { decode_native_strings(environment)? };
    core_process::process_exec_path(&command, &arguments, &environment)
}

/// Replace the current process image using a directory-relative path.
///
/// Replace the current process image in-place by executing a directory-relative target.
/// Flag behavior follows host exec-at semantics and may reject unsupported combinations.
///
/// # Platform
/// Unix and Windows.
/// Uses execveat(2) on Unix where available and runtime fallback or `notSupported` on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.exec`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_execat(
    context: &RuntimeCallContext,
    directory: resource::DirectoryHandle,
    path: fs::OsPath,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
    flags: ExecAtFlags,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_EXEC_EXECAT)?;
    let directory_fd = resolve_directory_fd(context, directory)?;
    let path = core_fs::os_path_to_utf8_string(path, "path")?;
    let arguments = unsafe { decode_native_strings(arguments)? };
    let environment = unsafe { decode_native_strings(environment)? };
    core_process::process_exec_at(directory_fd, &path, &arguments, &environment, flags.0)
}

/// Replace the current process image using an executable file handle.
///
/// Replace the current process image in-place from an already-open executable descriptor.
/// Descriptor validity, executable format, and permission checks are enforced by the host kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses fexecve(2) on Unix where available and runtime fallback or `notSupported` on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.exec`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_fexec(
    context: &RuntimeCallContext,
    executable: resource::FileHandle,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_EXEC_FEXEC)?;
    let executable_fd = resolve_file_fd(context, executable)?;
    let arguments = unsafe { decode_native_strings(arguments)? };
    let environment = unsafe { decode_native_strings(environment)? };
    core_process::process_fexec(executable_fd, &arguments, &environment)
}
