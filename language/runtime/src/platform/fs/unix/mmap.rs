use super::core::*;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::core::{decode_mmap_flags, validate_mapping_length};
use crate::platform::fs::*;
use crate::platform::resource::*;
use crate::platform::{core as core_platform, *};
use crate::runtime::{BindingCallContext, NativeSlice};

/// Create a file-backed memory mapping.
///
/// Map a file-backed region into virtual memory using the requested offset, length, and protection.
/// Page alignment, commit behavior, and memory fault semantics follow host virtual-memory rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses mmap(2) MAP_SHARED/MAP_PRIVATE on Unix and CreateFileMapping/MapViewOfFile on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.mmap`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_mmap_file(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    prot: MmapProt,
    flags: MmapFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve file descriptor
    let fd = file_descriptor(binding, handle)?;
    let mmap_flags = decode_mmap_flags(flags, false)?;
    let length = validate_mapping_length(length)?;

    // map flags and protections
    let mut native_flags = 0;
    if mmap_flags.is_shared {
        native_flags |= libc::MAP_SHARED;
    }
    if !mmap_flags.is_shared {
        native_flags |= libc::MAP_PRIVATE;
    }
    if mmap_flags.is_fixed {
        native_flags |= libc::MAP_FIXED;
    }
    let mut native_prot = 0;
    if prot.0 & 0x1 != 0 {
        native_prot |= libc::PROT_READ;
    }
    if prot.0 & 0x2 != 0 {
        native_prot |= libc::PROT_WRITE;
    }
    if prot.0 & 0x4 != 0 {
        native_prot |= libc::PROT_EXEC;
    }

    // map the file
    let offset = offset_to_off_t(offset)?;
    let mapping = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            length,
            native_prot,
            native_flags,
            fd,
            offset,
        )
    };
    if mapping == libc::MAP_FAILED {
        return Err(core_platform::io_error("mmap", None));
    }

    unsafe {
        *out = NativeSlice {
            data: mapping as *mut u8,
            len: length as u32,
        };
    }

    Ok(())
}

/// Create an anonymous memory mapping.
///
/// Map an anonymous zero-initialized region into virtual memory using host allocation primitives.
/// Page alignment, commit behavior, and memory fault semantics follow host virtual-memory rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses mmap(2) MAP_ANONYMOUS on Unix and VirtualAlloc/MapViewOfFile on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.mmap`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_mmap_anonymous(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    length: FileSize,
    prot: MmapProt,
    flags: MmapFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // map flags and protections
    let mut native_flags = libc::MAP_ANON;
    let mmap_flags = decode_mmap_flags(flags, true)?;
    let length = validate_mapping_length(length)?;
    if mmap_flags.is_shared {
        native_flags |= libc::MAP_SHARED;
    }
    if !mmap_flags.is_shared {
        native_flags |= libc::MAP_PRIVATE;
    }
    if mmap_flags.is_fixed {
        native_flags |= libc::MAP_FIXED;
    }
    let mut native_prot = 0;
    if prot.0 & 0x1 != 0 {
        native_prot |= libc::PROT_READ;
    }
    if prot.0 & 0x2 != 0 {
        native_prot |= libc::PROT_WRITE;
    }
    if prot.0 & 0x4 != 0 {
        native_prot |= libc::PROT_EXEC;
    }

    let mapping = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            length,
            native_prot,
            native_flags,
            -1,
            0,
        )
    };
    if mapping == libc::MAP_FAILED {
        return Err(core_platform::io_error("mmap", None));
    }

    unsafe {
        *out = NativeSlice {
            data: mapping as *mut u8,
            len: length as u32,
        };
    }

    Ok(())
}

/// Unmap a memory region.
///
/// Unmap the specified virtual-memory range and release its mapping resources.
/// Page alignment, commit behavior, and memory fault semantics follow host virtual-memory rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses munmap(2) on Unix and UnmapViewOfFile/VirtualFree on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.mmap`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_munmap(
    _binding: &BindingCallContext,
    mapping: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let slice = unsafe { mapping.as_slice()? };
    if slice.is_empty() {
        return Ok(());
    }
    let rc = unsafe { libc::munmap(mapping.data as *mut libc::c_void, mapping.len as usize) };
    if rc != 0 {
        return Err(core_platform::io_error("munmap", None));
    }
    Ok(())
}

/// Change memory protection for a mapping.
///
/// Change memory protection for a mapping via host kernel APIs.
/// Page alignment, commit behavior, and memory fault semantics follow host virtual-memory rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses mprotect(2) on Unix and VirtualProtect on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.mmap`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_mprotect(
    _binding: &BindingCallContext,
    mapping: NativeSlice<u8>,
    prot: MmapProt,
) -> RuntimeResult<()> {
    let mut native_prot = 0;
    if prot.0 & 0x1 != 0 {
        native_prot |= libc::PROT_READ;
    }
    if prot.0 & 0x2 != 0 {
        native_prot |= libc::PROT_WRITE;
    }
    if prot.0 & 0x4 != 0 {
        native_prot |= libc::PROT_EXEC;
    }
    let rc = unsafe {
        libc::mprotect(
            mapping.data as *mut libc::c_void,
            mapping.len as usize,
            native_prot,
        )
    };
    if rc != 0 {
        return Err(core_platform::io_error("mprotect", None));
    }
    Ok(())
}

/// Flush a mapping to storage.
///
/// Flush a mapping to storage via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses msync(2) on Unix and FlushViewOfFile on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.mmap`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_msync(
    _binding: &BindingCallContext,
    mapping: NativeSlice<u8>,
    flags: MmapSyncFlags,
) -> RuntimeResult<()> {
    let mut native_flags = 0;
    if flags.0 & 0x1 != 0 {
        native_flags |= libc::MS_SYNC;
    }
    if flags.0 & 0x2 != 0 {
        native_flags |= libc::MS_ASYNC;
    }
    if flags.0 & 0x4 != 0 {
        native_flags |= libc::MS_INVALIDATE;
    }
    let rc = unsafe {
        libc::msync(
            mapping.data as *mut libc::c_void,
            mapping.len as usize,
            native_flags,
        )
    };
    if rc != 0 {
        return Err(core_platform::io_error("msync", None));
    }
    Ok(())
}

/// Advise the kernel about access patterns.
///
/// Advise the kernel about access patterns via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses madvise(2) on Unix and advisory memory APIs where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.mmap`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_madvise(
    _binding: &BindingCallContext,
    mapping: NativeSlice<u8>,
    advice: MmapAdvice,
) -> RuntimeResult<()> {
    let advice = match advice {
        MmapAdvice::Normal => libc::MADV_NORMAL,
        MmapAdvice::Sequential => libc::MADV_SEQUENTIAL,
        MmapAdvice::Random => libc::MADV_RANDOM,
        MmapAdvice::WillNeed => libc::MADV_WILLNEED,
        MmapAdvice::DontNeed => libc::MADV_DONTNEED,
    };
    let rc = unsafe {
        libc::madvise(
            mapping.data as *mut libc::c_void,
            mapping.len as usize,
            advice,
        )
    };
    if rc != 0 {
        return Err(core_platform::io_error("madvise", None));
    }
    Ok(())
}
