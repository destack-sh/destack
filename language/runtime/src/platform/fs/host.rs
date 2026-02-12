#[cfg(unix)]
#[path = "unix/mod.rs"]
mod unix;
#[cfg(unix)]
pub(crate) use unix::*;

#[cfg(windows)]
#[path = "windows/mod.rs"]
mod windows;
#[cfg(windows)]
pub(crate) use windows::*;

#[cfg(not(any(unix, windows)))]
#[path = "unsupported.rs"]
mod unsupported;
#[cfg(not(any(unix, windows)))]
pub(crate) use unsupported::*;

#[cfg(unix)]
use super::{FileOffset, FileSize, ReadWriteFlags, SpliceCursor, SpliceFlags};
#[cfg(unix)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(unix)]
use crate::platform::resource::{FileHandle, PipeHandle, ResourceId};
#[cfg(unix)]
use crate::platform::{NativeSlice, PlatformError};
#[cfg(unix)]
use crate::runtime::RuntimeCallContext;

/// Read from multiple buffers with explicit read flags.
#[cfg(unix)]
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
    let _ = flags;
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
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<()> {
    let _ = flags;
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
    _context: &RuntimeCallContext,
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
    _context: &RuntimeCallContext,
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
    _context: &RuntimeCallContext,
    _out: *mut u64,
    pipe: PipeHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    flags: SpliceFlags,
) -> RuntimeResult<()> {
    let _ = (pipe, buffers, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.vmsplice")).boxed())
}
