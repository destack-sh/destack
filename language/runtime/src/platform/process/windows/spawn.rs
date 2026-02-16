#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::{bindings_generated as bindings, core as core_process};
use crate::platform::resource::{ResourceFinalizer, ResourceId};
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, NativeStringSlice, PlatformError,
};

use crate::runtime::RuntimeCallContext;
use bindings::*;
use std::os::windows::io::{FromRawHandle, IntoRawHandle, OwnedHandle};
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

/// Finalizer that closes one Windows process handle.
#[derive(Debug)]
struct ProcessHandleFinalizer {
    /// Raw process handle to close.
    handle: windows_sys::Win32::Foundation::HANDLE,
}

impl ProcessHandleFinalizer {
    /// Create a process handle finalizer from one raw handle.
    fn new(handle: windows_sys::Win32::Foundation::HANDLE) -> Self {
        Self { handle }
    }
}

impl ResourceFinalizer for ProcessHandleFinalizer {
    /// Close the process handle when the resource is finalized.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

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

/// Resolve a file handle into a Windows handle value.
fn resolve_file_handle(
    context: &RuntimeCallContext,
    handle: resource::FileHandle,
) -> RuntimeResult<windows_sys::Win32::Foundation::HANDLE> {
    core_fs::require_resource(
        context,
        handle.0,
        resource::ResourceKind::File,
        "file",
        |entry| {
            entry.handle().map(|handle| handle as _).ok_or_else(|| {
                RuntimeError::from(PlatformError::generic(
                    None,
                    "file handle missing raw handle",
                ))
                .boxed()
            })
        },
    )
}

/// Duplicate a raw Windows handle into owned stdio.
fn duplicate_handle_for_stdio(
    handle: windows_sys::Win32::Foundation::HANDLE,
    label: &str,
) -> RuntimeResult<std::process::Stdio> {
    use windows_sys::Win32::Foundation::{
        DUPLICATE_SAME_ACCESS, DuplicateHandle, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    // validate handle inputs
    if handle == 0 || handle == INVALID_HANDLE_VALUE {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "invalid handle",
        ))
        .boxed());
    }

    // duplicate the source handle into this process
    let process = unsafe { GetCurrentProcess() };
    let mut duplicated = 0;
    let rc = unsafe {
        DuplicateHandle(
            process,
            handle,
            process,
            &mut duplicated,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    };
    if rc == 0 {
        return Err(
            RuntimeError::from(PlatformError::io("failed to duplicate stdio handle")).boxed(),
        );
    }

    // transfer ownership into stdio
    let owned = unsafe { OwnedHandle::from_raw_handle(duplicated as _) };
    Ok(std::process::Stdio::from(owned))
}

/// Apply process spawn options to a command.
fn apply_spawn_options(command: &mut Command, options: ProcessSpawnOptions) -> RuntimeResult<()> {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, DETACHED_PROCESS};

    let cwd = core_fs::os_path_to_utf8_string(options.cwd, "options.cwd")?;
    if !cwd.is_empty() {
        command.current_dir(cwd);
    }

    let mut creation_flags = 0_u32;
    if options.detached {
        creation_flags |= DETACHED_PROCESS;
    }
    if options.new_process_group {
        creation_flags |= CREATE_NEW_PROCESS_GROUP;
    }
    if creation_flags != 0 {
        command.creation_flags(creation_flags);
    }

    // windows does not expose a direct equivalent for posix signal-disposition reset
    let _ = options.reset_signals;

    Ok(())
}

