use windows_sys::Win32::Storage::FileSystem::{
    LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx, UnlockFileEx,
};
use windows_sys::Win32::System::IO::{OVERLAPPED, OVERLAPPED_0_0};

use super::util::*;
use crate::diagnostic::RuntimeResult;
use crate::platform::fs::{FileHandle, FileLockFlags};
use crate::runtime::RuntimeCallContext;

/// File lock flag value for exclusive locks.
const LOCK_EX: u32 = 0x2;
/// File lock flag value for non-blocking locks.
const LOCK_NB: u32 = 0x4;
/// File lock flag value for unlocking.
const LOCK_UN: u32 = 0x8;

/// Lock a file handle.
pub(crate) unsafe fn destack_fs_lock(
    _context: &RuntimeCallContext,
    handle: FileHandle,
    flags: FileLockFlags,
) -> RuntimeResult<()> {
    let handle = file_handle(_context, handle)?;
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
    let lock_exclusive = flags.0 & LOCK_EX != 0;
    let lock_nonblock = flags.0 & LOCK_NB != 0;
    let lock_unlock = flags.0 & LOCK_UN != 0;
    if lock_unlock {
        let rc = unsafe { UnlockFileEx(handle, 0, u32::MAX, u32::MAX, &mut overlapped) };
        if rc == 0 {
            return Err(last_os_error("UnlockFileEx", None));
        }
        return Ok(());
    }
    let mut flags = 0;
    if lock_exclusive {
        flags |= LOCKFILE_EXCLUSIVE_LOCK;
    }
    if lock_nonblock {
        flags |= LOCKFILE_FAIL_IMMEDIATELY;
    }
    let rc = unsafe { LockFileEx(handle, flags, 0, u32::MAX, u32::MAX, &mut overlapped) };
    if rc == 0 {
        return Err(last_os_error("LockFileEx", None));
    }
    Ok(())
}
