use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::runtime::BindingCallContext;

use super::core::{page_size, validated_range};

/// Lock one memory range into physical memory.
pub(crate) unsafe fn destack_memory_lock(
    _context: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    // validate range before locking
    let page_size = page_size()?;
    let (pointer, length) = validated_range(address, length, page_size)?;

    // request page lock from the host
    let status = unsafe { libc::mlock(pointer, length) };
    if status != 0 {
        return Err(core_platform::io_error("mlock", None));
    }

    Ok(())
}

/// Unlock one memory range.
pub(crate) unsafe fn destack_memory_unlock(
    _context: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    // validate range before unlocking
    let page_size = page_size()?;
    let (pointer, length) = validated_range(address, length, page_size)?;

    // release the page lock
    let status = unsafe { libc::munlock(pointer, length) };
    if status != 0 {
        return Err(core_platform::io_error("munlock", None));
    }

    Ok(())
}
