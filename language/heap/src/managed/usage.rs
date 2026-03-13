use super::{ManagedHeap, ManagedSpanClass};
use crate::page::{ManagedPage, PageSlot, ValuePage};
use crate::{ManagedHeapUsage, ManagedLargeSpan, ManagedSpan};

impl ManagedHeap {
    /// Return the dedicated managed large-span threshold in values.
    pub(crate) fn large_span_values(&self) -> usize {
        self.large_span_values
    }

    /// Return the managed span class for one external value count.
    pub(crate) fn span_class(&self, len: usize) -> ManagedSpanClass {
        if len >= self.large_span_values {
            ManagedSpanClass::Large
        } else {
            ManagedSpanClass::Paged
        }
    }

    /// Return the gc state.
    pub fn gc_state(&self) -> &crate::GcState {
        &self.gc_state
    }

    /// Return the most recent gc stats if one cycle has completed.
    pub fn gc_stats(&self) -> Option<&crate::GcStats> {
        self.gc_state.last_stats.as_ref()
    }

    /// Return the approximate total managed heap bytes in use.
    pub fn heap_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the exact retained managed heap bytes.
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
        retained_bytes += self.pages.len() * std::mem::size_of::<ManagedPage>();
        retained_bytes += self.value_pages.len() * std::mem::size_of::<ValuePage>();

        // live page payload
        for page in self.pages.iter() {
            retained_bytes += page.retained_bytes();
        }
        for page in self.value_pages.iter() {
            retained_bytes += page.retained_bytes();
        }

        // span descriptors
        retained_bytes += self.free_spans.len() * std::mem::size_of::<ManagedSpan>();
        retained_bytes += self.large_spans.len() * std::mem::size_of::<ManagedLargeSpan>();
        for span in &self.large_spans {
            retained_bytes += span.retained_bytes();
        }

        // allocator and gc state
        retained_bytes += self.locations.len() * std::mem::size_of::<PageSlot>();
        retained_bytes += self.free_ids.len() * std::mem::size_of::<u64>();
        retained_bytes += self.free_large_span_ids.len() * std::mem::size_of::<u64>();
        retained_bytes += self.mark_queue.len() * std::mem::size_of::<crate::ManagedReference>();
        retained_bytes += std::mem::size_of_val(self.gc_state());

        self.retained_bytes = retained_bytes as u64;
    }

    /// Return the number of allocated managed allocations.
    pub fn allocation_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the exact usage for this live managed heap.
    pub fn usage(&self) -> ManagedHeapUsage {
        ManagedHeapUsage {
            allocation_count: self.allocated_count,
            allocation_bytes: self.allocated_bytes,
            retained_bytes: self.retained_bytes,
        }
    }
}
