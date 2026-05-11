#![allow(dead_code)]

use std::ffi::c_void;

use windows_sys::Win32::Foundation::{BOOL, ERROR_IO_PENDING, HANDLE};
use windows_sys::Win32::Networking::WinSock::{
    SOCKET, SOCKET_ERROR, WSA_IO_PENDING, WSAEINVAL, WSAENOPROTOOPT, WSAEOPNOTSUPP,
    WSAGetOverlappedResult, recv, send,
};
use windows_sys::Win32::Storage::FileSystem::{FILE_END, ReadFile, SetFilePointerEx, WriteFile};
use windows_sys::Win32::System::IO::{GetOverlappedResult, OVERLAPPED, OVERLAPPED_0_0};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
use crate::platform::fs::{
    FileHandle, FileOffset, FileSize, ReadWriteFlags, SpliceCursor, SpliceFlags, core as core_fs,
};
use crate::platform::net::SocketHandle;
use crate::platform::resource::{PipeHandle, ResourceId, ResourceKind};
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

#[link(name = "mswsock")]
unsafe extern "system" {
    fn TransmitFile(
        hsocket: SOCKET,
        hfile: HANDLE,
        nnumberofbytestowrite: u32,
        nnumberofbytespersend: u32,
        lpoverlapped: *mut OVERLAPPED,
        lptransmitbuffers: *mut c_void,
        dwflags: u32,
    ) -> BOOL;
}

/// Largest chunk one `TransmitFile` call can describe.
const TRANSMIT_FILE_MAX_CHUNK: u64 = u32::MAX as u64;

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

/// Return whether one `TransmitFile` failure should fall back to the copy loop.
fn should_fallback_from_transmit_file(error: i32) -> bool {
    matches!(error, WSAEINVAL | WSAEOPNOTSUPP | WSAENOPROTOOPT)
}

/// Send file contents through the existing read and send fallback.
fn sendfile_copy_fallback(
    binding: &BindingCallContext,
    socket: SOCKET,
    file: FileHandle,
    offset: FileOffset,
    length: FileSize,
) -> RuntimeResult<u64> {
    let mut remaining = length.0;
    let mut total = 0u64;
    let mut file_offset = offset;
    let buffer_length = core_fs::copy_fallback_buffer_length(length.0);
    let mut buffer = vec![0u8; buffer_length];

    while remaining > 0 {
        let chunk = remaining.min(buffer.len() as u64) as usize;
        let slice = NativeSlice {
            data: buffer.as_mut_ptr(),
            len: chunk as u32,
        };
        let mut bytes_read = 0u64;
        unsafe { destack_fs_pread(binding, &mut bytes_read, file, slice, file_offset) }?;
        if bytes_read == 0 {
            break;
        }

        // send the full read chunk before advancing the file offset
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

    Ok(total)
}

/// Splice endpoint kinds supported by the Windows fallback path.
enum SpliceEndpoint {
    /// File endpoint mapped through file bindings.
    File(FileHandle),
    /// Pipe endpoint backed by one raw Windows handle.
    Pipe(isize),
    /// Socket endpoint backed by one raw Winsock socket.
    Socket(SOCKET),
}

/// Resolve one splice endpoint from one generic resource id.
fn splice_endpoint(
    binding: &BindingCallContext,
    resource: ResourceId,
    label: &str,
) -> RuntimeResult<SpliceEndpoint> {
    // resolve one resource entry and map it into a supported endpoint
    let endpoint = binding.worker().resources.with_entry(resource, |entry| {
        if entry.kind == ResourceKind::File {
            return Some(SpliceEndpoint::File(FileHandle(resource)));
        }
        if entry.kind == ResourceKind::Pipe {
            return entry
                .handle()
                .map(|handle| SpliceEndpoint::Pipe(handle as isize));
        }
        if entry.kind == ResourceKind::Socket {
            return entry
                .socket()
                .map(|socket| SpliceEndpoint::Socket(socket as SOCKET));
        }

        None
    });

    // reject unsupported endpoint kinds
    let Some(endpoint) = endpoint.flatten() else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "resource is not one file, pipe, or socket endpoint",
        ))
        .boxed());
    };

    Ok(endpoint)
}

