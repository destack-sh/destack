use std::sync::Arc;

use crate::{
    Payload, RawAllocationShape, SharedHeapOptions, SharedHeapSpace, SharedRawSpace,
    SizeClassTable, test_layouts, test_shared_allocator,
};
use destack_mir::TraceMap;

use super::{read_mapped_bytes, trace_table, write_mapped_bytes};

/// The allocator chunk size for small-page shared image fixtures.
const TEST_ALLOCATOR_CHUNK_SIZE_BYTES: usize = 1024 * 1024;

/// Return the logical bytes for one shared raw allocation image.
fn raw_allocation_bytes(allocation: &crate::shared::raw::SharedRawAllocationImage) -> Vec<u8> {
    allocation.bytes[..allocation.byte_len].to_vec()
}

/// Preserve shared allocation bytes across image and fork boundaries.
#[test]
fn test_roundtrip_shared_memory_image_and_fork() {
    let options = SharedHeapOptions {
        page_size_bytes: 4,
        allocator_chunk_size_bytes: TEST_ALLOCATOR_CHUNK_SIZE_BYTES,
        heap_small_size_bytes: 16,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..SharedHeapOptions::default()
    };
    let allocator = test_shared_allocator(&options);
    let shared = SharedRawSpace::with_allocator(allocator).expect("shared raw should build");

    // capture two allocations so only one changes later
    let first = shared
        .allocate(
            RawAllocationShape::bytes(6),
            Payload::Bytes(&[1, 2, 3, 4, 5, 6]),
        )
        .expect("shared allocation should succeed");
    let second = shared
        .allocate(
            RawAllocationShape::bytes(6),
            Payload::Bytes(&[7, 8, 9, 10, 11, 12]),
        )
        .expect("shared allocation should succeed");
    let image = shared.image().expect("shared raw image should capture");
    let forked = shared.fork().expect("shared fork should retain live pages");
    let restored = SharedRawSpace::from_image_with_allocator(shared.allocator.clone(), &image)
        .expect("shared image restore should succeed");
    let forked_image = forked.image().expect("shared raw image should capture");
    let restored_image = restored.image().expect("shared raw image should capture");

    // forked and restored bytes should match the captured image
    assert_eq!(
        raw_allocation_bytes(
            forked_image
                .allocation(0)
                .expect("first forked allocation image should exist"),
        ),
        vec![1, 2, 3, 4, 5, 6]
    );
    assert_eq!(
        raw_allocation_bytes(
            forked_image
                .allocation(1)
                .expect("second forked allocation image should exist"),
        ),
        vec![7, 8, 9, 10, 11, 12]
    );
    assert_eq!(
        raw_allocation_bytes(
            restored_image
                .allocation(0)
                .expect("first restored allocation image should exist"),
        ),
        vec![1, 2, 3, 4, 5, 6]
    );
    assert_eq!(
        raw_allocation_bytes(
            restored_image
                .allocation(1)
                .expect("second restored allocation image should exist"),
        ),
        vec![7, 8, 9, 10, 11, 12]
    );

    // mutating one allocation should not affect the captured image
    let first = restored
        .replace_bytes(first, &[9, 2, 3, 4, 5, 6])
        .expect("shared replace should succeed");
    let mutated_image = restored.image().expect("shared raw image should capture");

    assert_eq!(
        raw_allocation_bytes(
            image
                .allocation(0)
                .expect("captured allocation image should exist"),
        ),
        vec![1, 2, 3, 4, 5, 6]
    );
    assert_eq!(
        raw_allocation_bytes(
            mutated_image
                .allocation(0)
                .expect("mutated allocation image should exist"),
        ),
        vec![9, 2, 3, 4, 5, 6]
    );
    assert_eq!(restored.read_bytes(first), Ok(vec![9, 2, 3, 4, 5, 6]));
    assert_eq!(restored.read_bytes(second), Ok(vec![7, 8, 9, 10, 11, 12]));
}

/// Preserve shared heap metadata and bytes across image roundtrips.
#[test]
fn test_roundtrip_shared_heap_space_image() {
    let options = SharedHeapOptions {
        page_size_bytes: 4,
        allocator_chunk_size_bytes: TEST_ALLOCATOR_CHUNK_SIZE_BYTES,
        heap_small_size_bytes: 16,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..SharedHeapOptions::default()
    };
    let allocator = test_shared_allocator(&options);
    let layouts = test_layouts(&[(6, TraceMap::empty()), (6, TraceMap::empty())]);
    let first_layout = &layouts[0];
    let second_layout = &layouts[1];
    let heap = SharedHeapSpace::with_options(allocator.clone(), &options)
        .expect("shared heap space should build");

    // capture two allocations in one shared small span
    let first_bytes = vec![1; 6];
    let second_bytes = vec![2; 6];
    let mut shared_cache = heap.allocation_cache();
    let first = heap
        .allocate(
            &mut shared_cache,
            &heap.allocation_plan(first_layout.allocation()),
            Payload::Bytes(&first_bytes),
            true,
        )
        .expect("shared heap allocation should succeed");
    let _second = heap
        .allocate(
            &mut shared_cache,
            &heap.allocation_plan(second_layout.allocation()),
            Payload::Bytes(&second_bytes),
            true,
        )
        .expect("shared heap allocation should succeed");
    let image = heap.image().expect("shared heap image should capture");
    let restored = SharedHeapSpace::from_image_with_allocator(allocator.clone(), &image)
        .expect("shared heap image restore should succeed");
    let restored_image = restored.image().expect("shared heap image should capture");

    // restored metadata should match the captured image
    assert_eq!(restored.scan(first, trace_table()), Ok(TraceMap::empty()));
    assert!(Arc::ptr_eq(&restored.allocator, &allocator));
    assert_eq!(image.spans().len(), restored_image.spans().len());

    // restored bytes should match the captured shared heap
    let first_address = restored.base_address() + first.offset();
    let bytes = read_mapped_bytes(first_address, first_bytes.len());

    assert_eq!(bytes, first_bytes);
    assert_eq!(
        restored_image.spans()[0].bytes[..first_bytes.len()],
        first_bytes
    );

    // mutating one slot should not affect the captured image
    restored
        .write_barrier_bytes(first, 0, &[0xFE], trace_table())
        .expect("shared heap write barrier should record");

    write_mapped_bytes(first_address, &[0xFE]);

    let mutated_image = restored.image().expect("shared heap image should capture");
    let mut expected_first = first_bytes.clone();
    expected_first[0] = 0xFE;

    let bytes = read_mapped_bytes(first_address, expected_first.len());

    assert_eq!(bytes, expected_first);
    assert_eq!(
        mutated_image.spans()[0].bytes[..expected_first.len()],
        expected_first
    );
    assert_eq!(image.spans()[0].bytes[..first_bytes.len()], first_bytes);
}
