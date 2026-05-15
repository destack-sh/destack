use std::sync::Arc;

use crate::{
    AccountingRegion, Allocator, HeapError, Payload, RawAllocationShape, SharedHeap,
    SharedHeapLimits, SharedHeapSpaceLimits, SharedRawLimits, SizeClassTable, test_aligned_layout,
    test_layout,
};
use destack_mir::ReferenceMap;

use super::read_mapped_bytes;

const SMALL_ALLOCATION_COUNT: usize = 1024;
const SMALL_ALLOCATION_BYTES: usize = 32;

/// Return the retained shared heap bytes for one allocation.
fn shared_heap_retained_bytes_after_allocate(bytes: &[u8]) -> u64 {
    let layout = test_layout(bytes.len(), ReferenceMap::empty());
    let options = crate::HeapOptions::shared();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );
    let shared = SharedHeap::with_allocator_limits_and_options(
        allocator,
        SharedHeapLimits::default(),
        options,
    )
    .expect("shared heap should build");
    let mut allocator = shared.allocator();
    let worker = shared.register_collector_worker();

    shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_layout(layout.allocation()),
            Payload::Bytes(bytes),
        )
        .expect("shared heap allocation should succeed");

    shared.usage().heap.retained_bytes
}

/// Track retained bytes for shared small-span allocation.
#[test]
fn test_track_shared_small_span_retained_bytes() {
    let layout = test_layout(SMALL_ALLOCATION_BYTES, ReferenceMap::empty());
    let options = crate::HeapOptions::shared();
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
    let shared = SharedHeap::with_allocator_limits_and_options(
        Arc::new(
            Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
                .expect("allocator should build"),
        ),
        SharedHeapLimits::default(),
        options,
    )
    .expect("shared heap should build");
    let mut allocator = shared.allocator();
    let worker = shared.register_collector_worker();

    for _ in 0..SMALL_ALLOCATION_COUNT {
        shared
            .allocate(
                &worker,
                &mut allocator,
                &shared.allocation_layout(layout.allocation()),
                Payload::Zeroed,
            )
            .expect("shared heap allocation should succeed");
    }
    shared.flush_allocator(&mut allocator);

    let usage = shared.usage().heap;

    assert_eq!(usage.allocation_count, SMALL_ALLOCATION_COUNT);
    assert_eq!(
        usage.allocated_bytes,
        (SMALL_ALLOCATION_COUNT * SMALL_ALLOCATION_BYTES) as u64
    );
    assert_eq!(usage.retained_bytes, retained_bytes as u64);
}

/// Publish worker-local shared allocations on allocator flush.
#[test]
fn test_flush_publishes_worker_shared_small_allocations() {
    let layout = test_layout(SMALL_ALLOCATION_BYTES, ReferenceMap::empty());
    let shared = SharedHeap::with_allocator_limits_and_options(
        Arc::new(Allocator::try_default().expect("allocator should build")),
        SharedHeapLimits::default(),
        crate::HeapOptions::shared(),
    )
    .expect("shared heap should build");
    let mut allocator = shared.allocator();
    let worker = shared.register_collector_worker();

    let first = shared
        .allocate_zeroed(
            &worker,
            &mut allocator,
            &shared.allocation_layout(layout.allocation()),
        )
        .expect("shared heap allocation should succeed");
    let second = shared
        .allocate_zeroed(
            &worker,
            &mut allocator,
            &shared.allocation_layout(layout.allocation()),
        )
        .expect("shared heap allocation should succeed");

    shared.flush_allocator(&mut allocator);

    assert!(shared.is_heap_live(first));
    assert!(shared.is_heap_live(second));
    let first_address = shared.heap_base_address() + first.offset();
    let second_address = shared.heap_base_address() + second.offset();

    // inspect the published small payloads directly
    let first_bytes = read_mapped_bytes(first_address, SMALL_ALLOCATION_BYTES);
    let second_bytes = read_mapped_bytes(second_address, SMALL_ALLOCATION_BYTES);

    assert_eq!(first_bytes, &[0; SMALL_ALLOCATION_BYTES]);
    assert_eq!(second_bytes, &[0; SMALL_ALLOCATION_BYTES]);
    assert_eq!(shared.usage().heap.allocation_count, 2);
}

