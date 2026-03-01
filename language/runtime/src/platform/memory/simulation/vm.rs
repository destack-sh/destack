#![allow(dead_code)]
#![allow(unused_imports)]

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::memory::{
    MemoryAdvice, MemoryNumaPolicy, MemoryProtection, MemoryRangeVm, MemoryRemapFlags,
    MemoryReserveFlags, ProtectedMemoryRangeVm,
};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// Build one simulation not-supported error.
fn not_supported(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}

/// Apply memory access advice.
pub(crate) fn destack_memory_advise(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    advice: MemoryAdvice,
) -> RuntimeResult<()> {
    let _ = (address, length, advice);
    Err(not_supported("destack.memory.advise.adviseRange"))
}

/// Discard memory contents.
pub(crate) fn destack_memory_discard(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.advise.discard"))
}

/// Toggle huge-page preference for one range.
pub(crate) fn destack_memory_huge_page(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (address, length, enabled);
    Err(not_supported("destack.memory.advise.hugePage"))
}

/// Lock one memory range into physical memory.
pub(crate) fn destack_memory_lock(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.lock.lockRange"))
}

/// Unlock one memory range.
pub(crate) fn destack_memory_unlock(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.lock.unlock"))
}

/// Commit one reserved range.
pub(crate) fn destack_memory_commit(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    protection: MemoryProtection,
) -> RuntimeResult<()> {
    let _ = (address, length, protection);
    Err(not_supported("destack.memory.map.commit"))
}

/// Decommit one range.
pub(crate) fn destack_memory_decommit(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.map.decommit"))
}

/// Bind one range to a NUMA policy.
pub(crate) fn destack_memory_numa_bind(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    policy: MemoryNumaPolicy,
    nodemask: u64,
) -> RuntimeResult<()> {
    let _ = (address, length, policy, nodemask);
    Err(not_supported("destack.memory.map.numaBind"))
}

/// Release one reserved range.
pub(crate) fn destack_memory_release(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(not_supported("destack.memory.map.release"))
}

/// Reserve one virtual memory range.
pub(crate) fn destack_memory_reserve(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    length: u64,
    addresshint: u64,
    flags: MemoryReserveFlags,
) -> RuntimeResult<MemoryRangeVm> {
    let _ = (length, addresshint, flags);
    Err(not_supported("destack.memory.map.reserve"))
}

/// Flush instruction cache for one range.
pub(crate) fn destack_memory_flush_instruction_cache(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    protection: MemoryProtection,
) -> RuntimeResult<()> {
    let _ = (address, length, protection);
    Err(not_supported("destack.memory.protect.protectRange"))
}

/// Resize one mapped range.
pub(crate) fn destack_memory_remap(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<u64> {
    Err(not_supported("destack.memory.query.allocationGranularity"))
}

/// Read the host huge-page allocation size when available.
pub(crate) fn destack_memory_huge_page_size(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<Option<u64>> {
    Err(not_supported("destack.memory.query.hugePageSize"))
}

/// Read the host virtual-memory page size.
pub(crate) fn destack_memory_page_size(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<u64> {
    Err(not_supported("destack.memory.query.pageSize"))
}
