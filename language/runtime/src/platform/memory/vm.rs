use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::memory::{MemoryRangeVm, ProtectedMemoryRangeVm};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.memory.advise.advise.
pub(super) fn destack_memory_advise(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    advice: u32,
) -> RuntimeResult<()> {
    let _ = (address, length, advice);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.advise.advise is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.memory.advise.discard.
pub(super) fn destack_memory_discard(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.advise.discard is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.memory.advise.hugePage.
pub(super) fn destack_memory_huge_page(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (address, length, enabled);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.advise.hugePage is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.memory.lock.lock.
pub(super) fn destack_memory_lock(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.lock.lock is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.memory.lock.unlock.
pub(super) fn destack_memory_unlock(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.lock.unlock is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.memory.map.commit.
pub(super) fn destack_memory_commit(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = (address, length, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.map.commit is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.memory.map.decommit.
pub(super) fn destack_memory_decommit(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.map.decommit is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.memory.map.numaBind.
pub(super) fn destack_memory_numa_bind(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    policy: u32,
    nodemask: u64,
) -> RuntimeResult<()> {
    let _ = (address, length, policy, nodemask);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.map.numaBind is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.memory.map.release.
pub(super) fn destack_memory_release(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.map.release is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.memory.map.reserve.
pub(super) fn destack_memory_reserve(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    length: u64,
    flags: u32,
) -> RuntimeResult<MemoryRangeVm> {
    let _ = (length, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.map.reserve is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.memory.protect.execute.
pub(super) fn destack_memory_protect_execute(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (address, length, enabled);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.protect.execute is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.memory.protect.flushInstructionCache.
pub(super) fn destack_memory_flush_instruction_cache(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.protect.flushInstructionCache is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.memory.protect.protect.
pub(super) fn destack_memory_protect(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
    protection: u32,
) -> RuntimeResult<()> {
    let _ = (address, length, protection);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.protect.protect is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.memory.protect.remap.
pub(super) fn destack_memory_remap(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    oldlength: u64,
    newlength: u64,
    flags: u32,
) -> RuntimeResult<ProtectedMemoryRangeVm> {
    let _ = (address, oldlength, newlength, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.protect.remap is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.memory.protect.setWriteXorExecute.
pub(super) fn destack_memory_set_write_xor_execute(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = enabled;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.protect.setWriteXorExecute is not available in the VM yet",
    ))
    .boxed())
}
