use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::memory::{
    MemoryProtection, MemoryRange, MemoryReserveFlags, ProtectedMemoryRange, core as memory_core,
};
use crate::runtime::BindingCallContext;

use super::core::{
    decode_allocate_flags, decode_reserve_flags, page_size, to_size_t, unix_protection,
    validated_hint, validated_range, zero_offset,
};

/// Reserve one virtual memory range.
pub(crate) unsafe fn destack_memory_reserve(
    _binding: &BindingCallContext,
    out: *mut MemoryRange,
    length: u64,
    addresshint: u64,
    flags: MemoryReserveFlags,
) -> RuntimeResult<()> {
    // validate output pointer and reserve parameters
    core_platform::ensure_out(out, "out")?;
    let page_size = page_size()?;
    let length = memory_core::nonzero_length(length, "length")?;
    memory_core::require_page_alignment(length, page_size, "length")?;

    // resolve optional hint and reserve flags
    let address_hint = validated_hint(addresshint, page_size)?;
    let native_flags = decode_reserve_flags(flags)?;

    // reserve virtual memory with no access permissions
    let mapped = unsafe {
        libc::mmap(
            address_hint,
            to_size_t(length),
            libc::PROT_NONE,
            native_flags,
            -1,
            zero_offset(),
        )
    };
    if mapped == libc::MAP_FAILED {
        return Err(core_platform::io_error("mmap", None));
    }

    // return the reserved range to the caller
    let mapped_address = core_platform::usize_to_u64(mapped as usize, "out.address")?;
    let mapped_length = core_platform::usize_to_u64(length, "out.length")?;
    unsafe {
        out.write(MemoryRange {
            address: mapped_address,
            length: mapped_length,
        });
    }

    Ok(())
}

/// Allocate one mapped range.
pub(crate) unsafe fn destack_memory_allocate(
    _binding: &BindingCallContext,
    out: *mut ProtectedMemoryRange,
    length: u64,
    addresshint: u64,
    protection: MemoryProtection,
    flags: MemoryReserveFlags,
) -> RuntimeResult<()> {
    // validate output pointer and allocation parameters
    core_platform::ensure_out(out, "out")?;
    let page_size = page_size()?;
    let length = memory_core::nonzero_length(length, "length")?;
    memory_core::require_page_alignment(length, page_size, "length")?;

    // resolve optional hint, allocation flags, and protection mode
    let address_hint = validated_hint(addresshint, page_size)?;
    let native_flags = decode_allocate_flags(flags)?;
    let native_protection = unix_protection(protection)?;

    // allocate one anonymous mapping with the requested properties
    let mapped = unsafe {
        libc::mmap(
            address_hint,
            to_size_t(length),
            native_protection,
            native_flags,
            -1,
            zero_offset(),
        )
    };
    if mapped == libc::MAP_FAILED {
        return Err(core_platform::io_error("mmap", None));
    }

    // return the mapped protected range to the caller
    let mapped_address = core_platform::usize_to_u64(mapped as usize, "out.address")?;
    let mapped_length = core_platform::usize_to_u64(length, "out.length")?;
    unsafe {
        out.write(ProtectedMemoryRange {
            address: mapped_address,
            length: mapped_length,
        });
    }

    Ok(())
}

/// Commit one reserved range.
pub(crate) unsafe fn destack_memory_commit(
    _binding: &BindingCallContext,
    address: u64,
    length: u64,
    protection: MemoryProtection,
) -> RuntimeResult<()> {
    // validate range and resolve native protection flags
    let page_size = page_size()?;
    let (pointer, length) = validated_range(address, length, page_size)?;
    let protection = unix_protection(protection)?;

    // commit by switching the mapping protection
    let status = unsafe { libc::mprotect(pointer, length, protection) };
    if status != 0 {
        return Err(core_platform::io_error("mprotect", None));
    }

    Ok(())
}

/// Decommit one range.
pub(crate) unsafe fn destack_memory_decommit(
    _binding: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    // validate range to decommit
    let page_size = page_size()?;
    let (pointer, length) = validated_range(address, length, page_size)?;

    // replace the range with an anonymous inaccessible mapping
    let mapped = unsafe {
        libc::mmap(
            pointer,
            to_size_t(length),
            libc::PROT_NONE,
            libc::MAP_PRIVATE | libc::MAP_ANON | libc::MAP_FIXED,
            -1,
            zero_offset(),
        )
    };
    if mapped == libc::MAP_FAILED {
        return Err(core_platform::io_error("mmap", None));
    }

    Ok(())
}

/// Release one reserved range.
pub(crate) unsafe fn destack_memory_release(
    _binding: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    // validate range to release
    let page_size = page_size()?;
    let (pointer, length) = validated_range(address, length, page_size)?;

    // release the mapping back to the host
    let status = unsafe { libc::munmap(pointer, length) };
    if status != 0 {
        return Err(core_platform::io_error("munmap", None));
    }

    Ok(())
}
