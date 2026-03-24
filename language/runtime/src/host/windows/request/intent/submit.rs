use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_NO_ASSOCIATION};
use windows_sys::Win32::UI::Shell::{ASSOCSTR_EXECUTABLE, AssocQueryStringW, ShellExecuteW};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{
    io_not_found, io_operation_error, not_supported, wide_from_str, wide_from_utf16,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs;
use crate::platform::fs::core as core_fs;

/// Win32 shell-execute return code for path-not-found.
const SHELL_EXECUTE_PATH_NOT_FOUND: isize = 3;
/// Win32 shell-execute return code for access-denied.
const SHELL_EXECUTE_ACCESS_DENIED: isize = 5;
/// Win32 shell-execute return code for out-of-memory.
const SHELL_EXECUTE_OUT_OF_MEMORY: isize = 8;
/// Win32 shell-execute return code for sharing violation.
const SHELL_EXECUTE_SHARING_VIOLATION: isize = 26;
/// Win32 shell-execute return code for no association.
const SHELL_EXECUTE_NO_ASSOCIATION: isize = 31;
/// Win32 shell-execute return code for DLL not found.
const SHELL_EXECUTE_DLL_NOT_FOUND: isize = 32;
/// Canonical operation name for `intentCanOpenUrl`.
const HOST_INTENT_CAN_OPEN_URL_OPERATION: &str = "destack.os.intent.canOpenUrl";
/// Canonical operation name for `intentOpenPath`.
const HOST_INTENT_OPEN_PATH_OPERATION: &str = "destack.os.intent.openPath";
/// Canonical operation name for `intentOpenUrl`.
const HOST_INTENT_OPEN_URL_OPERATION: &str = "destack.os.intent.openUrl";

/// Query whether the host has an association for one URL scheme.
pub(crate) fn windows_can_open_url(url: &str) -> RuntimeResult<bool> {
    let scheme = url.split_once(':').map(|(scheme, _)| scheme).unwrap_or(url);
    let scheme = format!("{scheme}:");
    let scheme = wide_from_str("url", &scheme)?;
    let mut length = 0u32;
    let status = unsafe {
        AssocQueryStringW(
            0,
            ASSOCSTR_EXECUTABLE,
            scheme.as_ptr(),
            std::ptr::null(),
            std::ptr::null_mut(),
            &mut length,
        )
    };

    if status == ERROR_NO_ASSOCIATION as i32 || status == ERROR_FILE_NOT_FOUND as i32 {
        return Ok(false);
    }

    if status != 0 {
        return Err(io_operation_error(
            HOST_INTENT_CAN_OPEN_URL_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            format!("AssocQueryStringW failed with code {status}"),
        ));
    }

    Ok(true)
}

/// Route one URL open request through the Windows host.
pub(crate) fn windows_open_url(url: &str) -> RuntimeResult<()> {
    windows_open_target(url, HOST_INTENT_OPEN_URL_OPERATION)
}

/// Open one path target through ShellExecuteW.
pub(crate) fn windows_open_path_target(path: fs::OsPath) -> RuntimeResult<()> {
    let target = wide_from_os_path(path)?;

    windows_open_target_wide(&target, HOST_INTENT_OPEN_PATH_OPERATION)
}

/// Open one target string through ShellExecuteW.
fn windows_open_target(target: &str, operation: &'static str) -> RuntimeResult<()> {
    let target = wide_from_str("target", target)?;

    windows_open_target_wide(&target, operation)
}

/// Open one UTF-16 target through ShellExecuteW.
fn windows_open_target_wide(target_wide: &[u16], operation: &'static str) -> RuntimeResult<()> {
    let verb = wide_from_str("verb", "open")?;
    let result = unsafe {
        ShellExecuteW(
            0,
            verb.as_ptr(),
            target_wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };

    if result as usize > 32 {
        return Ok(());
    }

    let error_code = result as isize;
    let platform_error = match error_code {
        SHELL_EXECUTE_PATH_NOT_FOUND => io_not_found(operation, "target was not found"),
        SHELL_EXECUTE_ACCESS_DENIED => io_operation_error(
            operation,
            Some(PlatformErrorCode::IoPermissionDenied),
            "host platform denied the open request",
        ),
        SHELL_EXECUTE_OUT_OF_MEMORY | SHELL_EXECUTE_SHARING_VIOLATION => io_operation_error(
            operation,
            Some(PlatformErrorCode::IoWouldBlock),
            format!("host platform rejected the open request with status {error_code}"),
        ),
        SHELL_EXECUTE_NO_ASSOCIATION | SHELL_EXECUTE_DLL_NOT_FOUND => not_supported(operation),
        _ => io_operation_error(
            operation,
            None,
            format!("ShellExecuteW failed with status {error_code}"),
        ),
    };

    Err(platform_error)
}

/// Convert one `OsPath` payload into a nul-terminated wide path.
fn wide_from_os_path(path: fs::OsPath) -> RuntimeResult<Vec<u16>> {
    match path {
        fs::OsPath::OsPathUtf16(path_utf16) => {
            let units = unsafe { path_utf16.utf16.0.as_slice()? };

            wide_from_utf16("path", units)
        }
        fs::OsPath::OsPathBytes(_) => {
            let path = core_fs::os_path_to_utf8_string(path, "path")?;

            wide_from_str("path", &path)
        }
    }
}
