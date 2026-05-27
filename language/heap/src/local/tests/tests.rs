use std::sync::{Arc, OnceLock};

use crate::allocator::Allocator;
use crate::local::{Heap, HeapLimits, HeapOptions};
use destack_mir::TraceTable;

static TRACE_TABLE: OnceLock<TraceTable> = OnceLock::new();

/// One heap test harness.
pub(crate) struct TestHeap {
    /// The heap under test.
    pub(crate) heap: Heap,
}

impl TestHeap {
    /// Create one test heap with default options.
    pub(crate) fn new() -> Self {
        let options = HeapOptions::local();
        let allocator = Arc::new(
            Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
                .expect("default allocator should build"),
        );

        Self {
            heap: Heap::with_allocator_limits_and_options(
                allocator,
                HeapLimits::default(),
                options,
            )
            .expect("default heap should build"),
        }
    }

    /// Create one test heap with explicit options.
    pub(crate) fn with_options(options: HeapOptions) -> Self {
        Self::with_limits_and_options(HeapLimits::default(), options)
    }

    /// Create one test heap with explicit limits and options.
    pub(crate) fn with_limits_and_options(limits: HeapLimits, options: HeapOptions) -> Self {
        let allocator = Arc::new(
            Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
                .expect("explicit allocator should build"),
        );

        Self {
            heap: Heap::with_allocator_limits_and_options(allocator, limits, options)
                .expect("explicit heap options should build"),
        }
    }
}

/// Return the shared empty trace table for heap tests.
pub(crate) fn trace_table() -> &'static TraceTable {
    TRACE_TABLE.get_or_init(TraceTable::new)
}

/// Read bytes from one mapped heap address.
pub(crate) fn read_mapped_bytes(address: usize, byte_len: usize) -> Vec<u8> {
    // SAFETY: tests only read ranges they just allocated or restored
    unsafe { std::slice::from_raw_parts(address as *const u8, byte_len).to_vec() }
}

/// Write one byte to one mapped heap address.
pub(crate) fn write_mapped_byte(address: usize, byte: u8) {
    // SAFETY: tests only write ranges they just allocated or restored
    unsafe {
        std::ptr::write(address as *mut u8, byte);
    }
}

/// Write bytes to one mapped heap address.
pub(crate) fn write_mapped_bytes(address: usize, bytes: &[u8]) {
    // SAFETY: tests only write ranges they just allocated or restored
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len());
    }
}
