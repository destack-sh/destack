use super::heap::RawHeap;
use crate::RawHeapUsage;
use crate::page::PageSlot;

impl RawHeap {
    /// Return the exact retained raw heap bytes.
    pub(crate) fn retained_bytes(&self) -> u64 {
        self.retained_bytes
    }

    /// Apply one exact retained-byte delta after a committed mutation.
    pub(crate) fn apply_retained_delta(&mut self, delta: i64) {
        if delta >= 0 {
            self.retained_bytes = self.retained_bytes.saturating_add(delta as u64);
        } else {
            self.retained_bytes = self.retained_bytes.saturating_sub((-delta) as u64);
        }
    }

    /// Recompute exact retained bytes from live state.
    pub(crate) fn recompute_retained_bytes(&mut self) {
        let mut retained_bytes = 0usize;

        // live page descriptors
        retained_bytes += self.pages.len() * std::mem::size_of::<crate::page::RawPage>();

        // live page payload
        for page in self.pages.iter() {
            retained_bytes += page.retained_bytes();
        }

        // span descriptors
        retained_bytes += self.spans.len() * std::mem::size_of::<crate::RawSpan>();
        retained_bytes += self.large_spans.len() * std::mem::size_of::<crate::RawSpan>();
        for span in &self.spans {
            retained_bytes += span.retained_bytes();
        }
        for span in &self.large_spans {
            retained_bytes += span.retained_bytes();
        }

        // allocator state
        retained_bytes += self.locations.len() * std::mem::size_of::<PageSlot>();
        retained_bytes += self.free_ids.len() * std::mem::size_of::<u64>();
        retained_bytes += self.free_span_ids.len() * std::mem::size_of::<u64>();
        retained_bytes += self.free_large_span_ids.len() * std::mem::size_of::<u64>();

        self.retained_bytes = retained_bytes as u64;
    }

    /// Return the exact usage for this live raw heap.
    pub fn usage(&self) -> RawHeapUsage {
        RawHeapUsage {
            allocation_count: self.allocated_count,
            allocation_bytes: self.allocated_bytes,
            retained_bytes: self.retained_bytes,
        }
    }
}
