use windows_sys::Win32::Storage::FileSystem::CopyFileW;

use super::util::*;
use crate::diagnostic::RuntimeResult;
use crate::platform::fs::{PathBytes, PathUtf16};
use crate::runtime::RuntimeCallContext;

/// Copy a file with byte paths.
pub(crate) unsafe fn destack_fs_copyfile_bytes(
    _context: &RuntimeCallContext,
    from: PathBytes,
    to: PathBytes,
    flags: u32,
) -> RuntimeResult<()> {
    let from = wide_from_bytes(from, "from")?;
    let to = wide_from_bytes(to, "to")?;
    let fail_if_exists = flags & 1 != 0;
    let rc = unsafe { CopyFileW(from.as_ptr(), to.as_ptr(), fail_if_exists as i32) };
    if rc == 0 {
        return Err(last_os_error("CopyFileW", None));
    }

    Ok(())
}

/// Copy a file with UTF-16 paths.
pub(crate) unsafe fn destack_fs_copyfile_utf16(
    _context: &RuntimeCallContext,
    from: PathUtf16,
    to: PathUtf16,
    flags: u32,
) -> RuntimeResult<()> {
    let from = wide_from_utf16(from, "from")?;
    let to = wide_from_utf16(to, "to")?;
    let fail_if_exists = flags & 1 != 0;
    let rc = unsafe { CopyFileW(from.as_ptr(), to.as_ptr(), fail_if_exists as i32) };
    if rc == 0 {
        return Err(last_os_error("CopyFileW", None));
    }

    Ok(())
}
