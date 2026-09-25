use std::sync::Arc;

use tspp_mir::TraceMap;

use crate::shared::storage::HeapStorage;
use crate::{Payload, SharedHeapOptions, SizeClassTable, test_layouts};

use super::{TestHeapPlan, read_mapped_bytes, test_memory, trace_view, write_mapped_bytes};

/// Preserve shared heap metadata and bytes across image roundtrips.
#[test]
fn test_roundtrip_shared_heap_storage_image() {
    let options = SharedHeapOptions {
        page_size_bytes: 4,
        heap_small_size_bytes: 16,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..SharedHeapOptions::default()
    };
    let memory = test_memory(options.page_size_bytes);
    let layouts = test_layouts(&[(6, TraceMap::empty()), (6, TraceMap::empty())]);
    let first_layout = &layouts[0];
    let second_layout = &layouts[1];
    let heap =
        HeapStorage::new(memory.clone(), &options).expect("shared heap storage should build");

    // capture two blocks in one shared small span
    let first_bytes = vec![1; 6];
    let second_bytes = vec![2; 6];
    let first_shape = first_layout.block();
    let second_shape = second_layout.block();
    let mut shared_cache = heap.allocation_cache();
    let first = heap
        .allocate(
            &mut shared_cache,
            &heap.test_allocation_plan(&first_shape),
            Payload::Bytes(&first_bytes),
            true,
        )
        .expect("shared heap block should succeed");
    let _second = heap
        .allocate(
            &mut shared_cache,
            &heap.test_allocation_plan(&second_shape),
            Payload::Bytes(&second_bytes),
            true,
        )
        .expect("shared heap block should succeed");
    let memory_image = memory.capture().expect("memory image should capture");
    let image = heap.image();
    let image_memory = Arc::new(memory_image.restore().expect("image memory should restore"));
    let restored_memory = Arc::new(memory_image.restore().expect("memory should restore"));
    let restored = HeapStorage::from_image(restored_memory.clone(), &image)
        .expect("shared heap image restore should succeed");
    let restored_image = restored.image();

    // restored metadata should match the captured image
    assert_eq!(
        restored.trace_map(first, trace_view()),
        Ok(TraceMap::empty())
    );
    assert!(Arc::ptr_eq(&restored.memory, &restored_memory));
    assert_eq!(image.spans().len(), restored_image.spans().len());

    // restored bytes should match the captured shared heap
    let first_address = restored.base_address() + first.offset();
    let bytes = read_mapped_bytes(first_address, first_bytes.len());

    assert_eq!(bytes, first_bytes);
    assert_eq!(
        restored_memory
            .read_bytes(restored_image.spans()[0].first_offset, first_bytes.len())
            .expect("restored span bytes should read"),
        first_bytes
    );

    // mutating one slot should not affect the captured image
    restored
        .write_barrier_bytes(first, 0, &[0xFE], trace_view())
        .expect("shared heap write barrier should record");

    write_mapped_bytes(first_address, &[0xFE]);

    let mutated_image = restored.image();
    let mut expected_first = first_bytes.clone();
    expected_first[0] = 0xFE;

    let bytes = read_mapped_bytes(first_address, expected_first.len());

    assert_eq!(bytes, expected_first);
    assert_eq!(
        restored_memory
            .read_bytes(mutated_image.spans()[0].first_offset, expected_first.len())
            .expect("mutated span bytes should read"),
        expected_first
    );
    assert_eq!(
        image_memory
            .read_bytes(image.spans()[0].first_offset, first_bytes.len())
            .expect("captured span bytes should read"),
        first_bytes
    );
}
