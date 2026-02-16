use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(unix)]
use crate::platform::diagnostic::process_error_code_from_errno;
#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::platform::process::ProcessSchedulerPolicy;
#[cfg(unix)]
use crate::platform::process::UserId;
use crate::platform::process::{
    GroupId, ProcessGroupIds, ProcessId, ProcessLimit, ProcessNamespaceKind,
    ProcessSchedulerConfig, ProcessUserIds, ProcessWaitKind, ProcessWaitStatus, Signal,
    SignalEvent, SignalMaskHow,
};
use crate::platform::{PlatformContext, PlatformError, PlatformErrorCode};

#[cfg(unix)]
use std::ffi::{CStr, CString};
#[cfg(unix)]
use std::time::{Duration, Instant};

#[cfg(windows)]
use crate::platform::core as core_platform;

/// Return the process arguments from the platform context.
pub fn process_args(platform: &PlatformContext) -> &[String] {
    platform.args()
}

/// Build an error for a missing environment variable.
pub(crate) fn missing_env_error(name: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument_value(
        "name",
        format!("environment variable not found: {}", name.into()),
    ))
    .boxed()
}

/// Subscription payload stored for signal handles.
#[derive(Debug, Clone)]
pub(crate) struct SignalSubscription {
    /// Signals associated with this subscription.
    pub signals: Vec<Signal>,
}

/// Process payload stored for spawned process handles.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SpawnedProcess {
    /// Process id associated with the handle.
    pub pid: ProcessId,
}

/// Process payload stored for process-fd style handles.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProcessFdBinding {
    /// Process id associated with the descriptor handle.
    pub pid: ProcessId,
}

/// Process payload stored for signal-fd style handles.
#[derive(Debug, Clone)]
pub(crate) struct SignalFdBinding {
    /// Signal mask associated with the descriptor handle.
    pub signals: Vec<Signal>,
}

/// Nonblocking wait flag used by process wait bindings.
#[cfg(unix)]
pub const PROCESS_WAIT_FLAG_NOHANG: u32 = libc::WNOHANG as u32;
/// Nonblocking wait flag used by process wait bindings.
#[cfg(windows)]
pub const PROCESS_WAIT_FLAG_NOHANG: u32 = 0x0000_0001;
/// Nonblocking wait flag used by process wait bindings.
#[cfg(not(any(unix, windows)))]
pub const PROCESS_WAIT_FLAG_NOHANG: u32 = 0x0000_0001;

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

/// Read an environment variable by raw byte name.
pub fn process_env_get_bytes(name: &[u8]) -> RuntimeResult<Option<Vec<u8>>> {
    // reject nul bytes to align with platform APIs
    if name.contains(&0) {
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
        Ok(Some(value.to_bytes().to_vec()))
    }

    // read environment on windows platforms
    #[cfg(windows)]
    {
        let name = std::str::from_utf8(name).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "name",
                "environment variable is not valid utf8",
            ))
            .boxed()
        })?;

        let value = process_env_get(name)?;
        Ok(value.map(|value| value.into_bytes()))
    }

    // report unsupported platforms
    #[cfg(not(any(unix, windows)))]
    {
        let _ = name;
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.envGetBytes")).boxed())
    }
}

/// Set an environment variable by raw byte name and value.
pub fn process_env_set_bytes(name: &[u8], value: &[u8]) -> RuntimeResult<()> {
    // reject nul bytes to align with platform APIs
    if name.contains(&0) || value.contains(&0) {
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
        let name = std::str::from_utf8(name).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "name",
                "environment variable is not valid utf8",
            ))
            .boxed()
        })?;
        let value = std::str::from_utf8(value).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "value",
                "environment variable is not valid utf8",
            ))
            .boxed()
        })?;

        process_env_set(name, value)
    }

    // report unsupported platforms
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (name, value);
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.envSetBytes")).boxed())
    }
}

/// Delete an environment variable by raw byte name.
pub fn process_env_delete_bytes(name: &[u8]) -> RuntimeResult<()> {
    // reject nul bytes to align with platform APIs
    if name.contains(&0) {
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
        let name = std::str::from_utf8(name).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "name",
                "environment variable is not valid utf8",
            ))
            .boxed()
        })?;

        process_env_delete(name)
    }

    // report unsupported platforms
    #[cfg(not(any(unix, windows)))]
    {
        let _ = name;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.envDeleteBytes",
        ))
        .boxed())
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

    #[cfg(windows)]
    {
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.ppid")).boxed())
    }

    #[cfg(not(any(unix, windows)))]
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

/// Return the effective user id.
pub fn process_euid() -> RuntimeResult<u32> {
    #[cfg(unix)]
    {
        Ok(unsafe { libc::geteuid() as u32 })
    }
    #[cfg(not(unix))]
    {
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.euid")).boxed())
    }
}

/// Return the effective group id.
pub fn process_egid() -> RuntimeResult<u32> {
    #[cfg(unix)]
    {
        Ok(unsafe { libc::getegid() as u32 })
    }
    #[cfg(not(unix))]
    {
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.egid")).boxed())
    }
}

/// Return real, effective, and saved group ids.
pub fn process_group_ids() -> RuntimeResult<ProcessGroupIds> {
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        let mut real: libc::gid_t = 0;
        let mut effective: libc::gid_t = 0;
        let mut saved: libc::gid_t = 0;
        let result = unsafe { libc::getresgid(&mut real, &mut effective, &mut saved) };
        if result != 0 {
            let error = std::io::Error::last_os_error();
            return Err(RuntimeError::from(PlatformError::io(format!(
                "failed to read group ids: {error}"
            )))
            .boxed());
        }

        Ok(ProcessGroupIds {
            real: GroupId(real as u32),
            effective: GroupId(effective as u32),
            saved: GroupId(saved as u32),
        })
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        let real = unsafe { libc::getgid() as u32 };
        let effective = unsafe { libc::getegid() as u32 };

        Ok(ProcessGroupIds {
            real: GroupId(real),
            effective: GroupId(effective),
            // darwin does not expose getresgid through libc, so mirror effective as best effort
            saved: GroupId(effective),
        })
    }

    #[cfg(not(unix))]
    {
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.groupIds")).boxed())
    }
}

/// Return supplementary group ids.
pub fn process_groups() -> RuntimeResult<Vec<GroupId>> {
    #[cfg(unix)]
    {
        let count = unsafe { libc::getgroups(0, std::ptr::null_mut()) };
        if count < 0 {
            let error = std::io::Error::last_os_error();
            return Err(RuntimeError::from(PlatformError::io(format!(
                "failed to read supplementary groups: {error}"
            )))
            .boxed());
        }

        let mut groups = vec![0 as libc::gid_t; count as usize];
        let result = unsafe { libc::getgroups(count, groups.as_mut_ptr()) };
        if result < 0 {
            let error = std::io::Error::last_os_error();
            return Err(RuntimeError::from(PlatformError::io(format!(
                "failed to read supplementary groups: {error}"
            )))
            .boxed());
        }

        let groups = groups
            .into_iter()
            .map(|group| GroupId(group as u32))
            .collect::<Vec<_>>();

        Ok(groups)
    }

    #[cfg(not(unix))]
    {
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.groups")).boxed())
    }
}

