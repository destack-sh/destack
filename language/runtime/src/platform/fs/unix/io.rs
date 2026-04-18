use super::core::*;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
#[cfg(not(target_os = "linux"))]
use crate::platform::fs::core as core_fs;
use crate::platform::fs::*;
use crate::platform::resource::*;
use crate::platform::{core as core_platform, *};
use crate::runtime::BindingCallContext;

use std::os::unix::io::RawFd;

#[cfg(target_os = "linux")]
fn read_write_flags_to_i32(flags: ReadWriteFlags, argument: &'static str) -> RuntimeResult<i32> {
    i32::try_from(flags.0).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            argument,
            "flags exceed supported range",
        ))
        .boxed()
    })
}

/// Splice flag bit for page move hint.
#[cfg(target_os = "linux")]
const SPLICE_FLAG_MOVE: u32 = 0x1;
/// Splice flag bit for nonblocking mode.
#[cfg(target_os = "linux")]
const SPLICE_FLAG_NONBLOCK: u32 = 0x2;
/// Splice flag bit for more data hint.
#[cfg(target_os = "linux")]
const SPLICE_FLAG_MORE: u32 = 0x4;
/// Splice flag bit for gift page semantics.
#[cfg(target_os = "linux")]
const SPLICE_FLAG_GIFT: u32 = 0x8;
/// Bitmask of all supported splice flag bits.
#[cfg(target_os = "linux")]
const SPLICE_SUPPORTED_FLAGS: u32 =
    SPLICE_FLAG_MOVE | SPLICE_FLAG_NONBLOCK | SPLICE_FLAG_MORE | SPLICE_FLAG_GIFT;

/// Resolve one descriptor endpoint for splice-style operations.
fn splice_descriptor(
    binding: &BindingCallContext,
    handle: ResourceId,
    label: &str,
) -> RuntimeResult<RawFd> {
    // resolve the resource and enforce splice-compatible kinds
    let resolved = binding
        .worker()
        .resources
        .with_entry(handle, |entry| {
            if !matches!(
                entry.kind,
                ResourceKind::File | ResourceKind::Pipe | ResourceKind::Socket
            ) {
                return None;
            }
            entry.fd()
        })
        .flatten();

    // return the descriptor or fail explicitly
    let Some(fd) = resolved else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            format!("unknown {label} handle"),
        ))
        .boxed());
    };

    Ok(fd)
}

