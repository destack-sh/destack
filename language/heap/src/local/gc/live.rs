use crate::local::storage::HeapStorage;
use crate::{HeapError, HeapReference, HeapResult};

impl HeapStorage {
    /// Return the currently live heap references.
    pub(crate) fn live_references(&self) -> HeapResult<Vec<HeapReference>> {
        let mut references = Vec::new();
        let mut start = 0;

        // contiguous young blocks
        while let Some(block_index) = self.young.live.first_set_from(start) {
            let Some(block) = self.young_range(block_index) else {
                return Err(HeapError::Internal {
                    context: "live young range missing",
                });
            };
            references.push(HeapReference::new(block.first_offset));
            start = block_index + 1;
        }

        // fixed-size young spans
        for (span_index, span) in self.young.spans.iter().enumerate() {
            let Some(bits) = self.young.span_bits(span_index) else {
                return Err(HeapError::internal("missing span"));
            };

            let Some(slot_count) = self.young.span_reserved_slot_count(span_index) else {
                return Err(HeapError::internal("missing span"));
            };

            for slot_index in 0..slot_count {
                if bits.freed.contains(slot_index) {
                    continue;
                }

                let offset = span.slot_offset(slot_index);

                references.push(HeapReference::new(offset));
            }
        }

        // mature small spans
        for span in self.small.spans.iter() {
            for slot_index in 0..span.slot_count {
                if !span.occupied.contains(slot_index) {
                    continue;
                }

                let slot_offset = span.class.size_class * slot_index;
                let base_offset = span.first_offset + slot_offset;

                references.push(HeapReference::new(base_offset));
            }
        }

        // mature large blocks
        for block in self.large.blocks.iter() {
            if !block.is_live {
                continue;
            }

            references.push(HeapReference::new(block.first_offset));
        }

        Ok(references)
    }
}
