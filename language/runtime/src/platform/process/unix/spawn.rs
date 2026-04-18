#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::{NativeSlice, NativeStringSlice};
use crate::platform::process::core as core_process;

use crate::runtime::BindingCallContext;
use std::ffi::{CStr, CString};

use crate::platform::fs::core as core_fs;
use crate::platform::process::{ProcessFdAction, ProcessId, ProcessSpawnOptions, ProcessStdio};
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

/// Resolved stdio slot payload used by the child process setup.
#[derive(Debug, Clone, Copy)]
enum ResolvedStdioDescriptor {
    /// Keep the parent descriptor unchanged.
    Inherit,
    /// Bind the descriptor to `/dev/null`.
    Null,
    /// Bind the descriptor to one explicit fd.
    Descriptor(i32),
}

/// Resolve a file handle into a unix descriptor.
fn resolve_file_fd(
    binding: &BindingCallContext,
    handle: resource::FileHandle,
) -> RuntimeResult<i32> {
    core_fs::require_resource(
        binding,
        handle.0,
        resource::ResourceKind::File,
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

/// Resolve a pipe handle into a unix descriptor.
fn resolve_pipe_fd(
    binding: &BindingCallContext,
    handle: resource::PipeHandle,
) -> RuntimeResult<i32> {
    core_fs::require_resource(
        binding,
        handle.0,
        resource::ResourceKind::Pipe,
        "pipe",
        |entry| {
            entry.fd().ok_or_else(|| {
                RuntimeError::from(PlatformError::generic(
                    None,
                    "pipe handle missing descriptor",
                ))
                .boxed()
            })
        },
    )
}

/// Read one errno value from the current thread errno slot.
fn last_errno() -> i32 {
    std::io::Error::last_os_error()
        .raw_os_error()
        .unwrap_or(libc::EINVAL)
}

/// Resolve spawn current-directory option into a C string payload.
fn resolve_spawn_cwd(options: ProcessSpawnOptions) -> RuntimeResult<Option<CString>> {
    let cwd = core_fs::os_path_to_utf8_string(options.cwd, "options.cwd")?;
    if cwd.is_empty() {
        return Ok(None);
    }

    let cwd = core_process::cstring_from_str(&cwd, "options.cwd", "cwd contains nul byte")?;
    Ok(Some(cwd))
}

/// Decode and validate fd actions for pre-exec setup.
fn resolve_fd_actions(actions: &[ProcessFdAction]) -> RuntimeResult<Vec<ResolvedFdAction>> {
    let mut resolved_actions = Vec::with_capacity(actions.len());
    for action in actions {
        match action {
            ProcessFdAction::ProcessFdActionClose(action_close) => {
                if action_close.descriptor < 0 {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "actions.descriptor",
                        "descriptor must be non-negative",
                    ))
                    .boxed());
                }

                resolved_actions.push(ResolvedFdAction::Close {
                    descriptor: action_close.descriptor,
                });
            }
            ProcessFdAction::ProcessFdActionDup2(action_dup2) => {
                if action_dup2.source < 0 || action_dup2.target < 0 {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "actions",
                        "dup2 action descriptors must be non-negative",
                    ))
                    .boxed());
                }

                resolved_actions.push(ResolvedFdAction::Dup2 {
                    source: action_dup2.source,
                    target: action_dup2.target,
                });
            }
            ProcessFdAction::ProcessFdActionOpen(action_open) => {
                if action_open.target < 0 {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "actions.target",
                        "descriptor must be non-negative",
                    ))
                    .boxed());
                }

                let path = core_fs::os_path_to_utf8_string(action_open.path, "actions.path")?;
                let path = CString::new(path).map_err(|_| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "actions.path",
                        "path contains nul byte",
                    ))
                    .boxed()
                })?;

                let flags = i32::try_from(action_open.flags.0).map_err(|_| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "actions.flags",
                        "flags are out of range",
                    ))
                    .boxed()
                })?;
                let mode = action_open.mode.0 as libc::mode_t;

                resolved_actions.push(ResolvedFdAction::Open {
                    target: action_open.target,
                    path,
                    flags,
                    mode,
                });
            }
        }
    }

    Ok(resolved_actions)
}

