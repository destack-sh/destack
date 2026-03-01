use windows_sys::Win32::System::Memory::{VirtualLock, VirtualUnlock};

use crate::diagnostic::RuntimeResult;
use crate::platform::memory::core as memory_core;
use crate::runtime::BindingCallContext;

use super::core::{io_error, page_size};

/// Lock one memory range into physical memory.
pub(crate) unsafe fn destack_memory_lock(
    _context: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    // validate range before locking
    let page_size = page_size()?;
    let address = memory_core::nonzero_address(address, "address")?;
    let length = memory_core::nonzero_length(length, "length")?;
    memory_core::require_page_alignment(address, page_size, "address")?;
    memory_core::require_page_alignment(length, page_size, "length")?;

    // lock pages into physical memory
    let status = unsafe { VirtualLock(address as *mut core::ffi::c_void, length) };
    if status == 0 {
        return Err(io_error("VirtualLock"));
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
    let address = memory_core::nonzero_address(address, "address")?;
    let length = memory_core::nonzero_length(length, "length")?;
    memory_core::require_page_alignment(address, page_size, "address")?;
    memory_core::require_page_alignment(length, page_size, "length")?;

    // release page lock
    let status = unsafe { VirtualUnlock(address as *mut core::ffi::c_void, length) };
    if status == 0 {
        return Err(io_error("VirtualUnlock"));
    }

    Ok(())
}
