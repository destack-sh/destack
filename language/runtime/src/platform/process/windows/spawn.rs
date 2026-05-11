#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringSlice};
use crate::platform::process::core as core_process;
use crate::platform::resource::{ResourceFinalizer, ResourceId};
use crate::platform::{PlatformError, core as core_platform};

use crate::runtime::BindingCallContext;
use windows_sys::Win32::Foundation::{
    CloseHandle, DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Console::{
    GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};
use windows_sys::Win32::System::Threading::{
    CREATE_NEW_PROCESS_GROUP, CREATE_UNICODE_ENVIRONMENT, CreateProcessW, DETACHED_PROCESS,
    GetCurrentProcess, PROCESS_INFORMATION, STARTF_USESTDHANDLES, STARTUPINFOW,
};

use crate::platform::fs::core as core_fs;
use crate::platform::process::{
    ProcessFdAction, ProcessId, ProcessSpawnOptions, ProcessStdio, ProcessStdioInherit,
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

/// Resolve a file handle into a Windows handle value.
fn resolve_file_handle(
    binding: &BindingCallContext,
    handle: resource::FileHandle,
) -> RuntimeResult<HANDLE> {
    core_fs::require_resource(
        binding,
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

/// Resolve a pipe handle into a Windows handle value.
fn resolve_pipe_handle(
    binding: &BindingCallContext,
    handle: resource::PipeHandle,
) -> RuntimeResult<HANDLE> {
    core_fs::require_resource(
        binding,
        handle.0,
        resource::ResourceKind::Pipe,
        "pipe",
        |entry| {
            entry.handle().map(|handle| handle as _).ok_or_else(|| {
                RuntimeError::from(PlatformError::generic(
                    None,
                    "pipe handle missing raw handle",
                ))
                .boxed()
            })
        },
    )
}

/// Duplicate a raw Windows handle into one inheritable child handle.
fn duplicate_handle_for_child(handle: HANDLE, label: &str) -> RuntimeResult<HANDLE> {
    // validate source handles
    if handle == 0 || handle == INVALID_HANDLE_VALUE {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "invalid handle",
        ))
        .boxed());
    }

    // duplicate the source handle as inheritable for CreateProcessW
    let process = unsafe { GetCurrentProcess() };
    let mut duplicated = 0;
    let rc = unsafe {
        DuplicateHandle(
            process,
            handle,
            process,
            &mut duplicated,
            0,
            1,
            DUPLICATE_SAME_ACCESS,
        )
    };
    if rc == 0 {
        return Err(
            RuntimeError::from(PlatformError::io("failed to duplicate stdio handle")).boxed(),
        );
    }

    Ok(duplicated)
}

/// Build a null-separated UTF-16 environment block from `KEY=VALUE` pairs.
fn build_environment_block(entries: &[String]) -> RuntimeResult<Vec<u16>> {
    let mut block = Vec::<u16>::new();
    for entry in entries {
        let Some((_name, _value)) = entry.split_once('=') else {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "environment",
                format!("invalid environment entry: {entry}"),
            ))
            .boxed());
        };

        if entry.contains('\0') {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "environment",
                "environment entry contains nul byte",
            ))
            .boxed());
        }

        block.extend(entry.encode_utf16());
        block.push(0);
    }

    if block.is_empty() {
        block.push(0);
    }
    block.push(0);

    Ok(block)
}

/// Quote one command-line argument according to Windows command-line rules.
fn quote_windows_argument(argument: &str) -> String {
    if !argument.is_empty()
        && !argument.contains(' ')
        && !argument.contains('\t')
        && !argument.contains('"')
        && !argument.contains('\\')
    {
        return argument.to_string();
    }

    let mut quoted = String::with_capacity(argument.len() + 2);
    quoted.push('"');

    let mut backslash_count = 0_usize;
    for ch in argument.chars() {
        if ch == '\\' {
            backslash_count += 1;
            continue;
        }

        if ch == '"' {
            quoted.extend(std::iter::repeat_n('\\', backslash_count * 2 + 1));
            quoted.push('"');
            backslash_count = 0;
            continue;
        }

        if backslash_count > 0 {
            quoted.extend(std::iter::repeat_n('\\', backslash_count));
            backslash_count = 0;
        }

        quoted.push(ch);
    }

    if backslash_count > 0 {
        quoted.extend(std::iter::repeat_n('\\', backslash_count * 2));
    }

    quoted.push('"');
    quoted
}

