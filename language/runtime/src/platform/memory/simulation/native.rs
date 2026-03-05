#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::not_supported;
use crate::platform::memory::{
    MemoryAdvice, MemoryNumaPolicy, MemoryProtection, MemoryRange, MemoryRemapFlags,
    MemoryReserveFlags, ProtectedMemoryRange,
};
use crate::runtime::BindingCallContext;

/// Apply memory access advice.
pub(crate) unsafe fn destack_memory_advise(
    _binding: &BindingCallContext,
    address: u64,
    length: u64,
    advice: MemoryAdvice,
) -> RuntimeResult<()> {
    let _ = (address, length, advice);
    Err(not_supported("destack.memory.advise.adviseRange"))
}

/// Discard memory contents.
pub(crate) unsafe fn destack_memory_discard(
    _binding: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.advise.discard"))
}

/// Toggle huge-page preference for one range.
pub(crate) unsafe fn destack_memory_huge_page(
    _binding: &BindingCallContext,
    address: u64,
    length: u64,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (address, length, enabled);
    Err(not_supported("destack.memory.advise.hugePage"))
}

/// Lock one memory range into physical memory.
pub(crate) unsafe fn destack_memory_lock(
    _binding: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.lock.lockRange"))
}

/// Unlock one memory range.
pub(crate) unsafe fn destack_memory_unlock(
    _binding: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.lock.unlock"))
}

/// Commit one reserved range.
pub(crate) unsafe fn destack_memory_commit(
    _binding: &BindingCallContext,
    address: u64,
    length: u64,
    protection: MemoryProtection,
) -> RuntimeResult<()> {
    let _ = (address, length, protection);
    Err(not_supported("destack.memory.map.commit"))
}

/// Decommit one range.
pub(crate) unsafe fn destack_memory_decommit(
    _binding: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.map.decommit"))
}

/// Bind one range to a NUMA policy.
pub(crate) unsafe fn destack_memory_numa_bind(
    _binding: &BindingCallContext,
    address: u64,
    length: u64,
    policy: MemoryNumaPolicy,
    nodemask: u64,
) -> RuntimeResult<()> {
    let _ = (address, length, policy, nodemask);
    Err(not_supported("destack.memory.map.numaBind"))
}

/// Release one reserved range.
pub(crate) unsafe fn destack_memory_release(
    _binding: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.map.release"))
}

/// Reserve one virtual memory range.
pub(crate) unsafe fn destack_memory_reserve(
    _binding: &BindingCallContext,
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
    _binding: &BindingCallContext,
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
    _binding: &BindingCallContext,
    address: u64,
    length: u64,
    protection: MemoryProtection,
) -> RuntimeResult<()> {
    let _ = (address, length, protection);
    Err(not_supported("destack.memory.protect.protectRange"))
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
    let _ = (out, address, oldlength, newlength, flags);
    Err(not_supported("destack.memory.protect.remap"))
}

/// Read the host allocation granularity.
pub(crate) unsafe fn destack_memory_allocation_granularity(
    _binding: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    let _ = out;
    Err(not_supported("destack.memory.query.allocationGranularity"))
}

/// Read the host huge-page allocation size when available.
pub(crate) unsafe fn destack_memory_huge_page_size(
    _binding: &BindingCallContext,
    out: *mut Option<u64>,
) -> RuntimeResult<()> {
    let _ = out;
    Err(not_supported("destack.memory.query.hugePageSize"))
}

/// Read the host virtual-memory page size.
pub(crate) unsafe fn destack_memory_page_size(
    _binding: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    let _ = out;
    Err(not_supported("destack.memory.query.pageSize"))
}
