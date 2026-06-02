use crate::local::storage::{HeapPlace, HeapStorage};
use crate::{
    HeapAllocationError, HeapError, HeapOptions, HeapReference, Payload, SizeClassTable,
    test_aligned_layout, test_allocator, test_layout,
};
use destack_mir::{TraceMap, TraceVariant};

use super::{read_mapped_bytes, trace_table};

/// Reject one zero-size heap block.
#[test]
fn test_allocate_heap_rejects_zero_size_layout() {
    // build a valid heap with an invalid block layout
    let options = HeapOptions::local();
    let layout = test_layout(0, TraceMap::empty());
    let mut heap = HeapStorage::build_with_options(test_allocator(&options), &options)
        .expect("heap storage should build");

    // reject zero-size heap objects loudly
    let error = heap
        .allocate(&heap.allocation_plan(layout.block()), Payload::Bytes(&[]))
        .expect_err("heap block should reject zero-size layouts");

    assert_eq!(
        error,
        HeapError::invalid_allocation(HeapAllocationError::ZeroSize)
    );
}

/// Reclaim one freed heap block and allow another block.
#[test]
fn test_free_heap_reclaims_live_allocation() {
    // allocate one live heap payload
    let options = HeapOptions::local();
    let layout = test_layout(1, TraceMap::empty());
    let mut heap = HeapStorage::build_with_options(test_allocator(&options), &options)
        .expect("heap storage should build");
    let reference = heap
        .allocate(
            &heap.allocation_plan(layout.block()),
            Payload::Bytes(&[0xAB]),
        )
        .expect("heap block should succeed");

    // free the block and retire its old reference
    assert!(heap.is_live(reference));
    heap.free(reference).expect("heap free should succeed");
    assert!(!heap.is_live(reference));

    // allocate again to prove the heap remains usable
    let next_reference = heap
        .allocate(
            &heap.allocation_plan(layout.block()),
            Payload::Bytes(&[0xCD]),
        )
        .expect("heap block should succeed");

    assert!(heap.is_live(next_reference));
}

