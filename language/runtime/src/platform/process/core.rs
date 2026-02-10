use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{PlatformContext, PlatformError};

#[cfg(unix)]
use std::ffi::{CStr, CString};

#[cfg(windows)]
use crate::platform::core as core_platform;

/// Return the process arguments from the platform context.
pub fn process_args(platform: &PlatformContext) -> &[String] {
    platform.args()
}

/// Read the current working directory.
pub fn process_cwd() -> RuntimeResult<String> {
    // read cwd on unix platforms
    #[cfg(unix)]
    {
        let cwd = unsafe { libc::getcwd(std::ptr::null_mut(), 0) };
        if cwd.is_null() {
            return Err(RuntimeError::from(PlatformError::io("failed to read cwd")).boxed());
        }
        let value = unsafe { CStr::from_ptr(cwd) };
        let value = String::from_utf8_lossy(value.to_bytes()).to_string();
        unsafe {
            libc::free(cwd as *mut libc::c_void);
        }
        Ok(value)
    }

    // read cwd on windows platforms
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Environment::GetCurrentDirectoryW;

        let required = unsafe { GetCurrentDirectoryW(0, std::ptr::null_mut()) };
        if required == 0 {
            let error = core_platform::last_error_code();
            return Err(RuntimeError::from(PlatformError::io(format!(
                "failed to read cwd: {error}"
            )))
            .boxed());
        }

        let mut buffer = vec![0u16; required as usize + 1];
        let length = unsafe { GetCurrentDirectoryW(buffer.len() as u32, buffer.as_mut_ptr()) };
        if length == 0 {
            let error = core_platform::last_error_code();
            return Err(RuntimeError::from(PlatformError::io(format!(
                "failed to read cwd: {error}"
            )))
            .boxed());
        }

        let path = String::from_utf16_lossy(&buffer[..length as usize]);
        Ok(path)
    }

    // report unsupported platforms
    #[cfg(not(any(unix, windows)))]
    {
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.cwd")).boxed())
    }
}

/// Change the current working directory.
pub fn process_chdir(path: &str) -> RuntimeResult<()> {
    // reject nul bytes to align with platform APIs
    if path.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "path contains nul byte",
        ))
        .boxed());
    }

    // change directory on unix platforms
    #[cfg(unix)]
    {
        let path = CString::new(path).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path contains nul byte",
            ))
            .boxed()
        })?;
        let rc = unsafe { libc::chdir(path.as_ptr()) };
        if rc != 0 {
            return Err(RuntimeError::from(PlatformError::io("failed to change cwd")).boxed());
        }
        Ok(())
    }

    // change directory on windows platforms
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Environment::SetCurrentDirectoryW;

        let mut wide: Vec<u16> = path.encode_utf16().collect();
        wide.push(0);
        let rc = unsafe { SetCurrentDirectoryW(wide.as_ptr()) };
        if rc == 0 {
            let error = core_platform::last_error_code();
            return Err(RuntimeError::from(PlatformError::io(format!(
                "failed to change cwd: {error}"
            )))
            .boxed());
        }
        Ok(())
    }

    // report unsupported platforms
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.chdir")).boxed())
    }
}

/// Read an environment variable by name.
pub fn process_env_get(name: &str) -> RuntimeResult<Option<String>> {
    // reject nul bytes to align with platform APIs
    if name.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "name",
            "environment variable contains nul byte",
        ))
        .boxed());
    }

    // read environment on unix platforms
    #[cfg(unix)]
    {
        let name = CString::new(name).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "name",
                "environment variable contains nul byte",
            ))
            .boxed()
        })?;
        let value = unsafe { libc::getenv(name.as_ptr()) };
        if value.is_null() {
            return Ok(None);
        }
        let value = unsafe { CStr::from_ptr(value) };
        let value = String::from_utf8_lossy(value.to_bytes()).to_string();
        Ok(Some(value))
    }

    // read environment on windows platforms
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::ERROR_ENVVAR_NOT_FOUND;
        use windows_sys::Win32::System::Environment::GetEnvironmentVariableW;

        let mut wide: Vec<u16> = name.encode_utf16().collect();
        wide.push(0);
        let required = unsafe { GetEnvironmentVariableW(wide.as_ptr(), std::ptr::null_mut(), 0) };
        if required == 0 {
            let error = core_platform::last_error_code() as u32;
            if error == ERROR_ENVVAR_NOT_FOUND {
                return Ok(None);
            }
            return Err(RuntimeError::from(PlatformError::io(format!(
                "failed to read environment: {error}"
            )))
            .boxed());
        }

        let mut buffer = vec![0u16; required as usize + 1];
        let length = unsafe {
            GetEnvironmentVariableW(wide.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32)
        };
        if length == 0 {
            let error = core_platform::last_error_code() as u32;
            return Err(RuntimeError::from(PlatformError::io(format!(
                "failed to read environment: {error}"
            )))
            .boxed());
        }

        let value = String::from_utf16_lossy(&buffer[..length as usize]);
        Ok(Some(value))
    }

    // report unsupported platforms
    #[cfg(not(any(unix, windows)))]
    {
        let _ = name;
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.envGet")).boxed())
    }
}

