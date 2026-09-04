use crate::local::storage::HeapStorage;
use crate::{HeapReference, HeapResult};

impl HeapStorage {
    /// Return the currently live heap references.
    pub(crate) fn live_references(&self) -> HeapResult<Vec<HeapReference>> {
        // collect occupied span slots
        let mut references = Vec::new();
        for span in self.small.spans.iter() {
            for slot_index in 0..span.slot_count {
                if !span.occupied.contains(slot_index) {
                    continue;
                }

                references.push(HeapReference::new(span.slot_offset(slot_index)));
            }
        }

        // collect live large blocks
        for block in self.large.blocks.iter().flatten() {
            references.push(HeapReference::new(block.first_offset));
        }

        Ok(references)
    }
}
