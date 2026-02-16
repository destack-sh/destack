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

/// Truncate a file.
///
/// Truncate the target file to the requested size using host file-size control APIs.
/// Growth behavior for sparse expansion and zero-fill follows host filesystem policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses truncate(2) on Unix and SetEndOfFile via path handle on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_truncate_bytes(
    _context: &RuntimeCallContext,
    path: PathBytes,
    size: FileOffset,
) -> RuntimeResult<()> {
    // truncate the file on unix platforms
    let c_path = resolve_path_bytes_cstring(path, "path")?;
    let offset = offset_to_off_t(size)?;
    let rc = unsafe { libc::truncate(c_path.as_ptr(), offset) };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("truncate", None))
}

/// Truncate a file.
///
/// Truncate the target file to the requested size using host file-size control APIs.
/// Growth behavior for sparse expansion and zero-fill follows host filesystem policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses truncate(2) on Unix and SetEndOfFile via path handle on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_truncate_utf16(
    _context: &RuntimeCallContext,
    path: PathUtf16,
    size: FileOffset,
) -> RuntimeResult<()> {
    // report unsupported truncate calls on non-windows platforms
    let _ = (path, size);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.truncateUtf16")).boxed())
}

/// Truncate a file.
///
/// Truncate the target file to the requested size using host file-size control APIs.
/// Growth behavior for sparse expansion and zero-fill follows host filesystem policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses truncate(2) on Unix and SetEndOfFile via path handle on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_truncate(
    context: &RuntimeCallContext,
    path: OsPath,
    size: FileOffset,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_truncate_bytes(context, path, size) },
        |path| unsafe { destack_fs_truncate_utf16(context, path, size) },
    )
}
