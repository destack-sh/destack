use windows_sys::Wdk::Storage::FileSystem::{FILE_OPEN, FILE_OPEN_REPARSE_POINT};
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::Storage::FileSystem::{
    FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::fs::{
    AtFlags, DirectoryHandle, OsPath, PathBytes, PathUtf16, Stat, StatFs, Statx, StatxFlags,
    StatxMask, core as core_fs,
};
use crate::runtime::BindingCallContext;

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
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the path
    let wide = wide_from_bytes(path, "path")?;

    // gather metadata
    let stat = stat_from_path(&wide, true)?;

    // write the output
    unsafe {
        *out = stat;
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
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the path
    let wide = wide_from_utf16(path, "path")?;

    // gather metadata
    let stat = stat_from_path(&wide, true)?;

    // write the output
    unsafe {
        *out = stat;
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
pub(crate) unsafe fn destack_fs_lstat_bytes(
    _context: &BindingCallContext,
    out: *mut Stat,
    path: PathBytes,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the path
    let wide = wide_from_bytes(path, "path")?;

    // gather metadata without following symlinks
    let stat = stat_from_path(&wide, false)?;

    // write the output
    unsafe {
        *out = stat;
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
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the path
    let wide = wide_from_utf16(path, "path")?;

    // gather metadata without following symlinks
    let stat = stat_from_path(&wide, false)?;

    // write the output
    unsafe {
        *out = stat;
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
pub(crate) unsafe fn destack_fs_statfs_bytes(
    _context: &BindingCallContext,
    out: *mut StatFs,
    path: PathBytes,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the path
    let wide = wide_from_bytes(path, "path")?;

    // gather filesystem metadata
    let statfs = statfs_from_path(&wide)?;

    // write the output
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
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the path
    let wide = wide_from_utf16(path, "path")?;

    // gather filesystem metadata
    let statfs = statfs_from_path(&wide)?;

    // write the output
    unsafe {
        *out = statfs;
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
pub(crate) unsafe fn destack_fs_statat_bytes(
    context: &BindingCallContext,
    out: *mut Stat,
    dir: DirectoryHandle,
    path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the path and flags
    let no_follow = flags.0 & AT_SYMLINK_NOFOLLOW != 0;
    let pathbuf = pathbuf_from_bytes(path, "path")?;

    // use the absolute path when available
    if pathbuf.is_absolute() {
        let wide = wide_from_pathbuf(&pathbuf);
        let stat = stat_from_path(&wide, !no_follow)?;
        unsafe {
            *out = stat;
        }
        return Ok(());
    }

    // resolve the directory handle
    let root = directory_handle(context, dir)?;
    let path = wide_from_pathbuf_no_nul(&pathbuf);

    // map flags into open options
    let mut options = 0;
    if no_follow {
        options |= FILE_OPEN_REPARSE_POINT;
    }

    // open a handle and stat it
    let handle = nt_create_file_at(
        root,
        &path,
        FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        FILE_OPEN,
        options,
        0,
    )?;
    let stat = stat_from_handle(handle)?;

    // close the handle and write the output
    unsafe {
        CloseHandle(handle);
        *out = stat;
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
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the path and flags
    let no_follow = flags.0 & AT_SYMLINK_NOFOLLOW != 0;
    let pathbuf = pathbuf_from_utf16(path, "path")?;

    // use the absolute path when available
    if pathbuf.is_absolute() {
        let wide = wide_from_pathbuf(&pathbuf);
        let stat = stat_from_path(&wide, !no_follow)?;
        unsafe {
            *out = stat;
        }
        return Ok(());
    }

    // resolve the directory handle
    let root = directory_handle(context, dir)?;
    let path = wide_from_pathbuf_no_nul(&pathbuf);

    // map flags into open options
    let mut options = 0;
    if no_follow {
        options |= FILE_OPEN_REPARSE_POINT;
    }

    // open a handle and stat it
    let handle = nt_create_file_at(
        root,
        &path,
        FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        FILE_OPEN,
        options,
        0,
    )?;
    let stat = stat_from_handle(handle)?;

    // close the handle and write the output
    unsafe {
        CloseHandle(handle);
        *out = stat;
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
