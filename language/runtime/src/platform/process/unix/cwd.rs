#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

use crate::runtime::BindingCallContext;

use crate::platform::fs;
use crate::platform::fs::core as core_fs;
use std::ffi::{CStr, CString};

/// Change the current working directory.
pub(crate) unsafe fn destack_process_chdir(
    _binding: &BindingCallContext,
    path: fs::OsPath,
) -> RuntimeResult<()> {
    let path = core_fs::os_path_to_utf8_string(path, "path")?;

    if path.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "path contains nul byte",
        ))
        .boxed());
    }

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

/// Return the current working directory.
pub(crate) unsafe fn destack_process_cwd(
    binding: &BindingCallContext,
    out: *mut fs::OsPath,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let cwd = process_cwd()?;

    let path = core_fs::os_path_from_utf8_string(binding, cwd);

    unsafe {
        *out = path;
    }

    Ok(())
}

/// Read the current working directory as a UTF-8 string.
pub(crate) fn process_cwd() -> RuntimeResult<String> {
    let raw_cwd = unsafe { libc::getcwd(std::ptr::null_mut(), 0) };
    if raw_cwd.is_null() {
        return Err(RuntimeError::from(PlatformError::io("failed to read cwd")).boxed());
    }

    let cwd = unsafe { CStr::from_ptr(raw_cwd) };
    let cwd = String::from_utf8_lossy(cwd.to_bytes()).to_string();
    unsafe {
        libc::free(raw_cwd as *mut libc::c_void);
    }

    Ok(cwd)
}
