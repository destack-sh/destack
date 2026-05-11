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
pub(crate) unsafe fn destack_fs_truncate_bytes(
    binding: &BindingCallContext,
    path: PathBytes,
    size: FileOffset,
) -> RuntimeResult<()> {
    // open the file and truncate it
    let mut handle = FileHandle(ResourceId(0));
    unsafe {
        destack_fs_open_bytes(
            binding,
            &mut handle as *mut FileHandle,
            path,
            OpenFlags(O_WRONLY),
            FileMode(0o666),
        )?;
        destack_fs_ftruncate(binding, handle, size)?;
        destack_fs_close(binding, handle)
    }
}

/// Truncate a file.
pub(crate) unsafe fn destack_fs_truncate_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
    size: FileOffset,
) -> RuntimeResult<()> {
    // open the file and truncate it
    let mut handle = FileHandle(ResourceId(0));
    unsafe {
        destack_fs_open_utf16(
            binding,
            &mut handle as *mut FileHandle,
            path,
            OpenFlags(O_WRONLY),
            FileMode(0o666),
        )?;
        destack_fs_ftruncate(binding, handle, size)?;
        destack_fs_close(binding, handle)
    }
}

/// Truncate a file.
pub(crate) unsafe fn destack_fs_truncate(
    binding: &BindingCallContext,
    path: OsPath,
    size: FileOffset,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_truncate_bytes(binding, path, size) },
        |path| unsafe { destack_fs_truncate_utf16(binding, path, size) },
    )
}
