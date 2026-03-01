use crate::diagnostic::RuntimeResult;
use crate::platform::memory::{
    MemoryProtection, MemoryRemapFlags, ProtectedMemoryRange, core as memory_core,
};
use crate::runtime::BindingCallContext;

#[cfg(any(target_os = "linux", target_os = "android"))]
use super::core::mapped_address;
use super::core::{
    REMAP_OPERATION, decode_remap_flags, io_error, page_size, unix_protection, validated_range,
};
#[cfg(any(target_os = "linux", target_os = "android"))]
use memory_core::usize_to_u64;
use memory_core::{ensure_out, not_supported};

/// Change memory protection for one range.
pub(crate) unsafe fn destack_memory_protect(
    _context: &BindingCallContext,
    address: u64,
    length: u64,
    protection: MemoryProtection,
) -> RuntimeResult<()> {
    // validate range and target protection
    let page_size = page_size()?;
    let (pointer, length) = validated_range(address, length, page_size)?;
    let protection = unix_protection(protection)?;

    // apply memory protection change
    let status = unsafe { libc::mprotect(pointer, length, protection) };
    if status != 0 {
        return Err(io_error("mprotect"));
    }

    Ok(())
}

/// Resize one mapped range.
pub(crate) unsafe fn destack_memory_remap(
    _context: &BindingCallContext,
    out: *mut ProtectedMemoryRange,
    address: u64,
    oldlength: u64,
    newlength: u64,
    flags: MemoryRemapFlags,
) -> RuntimeResult<()> {
    // validate output pointer and remap parameters
    ensure_out(out, "out")?;
    let page_size = page_size()?;
    let (pointer, old_length) = validated_range(address, oldlength, page_size)?;
    let new_length = memory_core::nonzero_length(newlength, "newLength")?;
    memory_core::require_page_alignment(new_length, page_size, "newLength")?;

    // decode remap behavior flags
    let may_move = decode_remap_flags(flags.0)?;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // map portable remap behavior to native mremap flags
        let mut native_flags = 0;
        if may_move {
            native_flags |= libc::MREMAP_MAYMOVE;
        }

        // resize the mapping and return the new range
        let remapped = unsafe { libc::mremap(pointer, old_length, new_length, native_flags) };
        if remapped == libc::MAP_FAILED {
            return Err(io_error("mremap"));
        }

        let remapped_address = mapped_address(remapped, "out.address")?;
        let remapped_length = usize_to_u64(new_length, "out.length")?;
        unsafe {
            out.write(ProtectedMemoryRange {
                address: remapped_address,
                length: remapped_length,
            });
        }

        return Ok(());
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        // mark remap as unsupported on this backend
        let _ = (pointer, old_length, new_length, may_move);
        Err(not_supported(REMAP_OPERATION))
    }
}

/// Flush instruction cache for one range.
pub(crate) unsafe fn destack_memory_flush_instruction_cache(
    _context: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    // validate the requested flush range
    let page_size = page_size()?;
    let (pointer, length) = validated_range(address, length, page_size)?;

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        // x86 cache coherency only needs a serialization fence here
        let _ = (pointer, length);
        std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
        return Ok(());
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        // use compiler-provided cache clear entrypoint on non-x86 targets
        unsafe extern "C" {
            fn __clear_cache(begin: *mut core::ffi::c_char, end: *mut core::ffi::c_char);
        }

        // flush the instruction cache for the requested byte span
        let begin = pointer as *mut core::ffi::c_char;
        let end = unsafe { begin.add(length) };
        unsafe {
            __clear_cache(begin, end);
        }

        Ok(())
    }
}
