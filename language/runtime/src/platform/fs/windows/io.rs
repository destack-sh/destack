#![allow(dead_code)]

use windows_sys::Win32::Foundation::ERROR_IO_PENDING;
use windows_sys::Win32::Networking::WinSock::{SOCKET_ERROR, send};
use windows_sys::Win32::Storage::FileSystem::{ReadFile, WriteFile};
use windows_sys::Win32::System::IO::{GetOverlappedResult, OVERLAPPED, OVERLAPPED_0_0};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{
    FileHandle, FileOffset, FileSize, ReadWriteFlags, SpliceCursor, SpliceFlags,
};
use crate::platform::net::SocketHandle;
use crate::platform::resource::{PipeHandle, ResourceId};
use crate::platform::{NativeSlice, PlatformError, core as core_platform};
use crate::runtime::RuntimeCallContext;

/// Build a socket error from the last WSA error.
fn last_socket_error(syscall: &str) -> Box<RuntimeError> {
    let errno = core_platform::last_wsa_error_code();
    let message = core_platform::error_message(syscall, errno);

    RuntimeError::from(PlatformError::net_with(
        None,
        None,
        Some(errno),
        Some(syscall.to_string()),
        None,
        None,
        message,
    ))
    .boxed()
}

/// Add an unsigned byte delta to a signed file offset.
fn add_offset(base: i64, delta: u64, name: &str) -> RuntimeResult<i64> {
    // convert the delta into signed range
    let delta = i64::try_from(delta).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "byte count exceeds signed offset range",
        ))
        .boxed()
    })?;

    // add with overflow checks
    base.checked_add(delta).ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            name,
            "offset overflow",
        ))
        .boxed()
    })
}

/// Read from a file at the given file offset.
///
/// Read bytes into one contiguous caller-provided buffer at an explicit file offset.
/// The descriptor's current file position is not changed by positioned reads.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses pread(2) on Unix and positioned ReadFile on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_pread(
    _context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the handle and buffer
    let handle = file_handle(_context, handle)?;
    let buffer = unsafe { buffer.as_mut_slice()? };

    // build the overlapped offset
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

    // issue the read
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
        let error = core_platform::last_error_code() as u32;
        if error != ERROR_IO_PENDING {
            return Err(last_os_error("ReadFile", None));
        }
        let rc = unsafe { GetOverlappedResult(handle, &overlapped, &mut bytes_read, 1) };
        if rc == 0 {
            return Err(last_os_error("GetOverlappedResult", None));
        }
    }

    // write the output
    unsafe {
        *out = bytes_read as u64;
    }

    Ok(())
}

/// Write to a file at the given file offset.
///
/// Write bytes from one contiguous caller-provided buffer at an explicit file offset.
/// The descriptor's current file position is not changed by positioned writes.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses pwrite(2) on Unix and positioned WriteFile on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_pwrite(
    _context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the handle and buffer
    let handle = file_handle(_context, handle)?;
    let buffer = unsafe { buffer.as_slice()? };

    // build the overlapped offset
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

    // issue the write
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
        let error = core_platform::last_error_code() as u32;
        if error != ERROR_IO_PENDING {
            return Err(last_os_error("WriteFile", None));
        }
        let rc = unsafe { GetOverlappedResult(handle, &overlapped, &mut bytes_written, 1) };
        if rc == 0 {
            return Err(last_os_error("GetOverlappedResult", None));
        }
    }

    // write the output
    unsafe {
        *out = bytes_written as u64;
    }

    Ok(())
}

/// Read into multiple buffers at the given file offset.
///
/// Read bytes into a scatter buffer list at an explicit file offset.
/// The descriptor's current file position is not changed by positioned vectored reads.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses preadv(2) on Unix and vectored positioned file I/O loop on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_preadv(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the buffer list
    let buffers = unsafe { buffers.as_slice()? };

    // read each buffer in sequence
    let mut total = 0u64;
    let mut current = offset.0;
    for buffer in buffers {
        let mut local = 0u64;
        unsafe {
            destack_fs_pread(
                context,
                &mut local as *mut u64,
                handle,
                *buffer,
                FileOffset(current),
            )?;
        }
        total += local;
        current = add_offset(current, local, "offset")?;
        if local < buffer.len as u64 {
            break;
        }
    }

    // write the output
    unsafe {
        *out = total;
    }

    Ok(())
}

