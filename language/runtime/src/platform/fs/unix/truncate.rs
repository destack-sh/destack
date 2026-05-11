use super::core::*;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::fs::{core as core_fs, *};
use crate::runtime::BindingCallContext;

/// Truncate a file.
pub(crate) unsafe fn destack_fs_truncate_bytes(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_fs_truncate_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
    size: FileOffset,
) -> RuntimeResult<()> {
    // truncate the file by converting utf16 path input
    core_fs::with_utf16_as_bytes(path, "path", |path| unsafe {
        destack_fs_truncate_bytes(binding, path, size)
    })
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
