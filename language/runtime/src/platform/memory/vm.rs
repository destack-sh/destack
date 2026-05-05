use crate::diagnostic::RuntimeResult;
use crate::platform::core::call_out;
use crate::platform::memory::{
    MemoryAdvice, MemoryProtection, MemoryRangeVm, MemoryRemapFlags, MemoryReserveFlags,
    ProtectedMemoryRangeVm,
};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

use super::host as host_memory;

/// Apply memory access advice.
pub(crate) fn destack_memory_advise(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    length: u64,
    advice: MemoryAdvice,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_advise(binding, address, length, advice) }
}

/// Discard memory contents.
pub(crate) fn destack_memory_discard(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_discard(binding, address, length) }
}

/// Lock one memory range into physical memory.
pub(crate) fn destack_memory_lock(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_lock(binding, address, length) }
}

/// Unlock one memory range.
pub(crate) fn destack_memory_unlock(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_unlock(binding, address, length) }
}

/// Commit one reserved range.
pub(crate) fn destack_memory_commit(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    length: u64,
    protection: MemoryProtection,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_commit(binding, address, length, protection) }
}

/// Decommit one range.
pub(crate) fn destack_memory_decommit(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_decommit(binding, address, length) }
}

/// Release one reserved range.
pub(crate) fn destack_memory_release(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_release(binding, address, length) }
}

/// Allocate one mapped range.
pub(crate) fn destack_memory_allocate(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    length: u64,
    addresshint: u64,
    protection: MemoryProtection,
    flags: MemoryReserveFlags,
) -> RuntimeResult<ProtectedMemoryRangeVm> {
    call_out(|out| unsafe {
        host_memory::destack_memory_allocate(binding, out, length, addresshint, protection, flags)
    })
}

/// Reserve one virtual memory range.
pub(crate) fn destack_memory_reserve(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    length: u64,
    addresshint: u64,
    flags: MemoryReserveFlags,
) -> RuntimeResult<MemoryRangeVm> {
    call_out(|out| unsafe {
        host_memory::destack_memory_reserve(binding, out, length, addresshint, flags)
    })
}

/// Flush instruction cache for one range.
pub(crate) fn destack_memory_flush_instruction_cache(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_flush_instruction_cache(binding, address, length) }
}

/// Change memory protection for one range.
pub(crate) fn destack_memory_protect(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    length: u64,
    protection: MemoryProtection,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_protect(binding, address, length, protection) }
}

/// Resize one mapped range.
pub(crate) fn destack_memory_remap(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    oldlength: u64,
    newlength: u64,
    flags: MemoryRemapFlags,
) -> RuntimeResult<ProtectedMemoryRangeVm> {
    call_out(|out| unsafe {
        host_memory::destack_memory_remap(binding, out, address, oldlength, newlength, flags)
    })
}

/// Read the host allocation granularity.
pub(crate) fn destack_memory_allocation_granularity(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_memory::destack_memory_allocation_granularity(binding, out) })
}

/// Read the host huge-page allocation size when available.
pub(crate) fn destack_memory_huge_page_size(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<Option<u64>> {
    call_out(|out| unsafe { host_memory::destack_memory_huge_page_size(binding, out) })
}

/// Read the host virtual-memory page size.
pub(crate) fn destack_memory_page_size(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_memory::destack_memory_page_size(binding, out) })
}
