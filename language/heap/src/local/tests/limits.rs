use crate::{
    AccountingRegion, DEFAULT_YOUNG_SIZE_BYTES, HeapError, HeapLimits, HeapOptions,
    HeapSpaceLimits, Payload, RawAllocationShape, RawLimits, test_layout,
};
use destack_mir::TraceMap;

use super::{TestHeap, trace_table};

const SMALL_ALLOCATION_COUNT: usize = 1024;
const SMALL_ALLOCATION_BYTES: usize = 32;

/// Return the retained heap bytes for one block in the given heap options.
fn heap_retained_bytes_after_allocate(options: HeapOptions, bytes: &[u8]) -> u64 {
    // allocate one managed payload under the requested options
    let layout = test_layout(bytes.len(), TraceMap::empty());
    let mut test_heap = TestHeap::with_limits_and_options(crate::HeapLimits::default(), options);
    let heap = &mut test_heap.heap;

    heap.allocate_payload(&heap.allocation_plan(layout.block()), Payload::Bytes(bytes))
        .expect("heap block should succeed");

    // report retained bytes after allocator rounding
    heap.usage().heap.retained_bytes
}

/// Return the retained local raw bytes after one replacement in the given heap options.
fn raw_retained_bytes_after_allocate(options: HeapOptions, bytes: &[u8]) -> u64 {
    // allocate one raw payload under the requested options
    let mut test_heap = TestHeap::with_options(options);
    let heap = &mut test_heap.heap;

    heap.allocate_raw(
        RawAllocationShape::bytes(bytes.len()),
        Payload::Bytes(bytes),
    )
    .expect("raw block should succeed");

    // report retained bytes after allocator rounding
    heap.usage().raw.retained_bytes
}

/// Track retained bytes for default young-space block.
#[test]
fn test_track_default_young_retained_bytes() {
    // fill the default nursery with small zeroed objects
    let layout = test_layout(SMALL_ALLOCATION_BYTES, TraceMap::empty());
    let mut test_heap = TestHeap::new();
    let heap = &mut test_heap.heap;

    for _ in 0..SMALL_ALLOCATION_COUNT {
        heap.allocate_payload(&heap.allocation_plan(layout.block()), Payload::Zeroed)
            .expect("heap block should succeed");
    }

    let usage = heap.usage().heap;

    // retained bytes should be the whole reserved nursery
    assert_eq!(usage.allocation_count, SMALL_ALLOCATION_COUNT);
    assert_eq!(
        usage.allocated_bytes,
        (SMALL_ALLOCATION_COUNT * SMALL_ALLOCATION_BYTES) as u64
    );
    assert_eq!(usage.retained_bytes, DEFAULT_YOUNG_SIZE_BYTES as u64);
}

/// Track retained bytes for local small-span block.
#[test]
fn test_track_small_span_retained_bytes() {
    // disable young space so all objects use mature small spans
    let options = HeapOptions {
        heap_young_size_bytes: 0,
        max_heap_young_allocation_size_bytes: 0,
        ..HeapOptions::local()
    };
    let layout = test_layout(SMALL_ALLOCATION_BYTES, TraceMap::empty());
    let class_index = options
        .size_classes
        .class_index_for(SMALL_ALLOCATION_BYTES)
        .expect("small block size should have a class");
    let size_class = options.size_classes.classes[class_index];
    let span_size_bytes = size_class
        .span_size_bytes(options.page_size_bytes, options.heap_small_size_bytes)
        .max(options.heap_small_size_bytes);
    let slot_count = span_size_bytes / size_class.bytes;
    let span_count = SMALL_ALLOCATION_COUNT.div_ceil(slot_count);
    let retained_bytes = span_count * span_size_bytes;
    let mut test_heap = TestHeap::with_options(options);
    let heap = &mut test_heap.heap;

    // allocate enough objects to cover several slots and spans
    for _ in 0..SMALL_ALLOCATION_COUNT {
        heap.allocate_payload(&heap.allocation_plan(layout.block()), Payload::Zeroed)
            .expect("heap block should succeed");
    }

    let usage = heap.usage().heap;

    // retained bytes should be rounded by span geometry
    assert_eq!(usage.allocation_count, SMALL_ALLOCATION_COUNT);
    assert_eq!(
        usage.allocated_bytes,
        (SMALL_ALLOCATION_COUNT * SMALL_ALLOCATION_BYTES) as u64
    );
    assert_eq!(usage.retained_bytes, retained_bytes as u64);
}

