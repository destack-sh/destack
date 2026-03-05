use windows_sys::Win32::System::Memory::GetLargePageMinimum;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::runtime::BindingCallContext;

use super::core::{allocation_granularity, page_size};

/// Read the host allocation granularity.
pub(crate) unsafe fn destack_memory_allocation_granularity(
    binding: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    // validate output pointer and query allocation granularity
    core_platform::ensure_out(out, "out")?;

    let granularity = allocation_granularity()?;
    let granularity = core_platform::usize_to_u64(granularity, "out")?;

    // write allocation granularity result
    unsafe {
        out.write(granularity);
    }

    Ok(())
}

/// Read the host huge-page allocation size when available.
pub(crate) unsafe fn destack_memory_huge_page_size(
    binding: &BindingCallContext,
    out: *mut Option<u64>,
) -> RuntimeResult<()> {
    // validate output pointer for optional huge-page size
    core_platform::ensure_out(out, "out")?;

    // query windows large-page minimum
    let huge_page_size = unsafe { GetLargePageMinimum() };
    if huge_page_size == 0 {
        // report unavailable huge-page size
        unsafe {
            out.write(None);
        }

        return Ok(());
    }

    // write huge-page size when available
    let huge_page_size = core_platform::usize_to_u64(huge_page_size, "out")?;
    unsafe {
        out.write(Some(huge_page_size));
    }

    Ok(())
}

/// Read the host virtual-memory page size.
pub(crate) unsafe fn destack_memory_page_size(
    binding: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    // validate output pointer and query page size
    core_platform::ensure_out(out, "out")?;

    let page_size = page_size()?;
    let page_size = core_platform::usize_to_u64(page_size, "out")?;

    // write page size result
    unsafe {
        out.write(page_size);
    }

    Ok(())
}