/// Build one Windows command line from one command and one argv tail.
fn build_command_line(command: &str, arguments: &[String]) -> String {
    let mut values = Vec::with_capacity(arguments.len() + 1);
    values.push(quote_windows_argument(command));
    for argument in arguments {
        values.push(quote_windows_argument(argument));
    }

    values.join(" ")
}

/// Open one inheritable null-device handle for stdio routing.
fn open_null_stdio_handle(is_input: bool) -> RuntimeResult<HANDLE> {
    let security_attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: std::ptr::null_mut(),
        bInheritHandle: 1,
    };

    let path = core_platform::wide_with_nul("NUL");
    let access = if is_input {
        FILE_GENERIC_READ
    } else {
        FILE_GENERIC_WRITE
    };
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            &security_attributes,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            0,
        )
    };
    if handle == 0 || handle == INVALID_HANDLE_VALUE {
        return Err(
            RuntimeError::from(PlatformError::io("failed to open null stdio handle")).boxed(),
        );
    }

    Ok(handle)
}

/// Close one owned stdio handle when it is valid.
fn close_spawn_handle(handle: HANDLE) {
    if handle != 0 && handle != INVALID_HANDLE_VALUE {
        unsafe {
            CloseHandle(handle);
        }
    }
}

/// Resolve one stdio slot into a concrete inheritable child handle.
fn resolve_spawn_stdio_handle(
    binding: &BindingCallContext,
    index: usize,
    descriptor: ProcessStdio,
) -> RuntimeResult<HANDLE> {
    let is_input = index == 0;
    match descriptor {
        ProcessStdio::ProcessStdioInherit(_) => {
            let standard = match index {
                0 => STD_INPUT_HANDLE,
                1 => STD_OUTPUT_HANDLE,
                2 => STD_ERROR_HANDLE,
                _ => unreachable!(),
            };
            let source = unsafe { GetStdHandle(standard) };
            duplicate_handle_for_child(source, "stdio.inherit")
        }
        ProcessStdio::ProcessStdioNull(_) => open_null_stdio_handle(is_input),
        ProcessStdio::ProcessStdioPipe(descriptor_pipe) => {
            let pipe_handle = resolve_pipe_handle(binding, descriptor_pipe.pipe)?;
            duplicate_handle_for_child(pipe_handle, "stdio.pipe")
        }
        ProcessStdio::ProcessStdioFile(descriptor_file) => {
            let file_handle = resolve_file_handle(binding, descriptor_file.file)?;
            duplicate_handle_for_child(file_handle, "stdio.file")
        }
        ProcessStdio::ProcessStdioDescriptor(descriptor_fd) => {
            let raw = unsafe { libc::get_osfhandle(descriptor_fd.descriptor) };
            if raw == -1 {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "stdio.descriptor",
                    "invalid descriptor",
                ))
                .boxed());
            }

            duplicate_handle_for_child(raw as HANDLE, "stdio.descriptor")
        }
    }
}

/// Resolve explicit stdio descriptors into startup handles.
fn resolve_spawn_stdio_handles(
    binding: &BindingCallContext,
    stdio: &[ProcessStdio],
) -> RuntimeResult<Option<[HANDLE; 3]>> {
    if stdio.is_empty() {
        return Ok(None);
    }
    if stdio.len() > 3 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "stdio",
            "stdio entries must target stdin, stdout, and stderr only",
        ))
        .boxed());
    }

    let defaults = [
        ProcessStdio::ProcessStdioInherit(ProcessStdioInherit {
            kind: binding.store_string("inherit"),
        }),
        ProcessStdio::ProcessStdioInherit(ProcessStdioInherit {
            kind: binding.store_string("inherit"),
        }),
        ProcessStdio::ProcessStdioInherit(ProcessStdioInherit {
            kind: binding.store_string("inherit"),
        }),
    ];

    let mut resolved = [0, 0, 0];
    for index in 0..3 {
        let descriptor = if index < stdio.len() {
            stdio[index]
        } else {
            defaults[index]
        };
        resolved[index] = resolve_spawn_stdio_handle(binding, index, descriptor)?;
    }

    Ok(Some(resolved))
}

