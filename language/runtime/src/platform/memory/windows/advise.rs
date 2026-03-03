use windows_sys::Win32::System::Memory::{
    DiscardVirtualMemory, PrefetchVirtualMemory, WIN32_MEMORY_RANGE_ENTRY,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::memory::{MemoryAdvice, core as memory_core};
use crate::runtime::BindingCallContext;

use super::core::{HUGE_PAGE_OPERATION, page_size};

/// Apply memory access advice.
pub(crate) unsafe fn destack_memory_advise(
    _context: &BindingCallContext,
    address: u64,
    length: u64,
    advice: MemoryAdvice,
) -> RuntimeResult<()> {
    // validate range before advisory operation
    let page_size = page_size()?;
    let address = memory_core::nonzero_address(address, "address")?;
    let length = memory_core::nonzero_length(length, "length")?;
    memory_core::require_page_alignment(address, page_size, "address")?;
    memory_core::require_page_alignment(length, page_size, "length")?;

    // dispatch to the closest windows advisory primitive
    match advice {
        MemoryAdvice::WillNeed => {
            let range = WIN32_MEMORY_RANGE_ENTRY {
                VirtualAddress: address as *mut core::ffi::c_void,
                NumberOfBytes: length,
            };
            let status = unsafe { PrefetchVirtualMemory(GetCurrentProcess(), 1, &range, 0) };
            if status == 0 {
                return Err(core_platform::io_error("PrefetchVirtualMemory"));
            }
        }
        MemoryAdvice::DontNeed => {
            let status = unsafe { DiscardVirtualMemory(address as *mut core::ffi::c_void, length) };
            if status == 0 {
                return Err(core_platform::io_error("DiscardVirtualMemory"));
            }
        }
        // no-op for advisory hints without direct windows mapping
        MemoryAdvice::Normal | MemoryAdvice::Sequential | MemoryAdvice::Random => {}
    }

    Ok(())
}

/// Discard memory contents.
pub(crate) unsafe fn destack_memory_discard(
    _context: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    // validate range before discard
    let page_size = page_size()?;
    let address = memory_core::nonzero_address(address, "address")?;
    let length = memory_core::nonzero_length(length, "length")?;
    memory_core::require_page_alignment(address, page_size, "address")?;
    memory_core::require_page_alignment(length, page_size, "length")?;

    // discard pages from the range
    let status = unsafe { DiscardVirtualMemory(address as *mut core::ffi::c_void, length) };
    if status == 0 {
        return Err(core_platform::io_error("DiscardVirtualMemory"));
    }

    Ok(())
}

/// Toggle huge-page preference for one range.
pub(crate) unsafe fn destack_memory_huge_page(
    _context: &BindingCallContext,
    address: u64,
    length: u64,
    enabled: bool,
) -> RuntimeResult<()> {
    // mark huge-page advice as unsupported on windows
    let _ = (address, length, enabled);
    Err(core_platform::not_supported(HUGE_PAGE_OPERATION))
}