/// Return real, effective, and saved user ids.
pub fn process_user_ids() -> RuntimeResult<ProcessUserIds> {
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        let mut real: libc::uid_t = 0;
        let mut effective: libc::uid_t = 0;
        let mut saved: libc::uid_t = 0;
        let result = unsafe { libc::getresuid(&mut real, &mut effective, &mut saved) };
        if result != 0 {
            let error = std::io::Error::last_os_error();
            return Err(RuntimeError::from(PlatformError::io(format!(
                "failed to read user ids: {error}"
            )))
            .boxed());
        }

        Ok(ProcessUserIds {
            real: UserId(real as u32),
            effective: UserId(effective as u32),
            saved: UserId(saved as u32),
        })
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        let real = unsafe { libc::getuid() as u32 };
        let effective = unsafe { libc::geteuid() as u32 };

        Ok(ProcessUserIds {
            real: UserId(real),
            effective: UserId(effective),
            // darwin does not expose getresuid through libc, so mirror effective as best effort
            saved: UserId(effective),
        })
    }

    #[cfg(not(unix))]
    {
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.userIds")).boxed())
    }
}

/// Set the effective group identifier only.
pub fn process_set_egid(group_id: u32) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let result = unsafe { libc::setegid(group_id as libc::gid_t) };
        if result != 0 {
            return Err(process_last_error(
                "setegid",
                format!("failed to set effective group id to {group_id}"),
            ));
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = group_id;
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.setEgid")).boxed())
    }
}

/// Set the effective user identifier only.
pub fn process_set_euid(user_id: u32) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let result = unsafe { libc::seteuid(user_id as libc::uid_t) };
        if result != 0 {
            return Err(process_last_error(
                "seteuid",
                format!("failed to set effective user id to {user_id}"),
            ));
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = user_id;
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.setEuid")).boxed())
    }
}

/// Set the group identifier.
pub fn process_set_gid(group_id: u32) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let result = unsafe { libc::setgid(group_id as libc::gid_t) };
        if result != 0 {
            return Err(process_last_error(
                "setgid",
                format!("failed to set group id to {group_id}"),
            ));
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = group_id;
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.setGid")).boxed())
    }
}

/// Set real, effective, and saved group identifiers together.
pub fn process_set_group_ids(ids: ProcessGroupIds) -> RuntimeResult<()> {
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        let result = unsafe {
            libc::setresgid(
                ids.real.0 as libc::gid_t,
                ids.effective.0 as libc::gid_t,
                ids.saved.0 as libc::gid_t,
            )
        };
        if result != 0 {
            return Err(process_last_error(
                "setresgid",
                format!("failed to set group ids to {:?}", ids),
            ));
        }

        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        if ids.saved != ids.effective {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "ids",
                "saved group id must match effective group id on this platform",
            ))
            .boxed());
        }

        let result =
            unsafe { libc::setregid(ids.real.0 as libc::gid_t, ids.effective.0 as libc::gid_t) };
        if result != 0 {
            return Err(process_last_error(
                "setregid",
                format!("failed to set group ids to {:?}", ids),
            ));
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = ids;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.ids.setGroupIds",
        ))
        .boxed())
    }
}

/// Set supplementary group identifiers.
pub fn process_set_groups(groups: &[GroupId]) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let mut raw_groups = Vec::with_capacity(groups.len());
        for group in groups {
            raw_groups.push(group.0 as libc::gid_t);
        }

        let count = i32::try_from(raw_groups.len()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "groups",
                "too many supplementary groups",
            ))
            .boxed()
        })?;
        let result = unsafe { libc::setgroups(count, raw_groups.as_ptr()) };
        if result != 0 {
            return Err(process_last_error(
                "setgroups",
                "failed to set supplementary groups",
            ));
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = groups;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.ids.setGroups",
        ))
        .boxed())
    }
}

/// Set the user identifier.
pub fn process_set_uid(user_id: u32) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let result = unsafe { libc::setuid(user_id as libc::uid_t) };
        if result != 0 {
            return Err(process_last_error(
                "setuid",
                format!("failed to set user id to {user_id}"),
            ));
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = user_id;
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.setUid")).boxed())
    }
}

/// Set real, effective, and saved user identifiers together.
pub fn process_set_user_ids(ids: ProcessUserIds) -> RuntimeResult<()> {
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        let result = unsafe {
            libc::setresuid(
                ids.real.0 as libc::uid_t,
                ids.effective.0 as libc::uid_t,
                ids.saved.0 as libc::uid_t,
            )
        };
        if result != 0 {
            return Err(process_last_error(
                "setresuid",
                format!("failed to set user ids to {:?}", ids),
            ));
        }

        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        if ids.saved != ids.effective {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "ids",
                "saved user id must match effective user id on this platform",
            ))
            .boxed());
        }

        let result =
            unsafe { libc::setreuid(ids.real.0 as libc::uid_t, ids.effective.0 as libc::uid_t) };
        if result != 0 {
            return Err(process_last_error(
                "setreuid",
                format!("failed to set user ids to {:?}", ids),
            ));
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = ids;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.ids.setUserIds",
        ))
        .boxed())
    }
}

/// Return a process group id for the provided process id.
pub fn process_getpgid(pid: u32) -> RuntimeResult<u32> {
    #[cfg(unix)]
    {
        let result = unsafe { libc::getpgid(pid as libc::pid_t) };
        if result < 0 {
            let error = std::io::Error::last_os_error();
            return Err(RuntimeError::from(PlatformError::io(format!(
                "failed to read process group id for pid {pid}: {error}"
            )))
            .boxed());
        }

        Ok(result as u32)
    }

    #[cfg(not(unix))]
    {
        let _ = pid;
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.getpgid")).boxed())
    }
}

/// Set a process group id for a process.
pub fn process_setpgid(pid: u32, process_group_id: u32) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let result = unsafe { libc::setpgid(pid as libc::pid_t, process_group_id as libc::pid_t) };
        if result != 0 {
            return Err(process_last_error(
                "setpgid",
                format!("failed to set pgid {process_group_id} for pid {pid}"),
            ));
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = (pid, process_group_id);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.session.setpgid",
        ))
        .boxed())
    }
}

/// Create a new process session.
pub fn process_setsid() -> RuntimeResult<ProcessId> {
    #[cfg(unix)]
    {
        let result = unsafe { libc::setsid() };
        if result < 0 {
            return Err(process_last_error(
                "setsid",
                "failed to create a new process session",
            ));
        }

        Ok(ProcessId(result as u32))
    }

    #[cfg(not(unix))]
    {
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.session.setsid",
        ))
        .boxed())
    }
}

/// Read one process resource limit.
pub fn process_get_limit(resource: u32) -> RuntimeResult<ProcessLimit> {
    #[cfg(unix)]
    {
        let mut raw_limit = unsafe { std::mem::zeroed::<libc::rlimit>() };
        let result = unsafe { libc::getrlimit(resource as _, &mut raw_limit) };
        if result != 0 {
            return Err(process_last_error(
                "getrlimit",
                format!("failed to get limit for resource {resource}"),
            ));
        }

        Ok(ProcessLimit {
            soft: raw_limit.rlim_cur as u64,
            hard: raw_limit.rlim_max as u64,
        })
    }

    #[cfg(not(unix))]
    {
        let _ = resource;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.limits.getLimit",
        ))
        .boxed())
    }
}

/// Set one process resource limit.
pub fn process_set_limit(resource: u32, limit: ProcessLimit) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let raw_limit = libc::rlimit {
            rlim_cur: limit.soft as libc::rlim_t,
            rlim_max: limit.hard as libc::rlim_t,
        };

        let result = unsafe { libc::setrlimit(resource as _, &raw_limit) };
        if result != 0 {
            return Err(process_last_error(
                "setrlimit",
                format!("failed to set limit for resource {resource}"),
            ));
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = (resource, limit);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.limits.setLimit",
        ))
        .boxed())
    }
}

