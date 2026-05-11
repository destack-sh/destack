#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::process::core as core_process;
use crate::platform::{NativeArray, PlatformError, core as core_platform};

use crate::runtime::BindingCallContext;

/// Delete an environment variable by UTF-8 name.
pub(crate) unsafe fn destack_process_env_delete(
    _binding: &BindingCallContext,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    use windows_sys::Win32::Foundation::ERROR_ENVVAR_NOT_FOUND;
    use windows_sys::Win32::System::Environment::SetEnvironmentVariableW;

    let name = unsafe { name.as_str()? };
    core_process::ensure_no_nul_str(name, "name", "environment variable contains nul byte")?;

    let mut name_wide: Vec<u16> = name.encode_utf16().collect();
    name_wide.push(0);
    let rc = unsafe { SetEnvironmentVariableW(name_wide.as_ptr(), std::ptr::null()) };
    if rc == 0 {
        let error = core_platform::last_error_code() as u32;
        if error != ERROR_ENVVAR_NOT_FOUND {
            return Err(RuntimeError::from(PlatformError::io(format!(
                "failed to delete environment: {error}",
            )))
            .boxed());
        }
    }

    Ok(())
}

/// Delete an environment variable by raw byte name.
pub(crate) unsafe fn destack_process_env_delete_bytes(
    _binding: &BindingCallContext,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    use windows_sys::Win32::Foundation::ERROR_ENVVAR_NOT_FOUND;
    use windows_sys::Win32::System::Environment::SetEnvironmentVariableW;

    let name = unsafe { name.as_slice()? };
    core_process::ensure_no_nul_bytes(name, "name", "environment variable contains nul byte")?;
    let name = std::str::from_utf8(name).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "name",
            "environment variable is not valid utf8",
        ))
        .boxed()
    })?;

    let mut name_wide: Vec<u16> = name.encode_utf16().collect();
    name_wide.push(0);
    let rc = unsafe { SetEnvironmentVariableW(name_wide.as_ptr(), std::ptr::null()) };
    if rc == 0 {
        let error = core_platform::last_error_code() as u32;
        if error != ERROR_ENVVAR_NOT_FOUND {
            return Err(RuntimeError::from(PlatformError::io(format!(
                "failed to delete environment: {error}",
            )))
            .boxed());
        }
    }

    Ok(())
}

/// Read an environment variable by UTF-8 name.
pub(crate) unsafe fn destack_process_env_get(
    binding: &BindingCallContext,
    out: *mut NativeStringRef,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    use windows_sys::Win32::Foundation::ERROR_ENVVAR_NOT_FOUND;
    use windows_sys::Win32::System::Environment::GetEnvironmentVariableW;

    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let name_value = unsafe { name.as_str()? };
    core_process::ensure_no_nul_str(name_value, "name", "environment variable contains nul byte")?;

    let mut wide: Vec<u16> = name_value.encode_utf16().collect();
    wide.push(0);
    let required = unsafe { GetEnvironmentVariableW(wide.as_ptr(), std::ptr::null_mut(), 0) };
    if required == 0 {
        let error = core_platform::last_error_code() as u32;
        if error == ERROR_ENVVAR_NOT_FOUND {
            return Err(core_process::missing_env_error(name_value.to_string()));
        }
        return Err(RuntimeError::from(PlatformError::io(format!(
            "failed to read environment: {error}",
        )))
        .boxed());
    }

    let mut buffer = vec![0u16; required as usize + 1];
    let length =
        unsafe { GetEnvironmentVariableW(wide.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32) };
    if length == 0 {
        let error = core_platform::last_error_code() as u32;
        return Err(RuntimeError::from(PlatformError::io(format!(
            "failed to read environment: {error}",
        )))
        .boxed());
    }

    let value = String::from_utf16_lossy(&buffer[..length as usize]);

    unsafe {
        *out = binding.store_string(&value);
    }

    Ok(())
}