/// Convert one splice flag bitset into host syscall flags.
#[cfg(target_os = "linux")]
fn splice_flags_to_native(flags: SpliceFlags, argument: &'static str) -> RuntimeResult<u32> {
    // reject unknown flag bits
    let unsupported_flags = flags.0 & !SPLICE_SUPPORTED_FLAGS;
    if unsupported_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            argument,
            format!("unsupported splice flags: {unsupported_flags:#x}"),
        ))
        .boxed());
    }

    // map runtime bits into linux splice flags
    let mut native_flags = 0u32;
    if (flags.0 & SPLICE_FLAG_MOVE) != 0 {
        native_flags |= libc::SPLICE_F_MOVE;
    }
    if (flags.0 & SPLICE_FLAG_NONBLOCK) != 0 {
        native_flags |= libc::SPLICE_F_NONBLOCK;
    }
    if (flags.0 & SPLICE_FLAG_MORE) != 0 {
        native_flags |= libc::SPLICE_F_MORE;
    }
    if (flags.0 & SPLICE_FLAG_GIFT) != 0 {
        native_flags |= libc::SPLICE_F_GIFT;
    }

    Ok(native_flags)
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
    binding: &BindingCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the target buffer
    let buffer = unsafe { buffer.as_mut_slice()? };

    // read from the file on unix platforms
    let fd = file_descriptor(binding, handle)?;
    let rc = unsafe { libc::read(fd, buffer.as_mut_ptr() as *mut libc::c_void, buffer.len()) };
    if rc < 0 {
        return Err(core_platform::io_error("read", None));
    }

    unsafe {
        *out = rc as u64;
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the target buffer
    let buffer = unsafe { buffer.as_mut_slice()? };

    // read from the file on unix platforms
    let fd = file_descriptor(binding, handle)?;
    let offset = offset_to_off_t(offset)?;
    let rc = unsafe {
        libc::pread(
            fd,
            buffer.as_mut_ptr() as *mut libc::c_void,
            buffer.len(),
            offset,
        )
    };
    if rc < 0 {
        return Err(core_platform::io_error("pread", None));
    }

    unsafe {
        *out = rc as u64;
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the buffer list
    let buffers = unsafe { buffers.as_slice()? };

    // read from the file on unix platforms
    let fd = file_descriptor(binding, handle)?;
    let mut iovecs = Vec::with_capacity(buffers.len());
    for buffer in buffers {
        let slice = unsafe { buffer.as_mut_slice()? };
        iovecs.push(libc::iovec {
            iov_base: slice.as_mut_ptr() as *mut libc::c_void,
            iov_len: slice.len(),
        });
    }
    let rc = unsafe { libc::readv(fd, iovecs.as_ptr(), iovecs.len() as libc::c_int) };
    if rc < 0 {
        return Err(core_platform::io_error("readv", None));
    }

    unsafe {
        *out = rc as u64;
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
    binding: &BindingCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the buffer list
    let buffers = unsafe { buffers.as_slice()? };

    // read from the file on unix platforms
    let fd = file_descriptor(binding, handle)?;
    let offset = offset_to_off_t(offset)?;
    let mut iovecs = Vec::with_capacity(buffers.len());
    for buffer in buffers {
        let slice = unsafe { buffer.as_mut_slice()? };
        iovecs.push(libc::iovec {
            iov_base: slice.as_mut_ptr() as *mut libc::c_void,
            iov_len: slice.len(),
        });
    }
    let rc = unsafe { libc::preadv(fd, iovecs.as_ptr(), iovecs.len() as libc::c_int, offset) };
    if rc < 0 {
        return Err(core_platform::io_error("preadv", None));
    }

    unsafe {
        *out = rc as u64;
    }

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
    binding: &BindingCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the source buffer
    let buffer = unsafe { buffer.as_slice()? };

    // write to the file on unix platforms
    let fd = file_descriptor(binding, handle)?;
    let rc = unsafe { libc::write(fd, buffer.as_ptr() as *const libc::c_void, buffer.len()) };
    if rc < 0 {
        return Err(core_platform::io_error("write", None));
    }

    unsafe {
        *out = rc as u64;
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
    binding: &BindingCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the source buffer
    let buffer = unsafe { buffer.as_slice()? };

    // write to the file on unix platforms
    let fd = file_descriptor(binding, handle)?;
    let offset = offset_to_off_t(offset)?;
    let rc = unsafe {
        libc::pwrite(
            fd,
            buffer.as_ptr() as *const libc::c_void,
            buffer.len(),
            offset,
        )
    };
    if rc < 0 {
        return Err(core_platform::io_error("pwrite", None));
    }

    unsafe {
        *out = rc as u64;
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
    binding: &BindingCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the buffer list
    let buffers = unsafe { buffers.as_slice()? };

    // write to the file on unix platforms
    let fd = file_descriptor(binding, handle)?;
    let mut iovecs = Vec::with_capacity(buffers.len());
    for buffer in buffers {
        let slice = unsafe { buffer.as_slice()? };
        iovecs.push(libc::iovec {
            iov_base: slice.as_ptr() as *mut libc::c_void,
            iov_len: slice.len(),
        });
    }
    let rc = unsafe { libc::writev(fd, iovecs.as_ptr(), iovecs.len() as libc::c_int) };
    if rc < 0 {
        return Err(core_platform::io_error("writev", None));
    }

    unsafe {
        *out = rc as u64;
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
    binding: &BindingCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the buffer list
    let buffers = unsafe { buffers.as_slice()? };

    // write to the file on unix platforms
    let fd = file_descriptor(binding, handle)?;
    let offset = offset_to_off_t(offset)?;
    let mut iovecs = Vec::with_capacity(buffers.len());
    for buffer in buffers {
        let slice = unsafe { buffer.as_slice()? };
        iovecs.push(libc::iovec {
            iov_base: slice.as_ptr() as *mut libc::c_void,
            iov_len: slice.len(),
        });
    }
    let rc = unsafe { libc::pwritev(fd, iovecs.as_ptr(), iovecs.len() as libc::c_int, offset) };
    if rc < 0 {
        return Err(core_platform::io_error("pwritev", None));
    }

    unsafe {
        *out = rc as u64;
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
    binding: &BindingCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<()> {
    // use native preadv2 on linux where the host supports flags
    #[cfg(target_os = "linux")]
    {
        // ensure the output pointer is valid
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }

        // resolve the buffer list
        let buffers = unsafe { buffers.as_slice()? };

        // read from the file on linux platforms
        let fd = file_descriptor(binding, handle)?;
        let offset = offset_to_off_t(offset)?;
        let flags = read_write_flags_to_i32(flags, "flags")?;
        let mut iovecs = Vec::with_capacity(buffers.len());
        for buffer in buffers {
            let slice = unsafe { buffer.as_mut_slice()? };
            iovecs.push(libc::iovec {
                iov_base: slice.as_mut_ptr() as *mut libc::c_void,
                iov_len: slice.len(),
            });
        }
        let rc = unsafe {
            libc::preadv2(
                fd,
                iovecs.as_ptr(),
                iovecs.len() as libc::c_int,
                offset,
                flags,
            )
        };
        if rc < 0 {
            return Err(core_platform::io_error("preadv2", None));
        }

        unsafe {
            *out = rc as u64;
        }

        Ok(())
    }

    // reject non zero flags on unix targets without preadv2 support
    #[cfg(not(target_os = "linux"))]
    {
        if flags.0 != 0 {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.fs.file.preadv2",
            ))
            .boxed());
        }

        unsafe { destack_fs_preadv(binding, out, handle, buffers, offset) }
    }
}

/// Write from multiple buffers with explicit write flags.
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
    binding: &BindingCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<()> {
    // use native pwritev2 on linux where the host supports flags
    #[cfg(target_os = "linux")]
    {
        // ensure the output pointer is valid
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }

        // resolve the buffer list
        let buffers = unsafe { buffers.as_slice()? };

        // write to the file on linux platforms
        let fd = file_descriptor(binding, handle)?;
        let offset = offset_to_off_t(offset)?;
        let flags = read_write_flags_to_i32(flags, "flags")?;
        let mut iovecs = Vec::with_capacity(buffers.len());
        for buffer in buffers {
            let slice = unsafe { buffer.as_slice()? };
            iovecs.push(libc::iovec {
                iov_base: slice.as_ptr() as *mut libc::c_void,
                iov_len: slice.len(),
            });
        }
        let rc = unsafe {
            libc::pwritev2(
                fd,
                iovecs.as_ptr(),
                iovecs.len() as libc::c_int,
                offset,
                flags,
            )
        };
        if rc < 0 {
            return Err(core_platform::io_error("pwritev2", None));
        }

        unsafe {
            *out = rc as u64;
        }

        Ok(())
    }

    // reject non zero flags on unix targets without pwritev2 support
    #[cfg(not(target_os = "linux"))]
    {
        if flags.0 != 0 {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.fs.file.pwritev2",
            ))
            .boxed());
        }

        unsafe { destack_fs_pwritev(binding, out, handle, buffers, offset) }
    }
}