/// Read one process priority value.
pub fn process_get_priority(pid: u32) -> RuntimeResult<i32> {
    #[cfg(unix)]
    {
        unsafe {
            *errno_location() = 0;
        }

        let priority = unsafe { libc::getpriority(libc::PRIO_PROCESS, pid as libc::id_t) };
        if priority == -1 {
            let errno = unsafe { *errno_location() };
            if errno != 0 {
                return Err(process_errno_error(
                    errno,
                    "getpriority",
                    format!("failed to get priority for pid {pid}"),
                ));
            }
        }

        Ok(priority)
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{CloseHandle, ERROR_INVALID_PARAMETER};
        use windows_sys::Win32::System::Threading::{
            GetPriorityClass, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };

        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if process == 0 {
            let error = core_platform::last_error_code() as u32;
            let code = if error == ERROR_INVALID_PARAMETER {
                PlatformErrorCode::ProcessNotFound
            } else {
                PlatformErrorCode::ProcessPermissionDenied
            };
            return Err(RuntimeError::from(PlatformError::process_with(
                Some(code),
                Some(error.to_string()),
                None,
                None,
                Some("OpenProcess".to_string()),
                format!("failed to open process {pid} for priority read"),
            ))
            .boxed());
        }

        let class = unsafe { GetPriorityClass(process) };
        let status = if class == 0 {
            let error = core_platform::last_error_code();
            Err(RuntimeError::from(PlatformError::io(format!(
                "failed to read process priority class: {error}",
            )))
            .boxed())
        } else {
            windows_priority_class_to_nice(class)
        };

        unsafe {
            CloseHandle(process);
        }

        status
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.sched.getPriority",
        ))
        .boxed())
    }
}

/// Set one process priority value.
pub fn process_set_priority(pid: u32, priority: i32) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let result = unsafe { libc::setpriority(libc::PRIO_PROCESS, pid as libc::id_t, priority) };
        if result != 0 {
            return Err(process_last_error(
                "setpriority",
                format!("failed to set priority for pid {pid}"),
            ));
        }

        Ok(())
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{CloseHandle, ERROR_INVALID_PARAMETER};
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_SET_INFORMATION, SetPriorityClass,
        };

        let process = unsafe { OpenProcess(PROCESS_SET_INFORMATION, 0, pid) };
        if process == 0 {
            let error = core_platform::last_error_code() as u32;
            let code = if error == ERROR_INVALID_PARAMETER {
                PlatformErrorCode::ProcessNotFound
            } else {
                PlatformErrorCode::ProcessPermissionDenied
            };
            return Err(RuntimeError::from(PlatformError::process_with(
                Some(code),
                Some(error.to_string()),
                None,
                None,
                Some("OpenProcess".to_string()),
                format!("failed to open process {pid} for priority update"),
            ))
            .boxed());
        }

        let priority_class = windows_nice_to_priority_class(priority);
        let result = unsafe { SetPriorityClass(process, priority_class) };
        let status = if result == 0 {
            let error = core_platform::last_error_code();
            Err(RuntimeError::from(PlatformError::io(format!(
                "failed to set process priority class: {error}",
            )))
            .boxed())
        } else {
            Ok(())
        };

        unsafe {
            CloseHandle(process);
        }

        status
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = (pid, priority);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.sched.setPriority",
        ))
        .boxed())
    }
}

/// Read process scheduler configuration.
pub fn process_get_scheduler(pid: u32) -> RuntimeResult<ProcessSchedulerConfig> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let policy = unsafe { libc::sched_getscheduler(pid as libc::pid_t) };
        if policy < 0 {
            return Err(process_last_error(
                "sched_getscheduler",
                format!("failed to read scheduler policy for pid {pid}"),
            ));
        }

        let mut raw_param = unsafe { std::mem::zeroed::<libc::sched_param>() };
        let param_result = unsafe { libc::sched_getparam(pid as libc::pid_t, &mut raw_param) };
        if param_result != 0 {
            return Err(process_last_error(
                "sched_getparam",
                format!("failed to read scheduler priority for pid {pid}"),
            ));
        }

        let policy = scheduler_policy_from_raw(policy)?;
        Ok(ProcessSchedulerConfig {
            policy,
            priority: raw_param.sched_priority,
            flags: 0,
        })
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = pid;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.sched.getScheduler",
        ))
        .boxed())
    }
}

/// Set process scheduler configuration.
pub fn process_set_scheduler(pid: u32, config: ProcessSchedulerConfig) -> RuntimeResult<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        if config.flags != 0 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "config.flags",
                "scheduler flags are not supported yet",
            ))
            .boxed());
        }

        let policy = scheduler_policy_to_raw(config.policy)?;
        let mut raw_param = unsafe { std::mem::zeroed::<libc::sched_param>() };
        raw_param.sched_priority = config.priority;

        let result = unsafe { libc::sched_setscheduler(pid as libc::pid_t, policy, &raw_param) };
        if result != 0 {
            return Err(process_last_error(
                "sched_setscheduler",
                format!("failed to set scheduler for pid {pid}"),
            ));
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (pid, config);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.sched.setScheduler",
        ))
        .boxed())
    }
}

/// Yield the current thread.
pub fn process_yield_now() -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let result = unsafe { libc::sched_yield() };
        if result != 0 {
            return Err(process_last_error(
                "sched_yield",
                "failed to yield scheduler timeslice",
            ));
        }

        Ok(())
    }

    #[cfg(windows)]
    {
        std::thread::yield_now();
        Ok(())
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.sched.yieldNow",
        ))
        .boxed())
    }
}

/// Read process CPU affinity.
pub fn process_get_affinity(pid: u32) -> RuntimeResult<Vec<u32>> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let mut cpu_set = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
        unsafe {
            libc::CPU_ZERO(&mut cpu_set);
        }

        let result = unsafe {
            libc::sched_getaffinity(
                pid as libc::pid_t,
                std::mem::size_of::<libc::cpu_set_t>(),
                &mut cpu_set,
            )
        };
        if result != 0 {
            return Err(process_last_error(
                "sched_getaffinity",
                format!("failed to read affinity for pid {pid}"),
            ));
        }

        let mut cpus = Vec::new();
        for cpu in 0..(libc::CPU_SETSIZE as usize) {
            let is_member = unsafe { libc::CPU_ISSET(cpu, &cpu_set) };
            if is_member {
                cpus.push(cpu as u32);
            }
        }

        Ok(cpus)
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = pid;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.sched.getAffinity",
        ))
        .boxed())
    }
}

/// Set process CPU affinity.
pub fn process_set_affinity(pid: u32, cpus: &[u32]) -> RuntimeResult<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let mut cpu_set = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
        unsafe {
            libc::CPU_ZERO(&mut cpu_set);
        }

        for cpu in cpus {
            let index = *cpu as usize;
            if index >= libc::CPU_SETSIZE as usize {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "cpus",
                    format!("cpu index {cpu} is out of range"),
                ))
                .boxed());
            }

            unsafe {
                libc::CPU_SET(index, &mut cpu_set);
            }
        }

        let result = unsafe {
            libc::sched_setaffinity(
                pid as libc::pid_t,
                std::mem::size_of::<libc::cpu_set_t>(),
                &cpu_set,
            )
        };
        if result != 0 {
            return Err(process_last_error(
                "sched_setaffinity",
                format!("failed to set affinity for pid {pid}"),
            ));
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (pid, cpus);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.sched.setAffinity",
        ))
        .boxed())
    }
}

