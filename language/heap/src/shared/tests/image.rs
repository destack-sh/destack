use std::sync::Arc;

use crate::{
    Allocator, HeapOptions, PageRun, Payload, RawAllocationShape, SharedHeapSpace, SharedRawSpace,
    SizeClassTable, test_allocator, test_layouts,
};
use destack_mir::ReferenceMap;

/// The allocator chunk size for small-page shared image fixtures.
const TEST_ALLOCATOR_CHUNK_BYTES: usize = 1024 * 1024;

/// Return the bytes from one shared image page run.
fn read_page_run_bytes(allocator: &Allocator, page_run: &PageRun, byte_len: usize) -> Vec<u8> {
    allocator
        .read_bytes_from(page_run, 0, byte_len)
        .expect("shared image bytes should resolve")
}

/// Preserve shared allocation bytes across image and fork boundaries.
#[test]
fn test_roundtrip_shared_memory_image_and_fork() {
    let options = HeapOptions {
        page_bytes: 4,
        allocator_chunk_bytes: TEST_ALLOCATOR_CHUNK_BYTES,
        heap_small_bytes: 16,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::shared()
    };
    let allocator = test_allocator(&options);
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
        read_page_run_bytes(
            &shared.allocator,
            &forked_image
                .allocation(0)
                .expect("first forked allocation image should exist")
                .pages,
            6
        ),
        vec![1, 2, 3, 4, 5, 6]
    );
    assert_eq!(
        read_page_run_bytes(
            &shared.allocator,
            &forked_image
                .allocation(1)
                .expect("second forked allocation image should exist")
                .pages,
            6
        ),
        vec![7, 8, 9, 10, 11, 12]
    );
    assert_eq!(
        read_page_run_bytes(
            &shared.allocator,
            &restored_image
                .allocation(0)
                .expect("first restored allocation image should exist")
                .pages,
            6,
        ),
        vec![1, 2, 3, 4, 5, 6]
    );
    assert_eq!(
        read_page_run_bytes(
            &shared.allocator,
            &restored_image
                .allocation(1)
                .expect("second restored allocation image should exist")
                .pages,
            6,
        ),
        vec![7, 8, 9, 10, 11, 12]
    );

    // mutating one allocation should not affect the captured image
    let first = restored
        .replace_bytes(first, &[9, 2, 3, 4, 5, 6])
        .expect("shared replace should succeed");
    let mutated_image = restored.image().expect("shared raw image should capture");

    assert_eq!(
        read_page_run_bytes(
            &shared.allocator,
            &image
                .allocation(0)
                .expect("captured allocation image should exist")
                .pages,
            6,
        ),
        vec![1, 2, 3, 4, 5, 6]
    );
    assert_eq!(
        read_page_run_bytes(
            &shared.allocator,
            &mutated_image
                .allocation(0)
                .expect("mutated allocation image should exist")
                .pages,
            6,
        ),
        vec![9, 2, 3, 4, 5, 6]
    );
    assert_eq!(restored.read_bytes(first), Ok(vec![9, 2, 3, 4, 5, 6]));
    assert_eq!(restored.read_bytes(second), Ok(vec![7, 8, 9, 10, 11, 12]));
}

/// Preserve shared heap metadata and bytes across image roundtrips.
#[test]
fn test_roundtrip_shared_heap_space_image() {
    let options = HeapOptions {
        page_bytes: 4,
        allocator_chunk_bytes: TEST_ALLOCATOR_CHUNK_BYTES,
        heap_small_bytes: 16,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::shared()
    };
    let allocator = test_allocator(&options);
    let layouts = test_layouts(&[(6, ReferenceMap::empty()), (6, ReferenceMap::empty())]);
    let first_layout = &layouts[0];
    let second_layout = &layouts[1];
    let heap = SharedHeapSpace::with_options(allocator.clone(), &options)
        .expect("shared heap space should build");

    // capture two allocations in one shared small span
    let first_bytes = vec![1; 6];
    let second_bytes = vec![2; 6];
    let mut shared_allocator = heap.allocator();
    let first = heap
        .allocate(
            &mut shared_allocator,
            &heap.allocation_layout(first_layout.allocation()),
            Payload::Bytes(&first_bytes),
            true,
        )
        .expect("shared heap allocation should succeed");
    let _second = heap
        .allocate(
            &mut shared_allocator,
            &heap.allocation_layout(second_layout.allocation()),
            Payload::Bytes(&second_bytes),
            true,
        )
        .expect("shared heap allocation should succeed");
    let image = heap.image().expect("shared heap image should capture");
    let restored = SharedHeapSpace::from_image_with_allocator(allocator.clone(), &image)
        .expect("shared heap image restore should succeed");
    let restored_image = restored.image().expect("shared heap image should capture");

    // restored metadata should match the captured image
    assert_eq!(restored.scan(first), Ok(ReferenceMap::empty()));
    assert!(Arc::ptr_eq(&restored.allocator, &allocator));
    assert_eq!(image.spans().len(), restored_image.spans().len());

    // restored bytes should match the captured shared heap
    let first_address = restored.base_address() + first.offset();
    let bytes =
        unsafe { std::slice::from_raw_parts(first_address as *const u8, first_bytes.len()) };

    assert_eq!(bytes, first_bytes);
    assert_eq!(
        read_page_run_bytes(
            allocator.as_ref(),
            &restored_image.spans()[0].pages,
            restored_image.spans()[0].class.size_class,
        )[..first_bytes.len()],
        first_bytes
    );

    // mutating one slot should not affect the captured image
    restored
        .write_barrier_bytes(first, 0, &[0xFE])
        .expect("shared heap write barrier should record");

    // write through the restored heap mapping
    unsafe {
        std::ptr::write(first_address as *mut u8, 0xFE);
    }

    let mutated_image = restored.image().expect("shared heap image should capture");
    let mut expected_first = first_bytes.clone();
    expected_first[0] = 0xFE;

    let bytes =
        unsafe { std::slice::from_raw_parts(first_address as *const u8, expected_first.len()) };

    assert_eq!(bytes, expected_first);
    assert_eq!(
        read_page_run_bytes(
            allocator.as_ref(),
            &mutated_image.spans()[0].pages,
            mutated_image.spans()[0].class.size_class,
        )[..expected_first.len()],
        expected_first
    );
    assert_eq!(
        read_page_run_bytes(
            allocator.as_ref(),
            &image.spans()[0].pages,
            image.spans()[0].class.size_class,
        )[..first_bytes.len()],
        first_bytes
    );
}
