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

/// Stat a file without following symlinks.
///
/// Stat a file without following symlinks via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses lstat(2) on Unix and reparse-point aware metadata query on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_lstat_bytes(
    _context: &BindingCallContext,
    out: *mut Stat,
    path: PathBytes,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the stat data without following symlinks
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::lstat(c_path.as_ptr(), &mut stat) };
    if rc != 0 {
        return Err(core_platform::io_error("lstat", None));
    }

    unsafe {
        *out = stat_from_libc(stat);
    }

    Ok(())
}

/// Stat a file without following symlinks.
///
/// Stat a file without following symlinks via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses lstat(2) on Unix and reparse-point aware metadata query on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_lstat_utf16(
    _context: &BindingCallContext,
    out: *mut Stat,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // report unsupported lstat calls on non-windows platforms
    let _ = (path, out);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lstatUtf16")).boxed())
}

/// Stat a file.
///
/// Stat a file via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses stat(2) on Unix and GetFileInformationByHandleEx on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_stat_bytes(
    _context: &BindingCallContext,
    out: *mut Stat,
    path: PathBytes,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the stat data on unix platforms
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::stat(c_path.as_ptr(), &mut stat) };
    if rc != 0 {
        return Err(core_platform::io_error("stat", None));
    }

    unsafe {
        *out = stat_from_libc(stat);
    }

    Ok(())
}

/// Stat a file.
///
/// Stat a file via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses stat(2) on Unix and GetFileInformationByHandleEx on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_stat_utf16(
    _context: &BindingCallContext,
    out: *mut Stat,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // report unsupported stat calls on non-windows platforms
    let _ = (path, out);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statUtf16")).boxed())
}

/// Stat a filesystem.
///
/// Stat a filesystem via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses statfs/statvfs on Unix and GetDiskFreeSpaceExW/GetVolumeInformationW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_statfs_bytes(
    _context: &BindingCallContext,
    out: *mut StatFs,
    path: PathBytes,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the statfs data on unix platforms
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let statfs = os::statfs_for_path(&c_path)?;
    unsafe {
        *out = statfs;
    }
    Ok(())
}

/// Stat a filesystem.
///
/// Stat a filesystem via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses statfs/statvfs on Unix and GetDiskFreeSpaceExW/GetVolumeInformationW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_statfs_utf16(
    _context: &BindingCallContext,
    out: *mut StatFs,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // report unsupported statfs calls on non-windows platforms
    let _ = (path, out);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statfsUtf16")).boxed())
}

/// Stat a file relative to a directory handle.
///
/// Stat a file relative to a directory handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fstatat(2) on Unix and handle-relative stat on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_statat_bytes(
    context: &BindingCallContext,
    out: *mut Stat,
    dir: DirectoryHandle,
    path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // stat the path on unix platforms
    let resource = directory_resource(context, dir)?;
    let path = resolve_path_bytes_cstring(path, "path")?;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            resource.fd,
            path.as_ptr(),
            stat.as_mut_ptr(),
            flags.0 as libc::c_int,
        )
    };
    if result != 0 {
        return Err(core_platform::io_error("statat", None));
    }
    let stat = unsafe { stat.assume_init() };
    unsafe {
        *out = stat_from_libc(stat);
    }
    Ok(())
}

/// Stat a file relative to a directory handle.
///
/// Stat a file relative to a directory handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fstatat(2) on Unix and handle-relative stat on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_statat_utf16(
    context: &BindingCallContext,
    out: *mut Stat,
    dir: DirectoryHandle,
    path: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // report unsupported statat calls on non-windows platforms
    let _ = (context, dir, path, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statatUtf16")).boxed())
}

/// Stat a file.
///
/// Stat a file via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses stat(2) on Unix and GetFileInformationByHandleEx on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_stat(
    context: &BindingCallContext,
    out: *mut Stat,
    path: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_stat_bytes(context, out, path) },
        |path| unsafe { destack_fs_stat_utf16(context, out, path) },
    )
}

