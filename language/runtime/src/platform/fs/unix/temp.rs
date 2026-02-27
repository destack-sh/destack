#![allow(unused_imports)]

use super::core::*;
use super::os;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{core as core_fs, *};
use crate::platform::resource::*;
use crate::platform::{core as core_platform, net as platform_net, *};
use crate::runtime::BindingCallContext;

use std::ffi::{CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;
use std::path::PathBuf;

/// Create a temporary directory.
///
/// Create a unique temporary directory from the template in the platform temp directory.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses mkdtemp(3) on Unix and GetTempPathW plus CreateDirectoryW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.temp`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_mkdtemp_bytes(
    context: &BindingCallContext,
    out: *mut PathBytes,
    template: PathBytes,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // create the temporary directory on unix platforms
    let bytes = unsafe { template.0.as_slice()? };
    if bytes.contains(&0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "template",
            "template contains nul byte",
        ))
        .boxed());
    }
    let mut buffer = bytes.to_vec();
    buffer.push(0);
    let ptr = buffer.as_mut_ptr() as *mut libc::c_char;
    let result = unsafe { libc::mkdtemp(ptr) };
    if result.is_null() {
        return Err(core_platform::io_error("mkdtemp", None));
    }

    let value = unsafe { CStr::from_ptr(result) }.to_bytes().to_vec();
    unsafe {
        *out = PathBytesAbi::<NativeAbi>(context.store_array(value));
    }

    Ok(())
}

#[allow(dead_code)]
/// Create a temporary directory.
///
/// Create a unique temporary directory from the template in the platform temp directory.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses mkdtemp(3) on Unix and GetTempPathW plus CreateDirectoryW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.temp`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_mkdtemp_utf16(
    context: &BindingCallContext,
    out: *mut PathUtf16,
    template: PathUtf16,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // create a temporary directory by converting utf16 input to bytes
    core_fs::with_utf16_as_bytes(template, "template", |template| {
        let mut bytes_output = core_fs::empty_path_bytes();
        unsafe { destack_fs_mkdtemp_bytes(context, &mut bytes_output, template) }?;
        let utf16_output = core_fs::path_utf16_from_bytes(context, bytes_output, "template")?;
        unsafe {
            *out = utf16_output;
        }
        Ok(())
    })
}

/// Create a temporary directory.
///
/// Create a unique temporary directory from the template in the platform temp directory.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses mkdtemp(3) on Unix and GetTempPathW plus CreateDirectoryW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.temp`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_mkdtemp(
    context: &BindingCallContext,
    out: *mut OsPath,
    template: OsPath,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    #[cfg(unix)]
    {
        match template.encoding {
            PathEncoding::Bytes => {
                let mut inner = core_fs::empty_path_bytes();
                unsafe { destack_fs_mkdtemp_bytes(context, &mut inner, template.bytes) }?;
                unsafe {
                    *out = core_fs::path_ref_from_bytes(inner);
                }
                Ok(())
            }
            PathEncoding::Utf16 => {
                let bytes = core_fs::with_utf16_as_bytes(template.utf16, "template", |template| {
                    let mut inner = core_fs::empty_path_bytes();
                    unsafe { destack_fs_mkdtemp_bytes(context, &mut inner, template) }?;
                    Ok(inner)
                })?;
                let utf16 = core_fs::path_utf16_from_bytes(context, bytes, "template")?;
                unsafe {
                    *out = core_fs::path_ref_from_utf16(utf16);
                }
                Ok(())
            }
        }
    }
    #[cfg(not(unix))]
    core_fs::with_path_ref(
        template,
        "template",
        |template| {
            let mut inner = core_fs::empty_path_bytes();
            unsafe { destack_fs_mkdtemp_bytes(context, &mut inner, template) }?;
            unsafe {
                *out = core_fs::path_ref_from_bytes(inner);
            }
            Ok(())
        },
        |template| {
            let mut inner = core_fs::empty_path_utf16();
            unsafe { destack_fs_mkdtemp_utf16(context, &mut inner, template) }?;
            unsafe {
                *out = core_fs::path_ref_from_utf16(inner);
            }
            Ok(())
        },
    )
}