/// Convert a scheduler policy enum into a host scheduler constant.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn scheduler_policy_to_raw(policy: ProcessSchedulerPolicy) -> RuntimeResult<libc::c_int> {
    match policy {
        ProcessSchedulerPolicy::Other => Ok(libc::SCHED_OTHER),
        ProcessSchedulerPolicy::Fifo => Ok(libc::SCHED_FIFO),
        ProcessSchedulerPolicy::RoundRobin => Ok(libc::SCHED_RR),
        ProcessSchedulerPolicy::Batch => {
            #[cfg(any(target_os = "linux", target_os = "android"))]
            {
                return Ok(libc::SCHED_BATCH);
            }
            #[cfg(not(any(target_os = "linux", target_os = "android")))]
            {
                return Err(RuntimeError::from(PlatformError::not_supported(
                    "destack.process.sched.batch",
                ))
                .boxed());
            }
        }
        ProcessSchedulerPolicy::Idle => {
            #[cfg(any(target_os = "linux", target_os = "android"))]
            {
                return Ok(libc::SCHED_IDLE);
            }
            #[cfg(not(any(target_os = "linux", target_os = "android")))]
            {
                return Err(RuntimeError::from(PlatformError::not_supported(
                    "destack.process.sched.idle",
                ))
                .boxed());
            }
        }
        ProcessSchedulerPolicy::Deadline => {
            #[cfg(any(target_os = "linux", target_os = "android"))]
            {
                return Ok(libc::SCHED_DEADLINE);
            }
            #[cfg(not(any(target_os = "linux", target_os = "android")))]
            {
                return Err(RuntimeError::from(PlatformError::not_supported(
                    "destack.process.sched.deadline",
                ))
                .boxed());
            }
        }
    }
}

/// Convert a host scheduler constant into a scheduler policy enum.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn scheduler_policy_from_raw(policy: libc::c_int) -> RuntimeResult<ProcessSchedulerPolicy> {
    if policy == libc::SCHED_OTHER {
        return Ok(ProcessSchedulerPolicy::Other);
    }
    if policy == libc::SCHED_FIFO {
        return Ok(ProcessSchedulerPolicy::Fifo);
    }
    if policy == libc::SCHED_RR {
        return Ok(ProcessSchedulerPolicy::RoundRobin);
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    if policy == libc::SCHED_BATCH {
        return Ok(ProcessSchedulerPolicy::Batch);
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    if policy == libc::SCHED_IDLE {
        return Ok(ProcessSchedulerPolicy::Idle);
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    if policy == libc::SCHED_DEADLINE {
        return Ok(ProcessSchedulerPolicy::Deadline);
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "policy",
        format!("unsupported scheduler policy value {policy}"),
    ))
    .boxed())
}

/// Wait for one child process state transition.
pub fn process_wait_pid(pid: u32, flags: u32) -> RuntimeResult<ProcessWaitStatus> {
    #[cfg(unix)]
    {
        // validate and decode wait flags
        let options = decode_wait_flags(flags)?;

        // run waitpid for the requested process id
        let mut raw_status: libc::c_int = 0;
        let waited = unsafe { libc::waitpid(pid as libc::pid_t, &mut raw_status, options) };

        // report non-blocking no-result as would-block
        if waited == 0 {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoWouldBlock),
                None,
                Some(libc::EWOULDBLOCK),
                Some("waitpid".to_string()),
                None,
                format!("waitpid would block for pid {pid}"),
            ))
            .boxed());
        }

        // map syscall failure into process errors
        if waited < 0 {
            return Err(waitpid_error(pid, flags));
        }

        Ok(wait_status_from_raw(waited, raw_status))
    }

    #[cfg(windows)]
    {
        const PROCESS_SYNCHRONIZE: u32 = 0x0010_0000;
        use windows_sys::Win32::Foundation::{
            CloseHandle, ERROR_INVALID_PARAMETER, STILL_ACTIVE, WAIT_FAILED, WAIT_OBJECT_0,
            WAIT_TIMEOUT,
        };
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, INFINITE, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
            WaitForSingleObject,
        };

        if flags & !PROCESS_WAIT_FLAG_NOHANG != 0 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "flags",
                "unsupported process wait flags",
            ))
            .boxed());
        }

        let process = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                0,
                pid,
            )
        };
        if process == 0 {
            let error = core_platform::last_error_code() as u32;
            let code = if error == ERROR_INVALID_PARAMETER {
                PlatformErrorCode::ProcessNotFound
            } else {
                PlatformErrorCode::ProcessPermissionDenied
            };
            return Err(RuntimeError::from(PlatformError::process_with(
                Some(code),
                Some(error.to_string()),
                None,
                None,
                Some("OpenProcess".to_string()),
                format!("failed to open process {pid} for wait"),
            ))
            .boxed());
        }

        let timeout = if flags & PROCESS_WAIT_FLAG_NOHANG != 0 {
            0
        } else {
            INFINITE
        };
        let wait_status = unsafe { WaitForSingleObject(process, timeout) };
        let status = match wait_status {
            WAIT_TIMEOUT => Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoWouldBlock),
                None,
                None,
                Some("WaitForSingleObject".to_string()),
                None,
                format!("wait would block for pid {pid}"),
            ))
            .boxed()),
            WAIT_OBJECT_0 => {
                let mut exit_code = 0_u32;
                let rc = unsafe { GetExitCodeProcess(process, &mut exit_code) };
                if rc == 0 {
                    let error = core_platform::last_error_code();
                    Err(RuntimeError::from(PlatformError::io(format!(
                        "failed to read process exit code: {error}",
                    )))
                    .boxed())
                } else if exit_code == STILL_ACTIVE as u32 {
                    Ok(ProcessWaitStatus {
                        pid: ProcessId(pid),
                        kind: ProcessWaitKind::Running,
                        exit_code: 0,
                        signal: Signal(0),
                        core_dumped: false,
                    })
                } else {
                    Ok(ProcessWaitStatus {
                        pid: ProcessId(pid),
                        kind: ProcessWaitKind::Exited,
                        exit_code: exit_code as i32,
                        signal: Signal(0),
                        core_dumped: false,
                    })
                }
            }
            WAIT_FAILED => {
                let error = core_platform::last_error_code();
                Err(RuntimeError::from(PlatformError::io(format!(
                    "wait failed for pid {pid}: {error}",
                )))
                .boxed())
            }
            _ => Err(
                RuntimeError::from(PlatformError::io("wait returned unexpected result")).boxed(),
            ),
        };

        unsafe {
            CloseHandle(process);
        }

        status
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = (pid, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.waitPid")).boxed())
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

    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{CloseHandle, ERROR_INVALID_PARAMETER};
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE, TerminateProcess,
        };

        let access = if signal == 0 {
            PROCESS_QUERY_LIMITED_INFORMATION
        } else {
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE
        };

        let process = unsafe { OpenProcess(access, 0, pid) };
        if process == 0 {
            let error = core_platform::last_error_code() as u32;
            let code = if error == ERROR_INVALID_PARAMETER {
                PlatformErrorCode::ProcessNotFound
            } else {
                PlatformErrorCode::ProcessPermissionDenied
            };
            return Err(RuntimeError::from(PlatformError::process_with(
                Some(code),
                Some(error.to_string()),
                None,
                None,
                Some("OpenProcess".to_string()),
                format!("failed to open process {pid}"),
            ))
            .boxed());
        }

        if signal == 0 {
            unsafe {
                CloseHandle(process);
            }
            return Ok(());
        }

        // terminate: map common unix signal ids onto terminate process
        if signal != 1 && signal != 2 && signal != 9 && signal != 15 {
            unsafe {
                CloseHandle(process);
            }
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "signal",
                format!("unsupported signal {signal} on windows"),
            ))
            .boxed());
        }

        let result = unsafe { TerminateProcess(process, 128_u32.saturating_add(signal)) };
        let status = if result == 0 {
            let error = core_platform::last_error_code();
            Err(RuntimeError::from(PlatformError::io(format!(
                "failed to terminate process {pid}: {error}",
            )))
            .boxed())
        } else {
            Ok(())
        };

        unsafe {
            CloseHandle(process);
        }

        status
    }

    #[cfg(not(any(unix, windows)))]
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