/// Read an environment variable by raw byte name.
pub(crate) unsafe fn destack_process_env_get_bytes(
    binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    use windows_sys::Win32::Foundation::ERROR_ENVVAR_NOT_FOUND;
    use windows_sys::Win32::System::Environment::GetEnvironmentVariableW;

    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let name_bytes = unsafe { name.as_slice()? };
    core_process::ensure_no_nul_bytes(
        name_bytes,
        "name",
        "environment variable contains nul byte",
    )?;
    let name = std::str::from_utf8(name_bytes).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "name",
            "environment variable is not valid utf8",
        ))
        .boxed()
    })?;

    let mut wide: Vec<u16> = name.encode_utf16().collect();
    wide.push(0);
    let required = unsafe { GetEnvironmentVariableW(wide.as_ptr(), std::ptr::null_mut(), 0) };
    if required == 0 {
        let error = core_platform::last_error_code() as u32;
        if error == ERROR_ENVVAR_NOT_FOUND {
            return Err(core_process::missing_env_error(
                String::from_utf8_lossy(name_bytes).to_string(),
            ));
        }
        return Err(RuntimeError::from(PlatformError::io(format!(
            "failed to read environment: {error}",
        )))
        .boxed());
    }

    let mut buffer = vec![0u16; required as usize + 1];
    let length =
        unsafe { GetEnvironmentVariableW(wide.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32) };
    if length == 0 {
        let error = core_platform::last_error_code() as u32;
        return Err(RuntimeError::from(PlatformError::io(format!(
            "failed to read environment: {error}",
        )))
        .boxed());
    }

    let value = String::from_utf16_lossy(&buffer[..length as usize]).into_bytes();

    unsafe {
        *out = binding.store_array(value);
    }

    Ok(())
}

/// Set an environment variable by UTF-8 name and value.
pub(crate) unsafe fn destack_process_env_set(
    _binding: &BindingCallContext,
    name: NativeStringRef,
    argument_value: NativeStringRef,
) -> RuntimeResult<()> {
    use windows_sys::Win32::System::Environment::SetEnvironmentVariableW;

    let name = unsafe { name.as_str()? };
    let value = unsafe { argument_value.as_str()? };
    core_process::ensure_no_nul_str(name, "name", "environment variable contains nul byte")?;
    core_process::ensure_no_nul_str(value, "value", "environment variable contains nul byte")?;

    let mut name_wide: Vec<u16> = name.encode_utf16().collect();
    name_wide.push(0);
    let mut value_wide: Vec<u16> = value.encode_utf16().collect();
    value_wide.push(0);

    let rc = unsafe { SetEnvironmentVariableW(name_wide.as_ptr(), value_wide.as_ptr()) };
    if rc == 0 {
        let error = core_platform::last_error_code();
        return Err(RuntimeError::from(PlatformError::io(format!(
            "failed to set environment: {error}",
        )))
        .boxed());
    }

    Ok(())
}

/// Set an environment variable by raw byte name and value.
pub(crate) unsafe fn destack_process_env_set_bytes(
    _binding: &BindingCallContext,
    name: NativeSlice<u8>,
    argument_value: NativeSlice<u8>,
) -> RuntimeResult<()> {
    use windows_sys::Win32::System::Environment::SetEnvironmentVariableW;

    let name = unsafe { name.as_slice()? };
    let value = unsafe { argument_value.as_slice()? };
    core_process::ensure_no_nul_bytes(name, "name", "environment variable contains nul byte")?;
    core_process::ensure_no_nul_bytes(value, "value", "environment variable contains nul byte")?;

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

    let mut name_wide: Vec<u16> = name.encode_utf16().collect();
    name_wide.push(0);
    let mut value_wide: Vec<u16> = value.encode_utf16().collect();
    value_wide.push(0);

    let rc = unsafe { SetEnvironmentVariableW(name_wide.as_ptr(), value_wide.as_ptr()) };
    if rc == 0 {
        let error = core_platform::last_error_code();
        return Err(RuntimeError::from(PlatformError::io(format!(
            "failed to set environment: {error}",
        )))
        .boxed());
    }

    Ok(())
}
