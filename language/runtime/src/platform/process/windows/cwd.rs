#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{PlatformError, core as core_platform};

use crate::runtime::BindingCallContext;

use crate::platform::fs;
use crate::platform::fs::core as core_fs;

/// Change the current working directory.
pub(crate) unsafe fn destack_process_chdir(
    _binding: &BindingCallContext,
    path: fs::OsPath,
) -> RuntimeResult<()> {
    use windows_sys::Win32::System::Environment::SetCurrentDirectoryW;

    let path = core_fs::os_path_to_utf8_string(path, "path")?;
    if path.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "path contains nul byte",
        ))
        .boxed());
    }

    let mut wide: Vec<u16> = path.encode_utf16().collect();
    wide.push(0);

    let rc = unsafe { SetCurrentDirectoryW(wide.as_ptr()) };
    if rc == 0 {
        let error = core_platform::last_error_code();
        return Err(RuntimeError::from(PlatformError::io(format!(
            "failed to change cwd: {error}",
        )))
        .boxed());
    }

    Ok(())
}

/// Return the current working directory.
pub(crate) unsafe fn destack_process_cwd(
    binding: &BindingCallContext,
    out: *mut fs::OsPath,
) -> RuntimeResult<()> {
    use windows_sys::Win32::System::Environment::GetCurrentDirectoryW;

    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let required = unsafe { GetCurrentDirectoryW(0, std::ptr::null_mut()) };
    if required == 0 {
        let error = core_platform::last_error_code();
        return Err(
            RuntimeError::from(PlatformError::io(format!("failed to read cwd: {error}",))).boxed(),
        );
    }

    let mut buffer = vec![0u16; required as usize + 1];
    let length = unsafe { GetCurrentDirectoryW(buffer.len() as u32, buffer.as_mut_ptr()) };
    if length == 0 {
        let error = core_platform::last_error_code();
        return Err(
            RuntimeError::from(PlatformError::io(format!("failed to read cwd: {error}",))).boxed(),
        );
    }

    let cwd = String::from_utf16_lossy(&buffer[..length as usize]);
    let path = core_fs::os_path_from_utf8_string(binding, cwd);

    unsafe {
        *out = path;
    }

    Ok(())
}
