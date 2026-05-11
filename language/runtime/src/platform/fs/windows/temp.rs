use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{OsPath, PathBytes, PathBytesAbi, PathUtf16, core as core_fs};
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

/// Create a temporary directory.
pub(crate) unsafe fn destack_fs_mkdtemp_bytes(
    binding: &BindingCallContext,
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
        *out = PathBytesAbi::<NativeAbi>(binding.store_array(bytes));
    }

    Ok(())
}

/// Create a temporary directory.
pub(crate) unsafe fn destack_fs_mkdtemp_utf16(
    binding: &BindingCallContext,
    out: *mut PathUtf16,
    template: PathUtf16,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate the template code units
    let units = utf16_units(template, "template")?;
    if units.contains(&0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "template",
            "path contains nul code unit",
        ))
        .boxed());
    }

    // decode the template path
    let template = core_platform::string_from_wide("template", &units)?;

    // create the temporary directory
    let path = mkdtemp_from_template(&template)?;

    // write the output
    unsafe {
        *out = path_utf16_from_pathbuf(binding, &path);
    }

    Ok(())
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

    core_fs::with_path_ref(
        template,
        "template",
        |template| {
            let mut inner = core_fs::empty_path_bytes();
            unsafe { destack_fs_mkdtemp_bytes(binding, &mut inner, template) }?;
            unsafe {
                *out = core_fs::path_ref_from_bytes(inner);
            }
            Ok(())
        },
        |template| {
            let mut inner = core_fs::empty_path_utf16();
            unsafe { destack_fs_mkdtemp_utf16(binding, &mut inner, template) }?;
            unsafe {
                *out = core_fs::path_ref_from_utf16(inner);
            }
            Ok(())
        },
    )
}