/// Replace the current process image with a command path.
pub fn process_exec_path(
    command: &str,
    arguments: &[String],
    environment: &[String],
) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let command = CString::new(command).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "command",
                "command contains nul byte",
            ))
            .boxed()
        })?;
        let (argument_values, argument_pointers) =
            build_exec_strings(arguments, command.as_c_str())?;
        let (environment_values, environment_pointers) = build_exec_environment(environment)?;

        let _keep_alive = (argument_values, environment_values);
        let result = unsafe {
            libc::execve(
                command.as_ptr(),
                argument_pointers.as_ptr(),
                environment_pointers.as_ptr(),
            )
        };
        if result != 0 {
            return Err(process_last_error(
                "execve",
                "failed to replace process image",
            ));
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = (command, arguments, environment);
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.exec.exec")).boxed())
    }
}

/// Replace the current process image using a directory-relative path.
pub fn process_exec_at(
    directory_fd: i32,
    path: &str,
    arguments: &[String],
    environment: &[String],
    flags: u32,
) -> RuntimeResult<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let path = CString::new(path).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path contains nul byte",
            ))
            .boxed()
        })?;
        let (argument_values, argument_pointers) = build_exec_strings(arguments, path.as_c_str())?;
        let (environment_values, environment_pointers) = build_exec_environment(environment)?;

        let _keep_alive = (argument_values, environment_values);
        let result = unsafe {
            libc::execveat(
                directory_fd,
                path.as_ptr(),
                argument_pointers.as_ptr(),
                environment_pointers.as_ptr(),
                flags as libc::c_int,
            )
        };
        if result != 0 {
            return Err(process_last_error(
                "execveat",
                "failed to replace process image from directory-relative path",
            ));
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (directory_fd, path, arguments, environment, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.exec.execat")).boxed())
    }
}

/// Replace the current process image using an executable file descriptor.
pub fn process_fexec(
    executable_fd: i32,
    arguments: &[String],
    environment: &[String],
) -> RuntimeResult<()> {
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
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
            build_exec_strings(arguments, executable_name.as_c_str())?;
        let (environment_values, environment_pointers) = build_exec_environment(environment)?;

        let _keep_alive = (argument_values, environment_values);
        let result = unsafe {
            libc::fexecve(
                executable_fd,
                argument_pointers.as_ptr(),
                environment_pointers.as_ptr(),
            )
        };
        if result != 0 {
            return Err(process_last_error(
                "fexecve",
                "failed to replace process image from executable descriptor",
            ));
        }

        Ok(())
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
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

/// Change the root directory for path resolution.
pub fn process_chroot(path: &str) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let path = CString::new(path).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path contains nul byte",
            ))
            .boxed()
        })?;
        let result = unsafe { libc::chroot(path.as_ptr()) };
        if result != 0 {
            return Err(process_last_error(
                "chroot",
                "failed to change root directory",
            ));
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = path;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.isolation.chroot",
        ))
        .boxed())
    }
}

/// Install one syscall filter program.
pub fn process_install_syscall_filter(program: &[u8], flags: u32) -> RuntimeResult<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let filter_size = std::mem::size_of::<libc::sock_filter>();
        if program.is_empty() || program.len() % filter_size != 0 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "program",
                "syscall filter program must be a non-empty sock_filter sequence",
            ))
            .boxed());
        }

        let mut filters = Vec::with_capacity(program.len() / filter_size);
        for chunk in program.chunks_exact(filter_size) {
            let filter =
                unsafe { std::ptr::read_unaligned(chunk.as_ptr() as *const libc::sock_filter) };
            filters.push(filter);
        }

        let mut program = libc::sock_fprog {
            len: filters.len() as u16,
            filter: filters.as_mut_ptr(),
        };

        let no_new_privs = unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
        if no_new_privs != 0 {
            return Err(process_last_error(
                "prctl(PR_SET_NO_NEW_PRIVS)",
                "failed to set no_new_privs before seccomp",
            ));
        }

        let result = unsafe {
            libc::syscall(
                libc::SYS_seccomp,
                libc::SECCOMP_SET_MODE_FILTER,
                flags as libc::c_ulong,
                &mut program as *mut libc::sock_fprog,
            )
        };
        if result != 0 {
            return Err(process_last_error(
                "seccomp",
                "failed to install syscall filter",
            ));
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (program, flags);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.isolation.installSyscallFilter",
        ))
        .boxed())
    }
}

/// Set process host name inside the active UTS namespace.
pub fn process_set_host_name(name: &str) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        if name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "name",
                "host name must not be empty",
            ))
            .boxed());
        }

        let bytes = name.as_bytes();
        let length = i32::try_from(bytes.len()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "name",
                "host name is too long",
            ))
            .boxed()
        })?;
        let result = unsafe { libc::sethostname(bytes.as_ptr() as *const libc::c_char, length) };
        if result != 0 {
            return Err(process_last_error("sethostname", "failed to set host name"));
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = name;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.isolation.setHostName",
        ))
        .boxed())
    }
}

/// Set network namespace context for subsequent network operations.
pub fn process_set_network_namespace(path: &str) -> RuntimeResult<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        process_setns_path(path, libc::CLONE_NEWNET)
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = path;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.isolation.setNetworkNamespace",
        ))
        .boxed())
    }
}

/// Enter one namespace owned by another process.
pub fn process_setns(pid: u32, namespace: ProcessNamespaceKind) -> RuntimeResult<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let name = namespace_name(namespace);
        let namespace_path = format!("/proc/{pid}/ns/{name}");
        let namespace_flag = namespace_flag(namespace);
        process_setns_path(&namespace_path, namespace_flag)
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (pid, namespace);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.isolation.setns",
        ))
        .boxed())
    }
}

/// Unshare one or more namespaces.
pub fn process_unshare(flags: u64) -> RuntimeResult<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let flags = i32::try_from(flags).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "flags",
                "unshare flags exceed supported width",
            ))
            .boxed()
        })?;
        let result = unsafe { libc::unshare(flags) };
        if result != 0 {
            return Err(process_last_error(
                "unshare",
                "failed to unshare namespaces",
            ));
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = flags;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.isolation.unshare",
        ))
        .boxed())
    }
}

/// Join one control group.
pub fn process_cgroup_join(path: &str) -> RuntimeResult<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        if path.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "cgroup path must not be empty",
            ))
            .boxed());
        }
        if path.contains('\0') {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "cgroup path contains nul byte",
            ))
            .boxed());
        }

        let pid = process_pid()?;
        let cgroup_procs_path = format!("{path}/cgroup.procs");
        std::fs::write(&cgroup_procs_path, format!("{pid}\n")).map_err(|error| {
            RuntimeError::from(PlatformError::io(format!(
                "failed to join cgroup at {cgroup_procs_path}: {error}",
            )))
            .boxed()
        })?;

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = path;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.group.cgroupJoin",
        ))
        .boxed())
    }
}

/// Read one control-group resource limit.
pub fn process_cgroup_get_limit(path: &str, resource: u32) -> RuntimeResult<ProcessLimit> {
    let _ = (path, resource);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.cgroupGetLimit",
    ))
    .boxed())
}

