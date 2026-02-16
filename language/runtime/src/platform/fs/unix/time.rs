#![allow(unused_imports)]

use super::core::*;
use super::os;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{core as core_fs, *};
use crate::platform::resource::*;
use crate::platform::{core as core_platform, net as platform_net, *};
use crate::runtime::RuntimeCallContext;

use std::ffi::{CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;
use std::path::PathBuf;

/// Update access and modification times without following symlinks.
///
/// Update access and modification times without following symlinks via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses lutimes/utimensat with nofollow on Unix and reparse-point time updates on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_lutimes_bytes(
    _context: &RuntimeCallContext,
    path: PathBytes,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    // apply timestamps on unix platforms without following symlinks
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let times = [timespec_from_nanos(atimens), timespec_from_nanos(mtimens)];
    let rc = unsafe {
        libc::utimensat(
            libc::AT_FDCWD,
            c_path.as_ptr(),
            times.as_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("utimensat", None))
}

/// Update access and modification times without following symlinks.
///
/// Update access and modification times without following symlinks via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses lutimes/utimensat with nofollow on Unix and reparse-point time updates on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_lutimes_utf16(
    _context: &RuntimeCallContext,
    path: PathUtf16,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    // report unsupported lutimes calls on non-windows platforms
    let _ = (path, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lutimesUtf16")).boxed())
}

/// Update access and modification times relative to a directory handle.
///
/// Update access and modification times relative to a directory handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses utimensat(2) on Unix and handle-relative SetFileTime on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_utimensat_bytes(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    atimens: u64,
    mtimens: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // update timestamps relative to the directory on unix platforms
    let resource = directory_resource(context, dir)?;
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let times = [timespec_from_nanos(atimens), timespec_from_nanos(mtimens)];
    let rc = unsafe {
        libc::utimensat(
            resource.fd,
            c_path.as_ptr(),
            times.as_ptr(),
            flags.0 as libc::c_int,
        )
    };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("utimensat", None))
}

/// Update access and modification times relative to a directory handle.
///
/// Update access and modification times relative to a directory handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses utimensat(2) on Unix and handle-relative SetFileTime on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_utimensat_utf16(
    _context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    atimens: u64,
    mtimens: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // report unsupported utimensat calls on non-windows platforms
    let _ = (dir, path, atimens, mtimens, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.utimensatUtf16")).boxed())
}

/// Update access and modification times.
///
/// Update access and modification times via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses utimensat/utimes on Unix and SetFileTime on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_utimes_bytes(
    _context: &RuntimeCallContext,
    path: PathBytes,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    // apply timestamps on unix platforms
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let times = [timespec_from_nanos(atimens), timespec_from_nanos(mtimens)];
    let rc = unsafe { libc::utimensat(libc::AT_FDCWD, c_path.as_ptr(), times.as_ptr(), 0) };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("utimensat", None))
}

/// Update access and modification times.
///
/// Update access and modification times via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses utimensat/utimes on Unix and SetFileTime on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_utimes_utf16(
    _context: &RuntimeCallContext,
    path: PathUtf16,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    // report unsupported utimes calls on non-windows platforms
    let _ = (path, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.utimesUtf16")).boxed())
}

/// Update access and modification times.
///
/// Update access and modification times via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses utimensat/utimes on Unix and SetFileTime on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_utimes(
    context: &RuntimeCallContext,
    path: OsPath,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_utimes_bytes(context, path, atime_ns, mtime_ns) },
        |path| unsafe { destack_fs_utimes_utf16(context, path, atime_ns, mtime_ns) },
    )
}

/// Update access and modification times without following symlinks.
///
/// Update access and modification times without following symlinks via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses lutimes/utimensat with nofollow on Unix and reparse-point time updates on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_lutimes(
    context: &RuntimeCallContext,
    path: OsPath,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_lutimes_bytes(context, path, atime_ns, mtime_ns) },
        |path| unsafe { destack_fs_lutimes_utf16(context, path, atime_ns, mtime_ns) },
    )
}

/// Update access and modification times relative to a directory handle.
///
/// Update access and modification times relative to a directory handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses utimensat(2) on Unix and handle-relative SetFileTime on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_utimensat(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: OsPath,
    atime_ns: u64,
    mtime_ns: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_utimensat_bytes(context, dir, path, atime_ns, mtime_ns, flags) },
        |path| unsafe { destack_fs_utimensat_utf16(context, dir, path, atime_ns, mtime_ns, flags) },
    )
}
