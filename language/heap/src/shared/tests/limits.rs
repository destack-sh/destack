use std::sync::Arc;

use crate::{
    AccountingRegion, Allocator, HeapError, SharedHeap, SharedHeapLimits, SharedHeapSpaceLimits,
    SharedRawLimits, test_layout,
};
use destack_mir::ReferenceMap;

/// Return the active shared heap bytes for one allocation.
fn shared_heap_active_bytes_after_allocate(bytes: &[u8]) -> u64 {
    let (layouts, layout_id) = test_layout(bytes.len(), ReferenceMap::empty());
    let options = crate::HeapOptions::shared();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_arena_bytes)
            .expect("allocator should build"),
    );
    let shared = SharedHeap::with_allocator_limits_layouts_and_options(
        allocator,
        layouts,
        SharedHeapLimits::default(),
        options,
    )
    .expect("shared heap should build");

    shared
        .allocate_heap_bytes(bytes, layout_id)
        .expect("shared heap allocation should succeed");

    shared.usage().heap.active_bytes
}

/// Reject one shared heap allocation when the shared heap limit would be exceeded.
#[test]
fn test_reject_shared_heap_allocation_when_limit_exceeded() {
    let expected_used_bytes = shared_heap_active_bytes_after_allocate(&[1]);
    let (layouts, layout_id) = test_layout(1, ReferenceMap::empty());
    let shared = SharedHeap::with_allocator_limits_layouts_and_options(
        Arc::new(Allocator::try_default().expect("allocator should build")),
        layouts,
        SharedHeapLimits {
            max_bytes: None,
            heap: SharedHeapSpaceLimits { max_bytes: Some(0) },
            raw: SharedRawLimits { max_bytes: None },
        },
        crate::HeapOptions::shared(),
    )
    .expect("shared heap should build");

    let error = shared
        .allocate_heap_bytes(&[1], layout_id)
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
    let allocator = Arc::new(Allocator::try_default().expect("allocator should build"));
    let options = crate::HeapOptions::shared();
    let shared = SharedHeap::with_allocator_limits_layouts_and_options(
        allocator.clone(),
        Arc::new(destack_mir::LayoutTable::new()),
        SharedHeapLimits::default(),
        options.clone(),
    )
    .expect("shared heap should build");
    let pointer = shared
        .allocate_raw_bytes(&vec![0xAA; 4097])
        .expect("shared raw allocation should succeed");
    let baseline = shared.usage().raw.active_bytes;
    let image = shared.image();
    let shared = SharedHeap::from_image_with_allocator_limits_and_options(
        allocator,
        &image,
        SharedHeapLimits {
            max_bytes: None,
            heap: SharedHeapSpaceLimits { max_bytes: None },
            raw: SharedRawLimits {
                max_bytes: Some(baseline),
            },
        },
        options,
    )
    .expect("shared image restore should fit the baseline raw limit");
    let mapped_byte_delta = shared
        .raw
        .replace_mapped_byte_delta(pointer, 8193)
        .expect("shared raw replacement should project");
    let expected_used_bytes = baseline
        .checked_add(mapped_byte_delta as u64)
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
    let allocator = Arc::new(Allocator::try_default().expect("allocator should build"));
    let (layouts, layout_id) = test_layout(1, ReferenceMap::empty());
    let options = crate::HeapOptions::shared();
    let shared = SharedHeap::with_allocator_limits_layouts_and_options(
        allocator.clone(),
        layouts,
        SharedHeapLimits::default(),
        options.clone(),
    )
    .expect("shared heap should build");
    shared
        .allocate_heap_bytes(&[1], layout_id)
        .expect("shared heap allocation should succeed");
    let used_bytes = shared.usage().heap.active_bytes;
    let image = shared.image();

    let error = SharedHeap::from_image_with_allocator_limits_and_options(
        allocator,
        &image,
        SharedHeapLimits {
            max_bytes: None,
            heap: SharedHeapSpaceLimits { max_bytes: Some(0) },
            raw: SharedRawLimits { max_bytes: None },
        },
        options,
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
