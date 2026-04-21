use std::sync::Arc;

use crate::{
    Arena, HeapError, HeapSpace, SharedHeap, SharedHeapLimits, SharedManagedLimits, SharedRawLimits,
};
use destack_mir::LayoutTrace;

/// Reject one shared managed allocation when the shared managed limit would be exceeded.
#[test]
fn test_reject_shared_managed_allocation_when_limit_exceeded() {
    let shared = SharedHeap::with_arena_and_limits(
        Arc::new(Arena::new()),
        SharedHeapLimits {
            max_bytes: None,
            managed: SharedManagedLimits { max_bytes: Some(0) },
            raw: SharedRawLimits { max_bytes: None },
        },
    );

    let error = shared
        .allocate_managed_bytes(&[1], LayoutTrace::empty(), None)
        .expect_err("shared managed allocation should be rejected");

    assert!(matches!(
        error,
        HeapError::LimitExceeded {
            space: HeapSpace::SharedManaged,
            ..
        }
    ));
    assert_eq!(
        shared
            .usage()
            .expect("shared usage should resolve")
            .managed
            .active_bytes,
        0
    );
}

/// Reject one shared raw replacement when the shared raw limit would be exceeded.
#[test]
fn test_reject_shared_raw_replace_when_limit_exceeded() {
    let arena = Arc::new(Arena::new());
    let shared = SharedHeap::with_arena(arena.clone());
    let pointer = shared
        .allocate_raw_bytes(&vec![0xAA; 4097])
        .expect("shared raw allocation should succeed");
    let baseline = shared
        .usage()
        .expect("shared usage should resolve")
        .raw
        .active_bytes;
    let image = shared.image();
    let shared = SharedHeap::from_image_with_arena_and_limits(
        arena,
        &image,
        SharedHeapLimits {
            max_bytes: None,
            managed: SharedManagedLimits { max_bytes: None },
            raw: SharedRawLimits {
                max_bytes: Some(baseline),
            },
        },
    )
    .expect("shared image restore should fit the baseline raw limit");

    let error = shared
        .replace_raw_bytes(pointer, &vec![0xBB; 8193])
        .expect_err("shared raw replacement should be rejected");

    assert!(matches!(
        error,
        HeapError::LimitExceeded {
            space: HeapSpace::SharedRaw,
            ..
        }
    ));
    assert_eq!(shared.read_raw_bytes(pointer), Ok(vec![0xAA; 4097]));
}

/// Reject one restored shared heap image when the explicit limits are already exceeded.
#[test]
fn test_reject_shared_heap_image_when_limits_start_over_budget() {
    let arena = Arc::new(Arena::new());
    let shared = SharedHeap::with_arena(arena.clone());
    shared
        .allocate_managed_bytes(&[1], LayoutTrace::empty(), None)
        .expect("shared managed allocation should succeed");
    let image = shared.image();

    let error = SharedHeap::from_image_with_arena_and_limits(
        arena,
        &image,
        SharedHeapLimits {
            max_bytes: None,
            managed: SharedManagedLimits { max_bytes: Some(0) },
            raw: SharedRawLimits { max_bytes: None },
        },
    )
    .expect_err("shared image restore should reject an over-budget baseline");

    assert!(matches!(
        error,
        HeapError::LimitExceeded {
            space: HeapSpace::SharedManaged,
            ..
        }
    ));
}
