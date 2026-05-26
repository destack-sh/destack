use crate::local::space::HeapPlace;
use crate::{
    HeapError, HeapOptions, HeapReference, HeapSpace, Payload, SizeClassTable, test_aligned_layout,
    test_allocator, test_layout,
};
use destack_mir::{TraceMap, TraceVariant};

use super::read_mapped_bytes;

/// Reject one zero-size managed heap allocation.
#[test]
fn test_allocate_heap_rejects_zero_size_layout() {
    let options = HeapOptions::local();
    let layout = test_layout(0, TraceMap::empty());
    let mut heap = HeapSpace::with_options(test_allocator(&options), &options)
        .expect("heap space should build");

    let error = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[]),
        )
        .expect_err("heap allocation should reject zero-size layouts");

    assert_eq!(error, HeapError::ZeroSizeAllocation);
}

/// Reclaim one freed heap allocation and allow another allocation.
#[test]
fn test_free_heap_reclaims_live_allocation() {
    let options = HeapOptions::local();
    let layout = test_layout(1, TraceMap::empty());
    let mut heap = HeapSpace::with_options(test_allocator(&options), &options)
        .expect("heap space should build");
    let reference = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[0xAB]),
        )
        .expect("heap allocation should succeed");

    assert!(heap.is_live(reference));
    heap.free(reference).expect("heap free should succeed");
    assert!(!heap.is_live(reference));

    let next_reference = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[0xCD]),
        )
        .expect("heap allocation should succeed");

    assert!(heap.is_live(next_reference));
}

/// Clear one reused small heap slot before writing a shorter payload.
#[test]
fn test_allocate_heap_clears_reused_small_slot_tail() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        heap_small_bytes: 16,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let full_layout = test_layout(8, TraceMap::empty());
    let short_layout = test_layout(1, TraceMap::empty());
    let mut heap = HeapSpace::with_options(test_allocator(&options), &options)
        .expect("heap space should build");

    let first = heap
        .allocate(
            &heap.allocation_plan(full_layout.allocation()),
            Payload::Bytes(&[0xAA; 8]),
        )
        .expect("heap allocation should succeed");
    let second = heap
        .allocate(
            &heap.allocation_plan(short_layout.allocation()),
            Payload::Bytes(&[0xBB]),
        )
        .expect("heap allocation should succeed");

    heap.free(first).expect("heap free should succeed");

    let reused = heap
        .allocate(
            &heap.allocation_plan(short_layout.allocation()),
            Payload::Bytes(&[0xCC]),
        )
        .expect("heap allocation should succeed");

    assert!(heap.is_live(second));
    assert!(heap.is_live(reused));
    let address = heap.base_address() + reused.offset();

    let bytes = read_mapped_bytes(address, 8);

    assert_eq!(bytes, &[0xCC, 0, 0, 0, 0, 0, 0, 0]);
}

