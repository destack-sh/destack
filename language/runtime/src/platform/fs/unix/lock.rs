use super::core::*;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::fs::{core as core_fs, *};
use crate::platform::resource::*;
use crate::runtime::BindingCallContext;

/// Apply file locks to a file handle.
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
