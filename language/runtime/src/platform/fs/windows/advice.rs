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

/// Advise the kernel about access patterns.
///
/// Provide expected access pattern hints for one descriptor range.
/// Advice is best effort and does not change correctness or visibility semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses posix_fadvise(2) on Unix and notSupported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_fadvise(
    _context: &RuntimeCallContext,
    _handle: FileHandle,
    _offset: FileOffset,
    _length: FileSize,
    _advice: FileAdvice,
) -> RuntimeResult<()> {
    Ok(())
}

/// Allocate or punch file space.
///
/// Reserve, deallocate, or punch one byte range using host allocation controls.
/// Flag combinations define keep-size and hole-punch behavior where supported.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fallocate(2) or posix_fallocate on Unix and allocation/truncate APIs on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_fallocate(
    _context: &RuntimeCallContext,
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
            RuntimeError::from(PlatformError::not_supported("destack.fs.fallocate flags")).boxed(),
        );
    }

    // resolve the handle
    let handle = file_handle(_context, handle)?;

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
///
/// Request writeback of one byte range for the target descriptor.
/// Range ordering, blocking behavior, and fallback support follow host kernel policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses sync_file_range(2) on linux and runtime fallback on other targets.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.sync`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_sync_file_range(
    _context: &RuntimeCallContext,
    handle: FileHandle,
    _offset: FileOffset,
    _length: FileSize,
    _flags: SyncFlags,
) -> RuntimeResult<()> {
    unsafe { destack_fs_fdatasync(_context, handle) }
}