/// Keep young allocations separated by size-class width.
#[test]
fn test_allocate_heap_uses_size_class_stride_for_young_runs() {
    let options = HeapOptions {
        size_classes: SizeClassTable::new([8, 16]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let first_layout = test_layout(9, TraceMap::empty());
    let second_layout = test_layout(10, TraceMap::empty());
    let mut heap = HeapSpace::with_options(test_allocator(&options), &options)
        .expect("heap space should build");

    let first = heap
        .allocate(
            &heap.allocation_plan(first_layout.allocation()),
            Payload::Bytes(&[0xAA; 9]),
        )
        .expect("first heap allocation should succeed");
    let second = heap
        .allocate(
            &heap.allocation_plan(second_layout.allocation()),
            Payload::Bytes(&[0xBB; 10]),
        )
        .expect("second heap allocation should succeed");

    assert_eq!(second.offset() - first.offset(), 16);
    let first_address = heap.base_address() + first.offset();
    let second_address = heap.base_address() + second.offset();

    let first_bytes = read_mapped_bytes(first_address, 9);
    let second_bytes = read_mapped_bytes(second_address, 10);

    assert_eq!(first_bytes, &[0xAA; 9]);
    assert_eq!(second_bytes, &[0xBB; 10]);
}

/// Keep tagged trace maps on allocation records.
#[test]
fn test_allocate_heap_routes_tagged_trace_map_to_large() {
    let options = HeapOptions {
        heap_young_bytes: 64,
        max_heap_young_allocation_bytes: 64,
        heap_small_bytes: 64,
        size_classes: SizeClassTable::new([16]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let trace_map = TraceMap::Tagged {
        tag_offset: 0,
        tag_bytes: 1,
        variants: vec![TraceVariant {
            tag: 0,
            storage_offset: 8,
            map: TraceMap::Fixed {
                local_offsets: vec![0].into_boxed_slice(),
                shared_offsets: Box::default(),
            },
        }]
        .into_boxed_slice(),
    };
    let layout = test_layout(16, trace_map);
    let mut heap = HeapSpace::with_options(test_allocator(&options), &options)
        .expect("heap space should build");

    let reference = heap
        .allocate(&heap.allocation_plan(layout.allocation()), Payload::Zeroed)
        .expect("heap allocation should succeed");

    assert!(matches!(heap.place(reference), Some(HeapPlace::Large(_))));
}

/// Keep over-aligned allocations on aligned mature slots.
#[test]
fn test_allocate_heap_honors_layout_alignment() {
    let options = HeapOptions {
        heap_young_bytes: 64,
        max_heap_young_allocation_bytes: 64,
        heap_small_bytes: 64,
        size_classes: SizeClassTable::new([8, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let layout = test_aligned_layout(17, 16, TraceMap::empty());
    let mut heap = HeapSpace::with_options(test_allocator(&options), &options)
        .expect("heap space should build");

    let first = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[0xAA; 17]),
        )
        .expect("first aligned heap allocation should succeed");
    let second = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[0xBB; 17]),
        )
        .expect("second aligned heap allocation should succeed");

    assert_eq!(first.offset() % 16, 0);
    assert_eq!(second.offset() % 16, 0);
    assert_eq!(second.offset() - first.offset(), 32);
}

/// Reject one write that crosses allocation bounds from an interior reference.
#[test]
fn test_write_heap_rejects_interior_reference_crossing_bounds() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        heap_small_bytes: 8,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let layout = test_layout(1, TraceMap::empty());
    let mut heap = HeapSpace::with_options(test_allocator(&options), &options)
        .expect("heap space should build");
    let reference = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[0xAB]),
        )
        .expect("heap allocation should succeed");
    let reference = reference.add_bytes(7);

    let error = heap
        .write_barrier(reference, 0, 2)
        .expect_err("heap write should reject bounds crossing");

    assert_eq!(
        error,
        HeapError::InvalidByteRange {
            start: 7,
            len: 2,
            capacity: 8
        }
    );
}

/// Reclaim one freed heap large allocation and allow another large allocation.
#[test]
fn test_free_heap_reclaims_large_allocation() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        heap_small_bytes: 32,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let large_byte_len = options
        .size_classes
        .max_small_allocation_bytes()
        .expect("size class table should not be empty")
        + 1;
    let layout = test_layout(large_byte_len, TraceMap::empty());
    let mut heap = HeapSpace::with_options(test_allocator(&options), &options)
        .expect("heap space should build");
    let first = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&vec![0xAB; large_byte_len]),
        )
        .expect("heap large allocation should succeed");

    assert!(matches!(heap.place(first), Some(HeapPlace::Large(_))));

    heap.free(first).expect("heap large free should succeed");

    let second = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&vec![0xCD; large_byte_len]),
        )
        .expect("heap large reallocation should succeed");

    assert!(matches!(heap.place(second), Some(HeapPlace::Large(_))));
}

/// Reject one invalid heap reference loudly.
#[test]
fn test_free_heap_rejects_invalid_reference() {
    let options = HeapOptions::local();
    let mut heap = HeapSpace::with_options(test_allocator(&options), &options)
        .expect("heap space should build");

    let reference = HeapReference::new(7);
    let error = heap.free(reference).expect_err("heap free should fail");

    assert_eq!(error, HeapError::InvalidHeapReference { reference });
}
