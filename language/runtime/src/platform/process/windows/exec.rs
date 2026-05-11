#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::NativeStringSlice;

use crate::runtime::BindingCallContext;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;

use crate::platform::fs::{core as core_fs, native as fs_native};
use crate::platform::process::ExecAtFlags;
use crate::platform::{fs, resource};
use windows_sys::Win32::Foundation::HANDLE;

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

/// Build one command line string from a command and argv tail.
fn build_command_line(command: &str, arguments: &[String]) -> String {
    let mut values = Vec::with_capacity(arguments.len() + 1);
    values.push(quote_windows_argument(command));
    for argument in arguments {
        values.push(quote_windows_argument(argument));
    }

    values.join(" ")
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
    binding: &BindingCallContext,
    handle_id: resource::ResourceId,
    kind: resource::ResourceKind,
    label: &str,
) -> RuntimeResult<String> {
    let handle = core_fs::require_resource(binding, handle_id, kind, label, |entry| {
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

    let wide = fs_native::final_path_from_handle(handle)?;
    let path = std::ffi::OsString::from_wide(&wide);
    let path = path.to_string_lossy().to_string();

    Ok(normalize_handle_path(path))
}

/// Replace the process image by spawning one command and exiting with its status.
fn exec_replace_with_path(
    command: String,
    arguments: Vec<String>,
    environment: Vec<String>,
) -> RuntimeResult<()> {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::Threading::{
        CREATE_UNICODE_ENVIRONMENT, CreateProcessW, ExitProcess, GetExitCodeProcess, INFINITE,
        PROCESS_INFORMATION, STARTUPINFOW, WaitForSingleObject,
    };

    let environment_block = build_environment_block(&environment)?;
    let command_line = build_command_line(&command, &arguments);

    let mut command_wide: Vec<u16> = command.encode_utf16().collect();
    command_wide.push(0);

    let mut command_line_wide: Vec<u16> = command_line.encode_utf16().collect();
    command_line_wide.push(0);

    let startup_info = STARTUPINFOW {
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
        hStdInput: 0 as HANDLE,
        hStdOutput: 0 as HANDLE,
        hStdError: 0 as HANDLE,
    };
    let mut process_info = PROCESS_INFORMATION {
        hProcess: 0,
        hThread: 0,
        dwProcessId: 0,
        dwThreadId: 0,
    };

    let created = unsafe {
        CreateProcessW(
            command_wide.as_ptr(),
            command_line_wide.as_mut_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            CREATE_UNICODE_ENVIRONMENT,
            environment_block.as_ptr() as *mut libc::c_void,
            std::ptr::null(),
            &startup_info,
            &mut process_info,
        )
    };
    if created == 0 {
        return Err(
            RuntimeError::from(PlatformError::io("failed to spawn replacement process")).boxed(),
        );
    }

    let wait_result = unsafe { WaitForSingleObject(process_info.hProcess, INFINITE) };
    if wait_result == windows_sys::Win32::Foundation::WAIT_FAILED {
        unsafe {
            CloseHandle(process_info.hThread);
            CloseHandle(process_info.hProcess);
        }
        return Err(
            RuntimeError::from(PlatformError::io("failed to wait replacement process")).boxed(),
        );
    }

    let mut exit_code = 1_u32;
    let read_exit_code = unsafe { GetExitCodeProcess(process_info.hProcess, &mut exit_code) };

    unsafe {
        CloseHandle(process_info.hThread);
        CloseHandle(process_info.hProcess);
    }

    if read_exit_code == 0 {
        return Err(RuntimeError::from(PlatformError::io(
            "failed to read replacement process exit code",
        ))
        .boxed());
    }

    unsafe { ExitProcess(exit_code) }
}
/// Replace the current process image with a command path.
pub(crate) unsafe fn destack_process_exec(
    _binding: &BindingCallContext,
    command: fs::OsPath,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
) -> RuntimeResult<()> {
    let command = core_fs::os_path_to_utf8_string(command, "command")?;
    let arguments = unsafe { decode_native_strings(arguments)? };
    let environment = unsafe { decode_native_strings(environment)? };
    exec_replace_with_path(command, arguments, environment)
}

/// Replace the current process image using a directory-relative path.
pub(crate) unsafe fn destack_process_execat(
    binding: &BindingCallContext,
    directory: resource::DirectoryHandle,
    path: fs::OsPath,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
    flags: ExecAtFlags,
) -> RuntimeResult<()> {
    if flags.0 != 0 {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.exec.pathat.flags",
        ))
        .boxed());
    }

    let directory_path = path_from_handle(
        binding,
        directory.0,
        resource::ResourceKind::Directory,
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
pub(crate) unsafe fn destack_process_fexec(
    binding: &BindingCallContext,
    executable: resource::FileHandle,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
) -> RuntimeResult<()> {
    let command = path_from_handle(
        binding,
        executable.0,
        resource::ResourceKind::File,
        "executable",
    )?;
    let arguments = unsafe { decode_native_strings(arguments)? };
    let environment = unsafe { decode_native_strings(environment)? };
    exec_replace_with_path(command, arguments, environment)
}