/// Write from multiple buffers at the given file offset.
///
/// Write bytes from a gather buffer list at an explicit file offset.
/// The descriptor's current file position is not changed by positioned vectored writes.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses pwritev(2) on Unix and vectored positioned file I/O loop on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_pwritev(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the buffer list
    let buffers = unsafe { buffers.as_slice()? };

    // write each buffer in sequence
    let mut total = 0u64;
    let mut current = offset.0;
    for buffer in buffers {
        let mut local = 0u64;
        unsafe {
            destack_fs_pwrite(
                context,
                &mut local as *mut u64,
                handle,
                *buffer,
                FileOffset(current),
            )?;
        }
        total += local;
        current = add_offset(current, local, "offset")?;
        if local < buffer.len as u64 {
            break;
        }
    }

    // write the output
    unsafe {
        *out = total;
    }

    Ok(())
}

/// Read into multiple buffers at the given file offset with explicit read flags.
///
/// Read bytes into a scatter buffer list at an explicit file offset and apply host read flags.
/// Flag bits are passed through directly and may enable nowait or high-priority reads on supported kernels.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses preadv2(2) on Linux and runtime fallback to preadv on other targets when flags are zero.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_preadv2(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: honor read flags for preadv2 on Windows
    let _ = flags.0;

    unsafe { destack_fs_preadv(context, out, handle, buffers, offset) }
}

/// Write from multiple buffers at the given file offset with explicit write flags.
///
/// Write bytes from a gather buffer list at an explicit file offset and apply host write flags.
/// Flag bits are passed through directly and may enable append, sync, or nowait behavior on supported kernels.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses pwritev2(2) on Linux and runtime fallback to pwritev on other targets when flags are zero.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_pwritev2(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: honor write flags for pwritev2 on Windows
    let _ = flags.0;

    unsafe { destack_fs_pwritev(context, out, handle, buffers, offset) }
}

/// Read from a file into the provided slice.
///
/// Read bytes into one contiguous caller-provided buffer from the current file position.
/// The file position advances by the exact byte count returned by the host.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses read(2) on Unix and ReadFile on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_read(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the handle and cursor
    let cursor = file_resource(context, handle)?;
    let mut guard = cursor.lock();
    let offset = FileOffset(*guard);

    // perform the read
    unsafe { destack_fs_pread(context, out, handle, buffer, offset) }?;

    // update the cursor
    let bytes_read = unsafe { *out };
    *guard = add_offset(*guard, bytes_read, "cursor")?;

    Ok(())
}

/// Write to a file from the provided slice.
///
/// Write bytes from one contiguous caller-provided buffer at the current file position.
/// The file position advances by the exact byte count accepted by the host.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses write(2) on Unix and WriteFile on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_write(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the handle and cursor
    let cursor = file_resource(context, handle)?;
    let mut guard = cursor.lock();
    let offset = FileOffset(*guard);

    // perform the write
    unsafe { destack_fs_pwrite(context, out, handle, buffer, offset) }?;

    // update the cursor
    let bytes_written = unsafe { *out };
    *guard = add_offset(*guard, bytes_written, "cursor")?;

    Ok(())
}

/// Send file data to a socket.
///
/// Transfer file bytes directly from storage-backed pages to a socket endpoint.
/// Host fast-path behavior may bypass user-space copies when supported.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses sendfile(2) on Unix variants and TransmitFile or copy fallback on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_sendfile(
    context: &RuntimeCallContext,
    out: *mut u64,
    socket: SocketHandle,
    file: FileHandle,
    offset: FileOffset,
    length: FileSize,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the socket and file handles
    let socket = super::util::socket_handle(context, socket)?;

    // stream data from the file into the socket
    let mut remaining = length.0;
    let mut total = 0u64;
    let mut file_offset = offset;
    let mut buffer = vec![0u8; 1024 * 1024];
    while remaining > 0 {
        let chunk = remaining.min(buffer.len() as u64) as usize;
        let slice = NativeSlice {
            data: buffer.as_mut_ptr(),
            len: chunk as u32,
        };
        let mut bytes_read = 0u64;
        unsafe { destack_fs_pread(context, &mut bytes_read, file, slice, file_offset) }?;
        if bytes_read == 0 {
            break;
        }

        // send the read bytes to the socket
        let mut sent = 0u64;
        while sent < bytes_read {
            let ptr = unsafe { buffer.as_ptr().add(sent as usize) };
            let rc = unsafe { send(socket, ptr, (bytes_read - sent) as i32, 0) };
            if rc == SOCKET_ERROR {
                return Err(last_socket_error("send"));
            }
            sent = sent.saturating_add(rc as u64);
        }

        total = total.saturating_add(sent);
        file_offset = FileOffset(add_offset(file_offset.0, sent, "offset")?);
        remaining = remaining.saturating_sub(sent);
    }

    unsafe {
        *out = total;
    }

    Ok(())
}

