#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(clippy::missing_safety_doc)]

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::not_supported;
use crate::platform::memory::{
    MemoryAdvice, MemoryProtection, MemoryRange, MemoryRemapFlags, MemoryReserveFlags,
    ProtectedMemoryRange,
};
use crate::runtime::BindingCallContext;

/// Apply memory access advice.
pub(crate) unsafe fn destack_memory_advise(
    binding: &BindingCallContext,
    address: u64,
    length: u64,
    advice: MemoryAdvice,
) -> RuntimeResult<()> {
    let _ = (address, length, advice);
    Err(not_supported("destack.memory.advise.adviseRange"))
}

/// Discard memory contents.
pub(crate) unsafe fn destack_memory_discard(
    binding: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.advise.discard"))
}

/// Lock one memory range into physical memory.
pub(crate) unsafe fn destack_memory_lock(
    binding: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.lock.lockRange"))
}

/// Unlock one memory range.
pub(crate) unsafe fn destack_memory_unlock(
    binding: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.lock.unlock"))
}

/// Commit one reserved range.
pub(crate) unsafe fn destack_memory_commit(
    binding: &BindingCallContext,
    address: u64,
    length: u64,
    protection: MemoryProtection,
) -> RuntimeResult<()> {
    let _ = (address, length, protection);
    Err(not_supported("destack.memory.map.commit"))
}

/// Decommit one range.
pub(crate) unsafe fn destack_memory_decommit(
    binding: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.map.decommit"))
}

/// Release one reserved range.
pub(crate) unsafe fn destack_memory_release(
    binding: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.map.release"))
}

/// Allocate one mapped range.
pub(crate) unsafe fn destack_memory_allocate(
    binding: &BindingCallContext,
    out: *mut ProtectedMemoryRange,
    length: u64,
    addresshint: u64,
    protection: MemoryProtection,
    flags: MemoryReserveFlags,
) -> RuntimeResult<()> {
    let _ = (binding, out, length, addresshint, protection, flags);
    Err(not_supported("destack.memory.map.allocate"))
}

/// Reserve one virtual memory range.
pub(crate) unsafe fn destack_memory_reserve(
    binding: &BindingCallContext,
    out: *mut MemoryRange,
    length: u64,
    addresshint: u64,
    flags: MemoryReserveFlags,
) -> RuntimeResult<()> {
    let _ = (out, length, addresshint, flags);
    Err(not_supported("destack.memory.map.reserve"))
}

/// Flush instruction cache for one range.
pub(crate) unsafe fn destack_memory_flush_instruction_cache(
    binding: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported(
        "destack.memory.protect.flushInstructionCache",
    ))
}

/// Change memory protection for one range.
pub(crate) unsafe fn destack_memory_protect(
    binding: &BindingCallContext,
    address: u64,
    length: u64,
    protection: MemoryProtection,
) -> RuntimeResult<()> {
    let _ = (address, length, protection);
    Err(not_supported("destack.memory.protect.protectRange"))
}

/// Resize one mapped range.
pub(crate) unsafe fn destack_memory_remap(
    binding: &BindingCallContext,
    out: *mut ProtectedMemoryRange,
    address: u64,
    oldlength: u64,
    newlength: u64,
    flags: MemoryRemapFlags,
) -> RuntimeResult<()> {
    let _ = (out, address, oldlength, newlength, flags);
    Err(not_supported("destack.memory.protect.remap"))
}

/// Read the host allocation granularity.
pub(crate) unsafe fn destack_memory_allocation_granularity(
    binding: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    let _ = out;
    Err(not_supported("destack.memory.query.allocationGranularity"))
}

/// Read the host huge-page allocation size when available.
pub(crate) unsafe fn destack_memory_huge_page_size(
    binding: &BindingCallContext,
    out: *mut Option<u64>,
) -> RuntimeResult<()> {
    let _ = out;
    Err(not_supported("destack.memory.query.hugePageSize"))
}

/// Read the host virtual-memory page size.
pub(crate) unsafe fn destack_memory_page_size(
    binding: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    let _ = out;
    Err(not_supported("destack.memory.query.pageSize"))
}
