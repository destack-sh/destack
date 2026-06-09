use std::sync::Arc;

use crate::allocator::Allocator;
use crate::local::storage::{HeapStorage, HeapStorageImage, YoungImage};
use crate::{
    HeapConfigurationError, HeapError, HeapOptions, Payload, SizeClassTable, TestLayout,
    test_allocator, test_layouts,
};
use destack_mir::TraceMap;

use super::{
    TestHeap, heap_allocation_plan, read_mapped_bytes, trace_table, write_mapped_byte,
    write_mapped_bytes,
};

/// The allocator chunk size for small-page image fixtures.
const TEST_ALLOCATOR_CHUNK_SIZE_BYTES: usize = 1024 * 1024;

/// Build one heap storage with explicit empty layouts.
fn heap_storage_with_empty_layouts(
    allocator: Arc<Allocator>,
    options: &HeapOptions,
    allocation_byte_lens: &[usize],
) -> (HeapStorage, Vec<TestLayout>) {
    let layout_specs = allocation_byte_lens
        .iter()
        .map(|byte_len| (*byte_len, TraceMap::empty()))
        .collect::<Vec<_>>();
    let layouts = test_layouts(&layout_specs);
    let heap = HeapStorage::build_with_options(allocator, options).expect("heap should build");

    (heap, layouts)
}

/// Build one full heap with explicit empty layouts.
fn test_heap_with_empty_layouts(
    options: HeapOptions,
    allocation_byte_lens: &[usize],
) -> (TestHeap, Vec<TestLayout>) {
    let layout_specs = allocation_byte_lens
        .iter()
        .map(|byte_len| (*byte_len, TraceMap::empty()))
        .collect::<Vec<_>>();
    let layouts = test_layouts(&layout_specs);
    let heap = TestHeap::with_limits_and_options(crate::HeapLimits::default(), options);

    (heap, layouts)
}

/// Return the bytes for one large image block.
fn read_large_block_bytes(block: &crate::local::storage::LargeBlockImage) -> Vec<u8> {
    block.bytes[..block.byte_len].to_vec()
}

/// Return the bytes for one small-span slot.
fn read_small_slot_bytes(
    span: &crate::local::storage::SmallSpanImage,
    size_class: usize,
    slot_index: usize,
    byte_len: usize,
) -> Vec<u8> {
    let start = size_class * slot_index;

    span.bytes[start..start + byte_len].to_vec()
}

/// Write one heap payload range for image assertions.
fn write_payload(
    heap: &mut crate::Heap,
    reference: crate::HeapReference,
    start: usize,
    bytes: &[u8],
) {
    heap.write_barrier(reference, start, bytes.len(), trace_table())
        .expect("heap barrier should record");
    let address = heap.heap_base_address() + reference.offset() + start;

    write_mapped_bytes(address, bytes);
}

/// Return the bytes for one young space range.
fn read_young_range_bytes(young: &YoungImage, range_index: usize) -> Vec<u8> {
    let block = &young.ranges()[range_index];
    let start = block.first_offset;
    let end = start + block.byte_len;

    young.bytes()[start..end].to_vec()
}

/// Return the bytes for one young space span slot.
fn read_young_slot_bytes(young: &YoungImage, span_index: usize, slot_index: usize) -> Vec<u8> {
    let span = &young.spans()[span_index];
    let start = span.slot_offset(slot_index);
    let end = start + span.byte_len();

    young.bytes()[start..end].to_vec()
}

/// Return the first live heap block bytes from one captured image.
fn read_first_heap_image_bytes(image: &HeapStorageImage) -> Vec<u8> {
    // young blocks first
    for range_index in 0..image.young().ranges().len() {
        if image.young().live().contains(range_index) {
            return read_young_range_bytes(image.young(), range_index);
        }
    }

    // then fixed-size young span slots
    for (span_index, span) in image.young().spans().iter().enumerate() {
        let bits = &image.young().span_bits()[span_index];
        let reserved_count = span.reserved_slot_count_with(span.next_offset);
        for slot_index in 0..reserved_count {
            if bits.freed.contains(slot_index) {
                continue;
            }

            return read_young_slot_bytes(image.young(), span_index, slot_index);
        }
    }

    // then small slots
    for span in image.spans() {
        for slot_index in 0..span.slot_count {
            if !span.occupied.contains(slot_index) {
                continue;
            }

            return read_small_slot_bytes(
                span,
                span.class.size_class,
                slot_index,
                span.class.size_class,
            );
        }
    }

    // then large blocks
    for block in image.blocks() {
        if !block.is_live {
            continue;
        }

        return read_large_block_bytes(block);
    }

    panic!("heap image should contain one live block")
}

