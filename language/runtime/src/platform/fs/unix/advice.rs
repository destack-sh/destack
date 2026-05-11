use super::core::*;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::fs::*;
use crate::platform::resource::*;
use crate::runtime::BindingCallContext;
#[cfg(any(target_os = "macos", target_os = "ios"))]
use crate::{diagnostic::RuntimeError, platform::PlatformError};

/// Advise the kernel about access patterns.
pub(crate) unsafe fn destack_fs_fadvise(
    binding: &BindingCallContext,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    advice: FileAdvice,
) -> RuntimeResult<()> {
    // resolve the file descriptor
    let fd = file_descriptor(binding, handle)?;
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
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.fadvise")).boxed())
    }
}

/// Allocate or punch file space.
pub(crate) unsafe fn destack_fs_fallocate(
    binding: &BindingCallContext,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: AllocFlags,
) -> RuntimeResult<()> {
    // resolve the file descriptor
    let fd = file_descriptor(binding, handle)?;
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
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.fallocate")).boxed())
    }
}

/// Synchronize a file range.
pub(crate) unsafe fn destack_fs_sync_file_range(
    binding: &BindingCallContext,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: SyncFlags,
) -> RuntimeResult<()> {
    // resolve the file descriptor
    let fd = file_descriptor(binding, handle)?;
    let offset = offset_to_off_t(offset)?;
    let length = length.0 as libc::off_t;

    #[cfg(target_os = "linux")]
    {
        let rc = unsafe { libc::sync_file_range(fd, offset, length, flags.0 as libc::c_uint) };
        if rc != 0 {
            return Err(core_platform::io_error("sync_file_range", None));
        }
        Ok(())
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