/// Write one control-group resource limit.
pub fn process_cgroup_set_limit(
    path: &str,
    resource: u32,
    limit: ProcessLimit,
) -> RuntimeResult<()> {
    let _ = (path, resource, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.cgroupSetLimit",
    ))
    .boxed())
}

/// Assign one or more process ids to one Windows job object.
pub fn process_job_assign(name: &str, pids: &[ProcessId]) -> RuntimeResult<()> {
    let _ = (name, pids);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.jobAssign",
    ))
    .boxed())
}

/// Set one Windows job object resource limit.
pub fn process_job_set_limit(name: &str, resource: u32, limit: ProcessLimit) -> RuntimeResult<()> {
    let _ = (name, resource, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.jobSetLimit",
    ))
    .boxed())
}

/// Wait for one process state transition with a timeout.
pub fn process_wait_pid_timeout(pid: u32, timeout_ns: u64) -> RuntimeResult<ProcessWaitStatus> {
    #[cfg(unix)]
    {
        if timeout_ns == 0 {
            return process_wait_pid(pid, PROCESS_WAIT_FLAG_NOHANG);
        }

        let deadline = Instant::now() + Duration::from_nanos(timeout_ns);
        loop {
            match process_wait_pid(pid, PROCESS_WAIT_FLAG_NOHANG) {
                Ok(status) => return Ok(status),
                Err(error) => {
                    if !is_would_block_error(&error) {
                        return Err(error);
                    }

                    if Instant::now() >= deadline {
                        return Err(error);
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[cfg(windows)]
    {
        const PROCESS_SYNCHRONIZE: u32 = 0x0010_0000;
        let flags = if timeout_ns == 0 {
            PROCESS_WAIT_FLAG_NOHANG
        } else {
            0
        };
        if flags != 0 {
            return process_wait_pid(pid, flags);
        }

        use windows_sys::Win32::Foundation::{
            CloseHandle, ERROR_INVALID_PARAMETER, STILL_ACTIVE, WAIT_FAILED, WAIT_OBJECT_0,
            WAIT_TIMEOUT,
        };
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, WaitForSingleObject,
        };

        let process = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                0,
                pid,
            )
        };
        if process == 0 {
            let error = core_platform::last_error_code() as u32;
            let code = if error == ERROR_INVALID_PARAMETER {
                PlatformErrorCode::ProcessNotFound
            } else {
                PlatformErrorCode::ProcessPermissionDenied
            };
            return Err(RuntimeError::from(PlatformError::process_with(
                Some(code),
                Some(error.to_string()),
                None,
                None,
                Some("OpenProcess".to_string()),
                format!("failed to open process {pid} for timed wait"),
            ))
            .boxed());
        }

        let timeout_millis = (timeout_ns / 1_000_000).max(1);
        let timeout_millis = timeout_millis.min(u32::MAX as u64) as u32;
        let wait_status = unsafe { WaitForSingleObject(process, timeout_millis) };
        let status = match wait_status {
            WAIT_TIMEOUT => Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoWouldBlock),
                None,
                None,
                Some("WaitForSingleObject".to_string()),
                None,
                format!("wait timed out for pid {pid}"),
            ))
            .boxed()),
            WAIT_OBJECT_0 => {
                let mut exit_code = 0_u32;
                let rc = unsafe { GetExitCodeProcess(process, &mut exit_code) };
                if rc == 0 {
                    let error = core_platform::last_error_code();
                    Err(RuntimeError::from(PlatformError::io(format!(
                        "failed to read process exit code: {error}",
                    )))
                    .boxed())
                } else if exit_code == STILL_ACTIVE as u32 {
                    Ok(ProcessWaitStatus {
                        pid: ProcessId(pid),
                        kind: ProcessWaitKind::Running,
                        exit_code: 0,
                        signal: Signal(0),
                        core_dumped: false,
                    })
                } else {
                    Ok(ProcessWaitStatus {
                        pid: ProcessId(pid),
                        kind: ProcessWaitKind::Exited,
                        exit_code: exit_code as i32,
                        signal: Signal(0),
                        core_dumped: false,
                    })
                }
            }
            WAIT_FAILED => {
                let error = core_platform::last_error_code();
                Err(RuntimeError::from(PlatformError::io(format!(
                    "wait failed for pid {pid}: {error}",
                )))
                .boxed())
            }
            _ => Err(
                RuntimeError::from(PlatformError::io("wait returned unexpected result")).boxed(),
            ),
        };

        unsafe {
            CloseHandle(process);
        }

        status
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = (pid, timeout_ns);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.waitPidTimeout",
        ))
        .boxed())
    }
}

/// Return true when a runtime error maps to `ioWouldBlock`.
pub fn is_would_block_error(error: &RuntimeError) -> bool {
    let Some(platform_error) = error.platform_error() else {
        return false;
    };

    platform_error.code == PlatformErrorCode::IoWouldBlock
}

/// Build a process-domain error from the current errno state.
#[cfg(unix)]
fn process_last_error(syscall: &str, message: impl Into<String>) -> Box<RuntimeError> {
    let errno = std::io::Error::last_os_error()
        .raw_os_error()
        .unwrap_or(libc::EINVAL);
    process_errno_error(errno, syscall, message)
}

/// Build a process-domain error from a specific errno value.
#[cfg(unix)]
fn process_errno_error(errno: i32, syscall: &str, message: impl Into<String>) -> Box<RuntimeError> {
    let message = message.into();
    let mapped = match errno {
        libc::EINTR => PlatformErrorCode::IoInterrupted,
        value if value == libc::EAGAIN || value == libc::EWOULDBLOCK => {
            PlatformErrorCode::IoWouldBlock
        }
        _ => process_error_code_from_errno(errno).unwrap_or(PlatformErrorCode::Process),
    };

    if mapped == PlatformErrorCode::IoWouldBlock || mapped == PlatformErrorCode::IoInterrupted {
        return RuntimeError::from(PlatformError::io_with(
            Some(mapped),
            None,
            Some(errno),
            Some(syscall.to_string()),
            None,
            message,
        ))
        .boxed();
    }

    RuntimeError::from(PlatformError::process_with(
        Some(mapped),
        Some(errno.to_string()),
        None,
        None,
        Some(syscall.to_string()),
        message,
    ))
    .boxed()
}

/// Return a mutable pointer to the host errno slot.
#[cfg(all(unix, any(target_os = "linux", target_os = "android")))]
unsafe fn errno_location() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}

/// Return a mutable pointer to the host errno slot.
#[cfg(all(
    unix,
    any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd"
    )
))]
unsafe fn errno_location() -> *mut libc::c_int {
    unsafe { libc::__error() }
}

/// Build C string vectors and pointer arrays for exec arguments.
#[cfg(unix)]
fn build_exec_strings(
    arguments: &[String],
    fallback_argv0: &CStr,
) -> RuntimeResult<(Vec<CString>, Vec<*const libc::c_char>)> {
    let mut values = Vec::new();
    if arguments.is_empty() {
        values.push(fallback_argv0.to_owned());
    } else {
        for argument in arguments {
            let argument = CString::new(argument.as_str()).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "arguments",
                    "argument contains nul byte",
                ))
                .boxed()
            })?;
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
#[cfg(unix)]
fn build_exec_environment(
    environment: &[String],
) -> RuntimeResult<(Vec<CString>, Vec<*const libc::c_char>)> {
    let mut values = Vec::with_capacity(environment.len());
    for entry in environment {
        let entry = CString::new(entry.as_str()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "environment",
                "environment entry contains nul byte",
            ))
            .boxed()
        })?;
        values.push(entry);
    }

    let mut pointers = Vec::with_capacity(values.len() + 1);
    for value in &values {
        pointers.push(value.as_ptr());
    }
    pointers.push(std::ptr::null());

    Ok((values, pointers))
}

