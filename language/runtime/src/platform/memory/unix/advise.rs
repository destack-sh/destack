use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::memory::MemoryAdvice;
use crate::runtime::BindingCallContext;

use super::core::{page_size, validated_range};

/// Apply memory access advice.
pub(crate) unsafe fn destack_memory_advise(
    _binding: &BindingCallContext,
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
        return Err(core_platform::io_error("madvise", None));
    }

    Ok(())
}

/// Discard memory contents.
pub(crate) unsafe fn destack_memory_discard(
    _binding: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    // validate range for discard operation
    let page_size = page_size()?;
    let (pointer, length) = validated_range(address, length, page_size)?;

    // request discard semantics from the host
    let status = unsafe { libc::madvise(pointer, length, libc::MADV_DONTNEED) };
    if status != 0 {
        return Err(core_platform::io_error("madvise", None));
    }

    Ok(())
}
