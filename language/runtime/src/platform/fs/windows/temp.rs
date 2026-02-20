use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{OsPath, PathBytes, PathBytesAbi, PathUtf16, core as core_fs};
use crate::runtime::BindingCallContext;

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
    let template = string_from_wide(&units, "template")?;

    // create the temporary directory
    let path = mkdtemp_from_template(&template)?;

    // write the output
    unsafe {
        *out = path_utf16_from_pathbuf(context, &path);
    }

    Ok(())
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