/// Apply explicit stdio wiring to a command.
fn apply_spawn_stdio(
    context: &RuntimeCallContext,
    command: &mut Command,
    stdio: &[ProcessStdio],
) -> RuntimeResult<()> {
    if stdio.len() > 3 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "stdio",
            "stdio entries must target stdin, stdout, and stderr only",
        ))
        .boxed());
    }

    for (index, descriptor) in stdio.iter().enumerate() {
        let target = match descriptor.kind {
            ProcessStdioKind::Inherit => std::process::Stdio::inherit(),
            ProcessStdioKind::Null => std::process::Stdio::null(),
            ProcessStdioKind::Pipe => std::process::Stdio::piped(),
            ProcessStdioKind::File => {
                let file_handle = resolve_file_handle(context, descriptor.file)?;
                duplicate_handle_for_stdio(file_handle, "stdio.file")?
            }
            ProcessStdioKind::Descriptor => {
                let raw = unsafe { libc::get_osfhandle(descriptor.descriptor) };
                if raw == -1 {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "stdio.descriptor",
                        "invalid descriptor",
                    ))
                    .boxed());
                }
                duplicate_handle_for_stdio(raw as _, "stdio.descriptor")?
            }
        };

        match index {
            0 => {
                command.stdin(target);
            }
            1 => {
                command.stdout(target);
            }
            2 => {
                command.stderr(target);
            }
            _ => {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "stdio",
                    "unsupported stdio entry index",
                ))
                .boxed());
            }
        }
    }

    Ok(())
}

/// Spawn a child process and register its handle payload.
fn spawn_process(
    context: &RuntimeCallContext,
    out: *mut resource::ProcessHandle,
    command: String,
    arguments: Vec<String>,
    environment: Vec<String>,
    options: ProcessSpawnOptions,
    stdio: &[ProcessStdio],
) -> RuntimeResult<()> {
    let mut child = Command::new(command);
    child.args(&arguments);
    apply_environment_pairs(&mut child, &environment)?;
    apply_spawn_options(&mut child, options)?;
    apply_spawn_stdio(context, &mut child, stdio)?;

    let child = child.spawn().map_err(|error| {
        RuntimeError::from(PlatformError::io(format!(
            "failed to spawn process: {error}",
        )))
        .boxed()
    })?;
    let process_id = ProcessId(child.id());
    let process_handle = child.into_raw_handle();

    let entry = resource::ResourceEntry::new(resource::ResourceKind::Process)
        .with_label("process.spawn")
        .with_payload(core_process::SpawnedProcess { pid: process_id })
        .with_handle(process_handle)
        .with_finalizer(ProcessHandleFinalizer::new(process_handle as _));
    let resource_id = context.runtime().resources.insert(entry);

    unsafe {
        *out = resource::ProcessHandle(resource_id);
    }

    Ok(())
}

/// Spawn a child process with default stdio inheritance.
///
/// Spawn one child process using the provided command, argv, envp, and spawn options.
/// Descriptor inheritance and process-group behavior follow host process-launch semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses posix_spawn or fork-plus-exec on Unix and CreateProcessW on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, processSpawnFailed, notSupported.
///
/// # Security
/// Requires `process.spawn`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_spawn(
    context: &RuntimeCallContext,
    out: *mut resource::ProcessHandle,
    command: fs::OsPath,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
    options: ProcessSpawnOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let command = core_fs::os_path_to_utf8_string(command, "command")?;
    let arguments = unsafe { decode_native_strings(arguments)? };
    let environment = unsafe { decode_native_strings(environment)? };

    spawn_process(context, out, command, arguments, environment, options, &[])
}

/// Spawn a child process with explicit stdio and descriptor actions.
///
/// Spawn one child process and apply explicit stdio wiring and descriptor action scripts.
/// File-action ordering and inheritance behavior follow host spawn primitives.
///
/// # Platform
/// Unix and Windows.
/// Uses posix_spawn_file_actions on Unix and handle-inheritance setup on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, processSpawnFailed, notSupported.
///
/// # Security
/// Requires `process.spawn`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_spawn_with_actions(
    context: &RuntimeCallContext,
    out: *mut resource::ProcessHandle,
    command: fs::OsPath,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
    options: ProcessSpawnOptions,
    stdio: NativeSlice<ProcessStdio>,
    actions: NativeSlice<ProcessFdAction>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let command = core_fs::os_path_to_utf8_string(command, "command")?;
    let arguments = unsafe { decode_native_strings(arguments)? };
    let environment = unsafe { decode_native_strings(environment)? };
    let stdio = unsafe { stdio.as_slice()? };
    let actions = unsafe { actions.as_slice()? };

    // windows has no direct equivalent for arbitrary fd action scripts
    if !actions.is_empty() {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.spawn.actions",
        ))
        .boxed());
    }

    spawn_process(
        context,
        out,
        command,
        arguments,
        environment,
        options,
        stdio,
    )
}
