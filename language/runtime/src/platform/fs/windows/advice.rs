use std::mem;

use windows_sys::Win32::Storage::FileSystem::{
    FILE_ALLOCATION_INFO, FileAllocationInfo, SetFileInformationByHandle,
};

use super::destack_fs_fdatasync;
use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::fs::{AllocFlags, FileAdvice, FileHandle, FileOffset, FileSize, SyncFlags};
use crate::runtime::BindingCallContext;

/// Advise the kernel about access patterns.
pub(crate) unsafe fn destack_fs_fadvise(
    _binding: &BindingCallContext,
    _handle: FileHandle,
    _offset: FileOffset,
    _length: FileSize,
    _advice: FileAdvice,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.fadvise")).boxed())
}

/// Allocate or punch file space.
pub(crate) unsafe fn destack_fs_fallocate(
    binding: &BindingCallContext,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: AllocFlags,
) -> RuntimeResult<()> {
    // reject negative offsets
    if offset.0 < 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "offset",
            "offset must be non-negative",
        ))
        .boxed());
    }

    // validate flags
    if flags.0 != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.fs.file.fallocate")).boxed(),
        );
    }

    // resolve the handle
    let handle = file_handle(binding, handle)?;

    // compute the allocation end offset
    let length = i64::try_from(length.0).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "length",
            "length is too large",
        ))
        .boxed()
    })?;
    let end = offset.0.checked_add(length).ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "length",
            "allocation end overflows file offset",
        ))
        .boxed()
    })?;

    // set the allocation size
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

/// Synchronize a file range.
pub(crate) unsafe fn destack_fs_sync_file_range(
    binding: &BindingCallContext,
    handle: FileHandle,
    _offset: FileOffset,
    _length: FileSize,
    _flags: SyncFlags,
) -> RuntimeResult<()> {
    unsafe { destack_fs_fdatasync(binding, handle) }
}