/// Apply setns using one namespace descriptor path.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn process_setns_path(path: &str, namespace_flag: i32) -> RuntimeResult<()> {
    let path = CString::new(path).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "path contains nul byte",
        ))
        .boxed()
    })?;

    let descriptor = unsafe { libc::open(path.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC) };
    if descriptor < 0 {
        return Err(process_last_error(
            "open",
            "failed to open namespace descriptor",
        ));
    }

    let setns_result = unsafe { libc::setns(descriptor, namespace_flag) };
    let close_result = unsafe { libc::close(descriptor) };
    if close_result != 0 {
        return Err(process_last_error(
            "close",
            "failed to close namespace descriptor",
        ));
    }

    if setns_result != 0 {
        return Err(process_last_error("setns", "failed to switch namespace"));
    }

    Ok(())
}

/// Map namespace kind to procfs namespace name.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn namespace_name(namespace: ProcessNamespaceKind) -> &'static str {
    match namespace {
        ProcessNamespaceKind::Mount => "mnt",
        ProcessNamespaceKind::User => "user",
        ProcessNamespaceKind::Pid => "pid",
        ProcessNamespaceKind::Network => "net",
        ProcessNamespaceKind::Ipc => "ipc",
        ProcessNamespaceKind::Uts => "uts",
        ProcessNamespaceKind::Cgroup => "cgroup",
        ProcessNamespaceKind::Time => "time",
    }
}

/// Map namespace kind to setns clone flag.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn namespace_flag(namespace: ProcessNamespaceKind) -> i32 {
    match namespace {
        ProcessNamespaceKind::Mount => libc::CLONE_NEWNS,
        ProcessNamespaceKind::User => libc::CLONE_NEWUSER,
        ProcessNamespaceKind::Pid => libc::CLONE_NEWPID,
        ProcessNamespaceKind::Network => libc::CLONE_NEWNET,
        ProcessNamespaceKind::Ipc => libc::CLONE_NEWIPC,
        ProcessNamespaceKind::Uts => libc::CLONE_NEWUTS,
        ProcessNamespaceKind::Cgroup => libc::CLONE_NEWCGROUP,
        ProcessNamespaceKind::Time => libc::CLONE_NEWTIME,
    }
}

/// Decode public wait flags into host waitpid flags.
#[cfg(unix)]
fn decode_wait_flags(flags: u32) -> RuntimeResult<libc::c_int> {
    // only allow host waitpid flags for now
    let supported = (libc::WNOHANG | libc::WUNTRACED | libc::WCONTINUED) as u32;
    if flags & !supported != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "unsupported process wait flags",
        ))
        .boxed());
    }

    Ok(flags as libc::c_int)
}

/// Build a process wait error from the current errno state.
#[cfg(unix)]
fn waitpid_error(pid: u32, flags: u32) -> Box<RuntimeError> {
    // read errno from the last syscall
    let error = std::io::Error::last_os_error();
    let errno = error.raw_os_error().unwrap_or(libc::EINVAL);

    // map errno to runtime process codes
    let code = match errno {
        libc::ECHILD => PlatformErrorCode::ProcessNotFound,
        libc::EACCES | libc::EPERM => PlatformErrorCode::ProcessPermissionDenied,
        libc::EINTR => PlatformErrorCode::IoInterrupted,
        value if value == libc::EAGAIN || value == libc::EWOULDBLOCK => {
            PlatformErrorCode::IoWouldBlock
        }
        _ => PlatformErrorCode::ProcessWaitFailed,
    };

    // build io style errors for transient io categories
    if code == PlatformErrorCode::IoInterrupted || code == PlatformErrorCode::IoWouldBlock {
        return RuntimeError::from(PlatformError::io_with(
            Some(code),
            None,
            Some(errno),
            Some("waitpid".to_string()),
            None,
            format!("waitpid failed for pid {pid} with flags {flags}"),
        ))
        .boxed();
    }

    // build process domain errors for process categories
    RuntimeError::from(PlatformError::process_with(
        Some(code),
        Some(errno.to_string()),
        None,
        None,
        Some("waitpid".to_string()),
        format!("waitpid failed for pid {pid} with flags {flags}"),
    ))
    .boxed()
}

/// Decode a host wait status into the platform wait payload.
#[cfg(unix)]
fn wait_status_from_raw(waited: libc::pid_t, raw_status: libc::c_int) -> ProcessWaitStatus {
    // exited child
    if libc::WIFEXITED(raw_status) {
        return ProcessWaitStatus {
            pid: ProcessId(waited as u32),
            kind: ProcessWaitKind::Exited,
            exit_code: libc::WEXITSTATUS(raw_status),
            signal: Signal(0),
            core_dumped: false,
        };
    }

    // signaled child
    if libc::WIFSIGNALED(raw_status) {
        return ProcessWaitStatus {
            pid: ProcessId(waited as u32),
            kind: ProcessWaitKind::Signaled,
            exit_code: 0,
            signal: Signal(libc::WTERMSIG(raw_status) as u32),
            core_dumped: libc::WCOREDUMP(raw_status),
        };
    }

    // stopped child
    if libc::WIFSTOPPED(raw_status) {
        return ProcessWaitStatus {
            pid: ProcessId(waited as u32),
            kind: ProcessWaitKind::Stopped,
            exit_code: 0,
            signal: Signal(libc::WSTOPSIG(raw_status) as u32),
            core_dumped: false,
        };
    }

    // continued child
    if libc::WIFCONTINUED(raw_status) {
        return ProcessWaitStatus {
            pid: ProcessId(waited as u32),
            kind: ProcessWaitKind::Continued,
            exit_code: 0,
            signal: Signal(0),
            core_dumped: false,
        };
    }

    // fallback for unknown status payloads
    ProcessWaitStatus {
        pid: ProcessId(waited as u32),
        kind: ProcessWaitKind::Running,
        exit_code: 0,
        signal: Signal(0),
        core_dumped: false,
    }
}

/// Read the current thread signal mask.
pub fn process_signal_mask_read() -> RuntimeResult<Vec<Signal>> {
    #[cfg(unix)]
    {
        // read the current signal mask
        let mut current_mask = unsafe { std::mem::zeroed::<libc::sigset_t>() };
        let read_result = unsafe {
            libc::pthread_sigmask(libc::SIG_SETMASK, std::ptr::null(), &mut current_mask)
        };
        if read_result != 0 {
            return Err(signal_error_from_code(read_result, "pthread_sigmask(read)"));
        }

        // scan the mask for known signal ids
        let mut signals = Vec::new();
        for signal_number in 1..=MAX_SIGNAL_SCAN {
            let is_member =
                unsafe { libc::sigismember(&current_mask, signal_number as libc::c_int) };
            if is_member == 1 {
                signals.push(Signal(signal_number));
            }
        }

        Ok(signals)
    }

    #[cfg(not(unix))]
    {
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.signalMaskRead",
        ))
        .boxed())
    }
}

/// Update the current thread signal mask.
pub fn process_signal_mask_update(how: SignalMaskHow, signals: &[Signal]) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        // resolve the mask operation kind
        let operation = match how {
            SignalMaskHow::Set => libc::SIG_SETMASK,
            SignalMaskHow::Block => libc::SIG_BLOCK,
            SignalMaskHow::Unblock => libc::SIG_UNBLOCK,
        };

        // build the host signal set payload
        let signal_set = signal_set_from_slice(signals)?;

        // apply the mask update
        let update_result =
            unsafe { libc::pthread_sigmask(operation, &signal_set, std::ptr::null_mut()) };
        if update_result != 0 {
            return Err(signal_error_from_code(
                update_result,
                "pthread_sigmask(update)",
            ));
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = (how, signals);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.signalMaskUpdate",
        ))
        .boxed())
    }
}

