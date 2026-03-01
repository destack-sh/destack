use crate::diagnostic::RuntimeResult;
use crate::platform::memory::MemoryAdvice;
use crate::runtime::BindingCallContext;

use super::core::{HUGE_PAGE_OPERATION, io_error, page_size, validated_range};
use crate::platform::memory::core::not_supported;

/// Apply memory access advice.
pub(crate) unsafe fn destack_memory_advise(
    _context: &BindingCallContext,
    address: u64,
    length: u64,
    advice: MemoryAdvice,
) -> RuntimeResult<()> {
    // validate range for advisory operation
    let page_size = page_size()?;
    let (pointer, length) = validated_range(address, length, page_size)?;

    // map portable advice enum to native madvise constants
    let advice = match advice {
        MemoryAdvice::Normal => libc::MADV_NORMAL,
        MemoryAdvice::Sequential => libc::MADV_SEQUENTIAL,
        MemoryAdvice::Random => libc::MADV_RANDOM,
        MemoryAdvice::WillNeed => libc::MADV_WILLNEED,
        MemoryAdvice::DontNeed => libc::MADV_DONTNEED,
    };

    // apply the host advisory hint
    let status = unsafe { libc::madvise(pointer, length, advice) };
    if status != 0 {
        return Err(io_error("madvise"));
    }

    Ok(())
}

/// Discard memory contents.
pub(crate) unsafe fn destack_memory_discard(
    _context: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    // validate range for discard operation
    let page_size = page_size()?;
    let (pointer, length) = validated_range(address, length, page_size)?;

    // request discard semantics from the host
    let status = unsafe { libc::madvise(pointer, length, libc::MADV_DONTNEED) };
    if status != 0 {
        return Err(io_error("madvise"));
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
    // validate range for huge-page preference toggle
    let page_size = page_size()?;
    let (pointer, length) = validated_range(address, length, page_size)?;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // map enable state to linux huge-page advisory constants
        let advice = if enabled {
            libc::MADV_HUGEPAGE
        } else {
            libc::MADV_NOHUGEPAGE
        };

        // apply huge-page preference hint
        let status = unsafe { libc::madvise(pointer, length, advice) };
        if status != 0 {
            return Err(io_error("madvise"));
        }

        return Ok(());
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        // mark huge-page hinting as unsupported on this backend
        let _ = (pointer, length, enabled);
        Err(not_supported(HUGE_PAGE_OPERATION))
    }
}