/// Read bytes from one splice endpoint into one caller buffer.
fn splice_read(
    binding: &BindingCallContext,
    endpoint: &SpliceEndpoint,
    buffer: &mut [u8],
    source_offset: &mut Option<i64>,
) -> RuntimeResult<u64> {
    // dispatch one read path by endpoint kind
    match endpoint {
        SpliceEndpoint::File(handle) => {
            // use positioned reads when one explicit source offset is present
            if let Some(offset) = source_offset {
                let slice = NativeSlice {
                    data: buffer.as_mut_ptr(),
                    len: buffer.len() as u32,
                };
                let mut bytes_read = 0u64;
                unsafe {
                    destack_fs_pread(
                        binding,
                        &mut bytes_read,
                        *handle,
                        slice,
                        FileOffset(*offset),
                    )?;
                }
                *offset = add_offset(*offset, bytes_read, "sourceCursor.offset")?;

                return Ok(bytes_read);
            }

            // otherwise read from the current file cursor
            let slice = NativeSlice {
                data: buffer.as_mut_ptr(),
                len: buffer.len() as u32,
            };
            let mut bytes_read = 0u64;
            unsafe {
                destack_fs_read(binding, &mut bytes_read, *handle, slice)?;
            }

            Ok(bytes_read)
        }
        SpliceEndpoint::Pipe(handle) => {
            // reject offset cursors for pipes
            if source_offset.is_some() {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "sourceCursor",
                    "offset cursor is not supported for pipe resources",
                ))
                .boxed());
            }

            // issue one pipe read through ReadFile
            let mut bytes_read = 0u32;
            let rc = unsafe {
                ReadFile(
                    *handle,
                    buffer.as_mut_ptr() as *mut _,
                    buffer.len() as u32,
                    &mut bytes_read,
                    std::ptr::null_mut(),
                )
            };
            if rc == 0 {
                return Err(last_os_error("ReadFile", None));
            }

            Ok(bytes_read as u64)
        }
        SpliceEndpoint::Socket(socket) => {
            // reject offset cursors for sockets
            if source_offset.is_some() {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "sourceCursor",
                    "offset cursor is not supported for socket resources",
                ))
                .boxed());
            }

            // issue one socket receive
            let rc = unsafe { recv(*socket, buffer.as_mut_ptr(), buffer.len() as i32, 0) };
            if rc == SOCKET_ERROR {
                return Err(last_socket_error("recv"));
            }

            Ok(rc as u64)
        }
    }
}

/// Write bytes to one splice endpoint from one caller buffer.
fn splice_write(
    binding: &BindingCallContext,
    endpoint: &SpliceEndpoint,
    buffer: &[u8],
    target_offset: &mut Option<i64>,
) -> RuntimeResult<u64> {
    // dispatch one write path by endpoint kind
    match endpoint {
        SpliceEndpoint::File(handle) => {
            // use positioned writes when one explicit target offset is present
            if let Some(offset) = target_offset {
                let slice = NativeSlice {
                    data: buffer.as_ptr() as *mut u8,
                    len: buffer.len() as u32,
                };
                let mut bytes_written = 0u64;
                unsafe {
                    destack_fs_pwrite(
                        binding,
                        &mut bytes_written,
                        *handle,
                        slice,
                        FileOffset(*offset),
                    )?;
                }
                *offset = add_offset(*offset, bytes_written, "targetCursor.offset")?;

                return Ok(bytes_written);
            }

            // otherwise write at the current file cursor
            let slice = NativeSlice {
                data: buffer.as_ptr() as *mut u8,
                len: buffer.len() as u32,
            };
            let mut bytes_written = 0u64;
            unsafe {
                destack_fs_write(binding, &mut bytes_written, *handle, slice)?;
            }

            Ok(bytes_written)
        }
        SpliceEndpoint::Pipe(handle) => {
            // reject offset cursors for pipes
            if target_offset.is_some() {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "targetCursor",
                    "offset cursor is not supported for pipe resources",
                ))
                .boxed());
            }

            // issue one pipe write through WriteFile
            let mut bytes_written = 0u32;
            let rc = unsafe {
                WriteFile(
                    *handle,
                    buffer.as_ptr() as *const _,
                    buffer.len() as u32,
                    &mut bytes_written,
                    std::ptr::null_mut(),
                )
            };
            if rc == 0 {
                return Err(last_os_error("WriteFile", None));
            }

            Ok(bytes_written as u64)
        }
        SpliceEndpoint::Socket(socket) => {
            // reject offset cursors for sockets
            if target_offset.is_some() {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "targetCursor",
                    "offset cursor is not supported for socket resources",
                ))
                .boxed());
            }

            // issue one socket send
            let rc = unsafe { send(*socket, buffer.as_ptr(), buffer.len() as i32, 0) };
            if rc == SOCKET_ERROR {
                return Err(last_socket_error("send"));
            }

            Ok(rc as u64)
        }
    }
}

