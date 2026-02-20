use super::handle::{destack_fs_close, destack_fs_ftruncate};
use super::open::{destack_fs_open_bytes, destack_fs_open_utf16};
use crate::diagnostic::RuntimeResult;
use crate::platform::ResourceId;
use crate::platform::fs::{
    FileHandle, FileMode, FileOffset, OpenFlags, OsPath, PathBytes, PathUtf16, core as core_fs,
};
use crate::runtime::BindingCallContext;

/// Open flag value for write-only truncation.
const O_WRONLY: u32 = 0x1;

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
    context: &BindingCallContext,
    path: PathBytes,
    size: FileOffset,
) -> RuntimeResult<()> {
    // open the file and truncate it
    let mut handle = FileHandle(ResourceId(0));
    unsafe {
        destack_fs_open_bytes(
            context,
            &mut handle as *mut FileHandle,
            path,
            OpenFlags(O_WRONLY),
            FileMode(0o666),
        )?;
        destack_fs_ftruncate(context, handle, size)?;
        destack_fs_close(context, handle)
    }
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
    context: &BindingCallContext,
    path: PathUtf16,
    size: FileOffset,
) -> RuntimeResult<()> {
    // open the file and truncate it
    let mut handle = FileHandle(ResourceId(0));
    unsafe {
        destack_fs_open_utf16(
            context,
            &mut handle as *mut FileHandle,
            path,
            OpenFlags(O_WRONLY),
            FileMode(0o666),
        )?;
        destack_fs_ftruncate(context, handle, size)?;
        destack_fs_close(context, handle)
    }
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
    context: &BindingCallContext,
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
