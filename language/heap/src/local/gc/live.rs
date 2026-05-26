use crate::local::space::HeapSpace;
use crate::{HeapError, HeapReference, HeapResult};

impl HeapSpace {
    /// Return the currently live heap references.
    pub fn live_references(&self) -> HeapResult<Vec<HeapReference>> {
        let mut references = Vec::new();
        let mut start = 0;

        // contiguous young allocations
        while let Some(allocation_index) = self.young.live.first_set_from(start) {
            let Some(allocation) = self.young_range(allocation_index) else {
                return Err(HeapError::InvariantViolation {
                    context: "live young range missing",
                });
            };
            references.push(HeapReference::new(allocation.first_offset));
            start = allocation_index + 1;
        }

        // fixed-size young runs
        for (run_index, run) in self.young.runs.iter().enumerate() {
            let Some(bits) = self.young.run_bits(run_index) else {
                return Err(HeapError::MissingSpan {
                    span_index: run_index,
                });
            };

            let Some(slot_count) = self.young.run_reserved_slot_count(run_index) else {
                return Err(HeapError::MissingSpan {
                    span_index: run_index,
                });
            };

            for slot_index in 0..slot_count {
                if bits.freed.contains(slot_index) {
                    continue;
                }

                let offset = run.slot_offset(slot_index);

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

        // mature large allocations
        for allocation in self.large.allocations.iter() {
            if !allocation.is_live {
                continue;
            }

            references.push(HeapReference::new(allocation.first_offset));
        }

        Ok(references)
    }
}
