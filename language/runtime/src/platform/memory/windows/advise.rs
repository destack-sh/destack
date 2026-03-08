use windows_sys::Win32::Foundation::{ERROR_CALL_NOT_IMPLEMENTED, ERROR_NOT_SUPPORTED};
use windows_sys::Win32::System::Memory::{
    DiscardVirtualMemory, PrefetchVirtualMemory, WIN32_MEMORY_RANGE_ENTRY,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::memory::{MemoryAdvice, core as memory_core};
use crate::runtime::BindingCallContext;

use super::core::{HUGE_PAGE_OPERATION, page_size};

/// Operation tag for generic memory-advise support.
const ADVISE_OPERATION: &str = "destack.memory.advise";
/// Operation tag for the will-need advise lane.
const WILL_NEED_OPERATION: &str = "destack.memory.advise.willNeed";
/// Operation tag for explicit memory discard.
const DISCARD_OPERATION: &str = "destack.memory.discard";

/// Return whether one Win32 status code maps to not-supported behavior.
fn is_not_supported_status(status: u32) -> bool {
    status == ERROR_NOT_SUPPORTED || status == ERROR_CALL_NOT_IMPLEMENTED
}

/// Convert one `DiscardVirtualMemory` status into a runtime result.
fn discard_status_result(status: u32, operation: &'static str) -> RuntimeResult<()> {
    // treat zero status as success
    if status == 0 {
        return Ok(());
    }

    // map unsupported status values to one explicit notSupported lane
    if is_not_supported_status(status) {
        return Err(core_platform::not_supported(operation));
    }

    // preserve all remaining status values as explicit io errors
    Err(core_platform::io_error_with_code(
        "DiscardVirtualMemory",
        status as i32,
    ))
}

/// Apply memory access advice.
pub(crate) unsafe fn destack_memory_advise(
    _binding: &BindingCallContext,
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
                let code = core_platform::last_error_code() as u32;
                if code == 0 || is_not_supported_status(code) {
                    return Err(core_platform::not_supported(WILL_NEED_OPERATION));
                }

                return Err(core_platform::io_error_with_code(
                    "PrefetchVirtualMemory",
                    code as i32,
                ));
            }
        }
        MemoryAdvice::DontNeed => {
            let status = unsafe { DiscardVirtualMemory(address as *mut core::ffi::c_void, length) };
            discard_status_result(status, ADVISE_OPERATION)?;
        }
        // no-op for advisory hints without direct windows mapping
        MemoryAdvice::Normal | MemoryAdvice::Sequential | MemoryAdvice::Random => {}
    }

    Ok(())
}

/// Discard memory contents.
pub(crate) unsafe fn destack_memory_discard(
    _binding: &BindingCallContext,
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
    discard_status_result(status, DISCARD_OPERATION)?;

    Ok(())
}

/// Toggle huge-page preference for one range.
pub(crate) unsafe fn destack_memory_huge_page(
    _binding: &BindingCallContext,
    address: u64,
    length: u64,
    enabled: bool,
) -> RuntimeResult<()> {
    // mark huge-page advice as unsupported on windows
    let _ = (address, length, enabled);
    Err(core_platform::not_supported(HUGE_PAGE_OPERATION))
}
