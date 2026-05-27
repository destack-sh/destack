use std::sync::Arc;

use crate::{
    AccountingRegion, Allocator, HeapError, Payload, RawAllocationShape, SharedHeap,
    SharedHeapLimits, SharedHeapSpaceLimits, SharedRawLimits, SizeClassTable, test_aligned_layout,
    test_layout,
};
use destack_mir::TraceMap;

use super::{read_mapped_bytes, trace_table};

const SMALL_ALLOCATION_COUNT: usize = 1024;
const SMALL_ALLOCATION_BYTES: usize = 32;

/// Return the retained shared heap bytes for one allocation.
fn shared_heap_retained_bytes_after_allocate(bytes: &[u8]) -> u64 {
    // allocate one shared managed payload under default options
    let layout = test_layout(bytes.len(), TraceMap::empty());
    let options = crate::SharedHeapOptions::default();
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
    let mut allocator = shared.allocation_cache();
    let worker = shared.register_collector_worker();

    shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            Payload::Bytes(bytes),
            trace_table(),
        )
        .expect("shared heap allocation should succeed");

    // report retained bytes after shared allocator rounding
    shared.usage().heap.retained_bytes
}

/// Track retained bytes for shared small-span allocation.
#[test]
fn test_track_shared_small_span_retained_bytes() {
    // derive the retained-byte charge for the target size class
    let layout = test_layout(SMALL_ALLOCATION_BYTES, TraceMap::empty());
    let options = crate::SharedHeapOptions::default();
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
    let mut allocator = shared.allocation_cache();
    let worker = shared.register_collector_worker();

    // allocate enough objects to cover several shared span slots
    for _ in 0..SMALL_ALLOCATION_COUNT {
        shared
            .allocate(
                &worker,
                &mut allocator,
                &shared.allocation_plan(layout.allocation()),
                Payload::Zeroed,
                trace_table(),
            )
            .expect("shared heap allocation should succeed");
    }
    shared.flush_allocation_cache(&mut allocator);

    let usage = shared.usage().heap;

    // retained bytes should be rounded by shared span geometry
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
    // allocate two objects through the worker-local shared cache
    let layout = test_layout(SMALL_ALLOCATION_BYTES, TraceMap::empty());
    let shared = SharedHeap::with_allocator_limits_and_options(
        Arc::new(Allocator::try_default().expect("allocator should build")),
        SharedHeapLimits::default(),
        crate::SharedHeapOptions::default(),
    )
    .expect("shared heap should build");
    let mut allocator = shared.allocation_cache();
    let worker = shared.register_collector_worker();

    let first = shared
        .allocate_zeroed(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            trace_table(),
        )
        .expect("shared heap allocation should succeed");
    let second = shared
        .allocate_zeroed(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            trace_table(),
        )
        .expect("shared heap allocation should succeed");

    // flushing should publish worker-local slots into shared heap state
    shared.flush_allocation_cache(&mut allocator);

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
    // use a size class that can satisfy 16-byte alignment
    let layout = test_aligned_layout(17, 16, TraceMap::empty());
    let options = crate::SharedHeapOptions {
        heap_small_bytes: 64,
        size_classes: SizeClassTable::new([8, 24, 32]).expect("size classes should validate"),
        ..crate::SharedHeapOptions::default()
    };
    let shared = SharedHeap::with_allocator_limits_and_options(
        Arc::new(Allocator::try_default().expect("allocator should build")),
        SharedHeapLimits::default(),
        options,
    )
    .expect("shared heap should build");
    let mut allocator = shared.allocation_cache();
    let worker = shared.register_collector_worker();

    let first = shared
        .allocate_zeroed(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            trace_table(),
        )
        .expect("first shared heap allocation should succeed");
    let second = shared
        .allocate_zeroed(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            trace_table(),
        )
        .expect("second shared heap allocation should succeed");

    // every returned base should satisfy the layout alignment
    assert_eq!(first.offset() % 16, 0);
    assert_eq!(second.offset() % 16, 0);
    assert_eq!(second.offset() - first.offset(), 32);
}

/// Reject one shared heap allocation when the shared heap limit would be exceeded.
#[test]
fn test_reject_shared_heap_allocation_when_limit_exceeded() {
    // compute the projected retained-byte charge for one shared allocation
    let expected_used_bytes = shared_heap_retained_bytes_after_allocate(&[1]);
    let layout = test_layout(1, TraceMap::empty());
    let shared = SharedHeap::with_allocator_limits_and_options(
        Arc::new(Allocator::try_default().expect("allocator should build")),
        SharedHeapLimits {
            max_bytes: None,
            heap: SharedHeapSpaceLimits { max_bytes: Some(0) },
            raw: SharedRawLimits { max_bytes: None },
        },
        crate::SharedHeapOptions::default(),
    )
    .expect("shared heap should build");
    let mut allocator = shared.allocation_cache();
    let worker = shared.register_collector_worker();

    // reject before mutating shared heap accounting
    let error = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1]),
            trace_table(),
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
    // allocate one raw payload and cap retained bytes at the baseline
    let allocator = Arc::new(Allocator::try_default().expect("allocator should build"));
    let options = crate::SharedHeapOptions::default();
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

    // reject the replacement before mutating bytes or accounting
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
    // capture one shared heap image whose baseline retained bytes exceed zero
    let allocator = Arc::new(Allocator::try_default().expect("allocator should build"));
    let layout = test_layout(1, TraceMap::empty());
    let options = crate::SharedHeapOptions::default();
    let shared = SharedHeap::with_allocator_limits_and_options(
        allocator.clone(),
        SharedHeapLimits::default(),
        options.clone(),
    )
    .expect("shared heap should build");
    let mut allocator = shared.allocation_cache();
    let worker = shared.register_collector_worker();
    shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1]),
            trace_table(),
        )
        .expect("shared heap allocation should succeed");
    let used_bytes = shared.usage().heap.retained_bytes;
    let image = shared.image().expect("shared image should capture");

    // reject restore when the explicit limit cannot contain the baseline
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
