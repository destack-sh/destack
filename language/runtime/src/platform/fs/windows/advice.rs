use std::mem;

use windows_sys::Win32::Storage::FileSystem::{
    FILE_ALLOCATION_INFO, FileAllocationInfo, SetFileInformationByHandle,
};

use super::destack_fs_fdatasync;
use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::fs::{AllocFlags, FileAdvice, FileHandle, FileOffset, FileSize, SyncFlags};
use crate::runtime::RuntimeCallContext;

/// Advise the OS about file access patterns.
pub(crate) unsafe fn destack_fs_fadvise(
    _context: &RuntimeCallContext,
    _handle: FileHandle,
    _offset: FileOffset,
    _length: FileSize,
    _advice: FileAdvice,
) -> RuntimeResult<()> {
    Ok(())
}

/// Allocate space for a file handle.
pub(crate) unsafe fn destack_fs_fallocate(
    _context: &RuntimeCallContext,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: AllocFlags,
) -> RuntimeResult<()> {
    if flags.0 != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.fs.fallocate flags")).boxed(),
        );
    }

    let handle = file_handle(_context, handle)?;
    let end = offset.0.saturating_add(length.0) as i64;
    let allocation = FILE_ALLOCATION_INFO {
        AllocationSize: end,
    };
    let rc = unsafe {
        SetFileInformationByHandle(
            handle,
            FileAllocationInfo,
            &allocation as *const _ as *const _,
            mem::size_of::<FILE_ALLOCATION_INFO>() as u32,
        )
    };
    if rc == 0 {
        return Err(last_os_error(
            "SetFileInformationByHandle(FileAllocationInfo)",
            None,
        ));
    }

    Ok(())
}

/// Sync a file range to disk.
pub(crate) unsafe fn destack_fs_sync_file_range(
    _context: &RuntimeCallContext,
    handle: FileHandle,
    _offset: FileOffset,
    _length: FileSize,
    _flags: SyncFlags,
) -> RuntimeResult<()> {
    unsafe { destack_fs_fdatasync(_context, handle) }
}
