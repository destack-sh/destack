use std::sync::Arc;

use tspp_memory::MemoryMap;
use tspp_mir::TraceMap;

use crate::local::storage::{HeapStorage, HeapStorageImage, LargeBlockImage, SmallSpanImage};
use crate::{
    Heap, HeapConfigurationError, HeapError, HeapLimits, HeapOptions, HeapReference, Payload,
    SizeClassTable, TestLayout, test_layout, test_layouts,
};

use super::{
    TestHeapPlan, read_mapped_bytes, test_heap_with_limits, test_memory, test_storage, trace_view,
    write_mapped_byte, write_mapped_bytes,
};

/// Release freed large-block memory before capturing an image.
#[test]
fn test_capture_heap_image_releases_freed_large_block() {
    let options = HeapOptions {
        page_size_bytes: 16,
        heap_small_size_bytes: 32,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let layout = test_layout(9, TraceMap::empty());
    let mut heap = test_storage(&options);
    let reference = heap.test_allocate(layout.block(), Payload::Bytes(&[9; 9]));

    assert_eq!(heap.retained_bytes(), 16);

    heap.free(reference).expect("heap free should succeed");
    let image = heap.image().expect("heap image should capture");

    assert_eq!(heap.allocation_count(), 0);
    assert_eq!(heap.allocated_bytes(), 0);
    assert_eq!(heap.retained_bytes(), 0);
    assert!(image.blocks[0].is_none());
}

/// Build one heap storage with explicit empty layouts.
fn heap_storage_with_empty_layouts(
    memory: Arc<MemoryMap>,
    options: &HeapOptions,
    allocation_byte_lens: &[usize],
) -> (HeapStorage, Vec<TestLayout>) {
    let layout_specs = allocation_byte_lens
        .iter()
        .map(|byte_len| (*byte_len, TraceMap::empty()))
        .collect::<Vec<_>>();
    let layouts = test_layouts(&layout_specs);
    let heap = HeapStorage::new(memory, options).expect("heap should build");

    (heap, layouts)
}

/// Build one full heap with explicit empty layouts.
fn test_heap_with_empty_layouts(
    options: HeapOptions,
    allocation_byte_lens: &[usize],
) -> (Heap, Vec<TestLayout>) {
    let layout_specs = allocation_byte_lens
        .iter()
        .map(|byte_len| (*byte_len, TraceMap::empty()))
        .collect::<Vec<_>>();
    let layouts = test_layouts(&layout_specs);
    let heap = test_heap_with_limits(HeapLimits::default(), options);

    (heap, layouts)
}

/// Return the bytes for one large image block.
fn read_large_block_bytes(memory: &MemoryMap, block: &Option<LargeBlockImage>) -> Vec<u8> {
    let Some(block) = block else {
        panic!("large block should be live");
    };

    memory
        .read_bytes(block.first_offset, block.byte_len)
        .expect("large block bytes should read")
}

/// Return the bytes for one small-span slot.
fn read_small_slot_bytes(
    memory: &MemoryMap,
    span: &SmallSpanImage,
    size_class: usize,
    slot_index: usize,
    byte_len: usize,
) -> Vec<u8> {
    let start = size_class * slot_index;

    memory
        .read_bytes(span.first_offset + start, byte_len)
        .expect("small slot bytes should read")
}

/// Write one heap payload range for image assertions.
fn write_payload(heap: &mut Heap, reference: HeapReference, start: usize, bytes: &[u8]) {
    heap.write_barrier(reference, start, bytes.len(), trace_view())
        .expect("heap barrier should record");
    let address = heap.heap_base_address() + reference.offset() + start;

    write_mapped_bytes(address, bytes);
}

/// Return the first live heap block bytes from one captured image.
fn read_first_heap_image_bytes(memory: &MemoryMap, image: &HeapStorageImage) -> Vec<u8> {
    // slots first
    for span in &image.spans {
        for slot_index in 0..span.slot_count {
            if !span.occupied.contains(slot_index) {
                continue;
            }

            return read_small_slot_bytes(
                memory,
                span,
                span.class.size_class(),
                slot_index,
                span.class.size_class(),
            );
        }
    }

    // then large blocks
    for block in &image.blocks {
        if block.is_none() {
            continue;
        }

        return read_large_block_bytes(memory, block);
    }

    panic!("heap image should contain one live block")
}

/// Preserve heap metadata and bytes across image roundtrips.
#[test]
fn test_roundtrip_heap_storage_image() {
    let options = HeapOptions {
        heap_small_size_bytes: 32,
        page_size_bytes: 4,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let first_bytes = vec![1; 5000];
    let second_bytes = vec![2; 5000];
    let memory = test_memory(options.page_size_bytes);
    let (mut heap, layout_ids) = heap_storage_with_empty_layouts(
        memory.clone(),
        &options,
        &[first_bytes.len(), second_bytes.len()],
    );
    let first_layout = &layout_ids[0];
    let second_layout = &layout_ids[1];

    // capture two blocks so the restored copy has independent bytes
    let first = heap.test_allocate(first_layout.block(), Payload::Bytes(&first_bytes));
    let _second = heap.test_allocate(second_layout.block(), Payload::Bytes(&second_bytes));
    let memory_image = memory.capture().expect("memory image should capture");
    let image = heap.image().expect("heap image should capture");
    let image_memory = Arc::new(memory_image.restore().expect("image memory should restore"));
    let restored_memory = Arc::new(memory_image.restore().expect("memory should restore"));
    let mut restored = HeapStorage::from_image(restored_memory.clone(), &image, trace_view())
        .expect("heap image should restore");
    let restored_image = restored.image().expect("heap image should capture");

    // restored metadata should match the captured image
    assert!(Arc::ptr_eq(&restored.memory, &restored_memory));
    assert_eq!(image.blocks.len(), restored_image.blocks.len());
    assert_eq!(image.allocated_count, restored_image.allocated_count);
    assert_eq!(image.allocated_bytes, restored_image.allocated_bytes);

    // restored bytes should match the captured heap bytes
    assert_eq!(
        read_large_block_bytes(&restored_memory, &restored_image.blocks[0]),
        first_bytes
    );
    let first_address = restored.base_address() + first.offset();

    let bytes = read_mapped_bytes(first_address, first_bytes.len());

    assert_eq!(bytes, first_bytes);
    assert_eq!(
        read_large_block_bytes(&restored_memory, &restored_image.blocks[1]),
        second_bytes
    );

    // mutating one block should not affect the captured image
    restored
        .write_barrier(first, 0, 1, trace_view())
        .expect("heap write barrier should record");
    let address = restored.base_address() + first.offset();

    write_mapped_byte(address, 0xFE);

    let mutated_image = restored.image().expect("heap image should capture");

    let mut expected_first = first_bytes.clone();
    expected_first[0] = 0xFE;

    assert_eq!(
        read_large_block_bytes(&image_memory, &image.blocks[0]),
        first_bytes
    );
    assert_eq!(
        read_large_block_bytes(&restored_memory, &mutated_image.blocks[0]),
        expected_first
    );
    let bytes = read_mapped_bytes(first_address, expected_first.len());

    assert_eq!(bytes, expected_first);
    assert_eq!(
        read_large_block_bytes(&restored_memory, &mutated_image.blocks[1]),
        second_bytes
    );
}

/// Preserve heap images and forks across one full heap.
#[test]
fn test_roundtrip_heap_image_and_fork() {
    let heap_bytes = 7i64.to_le_bytes();
    let (mut test_heap, layout_ids) =
        test_heap_with_empty_layouts(HeapOptions::local(), &[heap_bytes.len()]);
    let layout = &layout_ids[0];
    let heap = &mut test_heap;
    let _heap_reference = heap.test_allocate(layout.block(), Payload::Bytes(&heap_bytes));

    // capture both the frozen image and the live fork
    let memory_image = heap
        .storage
        .memory
        .capture()
        .expect("memory image should capture");
    let image = heap.image().expect("heap image should capture");
    let fork_memory = Arc::new(memory_image.restore().expect("fork memory should restore"));
    let mut forked = heap
        .fork(fork_memory.clone(), trace_view())
        .expect("heap fork should retain live pages");
    let restored_memory = Arc::new(memory_image.restore().expect("memory should restore"));
    let mut restored = Heap::from_image(
        &image,
        restored_memory.clone(),
        HeapLimits::default(),
        trace_view(),
    )
    .expect("heap image should restore");
    let forked_image = forked.image().expect("heap image should capture");
    let restored_image = restored.image().expect("heap image should capture");

    assert_eq!(
        read_first_heap_image_bytes(&fork_memory, forked_image.storage()),
        heap_bytes.to_vec()
    );
    assert_eq!(
        read_first_heap_image_bytes(&restored_memory, restored_image.storage()),
        heap_bytes.to_vec()
    );
}

/// Preserve image bytes when writing one restored heap block.
#[test]
fn test_heap_image_write_preserves_captured_allocation_bytes() {
    let options = HeapOptions {
        heap_small_size_bytes: 32,
        page_size_bytes: 4096,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let first_bytes = vec![0xAA; 5000];
    let second_bytes = vec![0xBB; 5000];
    let (mut test_heap, layout_ids) =
        test_heap_with_empty_layouts(options, &[first_bytes.len(), second_bytes.len()]);
    let heap = &mut test_heap;
    let first = heap.test_allocate(layout_ids[0].block(), Payload::Bytes(&first_bytes));
    let _second = heap.test_allocate(layout_ids[1].block(), Payload::Bytes(&second_bytes));
    let memory_image = heap
        .storage
        .memory
        .capture()
        .expect("memory image should capture");
    let image = heap.image().expect("heap image should capture");
    let image_memory = Arc::new(memory_image.restore().expect("image memory should restore"));
    let restored_memory = Arc::new(memory_image.restore().expect("memory should restore"));
    let mut restored = Heap::from_image(
        &image,
        restored_memory.clone(),
        HeapLimits::default(),
        trace_view(),
    )
    .expect("heap image should restore");

    // mutating one block should only change the restored heap
    write_payload(&mut restored, first, 0, &[0xCC]);
    let mutated_image = restored.image().expect("heap image should capture");

    let mut expected_first = first_bytes;
    expected_first[0] = 0xCC;

    assert_eq!(
        read_large_block_bytes(&restored_memory, &mutated_image.storage().blocks[0]),
        expected_first
    );
    assert_eq!(
        read_large_block_bytes(&restored_memory, &mutated_image.storage().blocks[1]),
        second_bytes
    );
    assert_eq!(
        read_large_block_bytes(&image_memory, &image.storage().blocks[0]),
        vec![0xAA; 5000]
    );
}

/// Preserve image bytes when writing one restored heap page.
#[test]
fn test_heap_image_write_preserves_captured_page_bytes() {
    let options = HeapOptions {
        heap_small_size_bytes: 32,
        page_size_bytes: 4096,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let bytes = vec![0xAA; 9000];
    let (mut heap, layout_ids) = test_heap_with_empty_layouts(options, &[bytes.len()]);
    let heap = &mut heap;
    let reference = heap.test_allocate(layout_ids[0].block(), Payload::Bytes(&bytes));
    let memory_image = heap
        .storage
        .memory
        .capture()
        .expect("memory image should capture");
    let image = heap.image().expect("heap image should capture");
    let image_memory = Arc::new(memory_image.restore().expect("image memory should restore"));
    let restored_memory = Arc::new(memory_image.restore().expect("memory should restore"));
    let mut restored = Heap::from_image(
        &image,
        restored_memory.clone(),
        HeapLimits::default(),
        trace_view(),
    )
    .expect("heap image should restore");

    // mutating one page should change only the written bytes
    write_payload(&mut restored, reference, 4096, &[0xCC]);
    let mutated_image = restored.image().expect("heap image should capture");
    let mut expected = bytes.clone();
    expected[4096] = 0xCC;

    assert_eq!(
        read_large_block_bytes(&restored_memory, &mutated_image.storage().blocks[0]),
        expected
    );
    assert_eq!(
        read_large_block_bytes(&image_memory, &image.storage().blocks[0]),
        bytes
    );
}

/// Preserve image bytes when writing several restored pages.
#[test]
fn test_heap_image_write_preserves_captured_multi_page_bytes() {
    let options = HeapOptions {
        heap_small_size_bytes: 32,
        page_size_bytes: 4096,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let bytes = vec![0xAA; 5 * 4096];
    let (mut heap, layout_ids) = test_heap_with_empty_layouts(options, &[bytes.len()]);
    let heap = &mut heap;
    let reference = heap.test_allocate(layout_ids[0].block(), Payload::Bytes(&bytes));
    let memory_image = heap
        .storage
        .memory
        .capture()
        .expect("memory image should capture");
    let image = heap.image().expect("heap image should capture");
    let image_memory = Arc::new(memory_image.restore().expect("image memory should restore"));
    let restored_memory = Arc::new(memory_image.restore().expect("memory should restore"));
    let mut restored = Heap::from_image(
        &image,
        restored_memory.clone(),
        HeapLimits::default(),
        trace_view(),
    )
    .expect("heap image should restore");

    // mutating three pages should only change those bytes
    write_payload(&mut restored, reference, 0, &vec![0xCC; 3 * 4096]);
    let mutated_image = restored.image().expect("heap image should capture");
    let mut expected = bytes.clone();
    expected[..3 * 4096].fill(0xCC);

    assert_eq!(
        read_large_block_bytes(&restored_memory, &mutated_image.storage().blocks[0]),
        expected
    );
    assert_eq!(
        read_large_block_bytes(&image_memory, &image.storage().blocks[0]),
        bytes
    );
}

/// Preserve image bytes when most restored pages are written.
#[test]
fn test_heap_image_write_preserves_captured_many_page_bytes() {
    let options = HeapOptions {
        heap_small_size_bytes: 32,
        page_size_bytes: 4096,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let bytes = vec![0xAA; 5 * 4096];
    let (mut heap, layout_ids) = test_heap_with_empty_layouts(options, &[bytes.len()]);
    let heap = &mut heap;
    let reference = heap.test_allocate(layout_ids[0].block(), Payload::Bytes(&bytes));
    let memory_image = heap
        .storage
        .memory
        .capture()
        .expect("memory image should capture");
    let image = heap.image().expect("heap image should capture");
    let image_memory = Arc::new(memory_image.restore().expect("image memory should restore"));
    let restored_memory = Arc::new(memory_image.restore().expect("memory should restore"));
    let mut restored = Heap::from_image(
        &image,
        restored_memory.clone(),
        HeapLimits::default(),
        trace_view(),
    )
    .expect("heap image should restore");

    // mutating four pages should only change those bytes
    write_payload(&mut restored, reference, 0, &vec![0xCC; 4 * 4096]);
    let mutated_image = restored.image().expect("heap image should capture");
    let mut expected = bytes.clone();
    expected[..4 * 4096].fill(0xCC);

    assert_eq!(
        read_large_block_bytes(&restored_memory, &mutated_image.storage().blocks[0]),
        expected
    );
    assert_eq!(
        read_large_block_bytes(&image_memory, &image.storage().blocks[0]),
        bytes
    );
}

/// Preserve heap small-space bytes across image roundtrips.
#[test]
fn test_roundtrip_heap_small_storage_image() {
    let options = HeapOptions {
        page_size_bytes: 4,
        ..HeapOptions::local()
    };
    let memory = test_memory(options.page_size_bytes);
    let (mut heap, layout_ids) = heap_storage_with_empty_layouts(memory.clone(), &options, &[3, 3]);

    // small blocks should roundtrip as independent bytes
    let first = heap.test_allocate(layout_ids[0].block(), Payload::Bytes(&[1, 2, 3]));
    let _second = heap.test_allocate(layout_ids[1].block(), Payload::Bytes(&[4, 5, 6]));
    let memory_image = memory.capture().expect("memory image should capture");
    let image = heap.image().expect("heap image should capture");
    let image_memory = Arc::new(memory_image.restore().expect("image memory should restore"));
    let restored_memory = Arc::new(memory_image.restore().expect("memory should restore"));
    let mut restored = HeapStorage::from_image(restored_memory.clone(), &image, trace_view())
        .expect("heap image should restore");
    let restored_image = restored.image().expect("heap image should capture");

    assert_eq!(
        read_small_slot_bytes(
            &restored_memory,
            &restored_image.spans[0],
            restored_image.spans[0].class.size_class(),
            0,
            3,
        ),
        vec![1, 2, 3]
    );

    // mutating one small block should not affect the captured image
    restored
        .write_barrier(first, 1, 1, trace_view())
        .expect("heap write barrier should record");
    let address = restored.base_address() + first.offset() + 1;

    write_mapped_byte(address, 0xFE);

    let mutated_image = restored.image().expect("heap image should capture");

    assert_eq!(
        read_small_slot_bytes(
            &restored_memory,
            &mutated_image.spans[0],
            mutated_image.spans[0].class.size_class(),
            0,
            3,
        ),
        vec![1, 0xFE, 3]
    );
    assert_eq!(
        read_small_slot_bytes(
            &image_memory,
            &image.spans[0],
            image.spans[0].class.size_class(),
            0,
            3,
        ),
        vec![1, 2, 3]
    );
}

/// Reject invalid heap image metadata during restore.
#[test]
fn test_restore_full_heap_image_rejects_invalid_size_class() {
    let options = HeapOptions::local();
    let memory = test_memory(options.page_size_bytes);
    let (mut heap, layout_ids) = heap_storage_with_empty_layouts(memory.clone(), &options, &[3]);
    heap.test_allocate(layout_ids[0].block(), Payload::Bytes(&[1, 2, 3]));

    let mut image = heap.image().expect("heap image should capture");
    image.size_classes = SizeClassTable::new([16]).expect("size classes should validate");

    let error = HeapStorage::from_image(test_memory(options.page_size_bytes), &image, trace_view())
        .expect_err("heap restore should fail loudly");

    assert_eq!(
        error,
        HeapError::configuration(HeapConfigurationError::InvalidSizeClass { class_bytes: 8 })
    );
}

/// Reject invalid heap metadata during fork.
#[test]
fn test_fork_heap_storage_rejects_invalid_size_class() {
    let options = HeapOptions::local();
    let memory = test_memory(options.page_size_bytes);
    let (mut heap, layout_ids) = heap_storage_with_empty_layouts(memory.clone(), &options, &[3]);
    heap.test_allocate(layout_ids[0].block(), Payload::Bytes(&[1, 2, 3]));
    heap.small.size_classes = SizeClassTable::new([16]).expect("size classes should validate");
    let fork_memory = Arc::new(memory.fork_lazy().expect("test World memory should fork"));

    let error = heap
        .fork(fork_memory, trace_view())
        .expect_err("heap fork should fail loudly");

    assert_eq!(
        error,
        HeapError::configuration(HeapConfigurationError::InvalidSizeClass { class_bytes: 8 })
    );
}
