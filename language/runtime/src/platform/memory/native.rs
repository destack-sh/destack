#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::memory::bindings_generated as bindings;

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::memory::{MemoryRange, ProtectedMemoryRange};

/// Stub for destack.memory.advise.advise.
pub unsafe fn destack_memory_advise(
    context: &RuntimeCallContext,
    address: u64,
    length: u64,
    advice: u32,
) -> RuntimeResult<()> {
    context.check_policy(MEMORY_ADVISE_ADVISE)?;
    let _ = (address, length, advice);

    Err(RuntimeError::from(PlatformError::not_supported("destack.memory.advise.advise")).boxed())
}

/// Stub for destack.memory.advise.discard.
pub unsafe fn destack_memory_discard(
    context: &RuntimeCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    context.check_policy(MEMORY_ADVISE_DISCARD)?;
    let _ = (address, length);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.advise.discard",
    ))
    .boxed())
}

/// Stub for destack.memory.advise.hugePage.
pub unsafe fn destack_memory_huge_page(
    context: &RuntimeCallContext,
    address: u64,
    length: u64,
    enabled: bool,
) -> RuntimeResult<()> {
    context.check_policy(MEMORY_ADVISE_HUGE_PAGE)?;
    let _ = (address, length, enabled);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.advise.hugePage",
    ))
    .boxed())
}

/// Stub for destack.memory.lock.lock.
pub unsafe fn destack_memory_lock(
    context: &RuntimeCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    context.check_policy(MEMORY_LOCK_LOCK)?;
    let _ = (address, length);

    Err(RuntimeError::from(PlatformError::not_supported("destack.memory.lock.lock")).boxed())
}

/// Stub for destack.memory.lock.unlock.
pub unsafe fn destack_memory_unlock(
    context: &RuntimeCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    context.check_policy(MEMORY_LOCK_UNLOCK)?;
    let _ = (address, length);

    Err(RuntimeError::from(PlatformError::not_supported("destack.memory.lock.unlock")).boxed())
}

/// Stub for destack.memory.map.commit.
pub unsafe fn destack_memory_commit(
    context: &RuntimeCallContext,
    address: u64,
    length: u64,
    flags: u32,
) -> RuntimeResult<()> {
    context.check_policy(MEMORY_MAP_COMMIT)?;
    let _ = (address, length, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.memory.map.commit")).boxed())
}

/// Stub for destack.memory.map.decommit.
pub unsafe fn destack_memory_decommit(
    context: &RuntimeCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    context.check_policy(MEMORY_MAP_DECOMMIT)?;
    let _ = (address, length);

    Err(RuntimeError::from(PlatformError::not_supported("destack.memory.map.decommit")).boxed())
}

/// Stub for destack.memory.map.numaBind.
pub unsafe fn destack_memory_numa_bind(
    context: &RuntimeCallContext,
    address: u64,
    length: u64,
    policy: u32,
    nodemask: u64,
) -> RuntimeResult<()> {
    context.check_policy(MEMORY_MAP_NUMA_BIND)?;
    let _ = (address, length, policy, nodemask);

    Err(RuntimeError::from(PlatformError::not_supported("destack.memory.map.numaBind")).boxed())
}

/// Stub for destack.memory.map.release.
pub unsafe fn destack_memory_release(
    context: &RuntimeCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    context.check_policy(MEMORY_MAP_RELEASE)?;
    let _ = (address, length);

    Err(RuntimeError::from(PlatformError::not_supported("destack.memory.map.release")).boxed())
}

/// Stub for destack.memory.map.reserve.
pub unsafe fn destack_memory_reserve(
    context: &RuntimeCallContext,
    out: *mut MemoryRange,
    length: u64,
    flags: u32,
) -> RuntimeResult<()> {
    context.check_policy(MEMORY_MAP_RESERVE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, length, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.memory.map.reserve")).boxed())
}

/// Stub for destack.memory.protect.execute.
pub unsafe fn destack_memory_protect_execute(
    context: &RuntimeCallContext,
    address: u64,
    length: u64,
    enabled: bool,
) -> RuntimeResult<()> {
    context.check_policy(MEMORY_PROTECT_EXECUTE)?;
    let _ = (address, length, enabled);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.protect.execute",
    ))
    .boxed())
}

/// Stub for destack.memory.protect.flushInstructionCache.
pub unsafe fn destack_memory_flush_instruction_cache(
    context: &RuntimeCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    context.check_policy(MEMORY_PROTECT_FLUSH_INSTRUCTION_CACHE)?;
    let _ = (address, length);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.protect.flushInstructionCache",
    ))
    .boxed())
}

/// Stub for destack.memory.protect.protect.
pub unsafe fn destack_memory_protect(
    context: &RuntimeCallContext,
    address: u64,
    length: u64,
    protection: u32,
) -> RuntimeResult<()> {
    context.check_policy(MEMORY_PROTECT_PROTECT)?;
    let _ = (address, length, protection);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.protect.protect",
    ))
    .boxed())
}

/// Stub for destack.memory.protect.remap.
pub unsafe fn destack_memory_remap(
    context: &RuntimeCallContext,
    out: *mut ProtectedMemoryRange,
    address: u64,
    oldlength: u64,
    newlength: u64,
    flags: u32,
) -> RuntimeResult<()> {
    context.check_policy(MEMORY_PROTECT_REMAP)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, address, oldlength, newlength, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.memory.protect.remap")).boxed())
}

/// Stub for destack.memory.protect.setWriteXorExecute.
pub unsafe fn destack_memory_set_write_xor_execute(
    context: &RuntimeCallContext,
    enabled: bool,
) -> RuntimeResult<()> {
    context.check_policy(MEMORY_PROTECT_SET_WRITE_XOR_EXECUTE)?;
    let _ = enabled;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.memory.protect.setWriteXorExecute",
    ))
    .boxed())
}
