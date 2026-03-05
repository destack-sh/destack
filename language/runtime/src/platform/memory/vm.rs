use crate::diagnostic::RuntimeResult;
use crate::platform::memory::{
    MemoryAdvice, MemoryNumaPolicy, MemoryProtection, MemoryRangeVm, MemoryRemapFlags,
    MemoryReserveFlags, ProtectedMemoryRangeVm,
};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

use super::host as host_memory;

/// Invoke one host call that writes one output pointer.
fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    // allocate uninitialized storage for the host out pointer
    let mut out = std::mem::MaybeUninit::<T>::uninit();

    // execute the host call and initialize output
    call(out.as_mut_ptr())?;

    // return initialized output value
    Ok(unsafe { out.assume_init() })
}

/// Apply memory access advice.
pub(crate) fn destack_memory_advise(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    advice: MemoryAdvice,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_advise(binding, address, length, advice) }
}

/// Discard memory contents.
pub(crate) fn destack_memory_discard(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_discard(binding, address, length) }
}

/// Toggle huge-page preference for one range.
pub(crate) fn destack_memory_huge_page(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_huge_page(binding, address, length, enabled) }
}

/// Lock one memory range into physical memory.
pub(crate) fn destack_memory_lock(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_lock(binding, address, length) }
}

/// Unlock one memory range.
pub(crate) fn destack_memory_unlock(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_unlock(binding, address, length) }
}

/// Commit one reserved range.
pub(crate) fn destack_memory_commit(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    protection: MemoryProtection,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_commit(binding, address, length, protection) }
}

/// Decommit one range.
pub(crate) fn destack_memory_decommit(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_decommit(binding, address, length) }
}

/// Bind one range to a NUMA policy.
pub(crate) fn destack_memory_numa_bind(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    policy: MemoryNumaPolicy,
    nodemask: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_numa_bind(binding, address, length, policy, nodemask) }
}

/// Release one reserved range.
pub(crate) fn destack_memory_release(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_release(binding, address, length) }
}

/// Reserve one virtual memory range.
pub(crate) fn destack_memory_reserve(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
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
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_flush_instruction_cache(binding, address, length) }
}

/// Change memory protection for one range.
pub(crate) fn destack_memory_protect(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    protection: MemoryProtection,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_protect(binding, address, length, protection) }
}

/// Resize one mapped range.
pub(crate) fn destack_memory_remap(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
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
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_memory::destack_memory_allocation_granularity(binding, out) })
}

/// Read the host huge-page allocation size when available.
pub(crate) fn destack_memory_huge_page_size(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<Option<u64>> {
    call_out(|out| unsafe { host_memory::destack_memory_huge_page_size(binding, out) })
}

/// Read the host virtual-memory page size.
pub(crate) fn destack_memory_page_size(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_memory::destack_memory_page_size(binding, out) })
}
