use super::{
    decode_memory_range, page_aligned_length, protection_read_write, reserve_flags_none,
    with_harness_context,
};
use crate::platform::memory::MemoryAdvice;

/// Apply advisory memory operations.
#[cfg(any(unix, windows))]
#[test]
fn test_memory_advise_and_discard_roundtrip() {
    with_harness_context(|mut context| {
        let page_size = context.destack_memory_page_size()?;
        let length = page_aligned_length(page_size);

        let mapping = context.destack_memory_reserve(length, 0, reserve_flags_none())?;
        let mapping = decode_memory_range(mapping);
        context.destack_memory_commit(mapping.address, mapping.length, protection_read_write())?;

        context.destack_memory_advise(mapping.address, mapping.length, MemoryAdvice::Normal)?;
        context.destack_memory_discard(mapping.address, mapping.length)?;

        context.destack_memory_release(mapping.address, mapping.length)?;

        Ok(())
    });
}