/// Set an environment variable.
pub fn process_env_set(name: &str, value: &str) -> RuntimeResult<()> {
    // reject nul bytes to align with platform APIs
    if name.contains('\0') || value.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "name",
            "environment variable contains nul byte",
        ))
        .boxed());
    }

    // set environment on unix platforms
    #[cfg(unix)]
    {
        let name = CString::new(name).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "name",
                "environment variable contains nul byte",
            ))
            .boxed()
        })?;
        let value = CString::new(value).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "value",
                "environment variable contains nul byte",
            ))
            .boxed()
        })?;
        let rc = unsafe { libc::setenv(name.as_ptr(), value.as_ptr(), 1) };
        if rc != 0 {
            return Err(RuntimeError::from(PlatformError::io("failed to set environment")).boxed());
        }
        Ok(())
    }

    // set environment on windows platforms
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Environment::SetEnvironmentVariableW;

        let mut name_wide: Vec<u16> = name.encode_utf16().collect();
        name_wide.push(0);
        let mut value_wide: Vec<u16> = value.encode_utf16().collect();
        value_wide.push(0);

        let rc = unsafe { SetEnvironmentVariableW(name_wide.as_ptr(), value_wide.as_ptr()) };
        if rc == 0 {
            let error = core_platform::last_error_code();
            return Err(RuntimeError::from(PlatformError::io(format!(
                "failed to set environment: {error}"
            )))
            .boxed());
        }
        Ok(())
    }

    // report unsupported platforms
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (name, value);
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.envSet")).boxed())
    }
}

/// Delete an environment variable.
pub fn process_env_delete(name: &str) -> RuntimeResult<()> {
    // reject nul bytes to align with platform APIs
    if name.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "name",
            "environment variable contains nul byte",
        ))
        .boxed());
    }

    // delete environment on unix platforms
    #[cfg(unix)]
    {
        let name = CString::new(name).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "name",
                "environment variable contains nul byte",
            ))
            .boxed()
        })?;
        let rc = unsafe { libc::unsetenv(name.as_ptr()) };
        if rc != 0 {
            return Err(
                RuntimeError::from(PlatformError::io("failed to delete environment")).boxed(),
            );
        }
        Ok(())
    }

    // delete environment on windows platforms
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::ERROR_ENVVAR_NOT_FOUND;
        use windows_sys::Win32::System::Environment::SetEnvironmentVariableW;

        let mut name_wide: Vec<u16> = name.encode_utf16().collect();
        name_wide.push(0);
        let rc = unsafe { SetEnvironmentVariableW(name_wide.as_ptr(), std::ptr::null()) };
        if rc == 0 {
            let error = core_platform::last_error_code() as u32;
            if error != ERROR_ENVVAR_NOT_FOUND {
                return Err(RuntimeError::from(PlatformError::io(format!(
                    "failed to delete environment: {error}"
                )))
                .boxed());
            }
        }
        Ok(())
    }

    // report unsupported platforms
    #[cfg(not(any(unix, windows)))]
    {
        let _ = name;
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.envDelete")).boxed())
    }
}

/// Return the process identifier.
pub fn process_pid() -> RuntimeResult<u32> {
    #[cfg(unix)]
    {
        Ok(unsafe { libc::getpid() as u32 })
    }
    #[cfg(windows)]
    {
        Ok(unsafe { windows_sys::Win32::System::Threading::GetCurrentProcessId() })
    }
    #[cfg(not(any(unix, windows)))]
    {
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.pid")).boxed())
    }
}

/// Return the parent process identifier.
pub fn process_ppid() -> RuntimeResult<u32> {
    #[cfg(unix)]
    {
        Ok(unsafe { libc::getppid() as u32 })
    }
    #[cfg(not(unix))]
    {
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.ppid")).boxed())
    }
}

/// Return the user id.
pub fn process_uid() -> RuntimeResult<u32> {
    #[cfg(unix)]
    {
        Ok(unsafe { libc::getuid() as u32 })
    }
    #[cfg(not(unix))]
    {
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.uid")).boxed())
    }
}

/// Return the group id.
pub fn process_gid() -> RuntimeResult<u32> {
    #[cfg(unix)]
    {
        Ok(unsafe { libc::getgid() as u32 })
    }
    #[cfg(not(unix))]
    {
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.gid")).boxed())
    }
}

/// Update the process umask and return the previous value.
pub fn process_umask(mask: u32) -> RuntimeResult<u32> {
    #[cfg(unix)]
    {
        Ok(unsafe { libc::umask(mask as libc::mode_t) as u32 })
    }
    #[cfg(not(unix))]
    {
        let _ = mask;
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.umask")).boxed())
    }
}

/// Send a signal to the given process.
pub fn process_kill(pid: u32, signal: u32) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(pid as libc::pid_t, signal as libc::c_int) };
        if result < 0 {
            return Err(RuntimeError::from(PlatformError::io(format!(
                "failed to signal process {pid}"
            )))
            .boxed());
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = (pid, signal);
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.kill")).boxed())
    }
}

/// Exit the current process.
pub fn process_exit(code: u32) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        unsafe { libc::_exit(code as i32) }
    }
    #[cfg(windows)]
    {
        unsafe { windows_sys::Win32::System::Threading::ExitProcess(code) }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = code;
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.process.exit")).boxed(),
        );
    }

    #[allow(unreachable_code)]
    Ok(())
}
