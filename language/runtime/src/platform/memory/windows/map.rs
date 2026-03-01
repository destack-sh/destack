use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_DECOMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_NOACCESS, VirtualAlloc, VirtualFree,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::memory::{
    MemoryNumaPolicy, MemoryProtection, MemoryRange, MemoryReserveFlags, core as memory_core,
};
use crate::runtime::BindingCallContext;

use super::core::{
    NUMA_BIND_OPERATION, decode_reserve_flags, io_error, page_size, windows_protection,
};
use memory_core::{ensure_out, not_supported, usize_to_u64};

/// Reserve one virtual memory range.
pub(crate) unsafe fn destack_memory_reserve(
    _context: &BindingCallContext,
    out: *mut MemoryRange,
    length: u64,
    addresshint: u64,
    flags: MemoryReserveFlags,
) -> RuntimeResult<()> {
    // validate output pointer and reserve parameters
    ensure_out(out, "out")?;
    let page_size = page_size()?;
    let length = memory_core::nonzero_length(length, "length")?;
    memory_core::require_page_alignment(length, page_size, "length")?;

    // decode platform reserve flags and optional address hint
    let reserve_flags = decode_reserve_flags(flags)?;
    let hint = memory_core::optional_address_hint(addresshint, "addressHint")?;
    let hint = if let Some(hint) = hint {
        memory_core::require_page_alignment(hint, page_size, "addressHint")?;
        hint as *mut core::ffi::c_void
    } else {
        std::ptr::null_mut()
    };

    // reserve virtual address space with no access permissions
    let allocation_type = MEM_RESERVE | reserve_flags;
    let pointer = unsafe { VirtualAlloc(hint, length, allocation_type, PAGE_NOACCESS) };
    if pointer.is_null() {
        return Err(io_error("VirtualAlloc"));
    }

    // write resulting range metadata
    let mapped_address = usize_to_u64(pointer as usize, "out.address")?;
    let mapped_length = usize_to_u64(length, "out.length")?;
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
    _context: &BindingCallContext,
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
        return Err(io_error("VirtualAlloc"));
    }

    Ok(())
}

/// Decommit one range.
pub(crate) unsafe fn destack_memory_decommit(
    _context: &BindingCallContext,
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
        return Err(io_error("VirtualFree"));
    }

    Ok(())
}

/// Release one reserved range.
pub(crate) unsafe fn destack_memory_release(
    _context: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    // validate range before release
    let page_size = page_size()?;
    let address = memory_core::nonzero_address(address, "address")?;
    let length = memory_core::nonzero_length(length, "length")?;
    memory_core::require_page_alignment(address, page_size, "address")?;
    memory_core::require_page_alignment(length, page_size, "length")?;

    // release the reservation back to the host
    let status = unsafe { VirtualFree(address as *mut core::ffi::c_void, 0, MEM_RELEASE) };
    if status == 0 {
        return Err(io_error("VirtualFree"));
    }

    Ok(())
}

/// Bind one range to a NUMA policy.
pub(crate) unsafe fn destack_memory_numa_bind(
    _context: &BindingCallContext,
    address: u64,
    length: u64,
    policy: MemoryNumaPolicy,
    nodemask: u64,
) -> RuntimeResult<()> {
    // mark numa binding as unsupported on this backend
    let _ = (address, length, policy, nodemask);
    Err(not_supported(NUMA_BIND_OPERATION))
}