/// Stat a file relative to a directory handle.
///
/// Stat a file relative to a directory handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fstatat(2) on Unix and handle-relative stat on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_statat(
    context: &BindingCallContext,
    out: *mut Stat,
    dir: DirectoryHandle,
    path: OsPath,
    flags: AtFlags,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_statat_bytes(context, out, dir, path, flags) },
        |path| unsafe { destack_fs_statat_utf16(context, out, dir, path, flags) },
    )
}

/// Stat a file without following symlinks.
///
/// Stat a file without following symlinks via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses lstat(2) on Unix and reparse-point aware metadata query on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_lstat(
    context: &BindingCallContext,
    out: *mut Stat,
    path: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_lstat_bytes(context, out, path) },
        |path| unsafe { destack_fs_lstat_utf16(context, out, path) },
    )
}

/// Stat a filesystem.
///
/// Stat a filesystem via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses statfs/statvfs on Unix and GetDiskFreeSpaceExW/GetVolumeInformationW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_statfs(
    context: &BindingCallContext,
    out: *mut StatFs,
    path: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_statfs_bytes(context, out, path) },
        |path| unsafe { destack_fs_statfs_utf16(context, out, path) },
    )
}

/// Stat a path with statx semantics.
///
/// Stat a path with statx semantics via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses statx(2) on linux and runtime fallback on other targets.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_statx(
    context: &BindingCallContext,
    out: *mut Statx,
    dir: DirectoryHandle,
    path: OsPath,
    flags: StatxFlags,
    _mask: StatxMask,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    core_fs::with_path_ref(
        path,
        "path",
        |path| {
            #[cfg(unix)]
            {
                let directory_fd = directory_descriptor(context, dir)?;
                let path = path_bytes_to_cstring(path, "path")?;
                let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
                let result = unsafe {
                    libc::fstatat(
                        directory_fd,
                        path.as_ptr(),
                        stat.as_mut_ptr(),
                        flags.0 as libc::c_int,
                    )
                };
                if result != 0 {
                    return Err(RuntimeError::from(PlatformError::io(
                        "fstatat failed".to_string(),
                    ))
                    .boxed());
                }
                let stat = unsafe { stat.assume_init() };

                let atime_ns = (stat.st_atime as u64)
                    .saturating_mul(1_000_000_000)
                    .saturating_add(stat.st_atime_nsec as u64);
                let btime_ns = 0;
                let ctime_ns = (stat.st_ctime as u64)
                    .saturating_mul(1_000_000_000)
                    .saturating_add(stat.st_ctime_nsec as u64);
                let mtime_ns = (stat.st_mtime as u64)
                    .saturating_mul(1_000_000_000)
                    .saturating_add(stat.st_mtime_nsec as u64);

                unsafe {
                    *out = Statx {
                        mask: StatxMask(0),
                        blksize: stat.st_blksize as u32,
                        mount_id: 0,
                        dev_major: ((stat.st_dev >> 8) & 0xfff) as u32,
                        dev_minor: ((stat.st_dev & 0xff) | ((stat.st_dev >> 12) & 0xfff00)) as u32,
                        ino: stat.st_ino,
                        mode: FileMode(stat.st_mode as u32),
                        nlink: stat.st_nlink as u32,
                        uid: stat.st_uid,
                        gid: stat.st_gid,
                        rdev_major: ((stat.st_rdev >> 8) & 0xfff) as u32,
                        rdev_minor: ((stat.st_rdev & 0xff) | ((stat.st_rdev >> 12) & 0xfff00))
                            as u32,
                        size: FileSize(stat.st_size as u64),
                        blocks: stat.st_blocks as u64,
                        atime_ns,
                        btime_ns,
                        ctime_ns,
                        mtime_ns,
                    };
                }
                Ok(())
            }
            #[cfg(not(unix))]
            {
                let _ = (context, dir, path, flags);
                Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statx")).boxed())
            }
        },
        |_path| Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statx")).boxed()),
    )
}
