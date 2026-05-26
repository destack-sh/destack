use std::sync::Arc;

use crate::allocator::{Allocator, PageRun};
use crate::local::raw::RawSpaceImage;
use crate::local::space::{HeapSpaceImage, YoungImage};
use crate::{
    HeapError, HeapImage, HeapOptions, HeapSpace, Payload, RawAllocationShape, RawSpace,
    SizeClassTable, TestLayout, test_allocator, test_layouts,
};
use destack_mir::TraceMap;

use super::{TestHeap, read_mapped_bytes, write_mapped_byte, write_mapped_bytes};

/// The allocator chunk size for small-page image fixtures.
const TEST_ALLOCATOR_CHUNK_BYTES: usize = 1024 * 1024;

/// Build one heap space with explicit empty managed layouts.
fn heap_space_with_empty_layouts(
    allocator: Arc<Allocator>,
    options: &HeapOptions,
    allocation_byte_lens: &[usize],
) -> (HeapSpace, Vec<TestLayout>) {
    let layout_specs = allocation_byte_lens
        .iter()
        .map(|byte_len| (*byte_len, TraceMap::empty()))
        .collect::<Vec<_>>();
    let layouts = test_layouts(&layout_specs);
    let heap = HeapSpace::with_options(allocator, options).expect("heap should build");

    (heap, layouts)
}

/// Build one full heap with explicit empty managed layouts.
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

/// Return the bytes from one physical page run.
fn read_page_run_bytes(
    allocator: &Allocator,
    page_run: &PageRun,
    start: usize,
    byte_len: usize,
) -> Vec<u8> {
    allocator
        .read_bytes_from(page_run, start, byte_len)
        .expect("image bytes should resolve")
}

/// Return the bytes for one large image allocation.
fn read_large_allocation_bytes(
    allocator: &Allocator,
    page_run: &PageRun,
    byte_len: usize,
) -> Vec<u8> {
    read_page_run_bytes(allocator, page_run, 0, byte_len)
}

/// Return the bytes for one small-span slot.
fn read_small_slot_bytes(
    allocator: &Allocator,
    page_run: &PageRun,
    size_class: usize,
    slot_index: usize,
    byte_len: usize,
) -> Vec<u8> {
    let start = size_class * slot_index;

    read_page_run_bytes(allocator, page_run, start, byte_len)
}

/// Write one managed payload range for image assertions.
fn write_payload(
    heap: &mut crate::Heap,
    reference: crate::HeapReference,
    start: usize,
    bytes: &[u8],
) {
    heap.write_barrier(reference, start, bytes.len())
        .expect("heap barrier should record");
    let address = heap.heap_base_address() + reference.offset() + start;

    write_mapped_bytes(address, bytes);
}

/// Return the bytes for one young-space range.
fn read_young_range_bytes(
    allocator: &Allocator,
    young: &YoungImage,
    range_index: usize,
) -> Vec<u8> {
    let allocation = &young.ranges()[range_index];
    let start = allocation.first_offset;

    read_page_run_bytes(allocator, young.pages(), start, allocation.byte_len)
}

/// Return the first live heap allocation bytes from one captured image.
fn read_first_heap_image_bytes(allocator: &Allocator, image: &HeapSpaceImage) -> Vec<u8> {
    // young allocations first
    for range_index in 0..image.young().ranges().len() {
        if image.young().live().contains(range_index) {
            return read_young_range_bytes(allocator, image.young(), range_index);
        }
    }

    // then small slots
    for span in image.spans() {
        for slot_index in 0..span.slot_count {
            if !span.occupied.contains(slot_index) {
                continue;
            }

            return read_small_slot_bytes(
                allocator,
                &span.pages,
                span.class.size_class,
                slot_index,
                span.class.size_class,
            );
        }
    }

    // then large allocations
    for allocation in image.allocations() {
        if !allocation.is_live {
            continue;
        }

        return read_large_allocation_bytes(allocator, &allocation.pages, allocation.len);
    }

    panic!("heap image should contain one live allocation")
}

