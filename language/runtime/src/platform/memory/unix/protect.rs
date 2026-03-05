use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::memory::{
    MemoryProtection, MemoryRemapFlags, ProtectedMemoryRange, core as memory_core,
};
use crate::runtime::BindingCallContext;

use super::core::{
    REMAP_OPERATION, decode_remap_flags, page_size, unix_protection, validated_range,
};

/// Operation tag for instruction-cache flush bindings.
#[cfg(target_os = "android")]
const FLUSH_INSTRUCTION_CACHE_OPERATION: &str = "destack.memory.protect.flushInstructionCache";

#[cfg(all(
    not(any(target_arch = "x86", target_arch = "x86_64")),
    target_vendor = "apple"
))]
unsafe extern "C" {
    /// Invalidate one instruction-cache region on apple targets.
    fn sys_icache_invalidate(start: *const core::ffi::c_void, length: usize);
}

#[cfg(all(
    not(any(target_arch = "x86", target_arch = "x86_64")),
    not(target_vendor = "apple"),
    not(target_os = "android")
))]
unsafe extern "C" {
    /// Clear one instruction-cache region on non-apple targets.
    fn __clear_cache(begin: *mut core::ffi::c_char, end: *mut core::ffi::c_char);
}

/// Change memory protection for one range.
pub(crate) unsafe fn destack_memory_protect(
    _binding: &BindingCallContext,
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
        return Err(core_platform::io_error("mprotect", None));
    }

    Ok(())
}

/// Resize one mapped range.
pub(crate) unsafe fn destack_memory_remap(
    _binding: &BindingCallContext,
    out: *mut ProtectedMemoryRange,
    address: u64,
    oldlength: u64,
    newlength: u64,
    flags: MemoryRemapFlags,
) -> RuntimeResult<()> {
    // validate output pointer and remap parameters
    core_platform::ensure_out(out, "out")?;
    let page_size = page_size()?;
    let (pointer, old_length) = validated_range(address, oldlength, page_size)?;
    let new_length = memory_core::nonzero_length(newlength, "newLength")?;
    memory_core::require_page_alignment(new_length, page_size, "newLength")?;

    // decode remap behavior flags
    let may_move = decode_remap_flags(flags.0)?;

    #[cfg(target_os = "linux")]
    {
        // map portable remap behavior to native mremap flags
        let mut native_flags = 0;
        if may_move {
            native_flags |= libc::MREMAP_MAYMOVE;
        }

        // resize the mapping and return the new range
        let remapped = unsafe { libc::mremap(pointer, old_length, new_length, native_flags) };
        if remapped == libc::MAP_FAILED {
            return Err(core_platform::io_error("mremap", None));
        }

        let remapped_address = core_platform::usize_to_u64(remapped as usize, "out.address")?;
        let remapped_length = core_platform::usize_to_u64(new_length, "out.length")?;
        unsafe {
            out.write(ProtectedMemoryRange {
                address: remapped_address,
                length: remapped_length,
            });
        }

        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        // mark remap as unsupported on this backend
        let _ = (pointer, old_length, new_length, may_move);
        Err(core_platform::not_supported(REMAP_OPERATION))
    }
}

/// Flush instruction cache for one range.
pub(crate) unsafe fn destack_memory_flush_instruction_cache(
    _binding: &BindingCallContext,
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

    #[cfg(all(
        not(any(target_arch = "x86", target_arch = "x86_64")),
        target_vendor = "apple"
    ))]
    {
        // invalidate the instruction cache through the apple runtime entrypoint
        unsafe {
            sys_icache_invalidate(pointer.cast::<core::ffi::c_void>(), length);
        }

        Ok(())
    }

    #[cfg(all(
        not(any(target_arch = "x86", target_arch = "x86_64")),
        target_os = "android"
    ))]
    {
        // report explicit non-support until android exposes one stable runtime lane here
        let _ = (pointer, length);
        Err(core_platform::not_supported(
            FLUSH_INSTRUCTION_CACHE_OPERATION,
        ))
    }

    #[cfg(all(
        not(any(target_arch = "x86", target_arch = "x86_64")),
        not(target_vendor = "apple"),
        not(target_os = "android")
    ))]
    {
        // use compiler-provided cache clear entrypoint on non-apple non-x86 targets
        // flush the instruction cache for the requested byte span
        let begin = pointer as *mut core::ffi::c_char;
        let end = unsafe { begin.add(length) };
        unsafe {
            __clear_cache(begin, end);
        }

        Ok(())
    }
}
