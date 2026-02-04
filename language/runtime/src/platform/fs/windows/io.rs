use windows_sys::Win32::Foundation::ERROR_IO_PENDING;
use windows_sys::Win32::Storage::FileSystem::{ReadFile, WriteFile};
use windows_sys::Win32::System::IO::{GetOverlappedResult, OVERLAPPED, OVERLAPPED_0_0};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{FileHandle, FileOffset};
use crate::platform::{NativeSlice, PlatformError};
use crate::runtime::RuntimeCallContext;

/// Read from a file handle.
pub(crate) unsafe fn destack_fs_read(
    _context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let handle = file_handle(_context, handle)?;
    let buffer = unsafe { buffer.as_mut_slice()? };
    let mut overlapped = OVERLAPPED {
        Internal: 0,
        InternalHigh: 0,
        Anonymous: windows_sys::Win32::System::IO::OVERLAPPED_0 {
            Anonymous: OVERLAPPED_0_0 {
                Offset: offset.0 as u32,
                OffsetHigh: (offset.0 >> 32) as u32,
            },
        },
        hEvent: 0,
    };
    let mut bytes_read = 0u32;
    let rc = unsafe {
        ReadFile(
            handle,
            buffer.as_mut_ptr() as *mut _,
            buffer.len() as u32,
            &mut bytes_read,
            &mut overlapped,
        )
    };
    if rc == 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error().unwrap_or(0) as u32 != ERROR_IO_PENDING {
            return Err(last_os_error("ReadFile", None));
        }
        let rc = unsafe { GetOverlappedResult(handle, &mut overlapped, &mut bytes_read, 1) };
        if rc == 0 {
            return Err(last_os_error("GetOverlappedResult", None));
        }
    }
    unsafe {
        *out = bytes_read as u64;
    }
    Ok(())
}

/// Write to a file handle.
pub(crate) unsafe fn destack_fs_write(
    _context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let handle = file_handle(_context, handle)?;
    let buffer = unsafe { buffer.as_slice()? };
    let mut overlapped = OVERLAPPED {
        Internal: 0,
        InternalHigh: 0,
        Anonymous: windows_sys::Win32::System::IO::OVERLAPPED_0 {
            Anonymous: OVERLAPPED_0_0 {
                Offset: offset.0 as u32,
                OffsetHigh: (offset.0 >> 32) as u32,
            },
        },
        hEvent: 0,
    };
    let mut bytes_written = 0u32;
    let rc = unsafe {
        WriteFile(
            handle,
            buffer.as_ptr() as *const _,
            buffer.len() as u32,
            &mut bytes_written,
            &mut overlapped,
        )
    };
    if rc == 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error().unwrap_or(0) as u32 != ERROR_IO_PENDING {
            return Err(last_os_error("WriteFile", None));
        }
        let rc = unsafe { GetOverlappedResult(handle, &mut overlapped, &mut bytes_written, 1) };
        if rc == 0 {
            return Err(last_os_error("GetOverlappedResult", None));
        }
    }
    unsafe {
        *out = bytes_written as u64;
    }
    Ok(())
}

/// Read from a file handle into multiple buffers.
pub(crate) unsafe fn destack_fs_readv(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let buffers = unsafe { buffers.as_slice()? };
    let mut total = 0u64;
    let mut current = offset.0;
    for buffer in buffers {
        let mut local = 0u64;
        unsafe {
            destack_fs_read(
                context,
                &mut local as *mut u64,
                handle,
                *buffer,
                FileOffset(current),
            )?;
        }
        total += local;
        current += local;
        if local < buffer.len as u64 {
            break;
        }
    }
    unsafe {
        *out = total;
    }
    Ok(())
}

/// Write to a file handle from multiple buffers.
pub(crate) unsafe fn destack_fs_writev(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let buffers = unsafe { buffers.as_slice()? };
    let mut total = 0u64;
    let mut current = offset.0;
    for buffer in buffers {
        let mut local = 0u64;
        unsafe {
            destack_fs_write(
                context,
                &mut local as *mut u64,
                handle,
                *buffer,
                FileOffset(current),
            )?;
        }
        total += local;
        current += local;
        if local < buffer.len as u64 {
            break;
        }
    }
    unsafe {
        *out = total;
    }
    Ok(())
}