/// Return the first live raw allocation bytes from one captured image.
fn read_first_raw_image_bytes(image: &RawSpaceImage) -> Vec<u8> {
    // small slots first
    for span in image.spans() {
        for slot_index in 0..span.slot_count {
            if !span.occupied.contains(slot_index) {
                continue;
            }

            let start = span.class.size_class * slot_index;
            let byte_len = span.class.byte_len;

            return span.bytes[start..start + byte_len].to_vec();
        }
    }

    // then large allocations
    for allocation in image.allocations() {
        if !allocation.is_live {
            continue;
        }

        return allocation.bytes[..allocation.len].to_vec();
    }

    panic!("raw image should contain one live allocation")
}

/// Preserve heap metadata and bytes across image roundtrips.
#[test]
fn test_roundtrip_heap_space_image() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        heap_small_bytes: 32,
        page_bytes: 4,
        allocator_chunk_bytes: TEST_ALLOCATOR_CHUNK_BYTES,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let first_bytes = vec![1; 5000];
    let second_bytes = vec![2; 5000];
    let (mut heap, layout_ids) = heap_space_with_empty_layouts(
        allocator.clone(),
        &options,
        &[first_bytes.len(), second_bytes.len()],
    );
    let first_layout = &layout_ids[0];
    let second_layout = &layout_ids[1];

    // capture two allocations so the restored copy has independent bytes
    let first = heap
        .allocate(
            &heap.allocation_plan(first_layout.allocation()),
            Payload::Bytes(&first_bytes),
        )
        .expect("heap allocation should succeed");
    let _second = heap
        .allocate(
            &heap.allocation_plan(second_layout.allocation()),
            Payload::Bytes(&second_bytes),
        )
        .expect("heap allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored =
        HeapSpace::from_image(allocator.clone(), &image).expect("heap image should restore");
    let restored_image = restored.image().expect("heap image should capture");

    // restored metadata should match the captured image
    assert!(Arc::ptr_eq(restored.allocator(), &allocator));
    assert_eq!(
        image.allocations().len(),
        restored_image.allocations().len()
    );
    assert_eq!(image.allocated_count(), restored_image.allocated_count());
    assert_eq!(image.allocated_bytes(), restored_image.allocated_bytes());

    // restored bytes should match the captured heap bytes
    assert_eq!(
        read_large_allocation_bytes(
            restored.allocator(),
            &restored_image.allocations()[0].pages,
            first_bytes.len(),
        ),
        first_bytes
    );
    let first_address = restored.base_address() + first.offset();

    let bytes = read_mapped_bytes(first_address, first_bytes.len());

    assert_eq!(bytes, first_bytes);
    assert_eq!(
        read_large_allocation_bytes(
            restored.allocator(),
            &restored_image.allocations()[1].pages,
            second_bytes.len(),
        ),
        second_bytes
    );

    // mutating one allocation should not affect the captured image
    restored
        .write_barrier(first, 0, 1)
        .expect("heap write barrier should record");
    let address = restored.base_address() + first.offset();

    write_mapped_byte(address, 0xFE);

    let mutated_image = restored.image().expect("heap image should capture");

    let mut expected_first = first_bytes.clone();
    expected_first[0] = 0xFE;

    assert_eq!(
        read_large_allocation_bytes(
            allocator.as_ref(),
            &image.allocations()[0].pages,
            first_bytes.len(),
        ),
        first_bytes
    );
    assert_eq!(
        read_large_allocation_bytes(
            restored.allocator(),
            &mutated_image.allocations()[0].pages,
            expected_first.len(),
        ),
        expected_first
    );
    let bytes = read_mapped_bytes(first_address, expected_first.len());

    assert_eq!(bytes, expected_first);
    assert_eq!(
        read_large_allocation_bytes(
            restored.allocator(),
            &mutated_image.allocations()[1].pages,
            second_bytes.len(),
        ),
        second_bytes
    );
}

