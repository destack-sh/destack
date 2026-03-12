use super::core::*;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::fs::{core as core_fs, *};
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
    let operation = core_fs::decode_file_lock_flags(flags)?;
    let operation = match operation {
        core_fs::FileLockOperation::Shared { nonblocking } => {
            let mut operation = libc::LOCK_SH;
            if nonblocking {
                operation |= libc::LOCK_NB;
            }
            operation
        }
        core_fs::FileLockOperation::Exclusive { nonblocking } => {
            let mut operation = libc::LOCK_EX;
            if nonblocking {
                operation |= libc::LOCK_NB;
            }
            operation
        }
        core_fs::FileLockOperation::Unlock => libc::LOCK_UN,
    };
    let result = unsafe { libc::flock(fd, operation) };
    if result != 0 {
        return Err(core_platform::io_error("flock", None));
    }
    Ok(())
}
