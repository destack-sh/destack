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

/// Read from a file handle at a specific offset.
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

/// Write to a file handle at a specific offset.
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

/// Read from a file handle into multiple buffers at a specific offset.
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
        current += local;
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

/// Write to a file handle from multiple buffers at a specific offset.
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
        current += local;
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

/// Read from a file handle into multiple buffers with explicit read flags.
pub(crate) unsafe fn destack_fs_preadv2(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: honor read flags for preadv2 on Windows
    let _ = flags;

    unsafe { destack_fs_preadv(context, out, handle, buffers, offset) }
}

/// Write to a file handle from multiple buffers with explicit write flags.
pub(crate) unsafe fn destack_fs_pwritev2(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: honor write flags for pwritev2 on Windows
    let _ = flags;

    unsafe { destack_fs_pwritev(context, out, handle, buffers, offset) }
}

/// Read from a file handle using the current file offset.
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
    *guard = guard.saturating_add(bytes_read);

    Ok(())
}

/// Write to a file handle using the current file offset.
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
    *guard = guard.saturating_add(bytes_written);

    Ok(())
}

/// Send file data to a socket.
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
        file_offset = FileOffset(file_offset.0.saturating_add(sent));
        remaining = remaining.saturating_sub(sent);
    }

    unsafe {
        *out = total;
    }

    Ok(())
}

/// Move data between resources using in-kernel transfer paths.
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

/// Duplicate pipe data without copying into userspace.
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

/// Move user buffers into a pipe.
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

/// Read from a file handle into multiple buffers.
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

/// Write to a file handle from multiple buffers.
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