/// Transfer bytes between descriptors using kernel splice pipelines.
///
/// Move bytes between descriptor endpoints and optionally update explicit cursors for each side.
/// This operation is intended for zero-copy file, pipe, and socket data paths where the host supports splice semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses splice(2) on Linux and runtime fallback on targets without splice support.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.zero.copy`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_splice(
    context: &RuntimeCallContext,
    _out: *mut u64,
    source: ResourceId,
    sourcecursor: SpliceCursor,
    target: ResourceId,
    targetcursor: SpliceCursor,
    length: FileSize,
    flags: SpliceFlags,
) -> RuntimeResult<()> {
    let _ = (
        context,
        source,
        sourcecursor,
        target,
        targetcursor,
        length,
        flags,
    );
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.splice")).boxed())
}

/// Duplicate bytes from one pipe to another without consuming source bytes.
///
/// Clone bytes between two pipe descriptors while preserving source pipe contents.
/// This operation is useful for fanout pipelines where consumers share the same byte stream.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses tee(2) on Linux and runtime fallback on targets without tee support.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.zero.copy`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_tee(
    context: &RuntimeCallContext,
    _out: *mut u64,
    sourcepipe: PipeHandle,
    targetpipe: PipeHandle,
    length: FileSize,
    flags: SpliceFlags,
) -> RuntimeResult<()> {
    let _ = (context, sourcepipe, targetpipe, length, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.tee")).boxed())
}

/// Map user memory pages into a pipe as queued pipe buffers.
///
/// Publish one set of user buffers into a pipe endpoint for downstream splice pipelines.
/// Host kernels may pin pages or copy data depending on flags and memory state.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses vmsplice(2) on Linux and runtime fallback on targets without vmsplice support.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.zero.copy`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_vmsplice(
    context: &RuntimeCallContext,
    _out: *mut u64,
    pipe: PipeHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    flags: SpliceFlags,
) -> RuntimeResult<()> {
    let _ = (context, pipe, buffers, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.vmsplice")).boxed())
}

/// Read into multiple buffers.
///
/// Read bytes into a scatter buffer list from the current file position.
/// Buffer fill order follows host iovec semantics and advances the file position by bytes read.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses readv(2) on Unix and vectored file I/O loop on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_readv(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the buffer list
    let buffers = unsafe { buffers.as_slice()? };

    // read each buffer in sequence
    let mut total = 0u64;
    for buffer in buffers {
        let mut local = 0u64;
        unsafe {
            destack_fs_read(context, &mut local as *mut u64, handle, *buffer)?;
        }
        total += local;
        if local < buffer.len as u64 {
            break;
        }
    }

    unsafe {
        *out = total;
    }

    Ok(())
}

/// Write from multiple buffers.
///
/// Write bytes from a gather buffer list at the current file position.
/// Buffer consumption order follows host iovec semantics and advances the file position by bytes written.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses writev(2) on Unix and vectored file I/O loop on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_writev(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the buffer list
    let buffers = unsafe { buffers.as_slice()? };

    // write each buffer in sequence
    let mut total = 0u64;
    for buffer in buffers {
        let mut local = 0u64;
        unsafe {
            destack_fs_write(context, &mut local as *mut u64, handle, *buffer)?;
        }
        total += local;
        if local < buffer.len as u64 {
            break;
        }
    }

    unsafe {
        *out = total;
    }

    Ok(())
}