/// Read from a file at the given file offset.
pub(crate) unsafe fn destack_fs_pread(
    binding: &BindingCallContext,
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
    let handle = file_handle(binding, handle)?;
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
pub(crate) unsafe fn destack_fs_pwrite(
    binding: &BindingCallContext,
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
    let handle = file_handle(binding, handle)?;
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
pub(crate) unsafe fn destack_fs_preadv(
    binding: &BindingCallContext,
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
                binding,
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
pub(crate) unsafe fn destack_fs_pwritev(
    binding: &BindingCallContext,
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
                binding,
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
pub(crate) unsafe fn destack_fs_preadv2(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<()> {
    // reject non zero flags on windows where preadv2 flags are unavailable
    if flags.0 != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.fs.file.preadv2")).boxed(),
        );
    }

    unsafe { destack_fs_preadv(binding, out, handle, buffers, offset) }
}

/// Write from multiple buffers at the given file offset with explicit write flags.
pub(crate) unsafe fn destack_fs_pwritev2(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<()> {
    // reject non zero flags on windows where pwritev2 flags are unavailable
    if flags.0 != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.fs.file.pwritev2")).boxed(),
        );
    }

    unsafe { destack_fs_pwritev(binding, out, handle, buffers, offset) }
}

/// Read from a file into the provided slice.
pub(crate) unsafe fn destack_fs_read(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // use tracked cursor when the resource carries file state
    if let Ok(cursor) = file_resource(binding, handle) {
        let mut guard = cursor.lock();
        let offset = FileOffset(*guard);

        unsafe { destack_fs_pread(binding, out, handle, buffer, offset) }?;

        let bytes_read = unsafe { *out };
        *guard = add_offset(*guard, bytes_read, "cursor")?;
        return Ok(());
    }

    // fall back to stream style reads for untracked handles
    let handle = file_handle(binding, handle)?;
    let buffer = unsafe { buffer.as_mut_slice()? };
    let mut bytes_read = 0_u32;
    let rc = unsafe {
        ReadFile(
            handle,
            buffer.as_mut_ptr() as *mut _,
            buffer.len() as u32,
            &mut bytes_read,
            std::ptr::null_mut(),
        )
    };
    if rc == 0 {
        return Err(last_os_error("ReadFile", None));
    }

    unsafe {
        *out = bytes_read as u64;
    }

    Ok(())
}

/// Write to a file from the provided slice.
pub(crate) unsafe fn destack_fs_write(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // use tracked cursor when the resource carries file state
    if let Ok((cursor, status_flags)) = file_state(binding, handle) {
        let mut cursor = cursor.lock();
        let status_flags = *status_flags.lock();

        // append mode writes always target the current end of file
        let offset = if status_flags & libc::O_APPEND as u32 != 0 {
            let handle = file_handle(binding, handle)?;
            let mut end = 0i64;
            let rc = unsafe { SetFilePointerEx(handle, 0, &mut end, FILE_END) };
            if rc == 0 {
                return Err(last_os_error("SetFilePointerEx", None));
            }
            FileOffset(end)
        } else {
            FileOffset(*cursor)
        };

        unsafe { destack_fs_pwrite(binding, out, handle, buffer, offset) }?;

        let bytes_written = unsafe { *out };
        *cursor = add_offset(offset.0, bytes_written, "cursor")?;
        return Ok(());
    }

    // fall back to stream style writes for untracked handles
    let handle = file_handle(binding, handle)?;
    let buffer = unsafe { buffer.as_slice()? };
    let mut bytes_written = 0_u32;
    let rc = unsafe {
        WriteFile(
            handle,
            buffer.as_ptr() as *const _,
            buffer.len() as u32,
            &mut bytes_written,
            std::ptr::null_mut(),
        )
    };
    if rc == 0 {
        return Err(last_os_error("WriteFile", None));
    }

    unsafe {
        *out = bytes_written as u64;
    }

    Ok(())
}

/// Send file data to a socket.
pub(crate) unsafe fn destack_fs_sendfile(
    binding: &BindingCallContext,
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
    let socket = socket_handle(binding, socket)?;
    let file_handle = file_handle(binding, file)?;

    // prefer one real windows file-to-socket path before falling back
    let mut remaining = length.0;
    let mut total = 0u64;
    let mut file_offset = offset.0;
    while remaining > 0 {
        let chunk = remaining.min(TRANSMIT_FILE_MAX_CHUNK) as u32;
        let mut overlapped = OVERLAPPED {
            Internal: 0,
            InternalHigh: 0,
            Anonymous: windows_sys::Win32::System::IO::OVERLAPPED_0 {
                Anonymous: OVERLAPPED_0_0 {
                    Offset: file_offset as u32,
                    OffsetHigh: (file_offset >> 32) as u32,
                },
            },
            hEvent: 0,
        };

        let started = unsafe {
            TransmitFile(
                socket,
                file_handle,
                chunk,
                0,
                &mut overlapped,
                std::ptr::null_mut(),
                0,
            )
        };
        if started == 0 {
            let error = core_platform::last_wsa_error_code();
            if total == 0 && should_fallback_from_transmit_file(error) {
                let total = sendfile_copy_fallback(
                    binding,
                    socket,
                    file,
                    FileOffset(file_offset),
                    FileSize(remaining),
                )?;
                unsafe {
                    *out = total;
                }
                return Ok(());
            }

            if error != WSA_IO_PENDING {
                return Err(last_socket_error("TransmitFile"));
            }
        }

        let mut transferred = 0u32;
        let mut flags = 0u32;
        let completed =
            unsafe { WSAGetOverlappedResult(socket, &overlapped, &mut transferred, 1, &mut flags) };
        if completed == 0 {
            return Err(last_socket_error("WSAGetOverlappedResult"));
        }
        if transferred == 0 {
            break;
        }

        total = total.saturating_add(transferred as u64);
        file_offset = add_offset(file_offset, transferred as u64, "offset")?;
        remaining = remaining.saturating_sub(transferred as u64);
    }

    unsafe {
        *out = total;
    }

    Ok(())
}

/// Transfer bytes between descriptors using kernel splice pipelines.
pub(crate) unsafe fn destack_fs_splice(
    binding: &BindingCallContext,
    out: *mut u64,
    source: ResourceId,
    sourcecursor: SpliceCursor,
    target: ResourceId,
    targetcursor: SpliceCursor,
    length: FileSize,
    flags: SpliceFlags,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // reject splice flags that this fallback cannot honor
    if flags.0 != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.fs.file.splice")).boxed(),
        );
    }

    // resolve source and target endpoints
    let source_endpoint = splice_endpoint(binding, source, "source")?;
    let target_endpoint = splice_endpoint(binding, target, "target")?;

    // run one read/write copy loop up to the requested length
    let mut source_offset = sourcecursor.offset.map(|value| value.0);
    let mut target_offset = targetcursor.offset.map(|value| value.0);
    let mut remaining = length.0;
    let mut total = 0u64;
    let buffer_length = core_fs::copy_fallback_buffer_length(length.0);
    let mut buffer = vec![0u8; buffer_length];
    while remaining > 0 {
        // read one source chunk
        let chunk = remaining.min(buffer.len() as u64) as usize;
        let bytes_read = splice_read(
            binding,
            &source_endpoint,
            &mut buffer[..chunk],
            &mut source_offset,
        )?;
        if bytes_read == 0 {
            break;
        }

        // write the chunk fully to the target endpoint
        let mut written = 0usize;
        let expected = bytes_read as usize;
        while written < expected {
            let bytes_written = splice_write(
                binding,
                &target_endpoint,
                &buffer[written..expected],
                &mut target_offset,
            )?;
            if bytes_written == 0 {
                return Err(RuntimeError::from(PlatformError::io(
                    "splice fallback write returned zero bytes".to_string(),
                ))
                .boxed());
            }
            written = written.saturating_add(bytes_written as usize);
        }

        total = total.saturating_add(bytes_read);
        remaining = remaining.saturating_sub(bytes_read);
    }

    // write one output byte count
    unsafe {
        *out = total;
    }

    Ok(())
}

/// Duplicate bytes from one pipe to another without consuming source bytes.
pub(crate) unsafe fn destack_fs_tee(
    binding: &BindingCallContext,
    _out: *mut u64,
    sourcepipe: PipeHandle,
    targetpipe: PipeHandle,
    length: FileSize,
    flags: SpliceFlags,
) -> RuntimeResult<()> {
    let _ = (binding, sourcepipe, targetpipe, length, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.tee")).boxed())
}

/// Map user memory pages into a pipe as queued pipe buffers.
pub(crate) unsafe fn destack_fs_vmsplice(
    binding: &BindingCallContext,
    _out: *mut u64,
    pipe: PipeHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    flags: SpliceFlags,
) -> RuntimeResult<()> {
    let _ = (binding, pipe, buffers, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.vmsplice")).boxed())
}

/// Read into multiple buffers.
pub(crate) unsafe fn destack_fs_readv(
    binding: &BindingCallContext,
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
            destack_fs_read(binding, &mut local as *mut u64, handle, *buffer)?;
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
pub(crate) unsafe fn destack_fs_writev(
    binding: &BindingCallContext,
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
            destack_fs_write(binding, &mut local as *mut u64, handle, *buffer)?;
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