/// Build CreateProcess creation flags from process spawn options.
fn spawn_creation_flags(options: ProcessSpawnOptions) -> u32 {
    let mut flags = 0_u32;
    if options.detached {
        flags |= DETACHED_PROCESS;
    }
    if options.new_process_group {
        flags |= CREATE_NEW_PROCESS_GROUP;
    }

    // windows does not expose a direct equivalent for posix signal-disposition reset
    let _ = options.reset_signals;

    flags
}

/// Resolve spawn current-directory option into a nullable UTF-16 buffer.
fn spawn_current_directory(options: ProcessSpawnOptions) -> RuntimeResult<Option<Vec<u16>>> {
    let cwd = core_fs::os_path_to_utf8_string(options.cwd, "options.cwd")?;
    if cwd.is_empty() {
        return Ok(None);
    }

    Ok(Some(core_platform::wide_with_nul(&cwd)))
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
) -> RuntimeResult<()> {
    let environment_block = build_environment_block(&environment)?;
    let command_line = build_command_line(&command, &arguments);
    let creation_flags = spawn_creation_flags(options) | CREATE_UNICODE_ENVIRONMENT;
    let current_directory = spawn_current_directory(options)?;
    let stdio_handles = resolve_spawn_stdio_handles(binding, stdio)?;

    let mut command_line_wide = core_platform::wide_with_nul(&command_line);
    let current_directory_pointer = current_directory
        .as_ref()
        .map_or(std::ptr::null(), |cwd| cwd.as_ptr());

    let mut startup_info = STARTUPINFOW {
        cb: std::mem::size_of::<STARTUPINFOW>() as u32,
        lpReserved: std::ptr::null_mut(),
        lpDesktop: std::ptr::null_mut(),
        lpTitle: std::ptr::null_mut(),
        dwX: 0,
        dwY: 0,
        dwXSize: 0,
        dwYSize: 0,
        dwXCountChars: 0,
        dwYCountChars: 0,
        dwFillAttribute: 0,
        dwFlags: 0,
        wShowWindow: 0,
        cbReserved2: 0,
        lpReserved2: std::ptr::null_mut(),
        hStdInput: 0,
        hStdOutput: 0,
        hStdError: 0,
    };
    let mut process_info = PROCESS_INFORMATION {
        hProcess: 0,
        hThread: 0,
        dwProcessId: 0,
        dwThreadId: 0,
    };

    let inherit_handles = if let Some(handles) = &stdio_handles {
        startup_info.dwFlags = STARTF_USESTDHANDLES;
        startup_info.hStdInput = handles[0];
        startup_info.hStdOutput = handles[1];
        startup_info.hStdError = handles[2];
        1
    } else {
        0
    };

    let created = unsafe {
        CreateProcessW(
            std::ptr::null(),
            command_line_wide.as_mut_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            inherit_handles,
            creation_flags,
            environment_block.as_ptr() as *mut libc::c_void,
            current_directory_pointer,
            &startup_info,
            &mut process_info,
        )
    };

    if let Some(handles) = &stdio_handles {
        close_spawn_handle(handles[0]);
        close_spawn_handle(handles[1]);
        close_spawn_handle(handles[2]);
    }

    if created == 0 {
        let error = std::io::Error::last_os_error();
        return Err(RuntimeError::from(PlatformError::io(format!(
            "failed to spawn process: {error}",
        )))
        .boxed());
    }

    unsafe {
        CloseHandle(process_info.hThread);
    }

    let process_id = ProcessId(process_info.dwProcessId);
    let process_handle = process_info.hProcess as *mut libc::c_void;

    let entry = resource::ResourceEntry::new(resource::ResourceKind::Process)
        .with_label("process.spawn")
        .with_payload(core_process::SpawnedProcess { pid: process_id })
        .with_handle(process_handle)
        .with_finalizer(ProcessHandleFinalizer::new(process_handle as HANDLE));
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

    spawn_process(binding, out, command, arguments, environment, options, &[])
}

/// Spawn a child process with explicit stdio and descriptor actions.
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

    // windows has no direct equivalent for arbitrary fd action scripts
    if !actions.is_empty() {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.spawn.actions",
        ))
        .boxed());
    }

    spawn_process(
        binding,
        out,
        command,
        arguments,
        environment,
        options,
        stdio,
    )
}
