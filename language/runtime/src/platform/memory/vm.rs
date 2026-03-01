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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    advice: MemoryAdvice,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_advise(runtime, address, length, advice) }
}

/// Discard memory contents.
pub(crate) fn destack_memory_discard(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_discard(runtime, address, length) }
}

/// Toggle huge-page preference for one range.
pub(crate) fn destack_memory_huge_page(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_huge_page(runtime, address, length, enabled) }
}

/// Lock one memory range into physical memory.
pub(crate) fn destack_memory_lock(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_lock(runtime, address, length) }
}

/// Unlock one memory range.
pub(crate) fn destack_memory_unlock(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_unlock(runtime, address, length) }
}

/// Commit one reserved range.
pub(crate) fn destack_memory_commit(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    protection: MemoryProtection,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_commit(runtime, address, length, protection) }
}

/// Decommit one range.
pub(crate) fn destack_memory_decommit(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_decommit(runtime, address, length) }
}

/// Bind one range to a NUMA policy.
pub(crate) fn destack_memory_numa_bind(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    policy: MemoryNumaPolicy,
    nodemask: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_numa_bind(runtime, address, length, policy, nodemask) }
}

/// Release one reserved range.
pub(crate) fn destack_memory_release(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_release(runtime, address, length) }
}

/// Reserve one virtual memory range.
pub(crate) fn destack_memory_reserve(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    length: u64,
    addresshint: u64,
    flags: MemoryReserveFlags,
) -> RuntimeResult<MemoryRangeVm> {
    call_out(|out| unsafe {
        host_memory::destack_memory_reserve(runtime, out, length, addresshint, flags)
    })
}

/// Flush instruction cache for one range.
pub(crate) fn destack_memory_flush_instruction_cache(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_flush_instruction_cache(runtime, address, length) }
}

/// Change memory protection for one range.
pub(crate) fn destack_memory_protect(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    protection: MemoryProtection,
) -> RuntimeResult<()> {
    unsafe { host_memory::destack_memory_protect(runtime, address, length, protection) }
}

/// Resize one mapped range.
pub(crate) fn destack_memory_remap(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    oldlength: u64,
    newlength: u64,
    flags: MemoryRemapFlags,
) -> RuntimeResult<ProtectedMemoryRangeVm> {
    call_out(|out| unsafe {
        host_memory::destack_memory_remap(runtime, out, address, oldlength, newlength, flags)
    })
}

/// Read the host allocation granularity.
pub(crate) fn destack_memory_allocation_granularity(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_memory::destack_memory_allocation_granularity(runtime, out) })
}

/// Read the host huge-page allocation size when available.
pub(crate) fn destack_memory_huge_page_size(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<Option<u64>> {
    call_out(|out| unsafe { host_memory::destack_memory_huge_page_size(runtime, out) })
}

/// Read the host virtual-memory page size.
pub(crate) fn destack_memory_page_size(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_memory::destack_memory_page_size(runtime, out) })
}
