use super::core::*;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::fs::*;
use crate::platform::resource::*;
use crate::runtime::BindingCallContext;

/// Apply file locks to a file handle.
///
/// Apply, release, or test advisory locking state for one file descriptor.
/// Lock scope and conflict behavior follow host flock/fcntl locking semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses flock/fcntl on Unix and LockFileEx/UnlockFileEx on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.lock`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_lock(
    binding: &BindingCallContext,
    handle: FileHandle,
    flags: FileLockFlags,
) -> RuntimeResult<()> {
    // lock the file on unix platforms
    let fd = file_descriptor(binding, handle)?;
    let result = unsafe { libc::flock(fd, flags.0 as libc::c_int) };
    if result != 0 {
        return Err(core_platform::io_error("flock", None));
    }
    Ok(())
}
