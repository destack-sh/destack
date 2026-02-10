use std::os::windows::ffi::OsStrExt;

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{PathBytes, PathBytesAbi, PathUtf16, PathUtf16Abi};
use crate::runtime::RuntimeCallContext;

/// Create a temporary directory with byte paths.
pub(crate) unsafe fn destack_fs_mkdtemp_bytes(
    context: &RuntimeCallContext,
    out: *mut PathBytes,
    template: PathBytes,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the template path
    let template = string_from_bytes(template, "template")?;

    // create the temporary directory
    let path = mkdtemp_from_template(&template)?;

    // write the output
    let bytes = bytes_from_pathbuf(&path, "path")?;
    unsafe {
        *out = PathBytesAbi::<NativeAbi>(context.store_array(bytes));
    }

    Ok(())
}

/// Create a temporary directory with UTF-16 paths.
pub(crate) unsafe fn destack_fs_mkdtemp_utf16(
    context: &RuntimeCallContext,
    out: *mut PathUtf16,
    template: PathUtf16,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate the template code units
    let wide = unsafe { template.0.as_slice()? };
    if wide.contains(&0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "template",
            "path contains nul code unit",
        ))
        .boxed());
    }

    // decode the template path
    let template = string_from_wide(wide, "template")?;

    // create the temporary directory
    let path = mkdtemp_from_template(&template)?;

    // write the output
    let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    unsafe {
        *out = PathUtf16Abi::<NativeAbi>(context.store_array(wide));
    }

    Ok(())
}