/// Roundtrip raw images as independent byte payloads.
#[test]
fn test_roundtrip_raw_space_image() {
    let options = HeapOptions {
        page_bytes: 4,
        allocator_chunk_bytes: TEST_ALLOCATOR_CHUNK_BYTES,
        raw_small_bytes: 32,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let mut raw = RawSpace::with_options(allocator.clone(), &options)
        .expect("explicit raw options should build");

    // capture two allocations and mutate only one after restore
    let first_bytes = vec![1; 5000];
    let second_bytes = vec![2; 5000];
    let first = raw
        .allocate(
            RawAllocationShape::bytes(first_bytes.len()),
            Payload::Bytes(&first_bytes),
        )
        .expect("raw allocation should succeed");
    let _second = raw
        .allocate(
            RawAllocationShape::bytes(second_bytes.len()),
            Payload::Bytes(&second_bytes),
        )
        .expect("raw allocation should succeed");

    let image = raw.image().expect("raw image should capture");
    let mut restored =
        RawSpace::from_image(allocator.clone(), &image).expect("raw image should restore");

    // restored bytes should match without sharing image place
    assert!(Arc::ptr_eq(restored.allocator(), &allocator));
    assert_eq!(read_first_raw_image_bytes(&image), first_bytes);
    assert_eq!(restored.read_bytes(first), Ok(first_bytes.clone()));

    // mutating one allocation should not affect the captured image bytes
    restored
        .write_byte(first, 1, 0xFE)
        .expect("raw byte write should succeed");
    let mutated_image = restored.image().expect("raw image should capture");

    let mut expected_first = first_bytes;
    expected_first[1] = 0xFE;

    assert_eq!(read_first_raw_image_bytes(&image), vec![1; 5000]);
    assert_eq!(restored.read_bytes(first), Ok(expected_first.clone()));
    assert_eq!(read_first_raw_image_bytes(&mutated_image), expected_first);
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
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&heap_bytes),
        )
        .expect("heap allocation should succeed");
    let _raw = heap
        .allocate_raw(
            RawAllocationShape::bytes(4),
            Payload::Bytes(&[0xCA, 0xFE, 0xBA, 0xBE]),
        )
        .expect("raw allocation should succeed");

    // capture both the frozen image and the live fork
    let image = heap.image().expect("heap image should capture");
    let mut forked = heap.fork().expect("heap fork should retain live pages");
    let mut restored = crate::Heap::from_image(&image).expect("heap image should restore");
    let forked_image = forked.image().expect("heap image should capture");
    let restored_image = restored.image().expect("heap image should capture");

    assert_eq!(
        read_first_heap_image_bytes(forked.allocator(), forked_image.heap()),
        heap_bytes.to_vec()
    );
    assert_eq!(
        read_first_raw_image_bytes(forked_image.raw()),
        vec![0xCA, 0xFE, 0xBA, 0xBE]
    );
    assert_eq!(
        read_first_heap_image_bytes(restored.allocator(), restored_image.heap()),
        heap_bytes.to_vec()
    );
    assert_eq!(
        read_first_raw_image_bytes(restored_image.raw()),
        vec![0xCA, 0xFE, 0xBA, 0xBE]
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
    heap.allocate(
        &heap.allocation_plan(layout.allocation()),
        Payload::Bytes(&heap_bytes),
    )
    .expect("heap allocation should succeed");

    let image = heap.image().expect("heap image should capture");
    let snapshot = image.snapshot().expect("heap snapshot should capture");
    let mut restored = crate::Heap::from_snapshot(&snapshot).expect("heap snapshot should restore");
    let restored_image = restored.image().expect("heap image should capture");

    assert_eq!(
        read_first_heap_image_bytes(restored.allocator(), restored_image.heap()),
        heap_bytes.to_vec()
    );
}

