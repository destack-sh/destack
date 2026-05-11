#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::process::core as core_process;
use crate::platform::{NativeArray, PlatformError};

use crate::runtime::BindingCallContext;

use std::ffi::CStr;

/// Delete an environment variable by UTF-8 name.
pub(crate) unsafe fn destack_process_env_delete(
    _binding: &BindingCallContext,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let name = unsafe { name.as_str()? };

    let name =
        core_process::cstring_from_str(name, "name", "environment variable contains nul byte")?;

    let rc = unsafe { libc::unsetenv(name.as_ptr()) };
    if rc != 0 {
        return Err(RuntimeError::from(PlatformError::io("failed to delete environment")).boxed());
    }

    Ok(())
}

/// Delete an environment variable by raw byte name.
pub(crate) unsafe fn destack_process_env_delete_bytes(
    _binding: &BindingCallContext,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let name = unsafe { name.as_slice()? };

    let name =
        core_process::cstring_from_bytes(name, "name", "environment variable contains nul byte")?;

    let rc = unsafe { libc::unsetenv(name.as_ptr()) };
    if rc != 0 {
        return Err(RuntimeError::from(PlatformError::io("failed to delete environment")).boxed());
    }

    Ok(())
}

/// Read an environment variable by UTF-8 name.
pub(crate) unsafe fn destack_process_env_get(
    binding: &BindingCallContext,
    out: *mut NativeStringRef,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let name_value = unsafe { name.as_str()? };

    let name = core_process::cstring_from_str(
        name_value,
        "name",
        "environment variable contains nul byte",
    )?;
    let value = unsafe { libc::getenv(name.as_ptr()) };
    let value = if value.is_null() {
        None
    } else {
        let value = unsafe { CStr::from_ptr(value) };
        Some(String::from_utf8_lossy(value.to_bytes()).to_string())
    };
    let value = value.ok_or_else(|| core_process::missing_env_error(name_value.to_string()))?;

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
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let name_bytes = unsafe { name.as_slice()? };

    let name = core_process::cstring_from_bytes(
        name_bytes,
        "name",
        "environment variable contains nul byte",
    )?;
    let value = unsafe { libc::getenv(name.as_ptr()) };
    let value = if value.is_null() {
        None
    } else {
        let value = unsafe { CStr::from_ptr(value) };
        Some(value.to_bytes().to_vec())
    };
    let value = value.ok_or_else(|| {
        core_process::missing_env_error(String::from_utf8_lossy(name_bytes).to_string())
    })?;

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
    let name = unsafe { name.as_str()? };
    let value = unsafe { argument_value.as_str()? };

    let name =
        core_process::cstring_from_str(name, "name", "environment variable contains nul byte")?;
    let value =
        core_process::cstring_from_str(value, "value", "environment variable contains nul byte")?;

    let rc = unsafe { libc::setenv(name.as_ptr(), value.as_ptr(), 1) };
    if rc != 0 {
        return Err(RuntimeError::from(PlatformError::io("failed to set environment")).boxed());
    }

    Ok(())
}

/// Set an environment variable by raw byte name and value.
pub(crate) unsafe fn destack_process_env_set_bytes(
    _binding: &BindingCallContext,
    name: NativeSlice<u8>,
    argument_value: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let name = unsafe { name.as_slice()? };
    let value = unsafe { argument_value.as_slice()? };
    let name =
        core_process::cstring_from_bytes(name, "name", "environment variable contains nul byte")?;
    let value =
        core_process::cstring_from_bytes(value, "value", "environment variable contains nul byte")?;

    let rc = unsafe { libc::setenv(name.as_ptr(), value.as_ptr(), 1) };
    if rc != 0 {
        return Err(RuntimeError::from(PlatformError::io("failed to set environment")).boxed());
    }

    Ok(())
}
