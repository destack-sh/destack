use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{core as core_fs, *};
use crate::platform::{core as core_platform, *};
use crate::runtime::BindingCallContext;

use std::ffi::CStr;

const MKDTEMP_TEMPLATE_SUFFIX: &[u8] = b"XXXXXX";

/// Create a temporary directory.
pub(crate) unsafe fn destack_fs_mkdtemp_bytes(
    binding: &BindingCallContext,
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

    // require the standard trailing suffix
    if !bytes.ends_with(MKDTEMP_TEMPLATE_SUFFIX) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "template",
            "template must end with XXXXXX",
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
        *out = PathBytesAbi::<NativeAbi>(binding.store_array(value));
    }

    Ok(())
}

#[allow(dead_code)]
/// Create a temporary directory.
pub(crate) unsafe fn destack_fs_mkdtemp_utf16(
    binding: &BindingCallContext,
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
        unsafe { destack_fs_mkdtemp_bytes(binding, &mut bytes_output, template) }?;
        let utf16_output = core_fs::path_utf16_from_bytes(binding, bytes_output, "template")?;
        unsafe {
            *out = utf16_output;
        }
        Ok(())
    })
}

/// Create a temporary directory.
pub(crate) unsafe fn destack_fs_mkdtemp(
    binding: &BindingCallContext,
    out: *mut OsPath,
    template: OsPath,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    match template {
        OsPath::OsPathBytes(path_bytes) => {
            let mut inner = core_fs::empty_path_bytes();
            unsafe { destack_fs_mkdtemp_bytes(binding, &mut inner, path_bytes.bytes) }?;
            unsafe {
                *out = core_fs::path_ref_from_bytes(inner);
            }
            Ok(())
        }
        OsPath::OsPathUtf16(path_utf16) => {
            let bytes = core_fs::with_utf16_as_bytes(path_utf16.utf16, "template", |template| {
                let mut inner = core_fs::empty_path_bytes();
                unsafe { destack_fs_mkdtemp_bytes(binding, &mut inner, template) }?;
                Ok(inner)
            })?;
            let utf16 = core_fs::path_utf16_from_bytes(binding, bytes, "template")?;
            unsafe {
                *out = core_fs::path_ref_from_utf16(utf16);
            }
            Ok(())
        }
    }
}