/// Reject one heap block when the retained-byte limit would be exceeded.
#[test]
fn test_reject_heap_allocation_when_limit_exceeded() {
    // compute the projected retained-byte charge for one block
    let options = HeapOptions {
        heap_young_size_bytes: 0,
        max_heap_young_allocation_size_bytes: 0,
        ..HeapOptions::local()
    };
    let expected_used_bytes = heap_retained_bytes_after_allocate(options.clone(), &[1]);
    let layout = test_layout(1, TraceMap::empty());
    let mut test_heap = TestHeap::with_limits_and_options(crate::HeapLimits::default(), options);
    let heap = &mut test_heap.heap;
    let baseline = heap.usage().heap.retained_bytes;
    heap.set_limits(HeapLimits {
        max_bytes: None,
        heap: HeapSpaceLimits {
            max_bytes: Some(baseline),
        },
        raw: RawLimits { max_bytes: None },
    })
    .expect("baseline heap should fit its current retained-byte limit");

    // reject the block before mutating heap accounting
    let error = heap
        .allocate_payload(&heap.allocation_plan(layout.block()), Payload::Bytes(&[1]))
        .expect_err("heap block should be rejected");

    assert_eq!(
        error,
        HeapError::LimitExceeded {
            region: AccountingRegion::Heap,
            used_bytes: expected_used_bytes,
            max_bytes: baseline,
        }
    );
    assert_eq!(heap.heap_allocation_count(), 0);
    assert_eq!(heap.usage().heap.retained_bytes, baseline);
}

/// Reject one raw block when the retained-byte limit would be exceeded.
#[test]
fn test_reject_raw_allocation_when_limit_exceeded() {
    // compute the projected retained-byte charge for one block
    let expected_used_bytes = raw_retained_bytes_after_allocate(HeapOptions::local(), &[1]);
    let mut test_heap = TestHeap::new();
    let heap = &mut test_heap.heap;
    let baseline = heap.usage().raw.retained_bytes;
    heap.set_limits(HeapLimits {
        max_bytes: None,
        heap: HeapSpaceLimits { max_bytes: None },
        raw: RawLimits {
            max_bytes: Some(baseline),
        },
    })
    .expect("baseline raw heap should fit its current retained-byte limit");

    // reject the block before mutating raw accounting
    let error = heap
        .allocate_raw(RawAllocationShape::bytes(1), Payload::Bytes(&[1]))
        .expect_err("raw block should be rejected");

    assert_eq!(
        error,
        HeapError::LimitExceeded {
            region: AccountingRegion::Raw,
            used_bytes: expected_used_bytes,
            max_bytes: baseline,
        }
    );
    assert_eq!(heap.raw_allocation_count(), 0);
    assert_eq!(heap.usage().raw.retained_bytes, baseline);
}

/// Preserve custom heap limits across in-storage heap image restore.
#[test]
fn test_restore_heap_image_preserves_limits() {
    // install non-default hard limits before capture
    let mut test_heap = TestHeap::new();
    let heap = &mut test_heap.heap;
    let limits = HeapLimits {
        max_bytes: Some(heap.usage().retained_bytes() + 4096),
        heap: HeapSpaceLimits {
            max_bytes: Some(heap.usage().heap.retained_bytes + 2048),
        },
        raw: RawLimits {
            max_bytes: Some(heap.usage().raw.retained_bytes + 2048),
        },
    };
    heap.set_limits(limits)
        .expect("custom heap limits should fit current usage");
    let image = heap.image().expect("heap image should capture");

    // restoring one captured image should keep the existing hard limits
    heap.restore_image(&image, trace_table())
        .expect("heap image restore should succeed");

    assert_eq!(heap.limits(), limits);
}

/// Reject one raw replace when byte growth would exceed the retained-byte limit.
#[test]
fn test_reject_raw_replace_when_limit_exceeded() {
    // allocate one raw payload and cap retained bytes at the baseline
    let mut test_heap = TestHeap::with_options(HeapOptions::local());
    let heap = &mut test_heap.heap;
    let pointer = heap
        .allocate_raw(
            RawAllocationShape::bytes(4097),
            Payload::Bytes(&vec![0xAA; 4097]),
        )
        .expect("raw block should succeed");
    let baseline = heap.usage().raw.retained_bytes;
    heap.set_limits(HeapLimits {
        max_bytes: None,
        heap: HeapSpaceLimits { max_bytes: None },
        raw: RawLimits {
            max_bytes: Some(baseline),
        },
    })
    .expect("baseline raw heap should fit its current retained-byte limit");
    let retained_byte_delta = heap
        .raw
        .replace_retained_byte_delta(pointer, 8193)
        .expect("raw replacement should project");
    let expected_used_bytes = baseline + retained_byte_delta as u64;

    // reject the replacement before mutating bytes or accounting
    let error = heap
        .replace_raw_bytes(pointer, &vec![0xBB; 8193])
        .expect_err("raw replace should be rejected");

    assert_eq!(
        error,
        HeapError::LimitExceeded {
            region: AccountingRegion::Raw,
            used_bytes: expected_used_bytes,
            max_bytes: baseline,
        }
    );
    assert_eq!(heap.read_raw_bytes(pointer), Ok(vec![0xAA; 4097]));
    assert_eq!(heap.usage().raw.retained_bytes, baseline);
}
