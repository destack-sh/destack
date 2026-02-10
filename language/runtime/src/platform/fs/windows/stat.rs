use windows_sys::Wdk::Storage::FileSystem::{FILE_OPEN, FILE_OPEN_REPARSE_POINT};
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::Storage::FileSystem::{
    FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::fs::{AtFlags, DirectoryHandle, PathBytes, PathUtf16, Stat, StatFs};
use crate::runtime::RuntimeCallContext;

/// Stat a path with byte paths.
pub(crate) unsafe fn destack_fs_stat_bytes(
    _context: &RuntimeCallContext,
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

/// Stat a path with UTF-16 paths.
pub(crate) unsafe fn destack_fs_stat_utf16(
    _context: &RuntimeCallContext,
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

/// Stat a path (lstat) for byte paths.
pub(crate) unsafe fn destack_fs_lstat_bytes(
    _context: &RuntimeCallContext,
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

/// Stat a path (lstat) for UTF-16 paths.
pub(crate) unsafe fn destack_fs_lstat_utf16(
    _context: &RuntimeCallContext,
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

/// Statfs a path with byte paths.
pub(crate) unsafe fn destack_fs_statfs_bytes(
    _context: &RuntimeCallContext,
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

/// Statfs a path with UTF-16 paths.
pub(crate) unsafe fn destack_fs_statfs_utf16(
    _context: &RuntimeCallContext,
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

/// Stat a path relative to a directory handle with byte paths.
pub(crate) unsafe fn destack_fs_statat_bytes(
    context: &RuntimeCallContext,
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

/// Stat a path relative to a directory handle with UTF-16 paths.
pub(crate) unsafe fn destack_fs_statat_utf16(
    context: &RuntimeCallContext,
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