/// Preserve image bytes when writing one restored heap allocation.
#[test]
fn test_heap_heap_write_preserves_captured_allocation_bytes() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        heap_small_bytes: 32,
        page_bytes: 4096,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let first_bytes = vec![0xAA; 5000];
    let second_bytes = vec![0xBB; 5000];
    let (mut test_heap, layout_ids) =
        test_heap_with_empty_layouts(options, &[first_bytes.len(), second_bytes.len()]);
    let heap = &mut test_heap.heap;
    let first = heap
        .allocate(
            &heap.allocation_plan(layout_ids[0].allocation()),
            Payload::Bytes(&first_bytes),
        )
        .expect("heap allocation should succeed");
    let _second = heap
        .allocate(
            &heap.allocation_plan(layout_ids[1].allocation()),
            Payload::Bytes(&second_bytes),
        )
        .expect("heap allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored = crate::Heap::from_image(&image).expect("heap image should restore");

    // mutating one allocation should only change the restored heap
    write_payload(&mut restored, first, 0, &[0xCC]);
    let mutated_image = restored.image().expect("heap image should capture");

    let mut expected_first = first_bytes;
    expected_first[0] = 0xCC;

    assert_eq!(
        read_large_allocation_bytes(
            restored.allocator(),
            &mutated_image.heap().allocations()[0].pages,
            expected_first.len(),
        ),
        expected_first
    );
    assert_eq!(
        read_large_allocation_bytes(
            restored.allocator(),
            &mutated_image.heap().allocations()[1].pages,
            second_bytes.len(),
        ),
        second_bytes
    );
    assert_eq!(
        read_large_allocation_bytes(
            heap.allocator(),
            &image.heap().allocations()[0].pages,
            expected_first.len(),
        ),
        vec![0xAA; 5000]
    );
}

/// Preserve image bytes when writing one restored heap page.
#[test]
fn test_heap_heap_write_preserves_captured_page_bytes() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        heap_small_bytes: 32,
        page_bytes: 4096,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let bytes = vec![0xAA; 9000];
    let (mut test_heap, layout_ids) = test_heap_with_empty_layouts(options, &[bytes.len()]);
    let heap = &mut test_heap.heap;
    let reference = heap
        .allocate(
            &heap.allocation_plan(layout_ids[0].allocation()),
            Payload::Bytes(&bytes),
        )
        .expect("heap allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored = crate::Heap::from_image(&image).expect("heap image should restore");

    // mutating one page should change only the written bytes
    write_payload(&mut restored, reference, 4096, &[0xCC]);
    let mutated_image = restored.image().expect("heap image should capture");
    let mut expected = bytes.clone();
    expected[4096] = 0xCC;

    assert_eq!(
        read_large_allocation_bytes(
            restored.allocator(),
            &mutated_image.heap().allocations()[0].pages,
            expected.len(),
        ),
        expected
    );
    assert_eq!(
        read_large_allocation_bytes(
            heap.allocator(),
            &image.heap().allocations()[0].pages,
            bytes.len(),
        ),
        bytes
    );
}

/// Preserve image bytes when writing several restored pages.
#[test]
fn test_heap_heap_write_preserves_captured_multi_page_bytes() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        heap_small_bytes: 32,
        page_bytes: 4096,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let bytes = vec![0xAA; 5 * 4096];
    let (mut test_heap, layout_ids) = test_heap_with_empty_layouts(options, &[bytes.len()]);
    let heap = &mut test_heap.heap;
    let reference = heap
        .allocate(
            &heap.allocation_plan(layout_ids[0].allocation()),
            Payload::Bytes(&bytes),
        )
        .expect("heap allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored = crate::Heap::from_image(&image).expect("heap image should restore");

    // mutating three pages should only change those bytes
    write_payload(&mut restored, reference, 0, &vec![0xCC; 3 * 4096]);
    let mutated_image = restored.image().expect("heap image should capture");
    let mut expected = bytes.clone();
    expected[..3 * 4096].fill(0xCC);

    assert_eq!(
        read_large_allocation_bytes(
            restored.allocator(),
            &mutated_image.heap().allocations()[0].pages,
            expected.len(),
        ),
        expected
    );
    assert_eq!(
        read_large_allocation_bytes(
            heap.allocator(),
            &image.heap().allocations()[0].pages,
            bytes.len(),
        ),
        bytes
    );
}