/// Move data between resource handles.
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

    // run one read-write fallback on non-linux unix targets
    #[cfg(not(target_os = "linux"))]
    {
        // reject unsupported fallback flags explicitly
        if flags.0 != 0 {
            return Err(
                RuntimeError::from(PlatformError::not_supported("destack.fs.file.splice")).boxed(),
            );
        }

        // resolve source and target descriptors
        let source_fd = splice_descriptor(binding, source, "source")?;
        let target_fd = splice_descriptor(binding, target, "target")?;

        // initialize cursor state and transfer buffer
        let mut source_offset = sourcecursor.offset.map(|value| value.0);
        let mut target_offset = targetcursor.offset.map(|value| value.0);
        let mut remaining = length.0;
        let mut total = 0u64;
        let buffer_length = core_fs::copy_fallback_buffer_length(length.0);
        let mut buffer = vec![0u8; buffer_length];
        while remaining > 0 {
            // read one source chunk
            let chunk = remaining.min(buffer.len() as u64) as usize;
            let bytes_read = if let Some(offset) = source_offset.as_mut() {
                let rc = unsafe {
                    libc::pread(
                        source_fd,
                        buffer.as_mut_ptr() as *mut libc::c_void,
                        chunk,
                        *offset,
                    )
                };
                if rc < 0 {
                    return Err(core_platform::io_error("pread", None));
                }
                let bytes = rc as u64;
                *offset = offset.checked_add(bytes as i64).ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "sourceCursor.offset",
                        "source cursor overflow",
                    ))
                    .boxed()
                })?;
                bytes
            } else {
                let rc = unsafe {
                    libc::read(source_fd, buffer.as_mut_ptr() as *mut libc::c_void, chunk)
                };
                if rc < 0 {
                    return Err(core_platform::io_error("read", None));
                }
                rc as u64
            };
            if bytes_read == 0 {
                break;
            }

            // write one full chunk to the target
            let mut written = 0usize;
            let expected = bytes_read as usize;
            while written < expected {
                let bytes_written = if let Some(offset) = target_offset.as_mut() {
                    let rc = unsafe {
                        libc::pwrite(
                            target_fd,
                            buffer[written..expected].as_ptr() as *const libc::c_void,
                            expected - written,
                            *offset,
                        )
                    };
                    if rc < 0 {
                        return Err(core_platform::io_error("pwrite", None));
                    }
                    let bytes = rc as u64;
                    *offset = offset.checked_add(bytes as i64).ok_or_else(|| {
                        RuntimeError::from(PlatformError::invalid_argument_value(
                            "targetCursor.offset",
                            "target cursor overflow",
                        ))
                        .boxed()
                    })?;
                    bytes
                } else {
                    let rc = unsafe {
                        libc::write(
                            target_fd,
                            buffer[written..expected].as_ptr() as *const libc::c_void,
                            expected - written,
                        )
                    };
                    if rc < 0 {
                        return Err(core_platform::io_error("write", None));
                    }
                    rc as u64
                };
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

        // write byte count output
        unsafe {
            *out = total;
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        // resolve descriptors and syscall flags
        let source_fd = splice_descriptor(binding, source, "source")?;
        let target_fd = splice_descriptor(binding, target, "target")?;
        let native_flags = splice_flags_to_native(flags, "flags")?;
        let transfer_length = usize::try_from(length.0).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "length",
                "length exceeds host range",
            ))
            .boxed()
        })?;

        // convert optional source cursor
        let mut source_offset = if let Some(offset) = sourcecursor.offset {
            offset_to_off_t(offset)?
        } else {
            0
        };
        let source_offset_pointer = if sourcecursor.offset.is_some() {
            &mut source_offset as *mut libc::off_t
        } else {
            std::ptr::null_mut()
        };

        // convert optional target cursor
        let mut target_offset = if let Some(offset) = targetcursor.offset {
            offset_to_off_t(offset)?
        } else {
            0
        };
        let target_offset_pointer = if targetcursor.offset.is_some() {
            &mut target_offset as *mut libc::off_t
        } else {
            std::ptr::null_mut()
        };

        // run one splice transfer
        let transferred = unsafe {
            libc::splice(
                source_fd,
                source_offset_pointer,
                target_fd,
                target_offset_pointer,
                transfer_length,
                native_flags as libc::c_uint,
            )
        };
        if transferred < 0 {
            return Err(core_platform::io_error("splice", None));
        }

        // write byte count output
        unsafe {
            *out = transferred as u64;
        }

        Ok(())
    }
}

