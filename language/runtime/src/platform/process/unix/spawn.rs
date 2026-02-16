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
use std::ffi::CString;
use std::os::fd::{FromRawFd, OwnedFd};
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

/// File-action payload resolved for pre-exec application.
#[derive(Debug)]
enum ResolvedFdAction {
    /// Close one descriptor in the child.
    Close {
        /// Descriptor to close.
        descriptor: i32,
    },
    /// Duplicate one descriptor onto another.
    Dup2 {
        /// Source descriptor.
        source: i32,
        /// Target descriptor.
        target: i32,
    },
    /// Open one path and bind it to a target descriptor.
    Open {
        /// Target descriptor.
        target: i32,
        /// Path to open.
        path: CString,
        /// Open flags.
        flags: i32,
        /// File mode bits.
        mode: libc::mode_t,
    },
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

/// Resolve a file handle into a unix descriptor.
fn resolve_file_fd(
    context: &RuntimeCallContext,
    handle: resource::FileHandle,
) -> RuntimeResult<i32> {
    core_fs::require_resource(
        context,
        handle.0,
        crate::platform::resource::ResourceKind::File,
        "file",
        |entry| {
            entry.fd().ok_or_else(|| {
                RuntimeError::from(PlatformError::generic(
                    None,
                    "file handle missing descriptor",
                ))
                .boxed()
            })
        },
    )
}

/// Duplicate one descriptor into owned stdio for command spawning.
fn duplicate_descriptor_for_stdio(
    descriptor: i32,
    label: &str,
) -> RuntimeResult<std::process::Stdio> {
    if descriptor < 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "descriptor must be non-negative",
        ))
        .boxed());
    }

    let duplicate = unsafe { libc::dup(descriptor) };
    if duplicate < 0 {
        let error = std::io::Error::last_os_error();
        return Err(RuntimeError::from(PlatformError::io(format!(
            "failed to duplicate descriptor: {error}"
        )))
        .boxed());
    }

    let owned = unsafe { OwnedFd::from_raw_fd(duplicate) };
    Ok(std::process::Stdio::from(owned))
}

/// Apply process spawn options to a command.
fn apply_spawn_options(command: &mut Command, options: ProcessSpawnOptions) -> RuntimeResult<()> {
    let cwd = core_fs::os_path_to_utf8_string(options.cwd, "options.cwd")?;
    if !cwd.is_empty() {
        command.current_dir(cwd);
    }

    if options.detached || options.new_process_group || options.reset_signals {
        use std::os::unix::process::CommandExt;

        // configure process group state in the child before exec
        unsafe {
            command.pre_exec(move || {
                if options.reset_signals {
                    for signal in 1..=64 {
                        if signal == libc::SIGKILL || signal == libc::SIGSTOP {
                            continue;
                        }

                        let result = libc::signal(signal as libc::c_int, libc::SIG_DFL);
                        if result == libc::SIG_ERR {
                            let error = std::io::Error::last_os_error();
                            if error.raw_os_error() != Some(libc::EINVAL) {
                                return Err(error);
                            }
                        }
                    }
                }

                if options.detached {
                    let rc = libc::setsid();
                    if rc < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                } else if options.new_process_group {
                    let rc = libc::setpgid(0, 0);
                    if rc != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                }

                Ok(())
            });
        }
    }

    Ok(())
}

/// Decode and validate fd actions for pre-exec setup.
fn resolve_fd_actions(actions: &[ProcessFdAction]) -> RuntimeResult<Vec<ResolvedFdAction>> {
    let mut resolved_actions = Vec::with_capacity(actions.len());
    for action in actions {
        match action.op {
            ProcessFdActionKind::Close => {
                if action.source < 0 {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "actions.source",
                        "descriptor must be non-negative",
                    ))
                    .boxed());
                }

                resolved_actions.push(ResolvedFdAction::Close {
                    descriptor: action.source,
                });
            }
            ProcessFdActionKind::Dup2 => {
                if action.source < 0 || action.target < 0 {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "actions",
                        "dup2 action descriptors must be non-negative",
                    ))
                    .boxed());
                }

                resolved_actions.push(ResolvedFdAction::Dup2 {
                    source: action.source,
                    target: action.target,
                });
            }
            ProcessFdActionKind::Open => {
                if action.target < 0 {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "actions.target",
                        "descriptor must be non-negative",
                    ))
                    .boxed());
                }

                let path = core_fs::os_path_to_utf8_string(action.path, "actions.path")?;
                let path = CString::new(path).map_err(|_| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "actions.path",
                        "path contains nul byte",
                    ))
                    .boxed()
                })?;

                let flags = i32::try_from(action.flags.0).map_err(|_| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "actions.flags",
                        "flags are out of range",
                    ))
                    .boxed()
                })?;
                let mode = action.mode.0 as libc::mode_t;

                resolved_actions.push(ResolvedFdAction::Open {
                    target: action.target,
                    path,
                    flags,
                    mode,
                });
            }
        }
    }

    Ok(resolved_actions)
}