/// Clear one reused small heap slot before writing a shorter payload.
#[test]
fn test_allocate_heap_clears_reused_small_slot_tail() {
    // force small mature block reuse
    let options = HeapOptions {
        heap_young_size_bytes: 0,
        heap_small_size_bytes: 16,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let full_layout = test_layout(8, TraceMap::empty());
    let short_layout = test_layout(1, TraceMap::empty());
    let mut heap = HeapStorage::build_with_options(test_allocator(&options), &options)
        .expect("heap storage should build");

    let first = heap
        .allocate(
            &heap.allocation_plan(full_layout.block()),
            Payload::Bytes(&[0xAA; 8]),
        )
        .expect("heap block should succeed");
    let second = heap
        .allocate(
            &heap.allocation_plan(short_layout.block()),
            Payload::Bytes(&[0xBB]),
        )
        .expect("heap block should succeed");

    // free the full slot so a shorter payload can reuse it
    heap.free(first).expect("heap free should succeed");

    let reused = heap
        .allocate(
            &heap.allocation_plan(short_layout.block()),
            Payload::Bytes(&[0xCC]),
        )
        .expect("heap block should succeed");

    assert!(heap.is_live(second));
    assert!(heap.is_live(reused));
    let address = heap.base_address() + reused.offset();

    // reused slot tail bytes should be cleared
    let bytes = read_mapped_bytes(address, 8);

    assert_eq!(bytes, &[0xCC, 0, 0, 0, 0, 0, 0, 0]);
}

/// Keep young span metadata at exact payload lengths.
#[test]
fn test_allocate_heap_tracks_exact_young_span_payload_lengths() {
    // configure both payloads into the same 16-byte young span class
    let options = HeapOptions {
        size_classes: SizeClassTable::new([8, 16]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let first_layout = test_layout(9, TraceMap::empty());
    let second_layout = test_layout(10, TraceMap::empty());
    let mut heap = HeapStorage::build_with_options(test_allocator(&options), &options)
        .expect("heap storage should build");

    let first = heap
        .allocate(
            &heap.allocation_plan(first_layout.block()),
            Payload::Bytes(&[0xAA; 9]),
        )
        .expect("first heap block should succeed");
    let second = heap
        .allocate(
            &heap.allocation_plan(second_layout.block()),
            Payload::Bytes(&[0xBB; 10]),
        )
        .expect("second heap block should succeed");

    // resolve each block through its own live metadata
    let first_place = heap.place(first).expect("first block should be live");
    let second_place = heap.place(second).expect("second block should be live");
    let first_byte_len = heap
        .byte_len_for_place(first_place)
        .expect("first block byte length should resolve");
    let second_byte_len = heap
        .byte_len_for_place(second_place)
        .expect("second block byte length should resolve");
    let first_address = heap.base_address() + first.offset();
    let second_address = heap.base_address() + second.offset();

    // span metadata should keep each exact payload length
    assert_eq!(first_byte_len, 9);
    assert_eq!(second_byte_len, 10);

    // payload bytes should not overlap across adjacent slots
    let first_bytes = read_mapped_bytes(first_address, 9);
    let second_bytes = read_mapped_bytes(second_address, 10);

    assert_eq!(first_bytes, &[0xAA; 9]);
    assert_eq!(second_bytes, &[0xBB; 10]);
}

/// Keep tagged trace maps on block records.
#[test]
fn test_allocate_heap_routes_tagged_trace_map_to_large() {
    // tagged trace maps require block records, not young span metadata
    let options = HeapOptions {
        heap_young_size_bytes: 64,
        max_heap_young_allocation_size_bytes: 64,
        heap_small_size_bytes: 64,
        size_classes: SizeClassTable::new([16]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let trace_map = TraceMap::Tagged {
        tag_bytes: 1,
        variants: vec![TraceVariant {
            tag: 0,
            payload_offset: 8,
            map: TraceMap::Fixed {
                local_offsets: vec![0].into_boxed_slice(),
                shared_offsets: Box::default(),
            },
        }]
        .into_boxed_slice(),
    };
    let layout = test_layout(16, trace_map);
    let mut heap = HeapStorage::build_with_options(test_allocator(&options), &options)
        .expect("heap storage should build");

    let reference = heap
        .allocate(&heap.allocation_plan(layout.block()), Payload::Zeroed)
        .expect("heap block should succeed");

    // tagged payloads should bypass young space
    assert!(matches!(
        heap.place(reference),
        Some(HeapPlace::LargeBlock(_))
    ));
}

/// Keep over-aligned blocks on aligned mature slots.
#[test]
fn test_allocate_heap_honors_layout_alignment() {
    // use a size class that can satisfy 16-byte alignment
    let options = HeapOptions {
        heap_young_size_bytes: 64,
        max_heap_young_allocation_size_bytes: 64,
        heap_small_size_bytes: 64,
        size_classes: SizeClassTable::new([8, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let layout = test_aligned_layout(17, 16, TraceMap::empty());
    let mut heap = HeapStorage::build_with_options(test_allocator(&options), &options)
        .expect("heap storage should build");

    let first = heap
        .allocate(
            &heap.allocation_plan(layout.block()),
            Payload::Bytes(&[0xAA; 17]),
        )
        .expect("first aligned heap block should succeed");
    let second = heap
        .allocate(
            &heap.allocation_plan(layout.block()),
            Payload::Bytes(&[0xBB; 17]),
        )
        .expect("second aligned heap block should succeed");

    // every returned base should satisfy the layout alignment
    assert_eq!(first.offset() % 16, 0);
    assert_eq!(second.offset() % 16, 0);
    assert_eq!(second.offset() - first.offset(), 32);
}

/// Reject one write that crosses block bounds from an interior reference.
#[test]
fn test_write_heap_rejects_interior_reference_crossing_bounds() {
    // allocate one small mature payload and form an interior reference near the end
    let options = HeapOptions {
        heap_young_size_bytes: 0,
        heap_small_size_bytes: 8,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let layout = test_layout(1, TraceMap::empty());
    let mut heap = HeapStorage::build_with_options(test_allocator(&options), &options)
        .expect("heap storage should build");
    let reference = heap
        .allocate(
            &heap.allocation_plan(layout.block()),
            Payload::Bytes(&[0xAB]),
        )
        .expect("heap block should succeed");
    let reference = reference.add_bytes(7);

    // writing two bytes from offset 7 crosses the block boundary
    let error = heap
        .write_barrier(reference, 0, 2, trace_table())
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

/// Reclaim one freed heap large block and allow another large block.
#[test]
fn test_free_heap_reclaims_large_block() {
    // force blocks larger than the local small span classes
    let options = HeapOptions {
        heap_young_size_bytes: 0,
        heap_small_size_bytes: 32,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let large_byte_len = options
        .size_classes
        .max_small_allocation_bytes()
        .expect("size class table should not be empty")
        + 1;
    let layout = test_layout(large_byte_len, TraceMap::empty());
    let mut heap = HeapStorage::build_with_options(test_allocator(&options), &options)
        .expect("heap storage should build");
    let first = heap
        .allocate(
            &heap.allocation_plan(layout.block()),
            Payload::Bytes(&vec![0xAB; large_byte_len]),
        )
        .expect("heap large block should succeed");

    assert!(matches!(heap.place(first), Some(HeapPlace::LargeBlock(_))));

    // free the large block before allocating another one
    heap.free(first).expect("heap large free should succeed");

    let second = heap
        .allocate(
            &heap.allocation_plan(layout.block()),
            Payload::Bytes(&vec![0xCD; large_byte_len]),
        )
        .expect("heap large reallocation should succeed");

    // large block placement should stay on the large path
    assert!(matches!(heap.place(second), Some(HeapPlace::LargeBlock(_))));
}

/// Reject one invalid heap reference loudly.
#[test]
fn test_free_heap_rejects_invalid_reference() {
    // build a heap with no block at offset 7
    let options = HeapOptions::local();
    let mut heap = HeapStorage::build_with_options(test_allocator(&options), &options)
        .expect("heap storage should build");

    // reject the unknown address
    let reference = HeapReference::new(7);
    let error = heap.free(reference).expect_err("heap free should fail");

    assert_eq!(error, HeapError::invalid_heap_reference(reference));
}