/// Duplicate pipe data between pipe handles.
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
    binding: &BindingCallContext,
    out: *mut u64,
    sourcepipe: PipeHandle,
    targetpipe: PipeHandle,
    length: FileSize,
    flags: SpliceFlags,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // reject non-linux targets explicitly
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, sourcepipe, targetpipe, length, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.tee")).boxed())
    }

    #[cfg(target_os = "linux")]
    {
        // resolve descriptors and syscall flags
        let source_fd = splice_descriptor(binding, sourcepipe.0, "source pipe")?;
        let target_fd = splice_descriptor(binding, targetpipe.0, "target pipe")?;
        let native_flags = splice_flags_to_native(flags, "flags")?;
        let transfer_length = usize::try_from(length.0).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "length",
                "length exceeds host range",
            ))
            .boxed()
        })?;

        // run one tee transfer
        let transferred = unsafe {
            libc::tee(
                source_fd,
                target_fd,
                transfer_length,
                native_flags as libc::c_uint,
            )
        };
        if transferred < 0 {
            return Err(core_platform::io_error("tee", None));
        }

        // write byte count output
        unsafe {
            *out = transferred as u64;
        }

        Ok(())
    }
}

/// Move user buffers into a pipe.
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
    binding: &BindingCallContext,
    out: *mut u64,
    pipe: PipeHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    flags: SpliceFlags,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // reject non-linux targets explicitly
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, pipe, buffers, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.vmsplice")).boxed())
    }

    #[cfg(target_os = "linux")]
    {
        // decode the caller iovec list
        let segments = unsafe { buffers.as_slice()? };
        let mut iovecs = Vec::with_capacity(segments.len());
        for segment in segments {
            let segment = unsafe { segment.as_slice()? };
            iovecs.push(libc::iovec {
                iov_base: segment.as_ptr() as *mut libc::c_void,
                iov_len: segment.len(),
            });
        }

        // resolve the pipe descriptor and syscall flags
        let pipe_fd = splice_descriptor(binding, pipe.0, "pipe")?;
        let native_flags = splice_flags_to_native(flags, "flags")?;

        // run one vmsplice operation
        let transferred = unsafe {
            libc::vmsplice(
                pipe_fd,
                iovecs.as_ptr(),
                iovecs.len() as libc::size_t,
                native_flags as libc::c_uint,
            )
        };
        if transferred < 0 {
            return Err(core_platform::io_error("vmsplice", None));
        }

        // write byte count output
        unsafe {
            *out = transferred as u64;
        }

        Ok(())
    }
}