/// Wait for one signal from the provided set.
pub fn process_signal_wait(signals: &[Signal]) -> RuntimeResult<SignalEvent> {
    #[cfg(unix)]
    {
        // build the host signal set payload
        let signal_set = signal_set_from_slice(signals)?;

        // wait for one signal from the set
        let mut signal_value: libc::c_int = 0;
        let wait_result = unsafe { libc::sigwait(&signal_set, &mut signal_value) };
        if wait_result != 0 {
            return Err(signal_error_from_code(wait_result, "sigwait"));
        }

        Ok(SignalEvent {
            signal: Signal(signal_value as u32),
            pid: ProcessId(0),
        })
    }

    #[cfg(not(unix))]
    {
        let _ = signals;
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.signalWait")).boxed())
    }
}

/// Poll for one signal from the provided set without blocking.
pub fn process_signal_try_wait(signals: &[Signal]) -> RuntimeResult<SignalEvent> {
    #[cfg(unix)]
    {
        // collect pending signals from the process
        let mut pending_mask = unsafe { std::mem::zeroed::<libc::sigset_t>() };
        let pending_result = unsafe { libc::sigpending(&mut pending_mask) };
        if pending_result != 0 {
            let errno = std::io::Error::last_os_error()
                .raw_os_error()
                .unwrap_or(libc::EINVAL);
            return Err(signal_errno_error(errno, "sigpending"));
        }

        // find the first requested pending signal
        let mut pending_signal = None;
        for signal in signals {
            let is_member = unsafe { libc::sigismember(&pending_mask, signal.0 as libc::c_int) };
            if is_member == 1 {
                pending_signal = Some(*signal);
                break;
            }
        }

        // return would-block when no requested signal is pending
        let Some(signal) = pending_signal else {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoWouldBlock),
                None,
                Some(libc::EWOULDBLOCK),
                Some("sigpending".to_string()),
                None,
                "no requested signal is pending",
            ))
            .boxed());
        };

        // consume the pending signal with sigwait
        let single_set = signal_set_from_slice(&[signal])?;
        let mut consumed_value: libc::c_int = 0;
        let consume_result = unsafe { libc::sigwait(&single_set, &mut consumed_value) };
        if consume_result != 0 {
            return Err(signal_error_from_code(consume_result, "sigwait"));
        }

        Ok(SignalEvent {
            signal: Signal(consumed_value as u32),
            pid: ProcessId(0),
        })
    }

    #[cfg(not(unix))]
    {
        let _ = signals;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.signalTryWait",
        ))
        .boxed())
    }
}

/// Maximum signal number scanned when materializing a signal mask.
#[cfg(unix)]
const MAX_SIGNAL_SCAN: u32 = 128;

/// Build a host signal set from runtime signal numbers.
#[cfg(unix)]
fn signal_set_from_slice(signals: &[Signal]) -> RuntimeResult<libc::sigset_t> {
    // initialize an empty signal set
    let mut signal_set = unsafe { std::mem::zeroed::<libc::sigset_t>() };
    let empty_result = unsafe { libc::sigemptyset(&mut signal_set) };
    if empty_result != 0 {
        let errno = std::io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::EINVAL);
        return Err(signal_errno_error(errno, "sigemptyset"));
    }

    // add each signal to the set with validation
    for signal in signals {
        if signal.0 == 0 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "signals",
                "signal id 0 is invalid",
            ))
            .boxed());
        }

        let add_result = unsafe { libc::sigaddset(&mut signal_set, signal.0 as libc::c_int) };
        if add_result != 0 {
            let errno = std::io::Error::last_os_error()
                .raw_os_error()
                .unwrap_or(libc::EINVAL);
            return Err(signal_errno_error(errno, "sigaddset"));
        }
    }

    Ok(signal_set)
}

/// Build an error from a pthread style error code.
#[cfg(unix)]
fn signal_error_from_code(code: i32, syscall: &str) -> Box<RuntimeError> {
    // map known process and io categories
    let mapped = match code {
        libc::EACCES | libc::EPERM => PlatformErrorCode::ProcessPermissionDenied,
        libc::EINTR => PlatformErrorCode::IoInterrupted,
        value if value == libc::EAGAIN || value == libc::EWOULDBLOCK => {
            PlatformErrorCode::IoWouldBlock
        }
        libc::EINVAL => PlatformErrorCode::InvalidArgumentValue,
        _ => PlatformErrorCode::Process,
    };

    // build argument-domain validation errors
    if mapped == PlatformErrorCode::InvalidArgumentValue {
        return RuntimeError::from(PlatformError::invalid_argument_value(
            "signals",
            format!("{syscall} failed with EINVAL"),
        ))
        .boxed();
    }

    // build io-style categories
    if mapped == PlatformErrorCode::IoInterrupted || mapped == PlatformErrorCode::IoWouldBlock {
        return RuntimeError::from(PlatformError::io_with(
            Some(mapped),
            None,
            Some(code),
            Some(syscall.to_string()),
            None,
            format!("{syscall} failed with errno {code}"),
        ))
        .boxed();
    }

    // build process-domain categories
    RuntimeError::from(PlatformError::process_with(
        Some(mapped),
        Some(code.to_string()),
        None,
        None,
        Some(syscall.to_string()),
        format!("{syscall} failed with errno {code}"),
    ))
    .boxed()
}

/// Build an error from a syscall errno value.
#[cfg(unix)]
fn signal_errno_error(errno: i32, syscall: &str) -> Box<RuntimeError> {
    signal_error_from_code(errno, syscall)
}

/// Map a Windows process priority class to a unix-style nice value.
#[cfg(windows)]
fn windows_priority_class_to_nice(class: u32) -> RuntimeResult<i32> {
    use windows_sys::Win32::System::Threading::{
        ABOVE_NORMAL_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS,
        IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, REALTIME_PRIORITY_CLASS,
    };

    if class == REALTIME_PRIORITY_CLASS {
        return Ok(-20);
    }
    if class == HIGH_PRIORITY_CLASS {
        return Ok(-10);
    }
    if class == ABOVE_NORMAL_PRIORITY_CLASS {
        return Ok(-5);
    }
    if class == NORMAL_PRIORITY_CLASS {
        return Ok(0);
    }
    if class == BELOW_NORMAL_PRIORITY_CLASS {
        return Ok(10);
    }
    if class == IDLE_PRIORITY_CLASS {
        return Ok(19);
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "priorityClass",
        format!("unsupported priority class {class}"),
    ))
    .boxed())
}

/// Map a unix-style nice value to a Windows process priority class.
#[cfg(windows)]
fn windows_nice_to_priority_class(priority: i32) -> u32 {
    use windows_sys::Win32::System::Threading::{
        ABOVE_NORMAL_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS,
        IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, REALTIME_PRIORITY_CLASS,
    };

    if priority <= -15 {
        return REALTIME_PRIORITY_CLASS;
    }
    if priority <= -8 {
        return HIGH_PRIORITY_CLASS;
    }
    if priority <= -3 {
        return ABOVE_NORMAL_PRIORITY_CLASS;
    }
    if priority <= 4 {
        return NORMAL_PRIORITY_CLASS;
    }
    if priority <= 10 {
        return BELOW_NORMAL_PRIORITY_CLASS;
    }

    IDLE_PRIORITY_CLASS
}
