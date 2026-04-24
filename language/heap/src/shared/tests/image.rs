use std::sync::Arc;

use crate::{HeapOptions, Payload, SharedHeapSpace, SharedRawSpace, test_allocator, test_layouts};
use destack_mir::ReferenceMap;

/// Share unchanged shared allocations across image and fork boundaries.
#[test]
fn test_roundtrip_shared_memory_image_and_fork() {
    let options = HeapOptions {
        page_bytes: 4,
        ..HeapOptions::shared()
    };
    let allocator = test_allocator(&options);
    let shared = SharedRawSpace::with_allocator(allocator);

    // capture two allocations so only one has to detach later
    let first = shared
        .allocate(6, Payload::Bytes(&[1, 2, 3, 4, 5, 6]))
        .expect("shared allocation should succeed");
    let second = shared
        .allocate(6, Payload::Bytes(&[7, 8, 9, 10, 11, 12]))
        .expect("shared allocation should succeed");
    let image = shared.image();
    let forked = shared.fork().expect("shared fork should retain live pages");
    let restored = SharedRawSpace::from_image_with_allocator(shared.allocator.clone(), &image)
        .expect("shared image restore should succeed");
    let forked_image = forked.image();
    let restored_image = restored.image();

    // untouched pages should still share after fork and restore
    assert_eq!(
        image.allocation(0).unwrap().pages,
        forked_image.allocation(0).unwrap().pages
    );
    assert_eq!(
        image.allocation(1).unwrap().pages,
        forked_image.allocation(1).unwrap().pages
    );
    assert_eq!(
        image.allocation(0).unwrap().pages,
        restored_image.allocation(0).unwrap().pages
    );
    assert_eq!(
        image.allocation(1).unwrap().pages,
        restored_image.allocation(1).unwrap().pages
    );

    // mutating one allocation should detach only that allocation
    let first = restored
        .replace_bytes(first, &[9, 2, 3, 4, 5, 6])
        .expect("shared replace should succeed");
    let mutated_image = restored.image();

    assert_ne!(
        image.allocation(0).unwrap().pages,
        mutated_image.allocation(0).unwrap().pages
    );
    assert_eq!(
        image.allocation(1).unwrap().pages,
        mutated_image.allocation(1).unwrap().pages
    );
    assert_eq!(restored.read_bytes(first), Ok(vec![9, 2, 3, 4, 5, 6]));
    assert_eq!(restored.read_bytes(second), Ok(vec![7, 8, 9, 10, 11, 12]));
}

/// Preserve shared heap metadata across image roundtrips and detach only touched allocations.
#[test]
fn test_roundtrip_shared_heap_space_image() {
    let options = HeapOptions {
        page_bytes: 4,
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
    let first = heap
        .allocate(first_layout.allocation(), Payload::Bytes(&first_bytes))
        .expect("shared heap allocation should succeed");
    let _second = heap
        .allocate(second_layout.allocation(), Payload::Bytes(&second_bytes))
        .expect("shared heap allocation should succeed");
    let image = heap.image();
    let restored = SharedHeapSpace::from_image_with_allocator(allocator.clone(), &image)
        .expect("shared heap image restore should succeed");
    let restored_image = restored.image();

    // restored metadata should match and untouched pages should still share
    assert_eq!(restored.scan(first), Ok(ReferenceMap::empty()));
    assert!(Arc::ptr_eq(&restored.allocator, &allocator));
    assert_eq!(image.spans()[0].pages, restored_image.spans()[0].pages);

    // mutating one slot should detach the owning span
    restored
        .write_bytes(first, 0, &[0xFE])
        .expect("shared heap byte write should succeed");
    let mutated_image = restored.image();

    assert_ne!(image.spans()[0].pages, mutated_image.spans()[0].pages);
}
