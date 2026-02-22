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
    context: &BindingCallContext,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    advice: FileAdvice,
) -> RuntimeResult<()> {
    // resolve the file descriptor
    let fd = file_descriptor(context, handle)?;
    let offset = offset_to_off_t(offset)?;
    let length = length.0 as libc::off_t;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let advice = match advice {
            FileAdvice::Normal => libc::POSIX_FADV_NORMAL,
            FileAdvice::Sequential => libc::POSIX_FADV_SEQUENTIAL,
            FileAdvice::Random => libc::POSIX_FADV_RANDOM,
            FileAdvice::WillNeed => libc::POSIX_FADV_WILLNEED,
            FileAdvice::DontNeed => libc::POSIX_FADV_DONTNEED,
            FileAdvice::NoReuse => libc::POSIX_FADV_NOREUSE,
        };

        // advise the kernel
        let rc = unsafe { libc::posix_fadvise(fd, offset, length, advice) };
        if rc != 0 {
            return Err(core_platform::io_error_with_errno(
                "posix_fadvise",
                rc,
                None,
            ));
        }

        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        let _ = (offset, length);
        let enable_readahead = matches!(advice, FileAdvice::Sequential | FileAdvice::WillNeed);
        let rc = unsafe { libc::fcntl(fd, libc::F_RDAHEAD, enable_readahead as libc::c_int) };
        if rc == -1 {
            return Err(core_platform::io_error("fcntl(F_RDAHEAD)", None));
        }

        Ok(())
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios"
    )))]
    {
        let _ = (fd, offset, length, advice);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fadvise")).boxed())
    }
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
    context: &BindingCallContext,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: AllocFlags,
) -> RuntimeResult<()> {
    // resolve the file descriptor
    let fd = file_descriptor(context, handle)?;
    let offset = offset_to_off_t(offset)?;
    let length = length.0 as libc::off_t;

    // attempt to allocate space using the best available syscall
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let rc = unsafe { libc::fallocate(fd, flags.0 as libc::c_int, offset, length) };
        if rc != 0 {
            return Err(core_platform::io_error("fallocate", None));
        }
        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        if flags.0 != 0 {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.fs.fallocate flags",
            ))
            .boxed());
        }

        let mut store = libc::fstore_t {
            fst_flags: libc::F_ALLOCATECONTIG,
            fst_posmode: libc::F_PEOFPOSMODE,
            fst_offset: offset,
            fst_length: length,
            fst_bytesalloc: 0,
        };
        let mut rc = unsafe { libc::fcntl(fd, libc::F_PREALLOCATE, &mut store) };
        if rc == -1 {
            store.fst_flags = libc::F_ALLOCATEALL;
            rc = unsafe { libc::fcntl(fd, libc::F_PREALLOCATE, &mut store) };
        }
        if rc == -1 {
            return Err(core_platform::io_error("fcntl(F_PREALLOCATE)", None));
        }

        let end = offset.checked_add(length).ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value("length", "overflow")).boxed()
        })?;
        let rc = unsafe { libc::ftruncate(fd, end) };
        if rc != 0 {
            return Err(core_platform::io_error("ftruncate", None));
        }
        Ok(())
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios"
    )))]
    {
        let _ = (fd, offset, length, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fallocate")).boxed())
    }
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
    context: &BindingCallContext,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: SyncFlags,
) -> RuntimeResult<()> {
    // resolve the file descriptor
    let fd = file_descriptor(context, handle)?;
    let offset = offset_to_off_t(offset)?;
    let length = length.0 as libc::off_t;

    #[cfg(target_os = "linux")]
    {
        let rc = unsafe { libc::sync_file_range(fd, offset, length, flags.0 as libc::c_uint) };
        if rc != 0 {
            return Err(core_platform::io_error("sync_file_range", None));
        }
        return Ok(());
    }

    #[cfg(all(target_os = "android", target_arch = "arm"))]
    {
        let rc = unsafe {
            libc::syscall(
                libc::SYS_arm_sync_file_range,
                fd,
                offset,
                length,
                flags.0 as libc::c_uint,
            )
        };
        if rc != 0 {
            return Err(core_platform::io_error("sync_file_range", None));
        }
        return Ok(());
    }

    #[cfg(all(target_os = "android", not(target_arch = "arm")))]
    {
        let rc = unsafe {
            libc::syscall(
                libc::SYS_sync_file_range,
                fd,
                offset,
                length,
                flags.0 as libc::c_uint,
            )
        };
        if rc != 0 {
            return Err(core_platform::io_error("sync_file_range", None));
        }
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (offset, length, flags);
        let rc = unsafe { libc::fsync(fd) };
        if rc != 0 {
            return Err(core_platform::io_error("fsync", None));
        }
        Ok(())
    }
}
