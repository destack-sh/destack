#[cfg(test)]
use std::sync::{Mutex, OnceLock};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::core::{io_operation_error, io_would_block, not_supported};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::PowerState;
use crate::platform::os::power::core::{OS_POWER_STATE_OPERATION, OS_POWER_SUSPEND_OPERATION};
use crate::runtime::BindingCallContext;

#[cfg(not(test))]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, SetLastError};
use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_BUSY, ERROR_CALL_NOT_IMPLEMENTED, ERROR_NOT_ALL_ASSIGNED,
    ERROR_NOT_SUPPORTED, ERROR_PRIVILEGE_NOT_HELD,
};
#[cfg(not(test))]
use windows_sys::Win32::Security::{
    AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED,
    SE_SHUTDOWN_NAME, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
#[cfg(not(test))]
use windows_sys::Win32::System::Power::SetSuspendState;
use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
#[cfg(not(test))]
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

use crate::platform::core as core_platform;

/// RAII handle guard for Win32 handles.
#[cfg(not(test))]
struct HandleGuard {
    /// Native handle value.
    handle: HANDLE,
}

#[cfg(not(test))]
impl Drop for HandleGuard {
    /// Close the guarded native handle.
    fn drop(&mut self) {
        if self.handle != 0 {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }
}

/// Shared suspend hook used by windows tests.
#[cfg(test)]
type WindowsSuspendHook = fn() -> RuntimeResult<()>;

/// Return the shared windows suspend hook slot for tests.
#[cfg(test)]
fn windows_suspend_hook_slot() -> &'static Mutex<Option<WindowsSuspendHook>> {
    static HOOK: OnceLock<Mutex<Option<WindowsSuspendHook>>> = OnceLock::new();

    HOOK.get_or_init(|| Mutex::new(None))
}

/// Install one windows suspend hook for tests.
#[cfg(test)]
pub(crate) fn set_test_suspend_hook(hook: Option<WindowsSuspendHook>) {
    let mut slot = windows_suspend_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot = hook;
}

/// Resolve the active windows suspend hook for tests.
#[cfg(test)]
fn require_test_suspend_hook() -> RuntimeResult<WindowsSuspendHook> {
    let hook = windows_suspend_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .to_owned();

    let Some(hook) = hook else {
        return Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            "power suspend tests must install a suspend hook before calling suspend",
        ))
        .boxed());
    };

    Ok(hook)
}

/// Open one access token for the current process.
#[cfg(not(test))]
fn open_process_token() -> RuntimeResult<HandleGuard> {
    // open one process token for temporary privilege adjustment
    let process = unsafe { GetCurrentProcess() };
    let mut token = 0;
    let status =
        unsafe { OpenProcessToken(process, TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token) };
    if status == 0 {
        return Err(windows_suspend_error(
            "OpenProcessToken",
            core_platform::last_error_code() as u32,
        ));
    }

    Ok(HandleGuard { handle: token })
}

/// Enable SeShutdownPrivilege on one process token and return the previous token state.
#[cfg(not(test))]
fn enable_shutdown_privilege(token: HANDLE) -> RuntimeResult<TOKEN_PRIVILEGES> {
    // resolve one privilege identifier for suspend requests
    let mut luid = unsafe { std::mem::zeroed() };
    let status = unsafe { LookupPrivilegeValueW(std::ptr::null(), SE_SHUTDOWN_NAME, &mut luid) };
    if status == 0 {
        return Err(windows_suspend_error(
            "LookupPrivilegeValueW",
            core_platform::last_error_code() as u32,
        ));
    }

    // enable the shutdown privilege and capture the previous state
    let new_state = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES {
            Luid: luid,
            Attributes: SE_PRIVILEGE_ENABLED,
        }],
    };
    let mut previous_state = unsafe { std::mem::zeroed::<TOKEN_PRIVILEGES>() };
    let mut previous_state_size = std::mem::size_of::<TOKEN_PRIVILEGES>() as u32;

    unsafe {
        SetLastError(0);
    }

    let status = unsafe {
        AdjustTokenPrivileges(
            token,
            0,
            &new_state,
            previous_state_size,
            &mut previous_state,
            &mut previous_state_size,
        )
    };
    let error = core_platform::last_error_code() as u32;
    if status == 0 {
        return Err(windows_suspend_error("AdjustTokenPrivileges", error));
    }

    if error == ERROR_NOT_ALL_ASSIGNED {
        return Err(windows_suspend_error("AdjustTokenPrivileges", error));
    }

    Ok(previous_state)
}

