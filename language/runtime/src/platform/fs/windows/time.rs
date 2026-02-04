use windows_sys::Win32::Foundation::CloseHandle;

use super::util::*;
use crate::diagnostic::RuntimeResult;
use crate::platform::fs::{PathBytes, PathUtf16};
use crate::runtime::RuntimeCallContext;

/// Update file times for byte paths.
pub(crate) unsafe fn destack_fs_utimes_bytes(
    _context: &RuntimeCallContext,
    path: PathBytes,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    let wide = wide_from_bytes(path, "path")?;
    let handle = open_for_write_attributes(&wide, true)?;
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
    let wide = wide_from_utf16(path, "path")?;
    let handle = open_for_write_attributes(&wide, true)?;
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
    let wide = wide_from_bytes(path, "path")?;
    let handle = open_for_write_attributes(&wide, false)?;
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
    let wide = wide_from_utf16(path, "path")?;
    let handle = open_for_write_attributes(&wide, false)?;
    let result = set_handle_times(handle, atime_ns, mtime_ns);
    unsafe {
        CloseHandle(handle);
    }
    result
}
