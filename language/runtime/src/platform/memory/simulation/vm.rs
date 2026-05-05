#![allow(dead_code)]
#![allow(unused_imports)]

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::not_supported;
use crate::platform::memory::{
    MemoryAdvice, MemoryProtection, MemoryRangeVm, MemoryRemapFlags, MemoryReserveFlags,
    ProtectedMemoryRangeVm,
};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// Apply memory access advice.
pub(crate) fn destack_memory_advise(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    length: u64,
    advice: MemoryAdvice,
) -> RuntimeResult<()> {
    let _ = (address, length, advice);
    Err(not_supported("destack.memory.advise.adviseRange"))
}

/// Discard memory contents.
pub(crate) fn destack_memory_discard(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.advise.discard"))
}

/// Lock one memory range into physical memory.
pub(crate) fn destack_memory_lock(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.lock.lockRange"))
}

/// Unlock one memory range.
pub(crate) fn destack_memory_unlock(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.lock.unlock"))
}

/// Commit one reserved range.
pub(crate) fn destack_memory_commit(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    length: u64,
    protection: MemoryProtection,
) -> RuntimeResult<()> {
    let _ = (address, length, protection);
    Err(not_supported("destack.memory.map.commit"))
}

/// Decommit one range.
pub(crate) fn destack_memory_decommit(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.map.decommit"))
}

/// Release one reserved range.
pub(crate) fn destack_memory_release(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.map.release"))
}

/// Allocate one mapped range.
pub(crate) fn destack_memory_allocate(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    length: u64,
    addresshint: u64,
    protection: MemoryProtection,
    flags: MemoryReserveFlags,
) -> RuntimeResult<ProtectedMemoryRangeVm> {
    let _ = (length, addresshint, protection, flags);
    Err(not_supported("destack.memory.map.allocate"))
}

/// Reserve one virtual memory range.
pub(crate) fn destack_memory_reserve(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    length: u64,
    addresshint: u64,
    flags: MemoryReserveFlags,
) -> RuntimeResult<MemoryRangeVm> {
    let _ = (length, addresshint, flags);
    Err(not_supported("destack.memory.map.reserve"))
}

/// Flush instruction cache for one range.
pub(crate) fn destack_memory_flush_instruction_cache(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported(
        "destack.memory.protect.flushInstructionCache",
    ))
}

/// Change memory protection for one range.
pub(crate) fn destack_memory_protect(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    length: u64,
    protection: MemoryProtection,
) -> RuntimeResult<()> {
    let _ = (address, length, protection);
    Err(not_supported("destack.memory.protect.protectRange"))
}

/// Resize one mapped range.
pub(crate) fn destack_memory_remap(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    oldlength: u64,
    newlength: u64,
    flags: MemoryRemapFlags,
) -> RuntimeResult<ProtectedMemoryRangeVm> {
    let _ = (address, oldlength, newlength, flags);
    Err(not_supported("destack.memory.protect.remap"))
}

/// Read the host allocation granularity.
pub(crate) fn destack_memory_allocation_granularity(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<u64> {
    Err(not_supported("destack.memory.query.allocationGranularity"))
}

/// Read the host huge-page allocation size when available.
pub(crate) fn destack_memory_huge_page_size(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<Option<u64>> {
    Err(not_supported("destack.memory.query.hugePageSize"))
}

/// Read the host virtual-memory page size.
pub(crate) fn destack_memory_page_size(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<u64> {
    Err(not_supported("destack.memory.query.pageSize"))
}
