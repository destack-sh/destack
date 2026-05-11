use windows_sys::Win32::Storage::FileSystem::{
    LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx, UnlockFileEx,
};
use windows_sys::Win32::System::IO::{OVERLAPPED, OVERLAPPED_0_0};

use super::util::*;
use crate::diagnostic::RuntimeResult;
use crate::platform::fs::{FileHandle, FileLockFlags, core as core_fs};
use crate::runtime::BindingCallContext;

/// Apply file locks to a file handle.
pub(crate) unsafe fn destack_fs_lock(
    binding: &BindingCallContext,
    handle: FileHandle,
    flags: FileLockFlags,
) -> RuntimeResult<()> {
    // resolve the file handle
    let handle = file_handle(binding, handle)?;

    // build the overlapped structure
    let mut overlapped = OVERLAPPED {
        Internal: 0,
        InternalHigh: 0,
        Anonymous: windows_sys::Win32::System::IO::OVERLAPPED_0 {
            Anonymous: OVERLAPPED_0_0 {
                Offset: 0,
                OffsetHigh: 0,
            },
        },
        hEvent: 0,
    };

    // decode the lock flags
    let operation = core_fs::decode_file_lock_flags(flags)?;

    // handle unlock requests
    if matches!(operation, core_fs::FileLockOperation::Unlock) {
        let result = unsafe { UnlockFileEx(handle, 0, u32::MAX, u32::MAX, &mut overlapped) };
        if result == 0 {
            return Err(last_os_error("UnlockFileEx", None));
        }
        return Ok(());
    }

    // map flags into the win32 call
    let mut native_flags = 0;
    match operation {
        core_fs::FileLockOperation::Shared { nonblocking } => {
            if nonblocking {
                native_flags |= LOCKFILE_FAIL_IMMEDIATELY;
            }
        }
        core_fs::FileLockOperation::Exclusive { nonblocking } => {
            native_flags |= LOCKFILE_EXCLUSIVE_LOCK;
            if nonblocking {
                native_flags |= LOCKFILE_FAIL_IMMEDIATELY;
            }
        }
        core_fs::FileLockOperation::Unlock => {}
    }

    // acquire the lock
    let rc = unsafe { LockFileEx(handle, native_flags, 0, u32::MAX, u32::MAX, &mut overlapped) };
    if rc == 0 {
        return Err(last_os_error("LockFileEx", None));
    }

    Ok(())
}