/// Preserve heap metadata and bytes across image roundtrips.
#[test]
fn test_roundtrip_heap_storage_image() {
    let options = HeapOptions {
        heap_young_size_bytes: 0,
        heap_small_size_bytes: 32,
        page_size_bytes: 4,
        allocator_chunk_size_bytes: TEST_ALLOCATOR_CHUNK_SIZE_BYTES,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let first_bytes = vec![1; 5000];
    let second_bytes = vec![2; 5000];
    let (mut heap, layout_ids) = heap_storage_with_empty_layouts(
        allocator.clone(),
        &options,
        &[first_bytes.len(), second_bytes.len()],
    );
    let first_layout = &layout_ids[0];
    let second_layout = &layout_ids[1];

    // capture two blocks so the restored copy has independent bytes
    let first = heap
        .allocate(
            &heap_allocation_plan(&heap, first_layout.block()),
            Payload::Bytes(&first_bytes),
        )
        .expect("heap block should succeed");
    let _second = heap
        .allocate(
            &heap_allocation_plan(&heap, second_layout.block()),
            Payload::Bytes(&second_bytes),
        )
        .expect("heap block should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored = HeapStorage::from_image(allocator.clone(), &image, trace_table())
        .expect("heap image should restore");
    let restored_image = restored.image().expect("heap image should capture");

    // restored metadata should match the captured image
    assert!(Arc::ptr_eq(restored.allocator(), &allocator));
    assert_eq!(image.blocks().len(), restored_image.blocks().len());
    assert_eq!(image.allocated_count(), restored_image.allocated_count());
    assert_eq!(image.allocated_bytes(), restored_image.allocated_bytes());

    // restored bytes should match the captured heap bytes
    assert_eq!(
        read_large_block_bytes(&restored_image.blocks()[0]),
        first_bytes
    );
    let first_address = restored.base_address() + first.offset();

    let bytes = read_mapped_bytes(first_address, first_bytes.len());

    assert_eq!(bytes, first_bytes);
    assert_eq!(
        read_large_block_bytes(&restored_image.blocks()[1]),
        second_bytes
    );

    // mutating one block should not affect the captured image
    restored
        .write_barrier(first, 0, 1, trace_table())
        .expect("heap write barrier should record");
    let address = restored.base_address() + first.offset();

    write_mapped_byte(address, 0xFE);

    let mutated_image = restored.image().expect("heap image should capture");

    let mut expected_first = first_bytes.clone();
    expected_first[0] = 0xFE;

    assert_eq!(read_large_block_bytes(&image.blocks()[0]), first_bytes);
    assert_eq!(
        read_large_block_bytes(&mutated_image.blocks()[0]),
        expected_first
    );
    let bytes = read_mapped_bytes(first_address, expected_first.len());

    assert_eq!(bytes, expected_first);
    assert_eq!(
        read_large_block_bytes(&mutated_image.blocks()[1]),
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
    let heap = &mut test_heap.heap;
    let _heap_reference = heap
        .allocate_payload(
            &heap_allocation_plan(&heap, layout.block()),
            Payload::Bytes(&heap_bytes),
        )
        .expect("heap block should succeed");
    // capture both the frozen image and the live fork
    let image = heap.image().expect("heap image should capture");
    let mut forked = heap
        .fork(trace_table())
        .expect("heap fork should retain live pages");
    let mut restored =
        crate::Heap::from_image(&image, trace_table()).expect("heap image should restore");
    let forked_image = forked.image().expect("heap image should capture");
    let restored_image = restored.image().expect("heap image should capture");

    assert_eq!(
        read_first_heap_image_bytes(forked_image.storage()),
        heap_bytes.to_vec()
    );
    assert_eq!(
        read_first_heap_image_bytes(restored_image.storage()),
        heap_bytes.to_vec()
    );
}

/// Restore a heap from one serialized snapshot.
#[test]
fn test_roundtrip_heap_snapshot() {
    let heap_bytes = 11i64.to_le_bytes();
    let (mut test_heap, layout_ids) =
        test_heap_with_empty_layouts(HeapOptions::local(), &[heap_bytes.len()]);
    let layout = &layout_ids[0];
    let heap = &mut test_heap.heap;
    heap.allocate_payload(
        &heap_allocation_plan(&heap, layout.block()),
        Payload::Bytes(&heap_bytes),
    )
    .expect("heap block should succeed");

    let image = heap.image().expect("heap image should capture");
    let snapshot = image.snapshot();
    let mut restored =
        crate::Heap::from_snapshot(&snapshot, trace_table()).expect("heap snapshot should restore");
    let restored_image = restored.image().expect("heap image should capture");

    assert_eq!(
        read_first_heap_image_bytes(restored_image.storage()),
        heap_bytes.to_vec()
    );
}

/// Preserve image bytes when writing one restored heap block.
#[test]
fn test_heap_image_write_preserves_captured_allocation_bytes() {
    let options = HeapOptions {
        heap_young_size_bytes: 0,
        heap_small_size_bytes: 32,
        page_size_bytes: 4096,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let first_bytes = vec![0xAA; 5000];
    let second_bytes = vec![0xBB; 5000];
    let (mut test_heap, layout_ids) =
        test_heap_with_empty_layouts(options, &[first_bytes.len(), second_bytes.len()]);
    let heap = &mut test_heap.heap;
    let first = heap
        .allocate_payload(
            &heap_allocation_plan(&heap, layout_ids[0].block()),
            Payload::Bytes(&first_bytes),
        )
        .expect("heap block should succeed");
    let _second = heap
        .allocate_payload(
            &heap_allocation_plan(&heap, layout_ids[1].block()),
            Payload::Bytes(&second_bytes),
        )
        .expect("heap block should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored =
        crate::Heap::from_image(&image, trace_table()).expect("heap image should restore");

    // mutating one block should only change the restored heap
    write_payload(&mut restored, first, 0, &[0xCC]);
    let mutated_image = restored.image().expect("heap image should capture");

    let mut expected_first = first_bytes;
    expected_first[0] = 0xCC;

    assert_eq!(
        read_large_block_bytes(&mutated_image.storage().blocks()[0]),
        expected_first
    );
    assert_eq!(
        read_large_block_bytes(&mutated_image.storage().blocks()[1]),
        second_bytes
    );
    assert_eq!(
        read_large_block_bytes(&image.storage().blocks()[0]),
        vec![0xAA; 5000]
    );
}

/// Preserve image bytes when writing one restored heap page.
#[test]
fn test_heap_image_write_preserves_captured_page_bytes() {
    let options = HeapOptions {
        heap_young_size_bytes: 0,
        heap_small_size_bytes: 32,
        page_size_bytes: 4096,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let bytes = vec![0xAA; 9000];
    let (mut test_heap, layout_ids) = test_heap_with_empty_layouts(options, &[bytes.len()]);
    let heap = &mut test_heap.heap;
    let reference = heap
        .allocate_payload(
            &heap_allocation_plan(&heap, layout_ids[0].block()),
            Payload::Bytes(&bytes),
        )
        .expect("heap block should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored =
        crate::Heap::from_image(&image, trace_table()).expect("heap image should restore");

    // mutating one page should change only the written bytes
    write_payload(&mut restored, reference, 4096, &[0xCC]);
    let mutated_image = restored.image().expect("heap image should capture");
    let mut expected = bytes.clone();
    expected[4096] = 0xCC;

    assert_eq!(
        read_large_block_bytes(&mutated_image.storage().blocks()[0]),
        expected
    );
    assert_eq!(read_large_block_bytes(&image.storage().blocks()[0]), bytes);
}

/// Preserve image bytes when writing several restored pages.
#[test]
fn test_heap_image_write_preserves_captured_multi_page_bytes() {
    let options = HeapOptions {
        heap_young_size_bytes: 0,
        heap_small_size_bytes: 32,
        page_size_bytes: 4096,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let bytes = vec![0xAA; 5 * 4096];
    let (mut test_heap, layout_ids) = test_heap_with_empty_layouts(options, &[bytes.len()]);
    let heap = &mut test_heap.heap;
    let reference = heap
        .allocate_payload(
            &heap_allocation_plan(&heap, layout_ids[0].block()),
            Payload::Bytes(&bytes),
        )
        .expect("heap block should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored =
        crate::Heap::from_image(&image, trace_table()).expect("heap image should restore");

    // mutating three pages should only change those bytes
    write_payload(&mut restored, reference, 0, &vec![0xCC; 3 * 4096]);
    let mutated_image = restored.image().expect("heap image should capture");
    let mut expected = bytes.clone();
    expected[..3 * 4096].fill(0xCC);

    assert_eq!(
        read_large_block_bytes(&mutated_image.storage().blocks()[0]),
        expected
    );
    assert_eq!(read_large_block_bytes(&image.storage().blocks()[0]), bytes);
}

/// Preserve image bytes when most restored pages are written.
#[test]
fn test_heap_image_write_preserves_captured_many_page_bytes() {
    let options = HeapOptions {
        heap_young_size_bytes: 0,
        heap_small_size_bytes: 32,
        page_size_bytes: 4096,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let bytes = vec![0xAA; 5 * 4096];
    let (mut test_heap, layout_ids) = test_heap_with_empty_layouts(options, &[bytes.len()]);
    let heap = &mut test_heap.heap;
    let reference = heap
        .allocate_payload(
            &heap_allocation_plan(&heap, layout_ids[0].block()),
            Payload::Bytes(&bytes),
        )
        .expect("heap block should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored =
        crate::Heap::from_image(&image, trace_table()).expect("heap image should restore");

    // mutating four pages should only change those bytes
    write_payload(&mut restored, reference, 0, &vec![0xCC; 4 * 4096]);
    let mutated_image = restored.image().expect("heap image should capture");
    let mut expected = bytes.clone();
    expected[..4 * 4096].fill(0xCC);

    assert_eq!(
        read_large_block_bytes(&mutated_image.storage().blocks()[0]),
        expected
    );
    assert_eq!(read_large_block_bytes(&image.storage().blocks()[0]), bytes);
}

/// Preserve heap small-space bytes across image roundtrips.
#[test]
fn test_roundtrip_heap_small_storage_image() {
    let options = HeapOptions {
        heap_young_size_bytes: 0,
        page_size_bytes: 4,
        allocator_chunk_size_bytes: TEST_ALLOCATOR_CHUNK_SIZE_BYTES,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let (mut heap, layout_ids) =
        heap_storage_with_empty_layouts(allocator.clone(), &options, &[3, 3]);

    // small blocks should roundtrip as independent bytes
    let first = heap
        .allocate(
            &heap_allocation_plan(&heap, layout_ids[0].block()),
            Payload::Bytes(&[1, 2, 3]),
        )
        .expect("heap block should succeed");
    let _second = heap
        .allocate(
            &heap_allocation_plan(&heap, layout_ids[1].block()),
            Payload::Bytes(&[4, 5, 6]),
        )
        .expect("heap block should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored = HeapStorage::from_image(allocator.clone(), &image, trace_table())
        .expect("heap image should restore");
    let restored_image = restored.image().expect("heap image should capture");

    assert_eq!(
        read_small_slot_bytes(
            &restored_image.spans()[0],
            restored_image.spans()[0].class.size_class,
            0,
            3,
        ),
        vec![1, 2, 3]
    );

    // mutating one small block should not affect the captured image
    restored
        .write_barrier(first, 1, 1, trace_table())
        .expect("heap write barrier should record");
    let address = restored.base_address() + first.offset() + 1;

    write_mapped_byte(address, 0xFE);

    let mutated_image = restored.image().expect("heap image should capture");

    assert_eq!(
        read_small_slot_bytes(
            &mutated_image.spans()[0],
            mutated_image.spans()[0].class.size_class,
            0,
            3,
        ),
        vec![1, 0xFE, 3]
    );
    assert_eq!(
        read_small_slot_bytes(&image.spans()[0], image.spans()[0].class.size_class, 0, 3,),
        vec![1, 2, 3]
    );
}

/// Preserve heap young storage bytes across image roundtrips.
#[test]
fn test_roundtrip_heap_young_storage_image() {
    let options = HeapOptions {
        page_size_bytes: 4,
        allocator_chunk_size_bytes: TEST_ALLOCATOR_CHUNK_SIZE_BYTES,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let (mut heap, layout_ids) =
        heap_storage_with_empty_layouts(allocator.clone(), &options, &[3, 3]);

    // young blocks should roundtrip as independent bytes
    let first = heap
        .allocate(
            &heap_allocation_plan(&heap, layout_ids[0].block()),
            Payload::Bytes(&[1, 2, 3]),
        )
        .expect("heap block should succeed");
    let _second = heap
        .allocate(
            &heap_allocation_plan(&heap, layout_ids[1].block()),
            Payload::Bytes(&[4, 5, 6]),
        )
        .expect("heap block should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored = HeapStorage::from_image(allocator.clone(), &image, trace_table())
        .expect("heap image should restore");
    let restored_image = restored.image().expect("heap image should capture");

    assert_eq!(
        read_young_slot_bytes(restored_image.young(), 0, 0),
        vec![1, 2, 3]
    );

    // mutating one young block should not affect the captured image
    restored
        .write_barrier(first, 1, 1, trace_table())
        .expect("heap write barrier should record");
    let address = restored.base_address() + first.offset() + 1;

    write_mapped_byte(address, 0xFE);

    let mutated_image = restored.image().expect("heap image should capture");

    assert_eq!(
        read_young_slot_bytes(mutated_image.young(), 0, 0),
        vec![1, 0xFE, 3]
    );
    assert_eq!(read_young_slot_bytes(image.young(), 0, 0), vec![1, 2, 3]);
}

/// Reject invalid heap image metadata during restore.
#[test]
fn test_restore_full_heap_image_rejects_invalid_size_class() {
    let options = HeapOptions {
        heap_young_size_bytes: 0,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let (mut heap, layout_ids) = heap_storage_with_empty_layouts(allocator.clone(), &options, &[3]);
    heap.allocate(
        &heap_allocation_plan(&heap, layout_ids[0].block()),
        Payload::Bytes(&[1, 2, 3]),
    )
    .expect("heap block should succeed");

    let image = heap.image().expect("heap image should capture");
    let image =
        image.with_size_classes(SizeClassTable::new([16]).expect("size classes should validate"));

    let error = HeapStorage::from_image(allocator, &image, trace_table())
        .expect_err("heap restore should fail loudly");

    assert_eq!(
        error,
        HeapError::configuration(HeapConfigurationError::InvalidSizeClass { class_bytes: 8 })
    );
}

/// Reject invalid heap metadata during fork.
#[test]
fn test_fork_heap_storage_rejects_invalid_size_class() {
    let options = HeapOptions {
        heap_young_size_bytes: 0,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let (mut heap, layout_ids) = heap_storage_with_empty_layouts(allocator.clone(), &options, &[3]);
    heap.allocate(
        &heap_allocation_plan(&heap, layout_ids[0].block()),
        Payload::Bytes(&[1, 2, 3]),
    )
    .expect("heap block should succeed");
    heap.small.size_classes = SizeClassTable::new([16]).expect("size classes should validate");

    let error = heap
        .fork(trace_table())
        .expect_err("heap fork should fail loudly");

    assert_eq!(
        error,
        HeapError::configuration(HeapConfigurationError::InvalidSizeClass { class_bytes: 8 })
    );
}
