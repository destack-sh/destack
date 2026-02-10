use windows_sys::Win32::Security::Authorization::{SE_FILE_OBJECT, SetNamedSecurityInfoW};
use windows_sys::Win32::Security::{GROUP_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_READONLY, GetFileAttributesW, SetFileAttributesW,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{
    AccessMode, AtFlags, DirectoryHandle, FileMode, PathBytes, PathBytesAbi, PathUtf16,
};
use crate::runtime::RuntimeCallContext;

/// Check filesystem access for a byte path.
pub(crate) unsafe fn destack_fs_access_bytes(
    _context: &RuntimeCallContext,
    path: PathBytes,
    mode: AccessMode,
) -> RuntimeResult<()> {
    // decode the path
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
    // decode the path
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
    // decode the path
    let wide = wide_from_bytes(path, "path")?;

    // load the current attributes
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

    // update the file attributes
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
    // decode the path
    let wide = wide_from_utf16(path, "path")?;

    // load the current attributes
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

    // update the file attributes
    let rc = unsafe { SetFileAttributesW(wide.as_ptr(), attrs) };
    if rc == 0 {
        return Err(last_os_error("SetFileAttributesW", None));
    }

    Ok(())
}

/// Change permissions for a byte path relative to a directory handle.
pub(crate) unsafe fn destack_fs_fchmodat_bytes(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    mode: FileMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // reject unsupported flags on windows
    if flags.0 != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.fs.fchmodat")).boxed(),
        );
    }

    // resolve the path and delegate to chmod
    let pathbuf = pathbuf_from_bytes(path, "path")?;
    let full_path = if pathbuf.is_absolute() {
        pathbuf
    } else {
        let mut base = directory_path(context, dir)?;
        base.push(pathbuf);
        base
    };
    let bytes = bytes_from_pathbuf(&full_path, "path")?;
    let path = PathBytesAbi::<NativeAbi>(context.store_array(bytes));
    unsafe { destack_fs_chmod_bytes(context, path, mode) }
}

/// Change permissions for a UTF-16 path relative to a directory handle.
pub(crate) unsafe fn destack_fs_fchmodat_utf16(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    mode: FileMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // reject unsupported flags on windows
    if flags.0 != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.fs.fchmodat")).boxed(),
        );
    }

    // resolve the path and delegate to chmod
    let pathbuf = pathbuf_from_utf16(path, "path")?;
    let full_path = if pathbuf.is_absolute() {
        pathbuf
    } else {
        let mut base = directory_path(context, dir)?;
        base.push(pathbuf);
        base
    };
    let path = path_utf16_from_pathbuf(context, &full_path);
    unsafe { destack_fs_chmod_utf16(context, path, mode) }
}

/// Change ownership for a byte path.
pub(crate) unsafe fn destack_fs_chown_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    // resolve inputs
    let wide = wide_from_bytes(path, "path")?;
    let (owner, group) = posix_sids(context, uid, gid)?;

    // update ownership information
    let rc = unsafe {
        SetNamedSecurityInfoW(
            wide.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION,
            owner.as_ptr(),
            group.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if rc != 0 {
        return Err(win32_error("SetNamedSecurityInfoW", rc));
    }

    Ok(())
}

/// Change ownership for a byte path relative to a directory handle.
pub(crate) unsafe fn destack_fs_fchownat_bytes(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    uid: u32,
    gid: u32,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // reject unsupported flags on windows
    if flags.0 != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.fs.fchownat")).boxed(),
        );
    }

    // resolve the path and delegate to chown
    let pathbuf = pathbuf_from_bytes(path, "path")?;
    let full_path = if pathbuf.is_absolute() {
        pathbuf
    } else {
        let mut base = directory_path(context, dir)?;
        base.push(pathbuf);
        base
    };
    let bytes = bytes_from_pathbuf(&full_path, "path")?;
    let path = PathBytesAbi::<NativeAbi>(context.store_array(bytes));
    unsafe { destack_fs_chown_bytes(context, path, uid, gid) }
}

/// Change ownership for a UTF-16 path relative to a directory handle.
pub(crate) unsafe fn destack_fs_fchownat_utf16(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    uid: u32,
    gid: u32,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // reject unsupported flags on windows
    if flags.0 != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.fs.fchownat")).boxed(),
        );
    }

    // resolve the path and delegate to chown
    let pathbuf = pathbuf_from_utf16(path, "path")?;
    let full_path = if pathbuf.is_absolute() {
        pathbuf
    } else {
        let mut base = directory_path(context, dir)?;
        base.push(pathbuf);
        base
    };
    let path = path_utf16_from_pathbuf(context, &full_path);
    unsafe { destack_fs_chown_utf16(context, path, uid, gid) }
}

/// Change ownership for a UTF-16 path.
pub(crate) unsafe fn destack_fs_chown_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    // resolve inputs
    let wide = wide_from_utf16(path, "path")?;
    let (owner, group) = posix_sids(context, uid, gid)?;

    // update ownership information
    let rc = unsafe {
        SetNamedSecurityInfoW(
            wide.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION,
            owner.as_ptr(),
            group.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if rc != 0 {
        return Err(win32_error("SetNamedSecurityInfoW", rc));
    }

    Ok(())
}
