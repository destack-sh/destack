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
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use std::process::Command;

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
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Storage::FileSystem::GetFinalPathNameByHandleW;

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

/// Apply environment entries to a command from `KEY=VALUE` pairs.
fn apply_environment_pairs(command: &mut Command, entries: &[String]) -> RuntimeResult<()> {
    command.env_clear();

    for entry in entries {
        let Some((name, value)) = entry.split_once('=') else {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "environment",
                format!("invalid environment entry: {entry}"),
            ))
            .boxed());
        };

        command.env(name, value);
    }

    Ok(())
}

/// Normalize a final handle path into a launchable Windows path.
fn normalize_handle_path(path: String) -> String {
    if let Some(stripped) = path.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{stripped}");
    }
    if let Some(stripped) = path.strip_prefix(r"\\?\") {
        return stripped.to_string();
    }

    path
}

/// Resolve a raw handle path from one resource entry.
fn path_from_handle(
    context: &RuntimeCallContext,
    handle_id: resource::ResourceId,
    kind: crate::platform::resource::ResourceKind,
    label: &str,
) -> RuntimeResult<String> {
    let handle = core_fs::require_resource(context, handle_id, kind, label, |entry| {
        entry
            .handle()
            .map(|handle| handle as HANDLE)
            .ok_or_else(|| {
                RuntimeError::from(PlatformError::generic(
                    None,
                    format!("{label} missing raw handle"),
                ))
                .boxed()
            })
    })?;

    let mut buffer = vec![0u16; 512];
    loop {
        let length = unsafe {
            GetFinalPathNameByHandleW(handle, buffer.as_mut_ptr(), buffer.len() as u32, 0)
        };
        if length == 0 {
            return Err(RuntimeError::from(PlatformError::io(
                "failed to resolve final path from handle",
            ))
            .boxed());
        }
        if (length as usize) < buffer.len() {
            buffer.truncate(length as usize);
            let path = std::ffi::OsString::from_wide(&buffer);
            let path = path.to_string_lossy().to_string();
            return Ok(normalize_handle_path(path));
        }
        buffer.resize(length as usize + 1, 0);
    }
}

/// Replace the process image by spawning one command and exiting with its status.
fn exec_replace_with_path(
    command: String,
    arguments: Vec<String>,
    environment: Vec<String>,
) -> RuntimeResult<()> {
    let mut child = Command::new(command);
    child.args(&arguments);
    apply_environment_pairs(&mut child, &environment)?;

    let mut child = child.spawn().map_err(|error| {
        RuntimeError::from(PlatformError::io(format!(
            "failed to spawn replacement process: {error}",
        )))
        .boxed()
    })?;
    let status = child.wait().map_err(|error| {
        RuntimeError::from(PlatformError::io(format!(
            "failed to wait replacement process: {error}",
        )))
        .boxed()
    })?;
    let exit_code = status.code().unwrap_or(1);

    std::process::exit(exit_code)
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
    exec_replace_with_path(command, arguments, environment)
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
    if flags.0 != 0 {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.exec.execat.flags",
        ))
        .boxed());
    }

    let directory_path = path_from_handle(
        context,
        directory.0,
        crate::platform::resource::ResourceKind::Directory,
        "directory",
    )?;
    let path = core_fs::os_path_to_utf8_string(path, "path")?;
    let command = if Path::new(&path).is_absolute() {
        path
    } else {
        let resolved = Path::new(&directory_path).join(path);
        resolved.to_string_lossy().to_string()
    };

    let arguments = unsafe { decode_native_strings(arguments)? };
    let environment = unsafe { decode_native_strings(environment)? };
    exec_replace_with_path(command, arguments, environment)
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
    let command = path_from_handle(
        context,
        executable.0,
        crate::platform::resource::ResourceKind::File,
        "executable",
    )?;
    let arguments = unsafe { decode_native_strings(arguments)? };
    let environment = unsafe { decode_native_strings(environment)? };
    exec_replace_with_path(command, arguments, environment)
}
