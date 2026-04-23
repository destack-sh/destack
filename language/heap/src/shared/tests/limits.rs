use std::sync::Arc;

use crate::{
    AccountingRegion, Allocator, HeapError, SharedHeap, SharedHeapLimits, SharedHeapSpaceLimits,
    SharedRawLimits,
};
use destack_mir::LayoutTrace;

/// Return the active shared heap bytes for one allocation.
fn shared_heap_active_bytes_after_allocate(bytes: &[u8]) -> u64 {
    let shared = SharedHeap::new();

    shared
        .allocate_heap_bytes(bytes, LayoutTrace::empty(), None)
        .expect("shared heap allocation should succeed");

    shared.usage().heap.active_bytes
}

/// Reject one shared heap allocation when the shared heap limit would be exceeded.
#[test]
fn test_reject_shared_heap_allocation_when_limit_exceeded() {
    let expected_used_bytes = shared_heap_active_bytes_after_allocate(&[1]);
    let shared = SharedHeap::with_allocator_and_limits(
        Arc::new(Allocator::new()),
        SharedHeapLimits {
            max_bytes: None,
            heap: SharedHeapSpaceLimits { max_bytes: Some(0) },
            raw: SharedRawLimits { max_bytes: None },
        },
    );

    let error = shared
        .allocate_heap_bytes(&[1], LayoutTrace::empty(), None)
        .expect_err("shared heap allocation should be rejected");

    assert_eq!(
        error,
        HeapError::LimitExceeded {
            region: AccountingRegion::SharedHeap,
            used_bytes: expected_used_bytes,
            max_bytes: 0,
        }
    );
    assert_eq!(shared.usage().heap.active_bytes, 0);
}

/// Reject one shared raw replacement when the shared raw limit would be exceeded.
#[test]
fn test_reject_shared_raw_replace_when_limit_exceeded() {
    let allocator = Arc::new(Allocator::new());
    let shared = SharedHeap::with_allocator(allocator.clone());
    let pointer = shared
        .allocate_raw_bytes(&vec![0xAA; 4097])
        .expect("shared raw allocation should succeed");
    let baseline = shared.usage().raw.active_bytes;
    let image = shared.image();
    let shared = SharedHeap::from_image_with_allocator_and_limits(
        allocator,
        &image,
        SharedHeapLimits {
            max_bytes: None,
            heap: SharedHeapSpaceLimits { max_bytes: None },
            raw: SharedRawLimits {
                max_bytes: Some(baseline),
            },
        },
    )
    .expect("shared image restore should fit the baseline raw limit");
    let mapped_delta = shared
        .raw
        .replace_mapped_delta(pointer, 8193)
        .expect("shared raw replacement should project");
    let expected_used_bytes = baseline
        .checked_add(mapped_delta as u64)
        .expect("projected shared raw usage should fit");

    let error = shared
        .replace_raw_bytes(pointer, &vec![0xBB; 8193])
        .expect_err("shared raw replacement should be rejected");

    assert_eq!(
        error,
        HeapError::LimitExceeded {
            region: AccountingRegion::SharedRaw,
            used_bytes: expected_used_bytes,
            max_bytes: baseline,
        }
    );
    assert_eq!(shared.read_raw_bytes(pointer), Ok(vec![0xAA; 4097]));
}

/// Reject one restored shared heap image when the explicit limits are already exceeded.
#[test]
fn test_reject_shared_heap_image_when_limits_start_over_budget() {
    let allocator = Arc::new(Allocator::new());
    let shared = SharedHeap::with_allocator(allocator.clone());
    shared
        .allocate_heap_bytes(&[1], LayoutTrace::empty(), None)
        .expect("shared heap allocation should succeed");
    let used_bytes = shared.usage().heap.active_bytes;
    let image = shared.image();

    let error = SharedHeap::from_image_with_allocator_and_limits(
        allocator,
        &image,
        SharedHeapLimits {
            max_bytes: None,
            heap: SharedHeapSpaceLimits { max_bytes: Some(0) },
            raw: SharedRawLimits { max_bytes: None },
        },
    )
    .expect_err("shared image restore should reject an over-budget baseline");

    assert_eq!(
        error,
        HeapError::LimitExceeded {
            region: AccountingRegion::SharedHeap,
            used_bytes,
            max_bytes: 0,
        }
    );
}
