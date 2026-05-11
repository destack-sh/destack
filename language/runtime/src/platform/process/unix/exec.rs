#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::NativeStringSlice;
use crate::platform::process::core as core_process;

use crate::runtime::BindingCallContext;

use crate::platform::fs::core as core_fs;
use crate::platform::process::ExecAtFlags;
use crate::platform::{fs, resource};
use std::ffi::{CStr, CString};

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
    binding: &BindingCallContext,
    handle: resource::DirectoryHandle,
) -> RuntimeResult<i32> {
    let resolved = binding.worker().resources.with_entry(handle.0, |entry| {
        if entry.kind != resource::ResourceKind::Directory {
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
    binding: &BindingCallContext,
    handle: resource::FileHandle,
) -> RuntimeResult<i32> {
    let resolved = binding.worker().resources.with_entry(handle.0, |entry| {
        if entry.kind != resource::ResourceKind::File {
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
pub(crate) unsafe fn destack_process_exec(
    _binding: &BindingCallContext,
    command: fs::OsPath,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
) -> RuntimeResult<()> {
    let command = core_fs::os_path_to_utf8_string(command, "command")?;
    let arguments = unsafe { decode_native_strings(arguments)? };
    let environment = unsafe { decode_native_strings(environment)? };
    let command = core_process::cstring_from_str(&command, "command", "command contains nul byte")?;
    let (argument_values, argument_pointers) = build_exec_strings(&arguments, command.as_c_str())?;
    let (environment_values, environment_pointers) = build_exec_environment(&environment)?;

    let _keep_alive = (argument_values, environment_values);
    let result = unsafe {
        libc::execve(
            command.as_ptr(),
            argument_pointers.as_ptr(),
            environment_pointers.as_ptr(),
        )
    };
    if result != 0 {
        return Err(core_process::process_last_error(
            "execve",
            "failed to replace process image",
        ));
    }

    Ok(())
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
    let directory_fd = resolve_directory_fd(binding, directory)?;
    let path = core_fs::os_path_to_utf8_string(path, "path")?;
    let arguments = unsafe { decode_native_strings(arguments)? };
    let environment = unsafe { decode_native_strings(environment)? };

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let path = core_process::cstring_from_str(&path, "path", "path contains nul byte")?;
        let (argument_values, argument_pointers) = build_exec_strings(&arguments, path.as_c_str())?;
        let (environment_values, environment_pointers) = build_exec_environment(&environment)?;

        let _keep_alive = (argument_values, environment_values);
        // use the raw syscall lane to avoid toolchain libc symbol availability mismatches
        let result = unsafe {
            libc::syscall(
                libc::SYS_execveat,
                directory_fd,
                path.as_ptr(),
                argument_pointers.as_ptr() as *const *mut libc::c_char,
                environment_pointers.as_ptr() as *const *mut libc::c_char,
                flags.0 as libc::c_int,
            ) as libc::c_int
        };
        if result != 0 {
            return Err(core_process::process_last_error(
                "execveat",
                "failed to replace process image from directory-relative path",
            ));
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (directory_fd, path, arguments, environment, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.exec.pathat")).boxed())
    }
}

/// Replace the current process image using an executable file handle.
pub(crate) unsafe fn destack_process_fexec(
    binding: &BindingCallContext,
    executable: resource::FileHandle,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
) -> RuntimeResult<()> {
    let executable_fd = resolve_file_fd(binding, executable)?;
    let arguments = unsafe { decode_native_strings(arguments)? };
    let environment = unsafe { decode_native_strings(environment)? };

    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        let executable_name = CString::new("fd-exec").map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "executable",
                "invalid executable name",
            ))
            .boxed()
        })?;
        let (argument_values, argument_pointers) =
            build_exec_strings(&arguments, executable_name.as_c_str())?;
        let (environment_values, environment_pointers) = build_exec_environment(&environment)?;

        let _keep_alive = (argument_values, environment_values);
        let result = unsafe {
            libc::fexecve(
                executable_fd,
                argument_pointers.as_ptr(),
                environment_pointers.as_ptr(),
            )
        };
        if result != 0 {
            return Err(core_process::process_last_error(
                "fexecve",
                "failed to replace process image from executable descriptor",
            ));
        }

        Ok(())
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    {
        let _ = (executable_fd, arguments, environment);
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.exec.fexec")).boxed())
    }
}

/// Build C string vectors and pointer arrays for exec arguments.
fn build_exec_strings(
    arguments: &[String],
    fallback_argv0: &CStr,
) -> RuntimeResult<(Vec<CString>, Vec<*const libc::c_char>)> {
    let mut values = Vec::new();
    if arguments.is_empty() {
        values.push(fallback_argv0.to_owned());
    } else {
        for argument in arguments {
            let argument = core_process::cstring_from_str(
                argument.as_str(),
                "arguments",
                "argument contains nul byte",
            )?;
            values.push(argument);
        }
    }

    let mut pointers = Vec::with_capacity(values.len() + 1);
    for value in &values {
        pointers.push(value.as_ptr());
    }
    pointers.push(std::ptr::null());

    Ok((values, pointers))
}

/// Build C string vectors and pointer arrays for exec environment entries.
fn build_exec_environment(
    environment: &[String],
) -> RuntimeResult<(Vec<CString>, Vec<*const libc::c_char>)> {
    let mut values = Vec::with_capacity(environment.len());
    for entry in environment {
        let entry = core_process::cstring_from_str(
            entry.as_str(),
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

    Ok((values, pointers))
}
