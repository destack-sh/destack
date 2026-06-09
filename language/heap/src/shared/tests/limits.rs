use std::sync::Arc;

use crate::{
    AccountingRegion, Allocator, HeapError, Payload, SharedHeap, SharedHeapLimits,
    SharedHeapOptions, SizeClassTable, test_aligned_layout, test_layout,
};
use destack_mir::TraceMap;

use super::{heap_allocation_plan, read_mapped_bytes, test_allocate, trace_table};

const SMALL_ALLOCATION_COUNT: usize = 1024;
const SMALL_ALLOCATION_BYTES: usize = 32;

/// Return the retained shared heap bytes for one block.
fn shared_heap_retained_bytes_after_allocate(bytes: &[u8]) -> u64 {
    // allocate one shared heap payload under default options
    let layout = test_layout(bytes.len(), TraceMap::empty());
    let options = SharedHeapOptions::default();
    let allocator = Arc::new(
        Allocator::try_new(options.page_size_bytes, options.allocator_chunk_size_bytes)
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

    test_allocate(
        &shared,
        &worker,
        &mut allocator,
        layout.block(),
        Payload::Bytes(bytes),
    );

    // report retained bytes after shared allocator rounding
    shared.usage().retained_bytes
}

/// Track retained bytes for shared small-span block.
#[test]
fn test_track_shared_small_span_retained_bytes() {
    // derive the retained-byte charge for the target size class
    let layout = test_layout(SMALL_ALLOCATION_BYTES, TraceMap::empty());
    let options = SharedHeapOptions::default();
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
    let shared = SharedHeap::with_allocator_limits_and_options(
        Arc::new(
            Allocator::try_new(options.page_size_bytes, options.allocator_chunk_size_bytes)
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
        test_allocate(
            &shared,
            &worker,
            &mut allocator,
            layout.block(),
            Payload::Zeroed,
        );
    }
    shared.flush_allocation_cache(&mut allocator);

    let usage = shared.usage();

    // retained bytes should be rounded by shared span geometry
    assert_eq!(usage.allocation_count, SMALL_ALLOCATION_COUNT);
    assert_eq!(
        usage.allocated_bytes,
        (SMALL_ALLOCATION_COUNT * SMALL_ALLOCATION_BYTES) as u64
    );
    assert_eq!(usage.retained_bytes, retained_bytes as u64);
}

/// Publish worker-local shared blocks on allocator flush.
#[test]
fn test_flush_publishes_worker_shared_small_allocations() {
    // allocate two objects through the worker-local shared cache
    let layout = test_layout(SMALL_ALLOCATION_BYTES, TraceMap::empty());
    let shared = SharedHeap::with_allocator_limits_and_options(
        Arc::new(Allocator::try_default().expect("allocator should build")),
        SharedHeapLimits::default(),
        SharedHeapOptions::default(),
    )
    .expect("shared heap should build");
    let mut allocator = shared.allocation_cache();
    let worker = shared.register_collector_worker();

    let first = test_allocate(
        &shared,
        &worker,
        &mut allocator,
        layout.block(),
        Payload::Zeroed,
    );
    let second = test_allocate(
        &shared,
        &worker,
        &mut allocator,
        layout.block(),
        Payload::Zeroed,
    );

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
    assert_eq!(shared.usage().allocation_count, 2);
}

/// Freeing one worker-local shared block clears cache liveness.
#[test]
fn test_free_clears_worker_shared_small_liveness() {
    let layout = test_layout(SMALL_ALLOCATION_BYTES, TraceMap::empty());
    let shared = SharedHeap::with_allocator_limits_and_options(
        Arc::new(Allocator::try_default().expect("allocator should build")),
        SharedHeapLimits::default(),
        SharedHeapOptions::default(),
    )
    .expect("shared heap should build");
    let mut allocator = shared.allocation_cache();
    let worker = shared.register_collector_worker();

    // allocate into the worker-local cursor without publishing it globally
    let reference = test_allocate(
        &shared,
        &worker,
        &mut allocator,
        layout.block(),
        Payload::Zeroed,
    );

    assert!(allocator.contains_heap_reference(reference));
    assert!(!shared.is_heap_live(reference));

    // free must publish only enough cache state to release the block
    shared
        .free(&mut allocator, reference)
        .expect("shared heap free should succeed");

    assert!(!allocator.contains_heap_reference(reference));
    assert!(!shared.is_heap_live(reference));
    assert_eq!(shared.usage().allocation_count, 0);
}

/// Keep over-aligned shared blocks on aligned small slots.
#[test]
fn test_allocate_shared_honors_layout_alignment() {
    // use a size class that can satisfy 16-byte alignment
    let layout = test_aligned_layout(17, 16, TraceMap::empty());
    let options = SharedHeapOptions {
        heap_small_size_bytes: 64,
        size_classes: SizeClassTable::new([8, 24, 32]).expect("size classes should validate"),
        ..SharedHeapOptions::default()
    };
    let shared = SharedHeap::with_allocator_limits_and_options(
        Arc::new(Allocator::try_default().expect("allocator should build")),
        SharedHeapLimits::default(),
        options,
    )
    .expect("shared heap should build");
    let mut allocator = shared.allocation_cache();
    let worker = shared.register_collector_worker();

    let first = test_allocate(
        &shared,
        &worker,
        &mut allocator,
        layout.block(),
        Payload::Zeroed,
    );
    let second = test_allocate(
        &shared,
        &worker,
        &mut allocator,
        layout.block(),
        Payload::Zeroed,
    );

    // every returned base should satisfy the layout alignment
    assert_eq!(first.offset() % 16, 0);
    assert_eq!(second.offset() % 16, 0);
    assert_eq!(second.offset() - first.offset(), 32);
}

/// Reject one shared heap block when the shared heap limit would be exceeded.
#[test]
fn test_reject_shared_heap_allocation_when_limit_exceeded() {
    // compute the projected retained-byte charge for one shared block
    let expected_used_bytes = shared_heap_retained_bytes_after_allocate(&[1]);
    let layout = test_layout(1, TraceMap::empty());
    let shared = SharedHeap::with_allocator_limits_and_options(
        Arc::new(Allocator::try_default().expect("allocator should build")),
        SharedHeapLimits {
            max_bytes: None,
            retained_bytes: Some(0),
        },
        SharedHeapOptions::default(),
    )
    .expect("shared heap should build");
    let mut allocator = shared.allocation_cache();
    let worker = shared.register_collector_worker();

    // reject before mutating shared heap accounting
    let error = shared
        .allocate_payload(
            &worker,
            &mut allocator,
            &heap_allocation_plan(&shared, layout.block()),
            Payload::Bytes(&[1]),
            trace_table(),
        )
        .expect_err("shared heap block should be rejected");

    assert_eq!(
        error,
        HeapError::LimitExceeded {
            region: AccountingRegion::SharedHeap,
            used_bytes: expected_used_bytes,
            max_bytes: 0,
        }
    );
    assert_eq!(shared.usage().retained_bytes, 0);
}

/// Reject one restored shared heap image when the explicit limits are already exceeded.
#[test]
fn test_reject_shared_heap_image_when_limits_start_over_budget() {
    // capture one shared heap image whose baseline retained bytes exceed zero
    let allocator = Arc::new(Allocator::try_default().expect("allocator should build"));
    let layout = test_layout(1, TraceMap::empty());
    let options = SharedHeapOptions::default();
    let shared = SharedHeap::with_allocator_limits_and_options(
        allocator.clone(),
        SharedHeapLimits::default(),
        options.clone(),
    )
    .expect("shared heap should build");
    let mut allocator = shared.allocation_cache();
    let worker = shared.register_collector_worker();
    test_allocate(
        &shared,
        &worker,
        &mut allocator,
        layout.block(),
        Payload::Bytes(&[1]),
    );
    shared.flush_allocation_cache(&mut allocator);

    let used_bytes = shared.usage().retained_bytes;
    let image = shared.image().expect("shared image should capture");

    // reject restore when the explicit limit cannot contain the baseline
    let error = SharedHeap::from_image_with_limits(
        &image,
        SharedHeapLimits {
            max_bytes: None,
            retained_bytes: Some(0),
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