/// Preserve image bytes when most restored pages are written.
#[test]
fn test_heap_heap_write_preserves_captured_many_page_bytes() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        heap_small_bytes: 32,
        page_bytes: 4096,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let bytes = vec![0xAA; 5 * 4096];
    let (mut test_heap, layout_ids) = test_heap_with_empty_layouts(options, &[bytes.len()]);
    let heap = &mut test_heap.heap;
    let reference = heap
        .allocate(
            &heap.allocation_plan(layout_ids[0].allocation()),
            Payload::Bytes(&bytes),
        )
        .expect("heap allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored = crate::Heap::from_image(&image).expect("heap image should restore");

    // mutating four pages should only change those bytes
    write_payload(&mut restored, reference, 0, &vec![0xCC; 4 * 4096]);
    let mutated_image = restored.image().expect("heap image should capture");
    let mut expected = bytes.clone();
    expected[..4 * 4096].fill(0xCC);

    assert_eq!(
        read_large_allocation_bytes(
            restored.allocator(),
            &mutated_image.heap().allocations()[0].pages,
            expected.len(),
        ),
        expected
    );
    assert_eq!(
        read_large_allocation_bytes(
            heap.allocator(),
            &image.heap().allocations()[0].pages,
            bytes.len(),
        ),
        bytes
    );
}

/// Preserve heap small-space bytes across image roundtrips.
#[test]
fn test_roundtrip_heap_small_space_image() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        page_bytes: 4,
        allocator_chunk_bytes: TEST_ALLOCATOR_CHUNK_BYTES,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let (mut heap, layout_ids) =
        heap_space_with_empty_layouts(allocator.clone(), &options, &[3, 3]);

    // small allocations should roundtrip as independent bytes
    let first = heap
        .allocate(
            &heap.allocation_plan(layout_ids[0].allocation()),
            Payload::Bytes(&[1, 2, 3]),
        )
        .expect("heap allocation should succeed");
    let _second = heap
        .allocate(
            &heap.allocation_plan(layout_ids[1].allocation()),
            Payload::Bytes(&[4, 5, 6]),
        )
        .expect("heap allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored =
        HeapSpace::from_image(allocator.clone(), &image).expect("heap image should restore");
    let restored_image = restored.image().expect("heap image should capture");

    assert_eq!(
        read_small_slot_bytes(
            restored.allocator(),
            &restored_image.spans()[0].pages,
            restored_image.spans()[0].class.size_class,
            0,
            3,
        ),
        vec![1, 2, 3]
    );

    // mutating one small allocation should not affect the captured image
    restored
        .write_barrier(first, 1, 1)
        .expect("heap write barrier should record");
    let address = restored.base_address() + first.offset() + 1;

    write_mapped_byte(address, 0xFE);

    let mutated_image = restored.image().expect("heap image should capture");

    assert_eq!(
        read_small_slot_bytes(
            restored.allocator(),
            &mutated_image.spans()[0].pages,
            mutated_image.spans()[0].class.size_class,
            0,
            3,
        ),
        vec![1, 0xFE, 3]
    );
    assert_eq!(
        read_small_slot_bytes(
            allocator.as_ref(),
            &image.spans()[0].pages,
            image.spans()[0].class.size_class,
            0,
            3,
        ),
        vec![1, 2, 3]
    );
}