/// Keep over-aligned shared allocations on aligned small slots.
#[test]
fn test_allocate_shared_honors_layout_alignment() {
    let layout = test_aligned_layout(17, 16, ReferenceMap::empty());
    let options = crate::HeapOptions {
        heap_small_bytes: 64,
        size_classes: SizeClassTable::new([8, 24, 32]).expect("size classes should validate"),
        ..crate::HeapOptions::shared()
    };
    let shared = SharedHeap::with_allocator_limits_and_options(
        Arc::new(Allocator::try_default().expect("allocator should build")),
        SharedHeapLimits::default(),
        options,
    )
    .expect("shared heap should build");
    let mut allocator = shared.allocator();
    let worker = shared.register_collector_worker();

    let first = shared
        .allocate_zeroed(
            &worker,
            &mut allocator,
            &shared.allocation_layout(layout.allocation()),
        )
        .expect("first shared heap allocation should succeed");
    let second = shared
        .allocate_zeroed(
            &worker,
            &mut allocator,
            &shared.allocation_layout(layout.allocation()),
        )
        .expect("second shared heap allocation should succeed");

    assert_eq!(first.offset() % 16, 0);
    assert_eq!(second.offset() % 16, 0);
    assert_eq!(second.offset() - first.offset(), 32);
}

/// Reject one shared heap allocation when the shared heap limit would be exceeded.
#[test]
fn test_reject_shared_heap_allocation_when_limit_exceeded() {
    let expected_used_bytes = shared_heap_retained_bytes_after_allocate(&[1]);
    let layout = test_layout(1, ReferenceMap::empty());
    let shared = SharedHeap::with_allocator_limits_and_options(
        Arc::new(Allocator::try_default().expect("allocator should build")),
        SharedHeapLimits {
            max_bytes: None,
            heap: SharedHeapSpaceLimits { max_bytes: Some(0) },
            raw: SharedRawLimits { max_bytes: None },
        },
        crate::HeapOptions::shared(),
    )
    .expect("shared heap should build");
    let mut allocator = shared.allocator();
    let worker = shared.register_collector_worker();

    let error = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_layout(layout.allocation()),
            Payload::Bytes(&[1]),
        )
        .expect_err("shared heap allocation should be rejected");

    assert_eq!(
        error,
        HeapError::LimitExceeded {
            region: AccountingRegion::SharedHeap,
            used_bytes: expected_used_bytes,
            max_bytes: 0,
        }
    );
    assert_eq!(shared.usage().heap.retained_bytes, 0);
}

/// Reject one shared raw replacement when the shared raw limit would be exceeded.
#[test]
fn test_reject_shared_raw_replace_when_limit_exceeded() {
    let allocator = Arc::new(Allocator::try_default().expect("allocator should build"));
    let options = crate::HeapOptions::shared();
    let shared = SharedHeap::with_allocator_limits_and_options(
        allocator.clone(),
        SharedHeapLimits::default(),
        options.clone(),
    )
    .expect("shared heap should build");
    let pointer = shared
        .allocate_raw(
            RawAllocationShape::bytes(4097),
            Payload::Bytes(&vec![0xAA; 4097]),
        )
        .expect("shared raw allocation should succeed");
    let baseline = shared.usage().raw.retained_bytes;
    let image = shared.image().expect("shared image should capture");
    let shared = SharedHeap::from_image_with_limits(
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
    let retained_byte_delta = shared
        .raw
        .replace_retained_byte_delta(pointer, 8193)
        .expect("shared raw replacement should project");
    let expected_used_bytes = baseline + retained_byte_delta as u64;

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
    let layout = test_layout(1, ReferenceMap::empty());
    let options = crate::HeapOptions::shared();
    let shared = SharedHeap::with_allocator_limits_and_options(
        allocator.clone(),
        SharedHeapLimits::default(),
        options.clone(),
    )
    .expect("shared heap should build");
    let mut allocator = shared.allocator();
    let worker = shared.register_collector_worker();
    shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_layout(layout.allocation()),
            Payload::Bytes(&[1]),
        )
        .expect("shared heap allocation should succeed");
    let used_bytes = shared.usage().heap.retained_bytes;
    let image = shared.image().expect("shared image should capture");

    let error = SharedHeap::from_image_with_limits(
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
