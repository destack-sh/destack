use crate::{
    AccountingRegion, DEFAULT_YOUNG_BYTES, HeapError, HeapLimits, HeapOptions, HeapSpaceLimits,
    Payload, RawLimits, test_layout,
};
use destack_mir::ReferenceMap;

use super::TestHeap;

const SMALL_ALLOCATION_COUNT: usize = 1024;
const SMALL_ALLOCATION_BYTES: usize = 32;

/// Return the retained heap bytes for one allocation in the given heap options.
fn heap_retained_bytes_after_allocate(options: HeapOptions, bytes: &[u8]) -> u64 {
    let layout = test_layout(bytes.len(), ReferenceMap::empty());
    let mut test_heap = TestHeap::with_limits_and_options(crate::HeapLimits::default(), options);
    let heap = &mut test_heap.heap;

    heap.allocate(
        &heap.allocation_layout(layout.allocation()),
        Payload::Bytes(bytes),
    )
    .expect("heap allocation should succeed");

    heap.usage().heap.retained_bytes
}

/// Return the retained local raw bytes after one replacement in the given heap options.
fn raw_retained_bytes_after_allocate(options: HeapOptions, bytes: &[u8]) -> u64 {
    let mut test_heap = TestHeap::with_options(options);
    let heap = &mut test_heap.heap;

    heap.allocate_raw(bytes.len(), Payload::Bytes(bytes))
        .expect("raw allocation should succeed");

    heap.usage().raw.retained_bytes
}

/// Track retained bytes for default young-space allocation.
#[test]
fn test_track_default_young_retained_bytes() {
    let layout = test_layout(SMALL_ALLOCATION_BYTES, ReferenceMap::empty());
    let mut test_heap = TestHeap::new();
    let heap = &mut test_heap.heap;

    for _ in 0..SMALL_ALLOCATION_COUNT {
        heap.allocate(
            &heap.allocation_layout(layout.allocation()),
            Payload::Zeroed,
        )
        .expect("heap allocation should succeed");
    }

    let usage = heap.usage().heap;

    assert_eq!(usage.allocation_count, SMALL_ALLOCATION_COUNT);
    assert_eq!(
        usage.allocated_bytes,
        (SMALL_ALLOCATION_COUNT * SMALL_ALLOCATION_BYTES) as u64
    );
    assert_eq!(usage.retained_bytes, DEFAULT_YOUNG_BYTES as u64);
}

/// Track retained bytes for local small-span allocation.
#[test]
fn test_track_small_span_retained_bytes() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        max_heap_young_allocation_bytes: 0,
        ..HeapOptions::local()
    };
    let layout = test_layout(SMALL_ALLOCATION_BYTES, ReferenceMap::empty());
    let class_index = options
        .size_classes
        .class_index_for(SMALL_ALLOCATION_BYTES)
        .expect("small allocation size should have a class");
    let size_class = options.size_classes.classes[class_index];
    let span_bytes = size_class
        .span_bytes(options.page_bytes, options.heap_small_bytes)
        .max(options.heap_small_bytes);
    let slot_count = span_bytes / size_class.bytes;
    let span_count = SMALL_ALLOCATION_COUNT.div_ceil(slot_count);
    let retained_bytes = span_count * span_bytes;
    let mut test_heap = TestHeap::with_options(options);
    let heap = &mut test_heap.heap;

    for _ in 0..SMALL_ALLOCATION_COUNT {
        heap.allocate(
            &heap.allocation_layout(layout.allocation()),
            Payload::Zeroed,
        )
        .expect("heap allocation should succeed");
    }

    let usage = heap.usage().heap;

    assert_eq!(usage.allocation_count, SMALL_ALLOCATION_COUNT);
    assert_eq!(
        usage.allocated_bytes,
        (SMALL_ALLOCATION_COUNT * SMALL_ALLOCATION_BYTES) as u64
    );
    assert_eq!(usage.retained_bytes, retained_bytes as u64);
}

/// Reject one heap allocation when the retained-byte limit would be exceeded.
#[test]
fn test_reject_heap_allocation_when_limit_exceeded() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        max_heap_young_allocation_bytes: 0,
        ..HeapOptions::local()
    };
    let expected_used_bytes = heap_retained_bytes_after_allocate(options.clone(), &[1]);
    let layout = test_layout(1, ReferenceMap::empty());
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

    let error = heap
        .allocate(
            &heap.allocation_layout(layout.allocation()),
            Payload::Bytes(&[1]),
        )
        .expect_err("heap allocation should be rejected");

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

/// Reject one raw allocation when the retained-byte limit would be exceeded.
#[test]
fn test_reject_raw_allocation_when_limit_exceeded() {
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

    let error = heap
        .allocate_raw(1, Payload::Bytes(&[1]))
        .expect_err("raw allocation should be rejected");

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

/// Preserve custom heap limits across in-place heap image restore.
#[test]
fn test_restore_heap_image_preserves_limits() {
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
    heap.restore_image(&image)
        .expect("heap image restore should succeed");

    assert_eq!(heap.limits(), limits);
}

/// Reject one raw replace when byte growth would exceed the retained-byte limit.
#[test]
fn test_reject_raw_replace_when_limit_exceeded() {
    let mut test_heap = TestHeap::with_options(HeapOptions::local());
    let heap = &mut test_heap.heap;
    let pointer = heap
        .allocate_raw(4097, Payload::Bytes(&vec![0xAA; 4097]))
        .expect("raw allocation should succeed");
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
