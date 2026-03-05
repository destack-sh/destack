use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::memory::{
    MemoryNumaPolicy, MemoryProtection, MemoryRange, MemoryReserveFlags, core as memory_core,
};
use crate::runtime::BindingCallContext;

use super::core::{
    NUMA_BIND_OPERATION, decode_reserve_flags, page_size, to_size_t, unix_protection,
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

/// Bind one range to a NUMA policy.
pub(crate) unsafe fn destack_memory_numa_bind(
    _binding: &BindingCallContext,
    address: u64,
    length: u64,
    policy: MemoryNumaPolicy,
    nodemask: u64,
) -> RuntimeResult<()> {
    // validate range to bind
    let page_size = page_size()?;
    let (pointer, length) = validated_range(address, length, page_size)?;

    #[cfg(target_os = "linux")]
    {
        // map portable policy variants to linux mbind mode constants
        let mode = match policy {
            MemoryNumaPolicy::Default => 0,
            MemoryNumaPolicy::Bind => 2,
            MemoryNumaPolicy::Interleave => 3,
            MemoryNumaPolicy::Preferred => 1,
            MemoryNumaPolicy::Local => 4,
        };

        // validate nodemask width for the host c_ulong representation
        let nodemask_value = libc::c_ulong::try_from(nodemask).map_err(|_| {
            core_platform::invalid_argument("nodeMask", "node mask exceeds host word size")
        })?;
        let nodemask_pointer = &nodemask_value as *const libc::c_ulong;
        let maxnode = libc::c_ulong::from(usize::BITS);

        // invoke mbind through syscall to avoid libc feature drift
        let status = unsafe {
            libc::syscall(
                libc::SYS_mbind,
                pointer,
                length,
                mode,
                nodemask_pointer,
                maxnode,
                0,
            )
        };
        if status != 0 {
            return Err(core_platform::io_error("mbind", None));
        }

        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        // mark unsupported numa-bind operation on this unix backend
        let _ = (pointer, length, policy, nodemask);
        Err(core_platform::not_supported(NUMA_BIND_OPERATION))
    }
}
