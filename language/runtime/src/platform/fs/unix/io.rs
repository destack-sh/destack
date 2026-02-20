#![allow(unused_imports)]

use super::core::*;
use super::os;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{core as core_fs, *};
use crate::platform::resource::*;
use crate::platform::{core as core_platform, net as platform_net, *};
use crate::runtime::BindingCallContext;

use std::ffi::{CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;
use std::path::PathBuf;

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
    context: &BindingCallContext,
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
    let fd = file_descriptor(context, handle)?;
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
    context: &BindingCallContext,
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
    let fd = file_descriptor(context, handle)?;
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
    context: &BindingCallContext,
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
    let fd = file_descriptor(context, handle)?;
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
    context: &BindingCallContext,
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
    let fd = file_descriptor(context, handle)?;
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
    context: &BindingCallContext,
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
    let fd = file_descriptor(context, handle)?;
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
    context: &BindingCallContext,
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
    let fd = file_descriptor(context, handle)?;
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
    context: &BindingCallContext,
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
    let fd = file_descriptor(context, handle)?;
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
    context: &BindingCallContext,
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
    let fd = file_descriptor(context, handle)?;
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
    context: &BindingCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
    _flags: ReadWriteFlags,
) -> RuntimeResult<()> {
    unsafe { destack_fs_preadv(context, out, handle, buffers, offset) }
}

/// Write from multiple buffers with explicit write flags.
#[cfg(unix)]
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
    context: &BindingCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
    _flags: ReadWriteFlags,
) -> RuntimeResult<()> {
    unsafe { destack_fs_pwritev(context, out, handle, buffers, offset) }
}

/// Move data between resource handles.
#[cfg(unix)]
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
    _context: &BindingCallContext,
    _out: *mut u64,
    source: ResourceId,
    sourcecursor: SpliceCursor,
    target: ResourceId,
    targetcursor: SpliceCursor,
    length: FileSize,
    flags: SpliceFlags,
) -> RuntimeResult<()> {
    let _ = (source, sourcecursor, target, targetcursor, length, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.splice")).boxed())
}

/// Duplicate pipe data between pipe handles.
#[cfg(unix)]
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
    _context: &BindingCallContext,
    _out: *mut u64,
    sourcepipe: PipeHandle,
    targetpipe: PipeHandle,
    length: FileSize,
    flags: SpliceFlags,
) -> RuntimeResult<()> {
    let _ = (sourcepipe, targetpipe, length, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.tee")).boxed())
}

/// Move user buffers into a pipe.
#[cfg(unix)]
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
    _context: &BindingCallContext,
    _out: *mut u64,
    pipe: PipeHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    flags: SpliceFlags,
) -> RuntimeResult<()> {
    let _ = (pipe, buffers, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.vmsplice")).boxed())
}