/// Apply fd actions in the child pre-exec phase.
fn apply_spawn_fd_actions(
    command: &mut Command,
    actions: Vec<ResolvedFdAction>,
) -> RuntimeResult<()> {
    if actions.is_empty() {
        return Ok(());
    }

    use std::os::unix::process::CommandExt;

    // apply file actions in-order in the child before exec
    unsafe {
        command.pre_exec(move || {
            for action in &actions {
                match action {
                    ResolvedFdAction::Close { descriptor } => {
                        let rc = libc::close(*descriptor);
                        if rc < 0 {
                            let error = std::io::Error::last_os_error();
                            if error.raw_os_error() != Some(libc::EBADF) {
                                return Err(error);
                            }
                        }
                    }
                    ResolvedFdAction::Dup2 { source, target } => {
                        if source == target {
                            continue;
                        }

                        let rc = libc::dup2(*source, *target);
                        if rc < 0 {
                            return Err(std::io::Error::last_os_error());
                        }
                    }
                    ResolvedFdAction::Open {
                        target,
                        path,
                        flags,
                        mode,
                    } => {
                        let opened = libc::open(path.as_ptr(), *flags, *mode as libc::c_uint);
                        if opened < 0 {
                            return Err(std::io::Error::last_os_error());
                        }

                        if opened != *target {
                            let rc = libc::dup2(opened, *target);
                            if rc < 0 {
                                let _ = libc::close(opened);
                                return Err(std::io::Error::last_os_error());
                            }

                            let _ = libc::close(opened);
                        }
                    }
                }
            }

            Ok(())
        });
    }

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
                let file_descriptor = resolve_file_fd(context, descriptor.file)?;
                duplicate_descriptor_for_stdio(file_descriptor, "stdio.file")?
            }
            ProcessStdioKind::Descriptor => {
                duplicate_descriptor_for_stdio(descriptor.descriptor, "stdio.descriptor")?
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
    actions: &[ProcessFdAction],
) -> RuntimeResult<()> {
    let mut child = Command::new(command);
    child.args(&arguments);
    apply_environment_pairs(&mut child, &environment)?;
    apply_spawn_options(&mut child, options)?;
    apply_spawn_stdio(context, &mut child, stdio)?;
    let actions = resolve_fd_actions(actions)?;
    apply_spawn_fd_actions(&mut child, actions)?;

    let child = child.spawn().map_err(|error| {
        RuntimeError::from(PlatformError::io(format!(
            "failed to spawn process: {error}"
        )))
        .boxed()
    })?;
    let process_id = ProcessId(child.id());
    std::mem::drop(child);

    let entry = crate::platform::resource::ResourceEntry::new(
        crate::platform::resource::ResourceKind::Process,
    )
    .with_label("process.spawn")
    .with_payload(core_process::SpawnedProcess { pid: process_id });
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
    context.check_policy(PROCESS_SPAWN_SPAWN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let command = core_fs::os_path_to_utf8_string(command, "command")?;
    let arguments = unsafe { decode_native_strings(arguments)? };
    let environment = unsafe { decode_native_strings(environment)? };
    spawn_process(
        context,
        out,
        command,
        arguments,
        environment,
        options,
        &[],
        &[],
    )
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
    context.check_policy(PROCESS_SPAWN_WITH_ACTIONS)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let command = core_fs::os_path_to_utf8_string(command, "command")?;
    let arguments = unsafe { decode_native_strings(arguments)? };
    let environment = unsafe { decode_native_strings(environment)? };
    let stdio = unsafe { stdio.as_slice()? };
    let actions = unsafe { actions.as_slice()? };
    spawn_process(
        context,
        out,
        command,
        arguments,
        environment,
        options,
        stdio,
        actions,
    )
}
