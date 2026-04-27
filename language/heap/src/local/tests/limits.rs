use crate::{
    AccountingRegion, HeapError, HeapLimits, HeapOptions, HeapSpaceLimits, Payload, RawLimits,
    test_layout,
};
use destack_mir::ReferenceMap;

use super::TestHeap;

/// Return the retained heap bytes for one allocation in the given heap options.
fn heap_retained_bytes_after_allocate(options: HeapOptions, bytes: &[u8]) -> u64 {
    let layout = test_layout(bytes.len(), ReferenceMap::empty());
    let mut test_heap = TestHeap::with_limits_and_options(crate::HeapLimits::default(), options);
    let heap = &mut test_heap.heap;

    heap.allocate(layout.allocation(), Payload::Bytes(bytes))
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
        .allocate(layout.allocation(), Payload::Bytes(&[1]))
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