/// Resolve explicit stdio descriptors into child setup payloads.
fn resolve_spawn_stdio(
    binding: &BindingCallContext,
    stdio: &[ProcessStdio],
) -> RuntimeResult<[ResolvedStdioDescriptor; 3]> {
    if stdio.len() > 3 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "stdio",
            "stdio entries must target stdin, stdout, and stderr only",
        ))
        .boxed());
    }

    let mut resolved = [
        ResolvedStdioDescriptor::Inherit,
        ResolvedStdioDescriptor::Inherit,
        ResolvedStdioDescriptor::Inherit,
    ];
    for (index, descriptor) in stdio.iter().enumerate() {
        let value = match descriptor {
            ProcessStdio::ProcessStdioInherit(_) => ResolvedStdioDescriptor::Inherit,
            ProcessStdio::ProcessStdioNull(_) => ResolvedStdioDescriptor::Null,
            ProcessStdio::ProcessStdioPipe(descriptor_pipe) => {
                let pipe_descriptor = resolve_pipe_fd(binding, descriptor_pipe.pipe)?;
                if pipe_descriptor < 0 {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "stdio.pipe",
                        "descriptor must be non-negative",
                    ))
                    .boxed());
                }

                ResolvedStdioDescriptor::Descriptor(pipe_descriptor)
            }
            ProcessStdio::ProcessStdioFile(descriptor_file) => {
                let file_descriptor = resolve_file_fd(binding, descriptor_file.file)?;
                if file_descriptor < 0 {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "stdio.file",
                        "descriptor must be non-negative",
                    ))
                    .boxed());
                }

                ResolvedStdioDescriptor::Descriptor(file_descriptor)
            }
            ProcessStdio::ProcessStdioDescriptor(descriptor_fd) => {
                if descriptor_fd.descriptor < 0 {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "stdio.descriptor",
                        "descriptor must be non-negative",
                    ))
                    .boxed());
                }

                ResolvedStdioDescriptor::Descriptor(descriptor_fd.descriptor)
            }
        };
        resolved[index] = value;
    }

    Ok(resolved)
}

/// Build argv payload vectors for one spawn request.
fn build_spawn_arguments(
    command: &str,
    arguments: &[String],
) -> RuntimeResult<(Vec<CString>, Vec<*const libc::c_char>)> {
    let command = core_process::cstring_from_str(command, "command", "command contains nul byte")?;

    let mut values = Vec::with_capacity(arguments.len() + 1);
    values.push(command);
    for argument in arguments {
        let argument =
            core_process::cstring_from_str(argument, "arguments", "argument contains nul byte")?;
        values.push(argument);
    }

    let mut pointers = Vec::with_capacity(values.len() + 1);
    for value in &values {
        pointers.push(value.as_ptr());
    }
    pointers.push(std::ptr::null());

    Ok((values, pointers))
}

/// Build envp payload vectors for one spawn request.
fn build_spawn_environment(
    environment: &[String],
) -> RuntimeResult<(Vec<CString>, Vec<*const libc::c_char>, Option<String>)> {
    let mut values = Vec::with_capacity(environment.len());
    let mut path_value = None;

    for entry in environment {
        let Some((name, value)) = entry.split_once('=') else {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "environment",
                format!("invalid environment entry: {entry}"),
            ))
            .boxed());
        };

        if name == "PATH" {
            path_value = Some(value.to_string());
        }

        let entry = core_process::cstring_from_str(
            entry,
            "environment",
            "environment entry contains nul byte",
        )?;
        values.push(entry);
    }

    let mut pointers = Vec::with_capacity(values.len() + 1);
    for value in &values {
        pointers.push(value.as_ptr());
    }
    pointers.push(std::ptr::null());

    Ok((values, pointers, path_value))
}

/// Configure one descriptor binding in the child process.
fn bind_child_descriptor(source: i32, target: i32) -> Result<(), i32> {
    if source == target {
        return Ok(());
    }

    let rc = unsafe { libc::dup2(source, target) };
    if rc < 0 {
        return Err(last_errno());
    }

    Ok(())
}

/// Apply process spawn options in the child just before exec.
fn apply_spawn_options_child(options: ProcessSpawnOptions, cwd: Option<&CStr>) -> Result<(), i32> {
    if let Some(cwd) = cwd {
        let rc = unsafe { libc::chdir(cwd.as_ptr()) };
        if rc != 0 {
            return Err(last_errno());
        }
    }

    if options.reset_signals {
        for signal in 1..=64 {
            if signal == libc::SIGKILL || signal == libc::SIGSTOP {
                continue;
            }

            let result = unsafe { libc::signal(signal as libc::c_int, libc::SIG_DFL) };
            if result == libc::SIG_ERR {
                let errno = last_errno();
                if errno != libc::EINVAL {
                    return Err(errno);
                }
            }
        }
    }

    if options.detached {
        let rc = unsafe { libc::setsid() };
        if rc < 0 {
            return Err(last_errno());
        }
    } else if options.new_process_group {
        let rc = unsafe { libc::setpgid(0, 0) };
        if rc != 0 {
            return Err(last_errno());
        }
    }

    Ok(())
}

