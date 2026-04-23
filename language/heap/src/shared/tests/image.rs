use std::sync::Arc;

use crate::{HeapOptions, LayoutId, SharedHeapSpace, SharedRawSpace, TracePlan, test_allocator};
use destack_mir::LayoutTrace;

/// Share unchanged shared allocations across image and fork boundaries.
#[test]
fn test_roundtrip_shared_memory_image_and_fork() {
    let layout = HeapOptions {
        page_bytes: 4,
        ..HeapOptions::shared()
    };
    let allocator = test_allocator(&layout);
    let shared = SharedRawSpace::with_allocator(allocator);

    // capture two allocations so only one has to detach later
    let first = shared
        .allocate_bytes(&[1, 2, 3, 4, 5, 6])
        .expect("shared allocation should succeed");
    let second = shared
        .allocate_bytes(&[7, 8, 9, 10, 11, 12])
        .expect("shared allocation should succeed");
    let image = shared.image();
    let forked = shared.fork().expect("shared fork should retain live pages");
    let restored = SharedRawSpace::from_image_with_allocator(shared.allocator.clone(), &image)
        .expect("shared image restore should succeed");
    let forked_image = forked.image();
    let restored_image = restored.image();

    // untouched pages should still share after fork and restore
    assert_eq!(
        image.entry(0).unwrap().pages,
        forked_image.entry(0).unwrap().pages
    );
    assert_eq!(
        image.entry(1).unwrap().pages,
        forked_image.entry(1).unwrap().pages
    );
    assert_eq!(
        image.entry(0).unwrap().pages,
        restored_image.entry(0).unwrap().pages
    );
    assert_eq!(
        image.entry(1).unwrap().pages,
        restored_image.entry(1).unwrap().pages
    );

    // mutating one allocation should detach only that allocation
    let first = restored
        .replace_bytes(first, &[9, 2, 3, 4, 5, 6])
        .expect("shared replace should succeed");
    let mutated_image = restored.image();

    assert_ne!(
        image.entry(0).unwrap().pages,
        mutated_image.entry(0).unwrap().pages
    );
    assert_eq!(
        image.entry(1).unwrap().pages,
        mutated_image.entry(1).unwrap().pages
    );
    assert_eq!(restored.read_bytes(first), Ok(vec![9, 2, 3, 4, 5, 6]));
    assert_eq!(restored.read_bytes(second), Ok(vec![7, 8, 9, 10, 11, 12]));
}

/// Preserve shared heap metadata across image roundtrips and detach only touched entries.
#[test]
fn test_roundtrip_shared_heap_space_image() {
    let layout = HeapOptions {
        page_bytes: 4,
        ..HeapOptions::shared()
    };
    let allocator = test_allocator(&layout);
    let heap = SharedHeapSpace::with_allocator(allocator.clone());

    // capture two entries in one shared small span
    let first_bytes = vec![1; 6];
    let second_bytes = vec![2; 6];
    let first = heap
        .allocate_bytes(&first_bytes, LayoutTrace::empty(), None)
        .expect("shared heap allocation should succeed");
    let _second = heap
        .allocate_bytes(&second_bytes, LayoutTrace::empty(), None)
        .expect("shared heap allocation should succeed");
    heap.set_layout_id(first, LayoutId::new(41))
        .expect("shared heap storage layout id should update");
    let image = heap.image();
    let restored = SharedHeapSpace::from_image_with_allocator(allocator.clone(), &image)
        .expect("shared heap image restore should succeed");
    let restored_image = restored.image();

    // restored metadata should match and untouched pages should still share
    assert_eq!(restored.layout_id(first), Ok(Some(LayoutId::new(41))));
    assert_eq!(restored.scan(first), Ok(TracePlan::empty()));
    assert!(Arc::ptr_eq(&restored.allocator, &allocator));
    assert_eq!(image.spans()[0].pages, restored_image.spans()[0].pages);

    // mutating one slot should detach the owning span
    restored
        .write_bytes(first, 0, &[0xFE])
        .expect("shared heap byte write should succeed");
    let mutated_image = restored.image();

    assert_ne!(image.spans()[0].pages, mutated_image.spans()[0].pages);
}