/// Restore one previously saved token privilege state.
#[cfg(not(test))]
fn restore_shutdown_privilege(token: HANDLE, previous_state: &TOKEN_PRIVILEGES) {
    unsafe {
        AdjustTokenPrivileges(
            token,
            0,
            previous_state,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
    }
}

/// Request one windows suspend transition.
#[cfg(not(test))]
fn request_windows_suspend_call() -> bool {
    unsafe { SetSuspendState(0, 0, 0) != 0 }
}

/// Map one windows suspend error into one runtime error.
fn windows_suspend_error(syscall: &str, error: u32) -> Box<RuntimeError> {
    // unsupported host capability
    if matches!(error, ERROR_NOT_SUPPORTED | ERROR_CALL_NOT_IMPLEMENTED) {
        return not_supported(OS_POWER_SUSPEND_OPERATION);
    }

    // privilege and policy denial
    if matches!(
        error,
        ERROR_ACCESS_DENIED | ERROR_PRIVILEGE_NOT_HELD | ERROR_NOT_ALL_ASSIGNED
    ) {
        return io_operation_error(
            OS_POWER_SUSPEND_OPERATION,
            Some(PlatformErrorCode::IoPermissionDenied),
            format!("{syscall} failed with windows error {error}"),
        );
    }

    // transient host busy state
    if error == ERROR_BUSY {
        return io_would_block(
            OS_POWER_SUSPEND_OPERATION,
            format!("{syscall} failed with windows error {error}"),
        );
    }

    io_operation_error(
        OS_POWER_SUSPEND_OPERATION,
        None,
        format!("{syscall} failed with windows error {error}"),
    )
}

/// Read one host power-state value from windows APIs.
pub(crate) fn read_power_state(_binding: &BindingCallContext) -> RuntimeResult<PowerState> {
    // query host power-status payload
    let mut status = unsafe { std::mem::zeroed::<SYSTEM_POWER_STATUS>() };
    let result = unsafe { GetSystemPowerStatus(&mut status) };
    if result == 0 {
        return Err(core_platform::io_operation_error(
            OS_POWER_STATE_OPERATION,
            None,
            "GetSystemPowerStatus failed",
        ));
    }

    // map windows ac-line status into runtime power-state enum
    let state = match status.ACLineStatus {
        0 => PowerState::Battery,
        1 => PowerState::AC,
        _ => PowerState::Unknown,
    };

    Ok(state)
}

/// Request one host suspend transition through windows power-management APIs.
pub(crate) fn request_suspend(_binding: &BindingCallContext) -> RuntimeResult<()> {
    #[cfg(test)]
    {
        let hook = require_test_suspend_hook()?;

        return hook();
    }

    #[cfg(not(test))]
    // enable the required shutdown privilege for this request
    #[cfg(not(test))]
    let token = open_process_token()?;
    #[cfg(not(test))]
    let previous_state = enable_shutdown_privilege(token.handle)?;

    #[cfg(not(test))]
    // request one polite host suspend and restore the previous privilege state
    #[cfg(not(test))]
    let result = request_windows_suspend_call();
    #[cfg(not(test))]
    restore_shutdown_privilege(token.handle, &previous_state);

    #[cfg(not(test))]
    if result {
        return Ok(());
    }

    #[cfg(not(test))]
    Err(windows_suspend_error(
        "SetSuspendState",
        core_platform::last_error_code() as u32,
    ))
}

#[cfg(test)]
mod tests {
    use crate::platform::diagnostic::PlatformErrorCode;
    use crate::tests::platform::error_code_from_result;

    use super::windows_suspend_error;
    use windows_sys::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_BUSY, ERROR_CALL_NOT_IMPLEMENTED, ERROR_NOT_ALL_ASSIGNED,
        ERROR_NOT_SUPPORTED, ERROR_PRIVILEGE_NOT_HELD,
    };

    #[test]
    fn test_windows_suspend_error_maps_not_supported() {
        let not_supported_error = error_code_from_result::<()>(Err(windows_suspend_error(
            "SetSuspendState",
            ERROR_NOT_SUPPORTED,
        )))
        .expect("not-supported error should decode");
        let call_not_implemented = error_code_from_result::<()>(Err(windows_suspend_error(
            "SetSuspendState",
            ERROR_CALL_NOT_IMPLEMENTED,
        )))
        .expect("call-not-implemented error should decode");

        assert_eq!(not_supported_error, PlatformErrorCode::NotSupported);
        assert_eq!(call_not_implemented, PlatformErrorCode::NotSupported);
    }

    #[test]
    fn test_windows_suspend_error_maps_permission_denied() {
        let access_denied = error_code_from_result::<()>(Err(windows_suspend_error(
            "SetSuspendState",
            ERROR_ACCESS_DENIED,
        )))
        .expect("access-denied error should decode");
        let privilege_not_held = error_code_from_result::<()>(Err(windows_suspend_error(
            "SetSuspendState",
            ERROR_PRIVILEGE_NOT_HELD,
        )))
        .expect("privilege-not-held error should decode");
        let not_all_assigned = error_code_from_result::<()>(Err(windows_suspend_error(
            "AdjustTokenPrivileges",
            ERROR_NOT_ALL_ASSIGNED,
        )))
        .expect("not-all-assigned error should decode");

        assert_eq!(access_denied, PlatformErrorCode::IoPermissionDenied);
        assert_eq!(privilege_not_held, PlatformErrorCode::IoPermissionDenied);
        assert_eq!(not_all_assigned, PlatformErrorCode::IoPermissionDenied);
    }

    #[test]
    fn test_windows_suspend_error_maps_would_block() {
        let busy =
            error_code_from_result::<()>(Err(windows_suspend_error("SetSuspendState", ERROR_BUSY)))
                .expect("busy error should decode");

        assert_eq!(busy, PlatformErrorCode::IoWouldBlock);
    }
}