/// Apply explicit stdio wiring in the child just before exec.
fn apply_spawn_stdio_child(stdio: &[ResolvedStdioDescriptor; 3]) -> Result<(), i32> {
    for (index, descriptor) in stdio.iter().enumerate() {
        let target = index as i32;
        match descriptor {
            ResolvedStdioDescriptor::Inherit => {}
            ResolvedStdioDescriptor::Descriptor(source) => {
                bind_child_descriptor(*source, target)?;
            }
            ResolvedStdioDescriptor::Null => {
                let mode = if index == 0 {
                    libc::O_RDONLY
                } else {
                    libc::O_WRONLY
                };
                let null_fd = unsafe { libc::open(c"/dev/null".as_ptr(), mode) };
                if null_fd < 0 {
                    return Err(last_errno());
                }

                if let Err(errno) = bind_child_descriptor(null_fd, target) {
                    let _ = unsafe { libc::close(null_fd) };
                    return Err(errno);
                }

                let _ = unsafe { libc::close(null_fd) };
            }
        }
    }

    Ok(())
}

/// Apply fd actions in the child just before exec.
fn apply_spawn_fd_actions_child(actions: &[ResolvedFdAction]) -> Result<(), i32> {
    for action in actions {
        match action {
            ResolvedFdAction::Close { descriptor } => {
                let rc = unsafe { libc::close(*descriptor) };
                if rc < 0 {
                    let errno = last_errno();
                    if errno != libc::EBADF {
                        return Err(errno);
                    }
                }
            }
            ResolvedFdAction::Dup2 { source, target } => {
                bind_child_descriptor(*source, *target)?;
            }
            ResolvedFdAction::Open {
                target,
                path,
                flags,
                mode,
            } => {
                let opened = unsafe { libc::open(path.as_ptr(), *flags, *mode as libc::c_uint) };
                if opened < 0 {
                    return Err(last_errno());
                }

                if let Err(errno) = bind_child_descriptor(opened, *target) {
                    let _ = unsafe { libc::close(opened) };
                    return Err(errno);
                }

                let _ = unsafe { libc::close(opened) };
            }
        }
    }

    Ok(())
}

/// Set `FD_CLOEXEC` on one descriptor used by the spawn control pipe.
fn mark_cloexec(fd: i32) -> Result<(), i32> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    if flags < 0 {
        return Err(last_errno());
    }

    let rc = unsafe { libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC) };
    if rc < 0 {
        return Err(last_errno());
    }

    Ok(())
}

/// Create one close-on-exec pipe for parent-child exec error reporting.
fn create_spawn_error_pipe() -> RuntimeResult<(i32, i32)> {
    let mut pipe_fds = [0_i32; 2];
    let pipe_rc = unsafe { libc::pipe(pipe_fds.as_mut_ptr()) };
    if pipe_rc != 0 {
        return Err(core_process::process_last_error(
            "pipe",
            "failed to create spawn error pipe",
        ));
    }

    if let Err(errno) = mark_cloexec(pipe_fds[0]) {
        let _ = unsafe { libc::close(pipe_fds[0]) };
        let _ = unsafe { libc::close(pipe_fds[1]) };
        return Err(core_process::process_errno_error(
            errno,
            "fcntl",
            "failed to configure spawn error pipe read end",
        ));
    }
    if let Err(errno) = mark_cloexec(pipe_fds[1]) {
        let _ = unsafe { libc::close(pipe_fds[0]) };
        let _ = unsafe { libc::close(pipe_fds[1]) };
        return Err(core_process::process_errno_error(
            errno,
            "fcntl",
            "failed to configure spawn error pipe write end",
        ));
    }

    Ok((pipe_fds[0], pipe_fds[1]))
}

/// Read the child exec error report from one spawn control pipe.
fn read_spawn_error(read_fd: i32) -> RuntimeResult<Option<i32>> {
    let mut bytes = [0_u8; 4];
    let mut offset = 0_usize;
    loop {
        let read_count = unsafe {
            libc::read(
                read_fd,
                bytes[offset..].as_mut_ptr() as *mut libc::c_void,
                bytes.len() - offset,
            )
        };
        if read_count == 0 {
            if offset == 0 {
                return Ok(None);
            }

            return Err(RuntimeError::from(PlatformError::io(
                "spawn error pipe returned partial payload",
            ))
            .boxed());
        }

        if read_count < 0 {
            let errno = last_errno();
            if errno == libc::EINTR {
                continue;
            }

            return Err(core_process::process_errno_error(
                errno,
                "read",
                "failed to read spawn error payload",
            ));
        }

        offset += read_count as usize;
        if offset >= bytes.len() {
            return Ok(Some(i32::from_ne_bytes(bytes)));
        }
    }
}

