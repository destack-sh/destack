use windows_sys::Win32::System::Memory::{
    GetLargePageMinimum, MEM_COMMIT, MEM_DECOMMIT, MEM_LARGE_PAGES, MEM_RELEASE, MEM_RESERVE,
    MEMORY_BASIC_INFORMATION, PAGE_NOACCESS, VirtualAlloc, VirtualFree, VirtualQuery,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::memory::{
    MemoryProtection, MemoryRange, MemoryReserveFlags, ProtectedMemoryRange, core as memory_core,
};
use crate::runtime::BindingCallContext;

use super::core::{
    ALLOCATE_OPERATION, allocation_granularity, decode_allocate_flags, decode_reserve_flags,
    page_size, windows_protection,
};

/// Query one reservation span starting at one allocation base.
unsafe fn queried_reservation_length(address: usize) -> RuntimeResult<usize> {
    // query the first region covering the requested address
    let mut memory_info = unsafe { std::mem::zeroed::<MEMORY_BASIC_INFORMATION>() };
    let queried = unsafe {
        VirtualQuery(
            address as *const core::ffi::c_void,
            &mut memory_info,
            std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        )
    };
    if queried == 0 {
        return Err(core_platform::io_error("VirtualQuery"));
    }

    // require release to start from the original allocation base
    let allocation_base = memory_info.AllocationBase as usize;
    if allocation_base != address || memory_info.BaseAddress as usize != address {
        return Err(core_platform::invalid_argument(
            "address",
            "address is not the base of one reserved allocation",
        ));
    }

    // accumulate the contiguous reservation span for this allocation base
    let mut reservation_length = 0usize;
    let mut current_address = address;
    loop {
        let mut region_info = unsafe { std::mem::zeroed::<MEMORY_BASIC_INFORMATION>() };
        let queried = unsafe {
            VirtualQuery(
                current_address as *const core::ffi::c_void,
                &mut region_info,
                std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        };
        if queried == 0 {
            return Err(core_platform::io_error("VirtualQuery"));
        }

        // stop when the next region is outside the same allocation
        if region_info.AllocationBase as usize != allocation_base
            || region_info.BaseAddress as usize != current_address
        {
            break;
        }

        // accumulate this region and continue with the next one
        reservation_length = reservation_length
            .checked_add(region_info.RegionSize)
            .ok_or_else(|| {
                core_platform::invalid_argument("length", "reservation length overflowed")
            })?;
        current_address = current_address
            .checked_add(region_info.RegionSize)
            .ok_or_else(|| {
                core_platform::invalid_argument("length", "reservation address overflowed")
            })?;
    }

    Ok(reservation_length)
}

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
    let granularity = allocation_granularity()?;
    let length = memory_core::nonzero_length(length, "length")?;
    memory_core::require_page_alignment(length, page_size, "length")?;

    // decode platform reserve flags and optional address hint
    let reserve_flags = decode_reserve_flags(flags)?;
    let hint = memory_core::optional_address_hint(addresshint, "addressHint")?;
    let hint = if let Some(hint) = hint {
        memory_core::require_page_alignment(hint, granularity, "addressHint")?;
        hint as *mut core::ffi::c_void
    } else {
        std::ptr::null_mut()
    };

    // reserve virtual address space with no access permissions
    let allocation_type = MEM_RESERVE | reserve_flags;
    let pointer = unsafe { VirtualAlloc(hint, length, allocation_type, PAGE_NOACCESS) };
    if pointer.is_null() {
        return Err(core_platform::io_error("VirtualAlloc"));
    }

    // write resulting range metadata
    let mapped_address = core_platform::usize_to_u64(pointer as usize, "out.address")?;
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
    let granularity = allocation_granularity()?;
    let length = memory_core::nonzero_length(length, "length")?;
    memory_core::require_page_alignment(length, page_size, "length")?;

    // decode allocation flags and target protection
    let protection = windows_protection(protection)?;
    let mut allocation_type = MEM_RESERVE | MEM_COMMIT | decode_allocate_flags(flags)?;

    // resolve optional address hint with host alignment rules
    let large_pages = flags.0 & crate::platform::memory::MEMORY_RESERVE_LARGE_PAGES.0 != 0;
    let hint = memory_core::optional_address_hint(addresshint, "addressHint")?;
    let hint = if let Some(hint) = hint {
        let required_alignment = if large_pages {
            let huge_page_size = unsafe { GetLargePageMinimum() };
            if huge_page_size == 0 {
                return Err(core_platform::not_supported(ALLOCATE_OPERATION));
            }

            huge_page_size
        } else {
            granularity
        };
        memory_core::require_page_alignment(hint, required_alignment, "addressHint")?;
        hint as *mut core::ffi::c_void
    } else {
        std::ptr::null_mut()
    };

    // apply large-page allocation policy where the host exposes it
    if large_pages {
        let huge_page_size = unsafe { GetLargePageMinimum() };
        if huge_page_size == 0 {
            return Err(core_platform::not_supported(ALLOCATE_OPERATION));
        }

        memory_core::require_page_alignment(length, huge_page_size, "length")?;
        allocation_type |= MEM_LARGE_PAGES;
    }

    // allocate one committed mapping with the requested protection
    let pointer = unsafe { VirtualAlloc(hint, length, allocation_type, protection) };
    if pointer.is_null() {
        return Err(core_platform::io_error("VirtualAlloc"));
    }

    // return the mapped protected range to the caller
    let mapped_address = core_platform::usize_to_u64(pointer as usize, "out.address")?;
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
    // validate range and protection mode
    let page_size = page_size()?;
    let address = memory_core::nonzero_address(address, "address")?;
    let length = memory_core::nonzero_length(length, "length")?;
    memory_core::require_page_alignment(address, page_size, "address")?;
    memory_core::require_page_alignment(length, page_size, "length")?;

    // commit the requested range with target protection
    let protection = windows_protection(protection)?;
    let pointer = unsafe {
        VirtualAlloc(
            address as *mut core::ffi::c_void,
            length,
            MEM_COMMIT,
            protection,
        )
    };
    if pointer.is_null() {
        return Err(core_platform::io_error("VirtualAlloc"));
    }

    Ok(())
}

/// Decommit one range.
pub(crate) unsafe fn destack_memory_decommit(
    _binding: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    // validate range before decommit
    let page_size = page_size()?;
    let address = memory_core::nonzero_address(address, "address")?;
    let length = memory_core::nonzero_length(length, "length")?;
    memory_core::require_page_alignment(address, page_size, "address")?;
    memory_core::require_page_alignment(length, page_size, "length")?;

    // decommit pages while preserving reservation
    let status = unsafe { VirtualFree(address as *mut core::ffi::c_void, length, MEM_DECOMMIT) };
    if status == 0 {
        return Err(core_platform::io_error("VirtualFree"));
    }

    Ok(())
}

/// Release one reserved range.
pub(crate) unsafe fn destack_memory_release(
    _binding: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    // validate range before release
    let page_size = page_size()?;
    let address = memory_core::nonzero_address(address, "address")?;
    let length = memory_core::nonzero_length(length, "length")?;
    memory_core::require_page_alignment(address, page_size, "address")?;
    memory_core::require_page_alignment(length, page_size, "length")?;

    // require the exact reservation span that windows can release
    let reservation_length = unsafe { queried_reservation_length(address) }?;
    if reservation_length != length {
        return Err(core_platform::invalid_argument(
            "length",
            "length must match the full reserved allocation span",
        ));
    }

    // release the reservation back to the host
    let status = unsafe { VirtualFree(address as *mut core::ffi::c_void, 0, MEM_RELEASE) };
    if status == 0 {
        return Err(core_platform::io_error("VirtualFree"));
    }

    Ok(())
}
