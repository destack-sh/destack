use super::handle::{destack_fs_close, destack_fs_ftruncate};
use super::open::{destack_fs_open_bytes, destack_fs_open_utf16};
use crate::diagnostic::RuntimeResult;
use crate::platform::ResourceId;
use crate::platform::fs::{FileHandle, FileMode, FileOffset, OpenFlags, PathBytes, PathUtf16};
use crate::runtime::RuntimeCallContext;

/// Open flag value for write-only truncation.
const O_WRONLY: u32 = 0x1;

/// Truncate a file with byte paths.
pub(crate) unsafe fn destack_fs_truncate_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    size: FileOffset,
) -> RuntimeResult<()> {
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

/// Truncate a file with UTF-16 paths.
pub(crate) unsafe fn destack_fs_truncate_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    size: FileOffset,
) -> RuntimeResult<()> {
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