/// Execute one command with PATH lookup and explicit envp payload.
fn execute_spawn_command(
    command: &str,
    command_cstring: &CString,
    argument_pointers: &[*const libc::c_char],
    environment_pointers: &[*const libc::c_char],
    path_value: Option<&str>,
) -> i32 {
    if command.contains('/') {
        let _ = unsafe {
            libc::execve(
                command_cstring.as_ptr(),
                argument_pointers.as_ptr(),
                environment_pointers.as_ptr(),
            )
        };
        return last_errno();
    }

    let mut last_exec_errno = libc::ENOENT;
    let search_path = path_value.unwrap_or("/bin:/usr/bin");
    for segment in search_path.split(':') {
        let candidate = if segment.is_empty() {
            command.to_string()
        } else {
            format!("{segment}/{command}")
        };

        let candidate = match CString::new(candidate) {
            Ok(value) => value,
            Err(_) => return libc::EINVAL,
        };

        let _ = unsafe {
            libc::execve(
                candidate.as_ptr(),
                argument_pointers.as_ptr(),
                environment_pointers.as_ptr(),
            )
        };

        let errno = last_errno();
        if errno == libc::ENOENT || errno == libc::ENOTDIR {
            continue;
        }

        last_exec_errno = errno;
    }

    last_exec_errno
}

/// Spawn a child process and register its handle payload.
fn spawn_process(
    binding: &BindingCallContext,
    out: *mut resource::ProcessHandle,
    command: String,
    arguments: Vec<String>,
    environment: Vec<String>,
    options: ProcessSpawnOptions,
    stdio: &[ProcessStdio],
    actions: &[ProcessFdAction],
) -> RuntimeResult<()> {
    let cwd = resolve_spawn_cwd(options)?;
    let resolved_stdio = resolve_spawn_stdio(binding, stdio)?;
    let resolved_actions = resolve_fd_actions(actions)?;
    let (argument_values, argument_pointers) = build_spawn_arguments(&command, &arguments)?;
    let (environment_values, environment_pointers, path_value) =
        build_spawn_environment(&environment)?;
    let (read_fd, write_fd) = create_spawn_error_pipe()?;

    let child_pid = unsafe { libc::fork() };
    if child_pid < 0 {
        let _ = unsafe { libc::close(read_fd) };
        let _ = unsafe { libc::close(write_fd) };
        return Err(core_process::process_last_error(
            "fork",
            "failed to spawn process",
        ));
    }

    if child_pid == 0 {
        let _ = unsafe { libc::close(read_fd) };

        let child_errno = apply_spawn_options_child(options, cwd.as_deref())
            .and_then(|_| apply_spawn_stdio_child(&resolved_stdio))
            .and_then(|_| apply_spawn_fd_actions_child(&resolved_actions))
            .err()
            .unwrap_or_else(|| {
                execute_spawn_command(
                    &command,
                    &argument_values[0],
                    &argument_pointers,
                    &environment_pointers,
                    path_value.as_deref(),
                )
            });

        let mut errno_bytes = child_errno.to_ne_bytes();
        let mut offset = 0_usize;
        while offset < errno_bytes.len() {
            let write_count = unsafe {
                libc::write(
                    write_fd,
                    errno_bytes[offset..].as_mut_ptr() as *const libc::c_void,
                    errno_bytes.len() - offset,
                )
            };
            if write_count < 0 {
                let errno = last_errno();
                if errno == libc::EINTR {
                    continue;
                }

                break;
            }

            offset += write_count as usize;
        }

        unsafe { libc::_exit(127) };
    }

    let _ = unsafe { libc::close(write_fd) };
    let exec_errno = read_spawn_error(read_fd)?;
    let _ = unsafe { libc::close(read_fd) };
    if let Some(exec_errno) = exec_errno {
        let mut ignored_status = 0_i32;
        let _ = unsafe { libc::waitpid(child_pid, &mut ignored_status, 0) };
        return Err(core_process::process_errno_error(
            exec_errno,
            "execve",
            format!("failed to spawn process from command '{command}'"),
        ));
    }

    let process_id = ProcessId(child_pid as u32);
    let _keep_alive = (argument_values, environment_values);

    let entry = resource::ResourceEntry::new(resource::ResourceKind::Process)
        .with_label("process.spawn")
        .with_payload(core_process::SpawnedProcess { pid: process_id });
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

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
    binding: &BindingCallContext,
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
    spawn_process(
        binding,
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
    binding: &BindingCallContext,
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
    spawn_process(
        binding,
        out,
        command,
        arguments,
        environment,
        options,
        stdio,
        actions,
    )
}