/// Preserve heap young-space bytes across image roundtrips.
#[test]
fn test_roundtrip_heap_young_space_image() {
    let options = HeapOptions {
        page_bytes: 4,
        allocator_chunk_bytes: TEST_ALLOCATOR_CHUNK_BYTES,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let (mut heap, layout_ids) =
        heap_space_with_empty_layouts(allocator.clone(), &options, &[3, 3]);

    // young allocations should roundtrip as independent bytes
    let first = heap
        .allocate(
            &heap.allocation_plan(layout_ids[0].allocation()),
            Payload::Bytes(&[1, 2, 3]),
        )
        .expect("heap allocation should succeed");
    let _second = heap
        .allocate(
            &heap.allocation_plan(layout_ids[1].allocation()),
            Payload::Bytes(&[4, 5, 6]),
        )
        .expect("heap allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored =
        HeapSpace::from_image(allocator.clone(), &image).expect("heap image should restore");
    let restored_image = restored.image().expect("heap image should capture");

    assert_eq!(
        read_young_range_bytes(restored.allocator(), restored_image.young(), 0),
        vec![1, 2, 3]
    );

    // mutating one young allocation should not affect the captured image
    restored
        .write_barrier(first, 1, 1)
        .expect("heap write barrier should record");
    let address = restored.base_address() + first.offset() + 1;

    write_mapped_byte(address, 0xFE);

    let mutated_image = restored.image().expect("heap image should capture");

    assert_eq!(
        read_young_range_bytes(restored.allocator(), mutated_image.young(), 0),
        vec![1, 0xFE, 3]
    );
    assert_eq!(
        read_young_range_bytes(allocator.as_ref(), image.young(), 0),
        vec![1, 2, 3]
    );
}

/// Roundtrip raw small-space images as independent byte payloads.
#[test]
fn test_roundtrip_raw_small_space_image() {
    let options = HeapOptions {
        page_bytes: 4,
        allocator_chunk_bytes: TEST_ALLOCATOR_CHUNK_BYTES,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let mut raw = RawSpace::with_options(allocator.clone(), &options)
        .expect("explicit raw options should build");

    // small allocations should stay in spans and share those span pages after restore
    let first = raw
        .allocate(RawAllocationShape::new(3, 16), Payload::Bytes(&[1, 2, 3]))
        .expect("raw allocation should succeed");
    let _second = raw
        .allocate(RawAllocationShape::bytes(3), Payload::Bytes(&[4, 5, 6]))
        .expect("raw allocation should succeed");
    let image = raw.image().expect("raw image should capture");
    let mut restored =
        RawSpace::from_image(allocator.clone(), &image).expect("raw image should restore");
    let restored_image = restored.image().expect("raw image should capture");

    assert_eq!(read_first_raw_image_bytes(&image), vec![1, 2, 3]);
    assert_eq!(read_first_raw_image_bytes(&restored_image), vec![1, 2, 3]);
    assert_eq!(restored.read_bytes(first), Ok(vec![1, 2, 3]));
    assert_eq!(first.offset() % 16, 0);

    // mutating one small allocation should not affect the captured image bytes
    restored
        .write_byte(first, 1, 0xFE)
        .expect("raw byte write should succeed");
    let mutated_image = restored.image().expect("raw image should capture");

    assert_eq!(read_first_raw_image_bytes(&image), vec![1, 2, 3]);
    assert_eq!(read_first_raw_image_bytes(&mutated_image), vec![1, 0xFE, 3]);
}

/// Roundtrip one raw span image with one size class above `u16::MAX`.
#[test]
fn test_roundtrip_raw_small_space_image_with_large_size_class() {
    let options = HeapOptions {
        size_classes: SizeClassTable::new([70_000]).expect("size classes should validate"),
        raw_small_bytes: 70_000,
        heap_small_bytes: 70_000,
        heap_young_bytes: 0,
        max_heap_young_allocation_bytes: 0,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let mut raw = RawSpace::with_options(allocator.clone(), &options)
        .expect("explicit raw options should build");
    let bytes = vec![0xAB; 70_000];

    // one custom large size class should stay in small space without truncation
    let _pointer = raw
        .allocate(
            RawAllocationShape::bytes(bytes.len()),
            Payload::Bytes(&bytes),
        )
        .expect("raw allocation should succeed");
    let image = raw.image().expect("raw image should capture");
    let mut restored = RawSpace::from_image(allocator, &image).expect("raw image should restore");
    let restored_image = restored.image().expect("raw image should capture");

    assert_eq!(image.spans()[0].class.byte_len, bytes.len());
    assert_eq!(read_first_raw_image_bytes(&restored_image), bytes);
}

/// Reject invalid heap image metadata during restore.
#[test]
fn test_restore_full_heap_image_rejects_invalid_size_class() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let (mut heap, layout_ids) = heap_space_with_empty_layouts(allocator.clone(), &options, &[3]);
    heap.allocate(
        &heap.allocation_plan(layout_ids[0].allocation()),
        Payload::Bytes(&[1, 2, 3]),
    )
    .expect("heap allocation should succeed");

    let image = heap.image().expect("heap image should capture");
    let image =
        image.with_size_classes(SizeClassTable::new([16]).expect("size classes should validate"));

    let error =
        HeapSpace::from_image(allocator, &image).expect_err("heap restore should fail loudly");

    assert_eq!(error, HeapError::InvalidSizeClass { class_bytes: 8 });
}

/// Reject invalid heap metadata during fork.
#[test]
fn test_fork_heap_space_rejects_invalid_size_class() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let (mut heap, layout_ids) = heap_space_with_empty_layouts(allocator.clone(), &options, &[3]);
    heap.allocate(
        &heap.allocation_plan(layout_ids[0].allocation()),
        Payload::Bytes(&[1, 2, 3]),
    )
    .expect("heap allocation should succeed");
    heap.small.size_classes = SizeClassTable::new([16]).expect("size classes should validate");

    let error = heap.fork().expect_err("heap fork should fail loudly");

    assert_eq!(error, HeapError::InvalidSizeClass { class_bytes: 8 });
}

/// Reject invalid raw image metadata during restore.
#[test]
fn test_restore_raw_image_rejects_invalid_size_class() {
    let options = HeapOptions::local();
    let allocator = test_allocator(&options);
    let mut raw = RawSpace::with_options(allocator.clone(), &options)
        .expect("explicit raw options should build");
    raw.allocate(RawAllocationShape::bytes(3), Payload::Bytes(&[1, 2, 3]))
        .expect("raw allocation should succeed");

    let image = raw.image().expect("raw image should capture");
    let image =
        image.with_size_classes(SizeClassTable::new([16]).expect("size classes should validate"));

    let error =
        RawSpace::from_image(allocator, &image).expect_err("raw restore should fail loudly");

    assert_eq!(error, HeapError::InvalidSizeClass { class_bytes: 8 });
}

/// Reject invalid raw metadata during fork.
#[test]
fn test_fork_raw_space_rejects_invalid_size_class() {
    let options = HeapOptions::local();
    let allocator = test_allocator(&options);
    let mut raw = RawSpace::with_options(allocator.clone(), &options)
        .expect("explicit raw options should build");
    raw.allocate(RawAllocationShape::bytes(3), Payload::Bytes(&[1, 2, 3]))
        .expect("raw allocation should succeed");
    raw.small.size_classes =
        crate::SizeClassTable::new([16]).expect("size classes should validate");

    let error = raw.fork().expect_err("raw fork should fail loudly");

    assert_eq!(error, HeapError::InvalidSizeClass { class_bytes: 8 });
}

/// Reject invalid full-heap image metadata in raw space.
#[test]
fn test_restore_heap_image_rejects_invalid_raw_size_class() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        ..HeapOptions::local()
    };
    let (mut test_heap, layout_ids) = test_heap_with_empty_layouts(options, &[3]);
    let heap = &mut test_heap.heap;
    heap.allocate(
        &heap.allocation_plan(layout_ids[0].allocation()),
        Payload::Bytes(&[1, 2, 3]),
    )
    .expect("heap allocation should succeed");
    heap.allocate_raw(RawAllocationShape::bytes(3), Payload::Bytes(&[4, 5, 6]))
        .expect("raw allocation should succeed");

    let original_image = heap.image().expect("heap image should capture");
    let image = HeapImage::new(
        original_image.allocator().clone(),
        original_image.options().clone(),
        original_image.heap().clone(),
        original_image
            .raw()
            .clone()
            .with_size_classes(SizeClassTable::new([16]).expect("size classes should validate")),
    )
    .expect("heap image should build");
    drop(original_image);

    let error = crate::Heap::from_image(&image).expect_err("heap restore should fail loudly");

    assert_eq!(error, HeapError::InvalidSizeClass { class_bytes: 8 });
}

/// Reject invalid full-heap metadata during fork.
#[test]
fn test_fork_heap_rejects_invalid_raw_size_class() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        ..HeapOptions::local()
    };
    let (mut test_heap, layout_ids) = test_heap_with_empty_layouts(options, &[3]);
    let heap = &mut test_heap.heap;
    heap.allocate(
        &heap.allocation_plan(layout_ids[0].allocation()),
        Payload::Bytes(&[1, 2, 3]),
    )
    .expect("heap allocation should succeed");
    heap.allocate_raw(RawAllocationShape::bytes(3), Payload::Bytes(&[4, 5, 6]))
        .expect("raw allocation should succeed");
    heap.raw.small.size_classes = SizeClassTable::new([16]).expect("size classes should validate");

    let error = heap.fork().expect_err("heap fork should fail loudly");

    assert_eq!(error, HeapError::InvalidSizeClass { class_bytes: 8 });
}
