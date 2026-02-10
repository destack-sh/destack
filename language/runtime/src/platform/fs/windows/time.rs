use windows_sys::Win32::Foundation::CloseHandle;

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{AtFlags, DirectoryHandle, PathBytes, PathBytesAbi, PathUtf16};
use crate::runtime::RuntimeCallContext;

/// Update file times for byte paths.
pub(crate) unsafe fn destack_fs_utimes_bytes(
    _context: &RuntimeCallContext,
    path: PathBytes,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    // open the file with write-attributes access
    let wide = wide_from_bytes(path, "path")?;
    let handle = open_for_write_attributes(&wide, true)?;

    // update timestamps
    let result = set_handle_times(handle, atime_ns, mtime_ns);
    unsafe {
        CloseHandle(handle);
    }

    result
}

/// Update file times for UTF-16 paths.
pub(crate) unsafe fn destack_fs_utimes_utf16(
    _context: &RuntimeCallContext,
    path: PathUtf16,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    // open the file with write-attributes access
    let wide = wide_from_utf16(path, "path")?;
    let handle = open_for_write_attributes(&wide, true)?;

    // update timestamps
    let result = set_handle_times(handle, atime_ns, mtime_ns);
    unsafe {
        CloseHandle(handle);
    }

    result
}

/// Update link times for byte paths.
pub(crate) unsafe fn destack_fs_lutimes_bytes(
    _context: &RuntimeCallContext,
    path: PathBytes,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    // open the reparse point for write-attributes access
    let wide = wide_from_bytes(path, "path")?;
    let handle = open_for_write_attributes(&wide, false)?;

    // update timestamps
    let result = set_handle_times(handle, atime_ns, mtime_ns);
    unsafe {
        CloseHandle(handle);
    }

    result
}

/// Update link times for UTF-16 paths.
pub(crate) unsafe fn destack_fs_lutimes_utf16(
    _context: &RuntimeCallContext,
    path: PathUtf16,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    // open the reparse point for write-attributes access
    let wide = wide_from_utf16(path, "path")?;
    let handle = open_for_write_attributes(&wide, false)?;

    // update timestamps
    let result = set_handle_times(handle, atime_ns, mtime_ns);
    unsafe {
        CloseHandle(handle);
    }

    result
}

/// Update file times relative to a directory handle for byte paths.
pub(crate) unsafe fn destack_fs_utimensat_bytes(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    atime_ns: u64,
    mtime_ns: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // reject unsupported flags on windows
    if flags.0 != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.fs.utimensat")).boxed(),
        );
    }

    // resolve the path and delegate to utimes
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
    unsafe { destack_fs_utimes_bytes(context, path, atime_ns, mtime_ns) }
}

/// Update file times relative to a directory handle for UTF-16 paths.
pub(crate) unsafe fn destack_fs_utimensat_utf16(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    atime_ns: u64,
    mtime_ns: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // reject unsupported flags on windows
    if flags.0 != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.fs.utimensat")).boxed(),
        );
    }

    // resolve the path and delegate to utimes
    let pathbuf = pathbuf_from_utf16(path, "path")?;
    let full_path = if pathbuf.is_absolute() {
        pathbuf
    } else {
        let mut base = directory_path(context, dir)?;
        base.push(pathbuf);
        base
    };
    let path = path_utf16_from_pathbuf(context, &full_path);
    unsafe { destack_fs_utimes_utf16(context, path, atime_ns, mtime_ns) }
}
