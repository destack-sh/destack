use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_READONLY, GetFileAttributesW, SetFileAttributesW,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::fs::{AccessMode, FileMode, PathBytes, PathUtf16};
use crate::runtime::RuntimeCallContext;

/// Check filesystem access for a byte path.
pub(crate) unsafe fn destack_fs_access_bytes(
    _context: &RuntimeCallContext,
    path: PathBytes,
    mode: AccessMode,
) -> RuntimeResult<()> {
    let wide = wide_from_bytes(path, "path")?;

    // check whether the path exists
    let attrs = unsafe { GetFileAttributesW(wide.as_ptr()) };
    if attrs == u32::MAX {
        return Err(last_os_error("GetFileAttributesW", None));
    }

    // check write permissions using readonly attribute
    if mode.0 & 0o222 != 0 && (attrs & FILE_ATTRIBUTE_READONLY) != 0 {
        return Err(RuntimeError::from(PlatformError::io("access denied")).boxed());
    }

    Ok(())
}

/// Check filesystem access for a UTF-16 path.
pub(crate) unsafe fn destack_fs_access_utf16(
    _context: &RuntimeCallContext,
    path: PathUtf16,
    mode: AccessMode,
) -> RuntimeResult<()> {
    let wide = wide_from_utf16(path, "path")?;

    // check whether the path exists
    let attrs = unsafe { GetFileAttributesW(wide.as_ptr()) };
    if attrs == u32::MAX {
        return Err(last_os_error("GetFileAttributesW", None));
    }

    // check write permissions using readonly attribute
    if mode.0 & 0o222 != 0 && (attrs & FILE_ATTRIBUTE_READONLY) != 0 {
        return Err(RuntimeError::from(PlatformError::io("access denied")).boxed());
    }

    Ok(())
}

/// Change permissions for a byte path.
pub(crate) unsafe fn destack_fs_chmod_bytes(
    _context: &RuntimeCallContext,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    let wide = wide_from_bytes(path, "path")?;
    let mut attrs = unsafe { GetFileAttributesW(wide.as_ptr()) };
    if attrs == u32::MAX {
        return Err(last_os_error("GetFileAttributesW", None));
    }

    // map write bits to readonly attribute
    if mode.0 & 0o222 == 0 {
        attrs |= FILE_ATTRIBUTE_READONLY;
    } else {
        attrs &= !FILE_ATTRIBUTE_READONLY;
    }

    let rc = unsafe { SetFileAttributesW(wide.as_ptr(), attrs) };
    if rc == 0 {
        return Err(last_os_error("SetFileAttributesW", None));
    }

    Ok(())
}

/// Change permissions for a UTF-16 path.
pub(crate) unsafe fn destack_fs_chmod_utf16(
    _context: &RuntimeCallContext,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    let wide = wide_from_utf16(path, "path")?;
    let mut attrs = unsafe { GetFileAttributesW(wide.as_ptr()) };
    if attrs == u32::MAX {
        return Err(last_os_error("GetFileAttributesW", None));
    }

    // map write bits to readonly attribute
    if mode.0 & 0o222 == 0 {
        attrs |= FILE_ATTRIBUTE_READONLY;
    } else {
        attrs &= !FILE_ATTRIBUTE_READONLY;
    }

    let rc = unsafe { SetFileAttributesW(wide.as_ptr(), attrs) };
    if rc == 0 {
        return Err(last_os_error("SetFileAttributesW", None));
    }

    Ok(())
}

/// Change ownership for a byte path.
pub(crate) unsafe fn destack_fs_chown_bytes(
    _context: &RuntimeCallContext,
    _path: PathBytes,
    _uid: u32,
    _gid: u32,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chownBytes")).boxed())
}

/// Change ownership for a UTF-16 path.
pub(crate) unsafe fn destack_fs_chown_utf16(
    _context: &RuntimeCallContext,
    _path: PathUtf16,
    _uid: u32,
    _gid: u32,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chownUtf16")).boxed())
}
