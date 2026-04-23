use std::sync::Arc;

use destack_mir::LayoutId;

use crate::allocator::{Allocator, PageView};
use crate::local::raw::RawSpaceImage;
use crate::local::space::{HeapSpaceImage, YoungImage};
use crate::{
    HeapError, HeapImage, HeapOptions, HeapSpace, RawSpace, SizeClassTable, test_allocator,
    test_layouts,
};
use destack_mir::ReferenceMap;

use super::TestHeap;

/// Build one heap space with explicit empty managed layouts.
fn heap_space_with_empty_layouts(
    allocator: Arc<Allocator>,
    options: &HeapOptions,
    byte_lens: &[usize],
) -> (HeapSpace, Vec<LayoutId>) {
    let layout_specs = byte_lens
        .iter()
        .map(|byte_len| (*byte_len, ReferenceMap::empty()))
        .collect::<Vec<_>>();
    let (layouts, layout_ids) = test_layouts(&layout_specs);
    let heap = HeapSpace::with_layouts_and_options(allocator, layouts, options)
        .expect("heap should build");

    (heap, layout_ids)
}

/// Build one full heap with explicit empty managed layouts.
fn test_heap_with_empty_layouts(
    options: HeapOptions,
    byte_lens: &[usize],
) -> (TestHeap, Vec<LayoutId>) {
    let layout_specs = byte_lens
        .iter()
        .map(|byte_len| (*byte_len, ReferenceMap::empty()))
        .collect::<Vec<_>>();
    let (layouts, layout_ids) = test_layouts(&layout_specs);
    let heap =
        TestHeap::with_limits_and_layout_table(crate::HeapLimits::default(), options, layouts);

    (heap, layout_ids)
}

/// Return the visible bytes from one logical page view.
fn read_page_view_bytes(
    allocator: &Allocator,
    page_view: &PageView,
    start: usize,
    byte_len: usize,
) -> Vec<u8> {
    allocator
        .bytes_to_vec_from(page_view, start, byte_len)
        .expect("image bytes should resolve")
}

/// Return the bytes for one large image entry.
fn read_large_entry_bytes(allocator: &Allocator, page_view: &PageView, byte_len: usize) -> Vec<u8> {
    read_page_view_bytes(allocator, page_view, 0, byte_len)
}

/// Return the bytes for one small-span slot.
fn read_small_slot_bytes(
    allocator: &Allocator,
    page_view: &PageView,
    size_class: usize,
    slot_index: usize,
    byte_len: usize,
) -> Vec<u8> {
    let start = size_class * slot_index;

    read_page_view_bytes(allocator, page_view, start, byte_len)
}

/// Return the bytes for one young-space entry.
fn read_young_entry_bytes(
    allocator: &Allocator,
    young: &YoungImage,
    entry_index: usize,
) -> Vec<u8> {
    let entry = &young.entries()[entry_index];
    let start = young.page_bytes() * entry.first_page as usize + entry.first_offset as usize;

    read_page_view_bytes(allocator, young.pages(), start, entry.byte_len)
}

/// Return the first live heap allocation bytes from one captured image.
fn read_first_heap_image_bytes(allocator: &Allocator, image: &HeapSpaceImage) -> Vec<u8> {
    // young entries first
    for entry_index in 0..image.young().entries().len() {
        let entry = &image.young().entries()[entry_index];

        if entry.is_live {
            return read_young_entry_bytes(allocator, image.young(), entry_index);
        }
    }

    // then small slots
    for span in image.spans() {
        for slot_index in 0..span.slot_count {
            if !span.occupied.contains(slot_index) {
                continue;
            }

            let byte_len = span.byte_lens[slot_index];

            return read_small_slot_bytes(
                allocator,
                &span.pages,
                span.class.size_class,
                slot_index,
                byte_len,
            );
        }
    }

    // then large entries
    for entry in image.entries() {
        if !entry.is_live {
            continue;
        }

        return read_large_entry_bytes(allocator, &entry.pages, entry.len);
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

    // then large entries
    for entry in image.entries() {
        if !entry.is_live {
            continue;
        }

        return entry.bytes[..entry.len].to_vec();
    }

    panic!("raw image should contain one live allocation")
}

/// Return the first restored large raw pointer.
fn first_restored_large_raw_pointer(raw: &RawSpace) -> crate::RawPointer {
    raw.base_pointer(crate::local::raw::RawStorage::Large(
        crate::local::raw::LargeEntryId::new(1),
    ))
    .expect("restored raw entry should have one base pointer")
}

/// Return the first restored small raw pointer.
fn first_restored_small_raw_pointer(raw: &RawSpace) -> crate::RawPointer {
    let slot = crate::allocator::SpanSlot::new(0, 0).expect("first restored raw slot should exist");

    raw.base_pointer(crate::local::raw::RawStorage::Small(slot))
        .expect("restored raw slot should have one base pointer")
}

/// Preserve heap metadata across image roundtrips and detach only touched entries.
#[test]
fn test_roundtrip_heap_space_image() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        heap_small_bytes: 32,
        page_bytes: 4,
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
    let first_layout_id = layout_ids[0];
    let second_layout_id = layout_ids[1];

    // capture two entries so the restored copy has independent raw bytes
    let first = heap
        .allocate_bytes(&first_bytes, first_layout_id)
        .expect("heap allocation should succeed");
    let _second = heap
        .allocate_bytes(&second_bytes, second_layout_id)
        .expect("heap allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored =
        HeapSpace::from_image(allocator.clone(), &image).expect("heap image should restore");
    let restored_image = restored.image().expect("heap image should capture");

    // restored metadata should match and untouched pages should still share
    assert!(Arc::ptr_eq(restored.allocator(), &allocator));
    assert_eq!(image.entries()[0].pages, restored_image.entries()[0].pages);
    assert_eq!(image.entries()[1].pages, restored_image.entries()[1].pages);

    // mutating one allocation should detach only that allocation
    restored
        .write_byte(first, 0, 0xFE)
        .expect("heap byte write should succeed");
    let mutated_image = restored.image().expect("heap image should capture");

    assert_ne!(image.entries()[0].pages, mutated_image.entries()[0].pages);
    assert_eq!(image.entries()[1].pages, mutated_image.entries()[1].pages);
}

/// Roundtrip raw images as independent byte payloads.
#[test]
fn test_roundtrip_raw_space_image() {
    let options = HeapOptions {
        page_bytes: 4,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let mut raw = RawSpace::with_options(allocator.clone(), &options)
        .expect("explicit raw options should build");

    // capture two entries so only one has to detach later
    let first_bytes = vec![1; 5000];
    let second_bytes = vec![2; 5000];
    let first = raw
        .allocate_bytes(&first_bytes)
        .expect("raw allocation should succeed");
    let _second = raw
        .allocate_bytes(&second_bytes)
        .expect("raw allocation should succeed");

    let image = raw.image();
    let mut restored =
        RawSpace::from_image(allocator.clone(), &image).expect("raw image should restore");

    let restored_first = first_restored_large_raw_pointer(&restored);

    // restored bytes should match without sharing image storage
    assert!(Arc::ptr_eq(restored.allocator(), &allocator));
    assert_eq!(read_first_raw_image_bytes(&image), first_bytes);
    assert_eq!(
        restored.read_bytes(first),
        Err(HeapError::InvalidRawPointer { pointer: first })
    );
    assert_eq!(restored.read_bytes(restored_first), Ok(first_bytes.clone()));

    // mutating one allocation should not affect the captured image bytes
    restored
        .set_byte(restored_first, 1, 0xFE)
        .expect("raw byte write should succeed");
    let mutated_image = restored.image();

    let mut expected_first = first_bytes;
    expected_first[1] = 0xFE;

    assert_eq!(read_first_raw_image_bytes(&image), vec![1; 5000]);
    assert_eq!(
        restored.read_bytes(first),
        Err(HeapError::InvalidRawPointer { pointer: first })
    );
    assert_eq!(
        restored.read_bytes(restored_first),
        Ok(expected_first.clone())
    );
    assert_eq!(read_first_raw_image_bytes(&mutated_image), expected_first);
}

/// Preserve heap images and forks across the full heap root.
#[test]
fn test_roundtrip_heap_image_and_fork() {
    let heap_bytes = 7i64.to_le_bytes();
    let (mut test_heap, layout_ids) =
        test_heap_with_empty_layouts(HeapOptions::local(), &[heap_bytes.len()]);
    let layout_id = layout_ids[0];
    let heap = &mut test_heap.heap;
    let _heap_reference = heap
        .allocate_heap_bytes(&heap_bytes, layout_id)
        .expect("heap allocation should succeed");
    let _raw = heap
        .allocate_raw_bytes(&[0xCA, 0xFE, 0xBA, 0xBE])
        .expect("raw allocation should succeed");

    // capture both the frozen root and the live fork
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

/// Detach one heap allocation after restoring a shared heap image.
#[test]
fn test_heap_heap_write_detaches_only_touched_allocation() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        heap_small_bytes: 32,
        ..HeapOptions::local()
    };
    let first_bytes = vec![0xAA; 5000];
    let second_bytes = vec![0xBB; 5000];
    let (mut test_heap, layout_ids) =
        test_heap_with_empty_layouts(options, &[first_bytes.len(), second_bytes.len()]);
    let heap = &mut test_heap.heap;
    let first = heap
        .allocate_heap_bytes(&first_bytes, layout_ids[0])
        .expect("heap allocation should succeed");
    let _second = heap
        .allocate_heap_bytes(&second_bytes, layout_ids[1])
        .expect("heap allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored = crate::Heap::from_image(&image).expect("heap image should restore");

    // mutating one allocation should detach only that allocation
    restored
        .write_heap_bytes(first, 0, &[0xCC])
        .expect("heap bytes should update");
    let mutated_image = restored.image().expect("heap image should capture");

    assert_ne!(
        image.heap().entries()[0].pages,
        mutated_image.heap().entries()[0].pages
    );
    assert_eq!(
        image.heap().entries()[1].pages,
        mutated_image.heap().entries()[1].pages
    );
    let mut expected_first = first_bytes;
    expected_first[0] = 0xCC;

    assert_eq!(
        read_large_entry_bytes(
            restored.allocator(),
            &mutated_image.heap().entries()[0].pages,
            expected_first.len(),
        ),
        expected_first
    );
    assert_eq!(
        read_large_entry_bytes(
            restored.allocator(),
            &mutated_image.heap().entries()[1].pages,
            second_bytes.len(),
        ),
        second_bytes
    );
}

/// Detach only the touched page inside one shared heap allocation.
#[test]
fn test_heap_heap_write_detaches_only_touched_page() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        heap_small_bytes: 32,
        ..HeapOptions::local()
    };
    let bytes = vec![0xAA; 9000];
    let (mut test_heap, layout_ids) = test_heap_with_empty_layouts(options, &[bytes.len()]);
    let heap = &mut test_heap.heap;
    let reference = heap
        .allocate_heap_bytes(&bytes, layout_ids[0])
        .expect("heap allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored = crate::Heap::from_image(&image).expect("heap image should restore");

    // mutating one page should leave the untouched pages shared
    restored
        .write_heap_bytes(reference, 4096, &[0xCC])
        .expect("heap bytes should update");
    let mutated_image = restored.image().expect("heap image should capture");
    let original_pages = &image.heap().entries()[0].pages;
    let mutated_pages = &mutated_image.heap().entries()[0].pages;

    assert_eq!(original_pages.page(0), mutated_pages.page(0));
    assert_ne!(original_pages.page(1), mutated_pages.page(1));
    assert_eq!(original_pages.page(2), mutated_pages.page(2));
}

/// Keep untouched pages shared when the write touches a sparse set of pages.
#[test]
fn test_heap_heap_write_keeps_sparse_page_sharing() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        heap_small_bytes: 32,
        page_bytes: 4096,
        ..HeapOptions::local()
    };
    let bytes = vec![0xAA; 5 * 4096];
    let (mut test_heap, layout_ids) = test_heap_with_empty_layouts(options, &[bytes.len()]);
    let heap = &mut test_heap.heap;
    let reference = heap
        .allocate_heap_bytes(&bytes, layout_ids[0])
        .expect("heap allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored = crate::Heap::from_image(&image).expect("heap image should restore");

    // mutating three pages should only detach those pages
    restored
        .write_heap_bytes(reference, 0, &vec![0xCC; 3 * 4096])
        .expect("heap bytes should update");
    let mutated_image = restored.image().expect("heap image should capture");
    let original_pages = &image.heap().entries()[0].pages;
    let mutated_pages = &mutated_image.heap().entries()[0].pages;

    assert_ne!(original_pages.page(0), mutated_pages.page(0));
    assert_ne!(original_pages.page(1), mutated_pages.page(1));
    assert_ne!(original_pages.page(2), mutated_pages.page(2));
    assert_eq!(original_pages.page(3), mutated_pages.page(3));
    assert_eq!(original_pages.page(4), mutated_pages.page(4));
}

/// Keep untouched pages shared even when many pages are detached.
#[test]
fn test_heap_heap_write_keeps_sparse_page_sharing_across_many_patches() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        heap_small_bytes: 32,
        page_bytes: 4096,
        ..HeapOptions::local()
    };
    let bytes = vec![0xAA; 5 * 4096];
    let (mut test_heap, layout_ids) = test_heap_with_empty_layouts(options, &[bytes.len()]);
    let heap = &mut test_heap.heap;
    let reference = heap
        .allocate_heap_bytes(&bytes, layout_ids[0])
        .expect("heap allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored = crate::Heap::from_image(&image).expect("heap image should restore");

    // mutating four pages should still only detach those pages
    restored
        .write_heap_bytes(reference, 0, &vec![0xCC; 4 * 4096])
        .expect("heap bytes should update");
    let mutated_image = restored.image().expect("heap image should capture");
    let original_pages = &image.heap().entries()[0].pages;
    let mutated_pages = &mutated_image.heap().entries()[0].pages;

    assert_ne!(original_pages.page(0), mutated_pages.page(0));
    assert_ne!(original_pages.page(1), mutated_pages.page(1));
    assert_ne!(original_pages.page(2), mutated_pages.page(2));
    assert_ne!(original_pages.page(3), mutated_pages.page(3));
    assert_eq!(original_pages.page(4), mutated_pages.page(4));
}

/// Preserve allocator sharing for heap small-space spans across image roundtrips.
#[test]
fn test_roundtrip_heap_small_space_image() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        page_bytes: 4,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let (mut heap, layout_ids) =
        heap_space_with_empty_layouts(allocator.clone(), &options, &[3, 3]);

    // small entries should roundtrip as independent raw bytes
    let first = heap
        .allocate_bytes(&[1, 2, 3], layout_ids[0])
        .expect("heap allocation should succeed");
    let _second = heap
        .allocate_bytes(&[4, 5, 6], layout_ids[1])
        .expect("heap allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored =
        HeapSpace::from_image(allocator.clone(), &image).expect("heap image should restore");
    let restored_image = restored.image().expect("heap image should capture");

    assert_eq!(image.spans()[0].pages, restored_image.spans()[0].pages);

    // mutating one small allocation should detach the touched span
    restored
        .write_byte(first, 1, 0xFE)
        .expect("heap byte write should succeed");
    let mutated_image = restored.image().expect("heap image should capture");

    assert_ne!(image.spans()[0].pages, mutated_image.spans()[0].pages);
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
}

/// Preserve allocator sharing for heap young-space entries across image roundtrips.
#[test]
fn test_roundtrip_heap_young_space_image() {
    let options = HeapOptions {
        page_bytes: 4,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let (mut heap, layout_ids) =
        heap_space_with_empty_layouts(allocator.clone(), &options, &[3, 3]);

    // young entries should share young-space pages after restore
    let first = heap
        .allocate_bytes(&[1, 2, 3], layout_ids[0])
        .expect("heap allocation should succeed");
    let _second = heap
        .allocate_bytes(&[4, 5, 6], layout_ids[1])
        .expect("heap allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored =
        HeapSpace::from_image(allocator.clone(), &image).expect("heap image should restore");
    let restored_image = restored.image().expect("heap image should capture");

    assert_eq!(image.young().pages(), restored_image.young().pages());

    // mutating one young entry should detach the young-space pages
    restored
        .write_byte(first, 1, 0xFE)
        .expect("heap byte write should succeed");
    let mutated_image = restored.image().expect("heap image should capture");

    assert_ne!(image.young().pages(), mutated_image.young().pages());
    assert_eq!(
        read_young_entry_bytes(restored.allocator(), mutated_image.young(), 0),
        vec![1, 0xFE, 3]
    );
}

/// Roundtrip raw small-space images as independent byte payloads.
#[test]
fn test_roundtrip_raw_small_space_image() {
    let options = HeapOptions {
        page_bytes: 4,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let mut raw = RawSpace::with_options(allocator.clone(), &options)
        .expect("explicit raw options should build");

    // small entries should stay in spans and share those span pages after restore
    let first = raw
        .allocate_bytes(&[1, 2, 3])
        .expect("raw allocation should succeed");
    let _second = raw
        .allocate_bytes(&[4, 5, 6])
        .expect("raw allocation should succeed");
    let image = raw.image();
    let mut restored =
        RawSpace::from_image(allocator.clone(), &image).expect("raw image should restore");
    let restored_image = restored.image();
    let restored_first = first_restored_small_raw_pointer(&restored);

    assert_eq!(read_first_raw_image_bytes(&image), vec![1, 2, 3]);
    assert_eq!(read_first_raw_image_bytes(&restored_image), vec![1, 2, 3]);
    assert_eq!(
        restored.read_bytes(first),
        Err(HeapError::InvalidRawPointer { pointer: first })
    );
    assert_eq!(restored.read_bytes(restored_first), Ok(vec![1, 2, 3]));

    // mutating one small allocation should not affect the captured image bytes
    restored
        .set_byte(restored_first, 1, 0xFE)
        .expect("raw byte write should succeed");
    let mutated_image = restored.image();

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
        .allocate_bytes(&bytes)
        .expect("raw allocation should succeed");
    let image = raw.image();
    let mut restored = RawSpace::from_image(allocator, &image).expect("raw image should restore");
    let restored_image = restored.image();

    assert_eq!(image.spans()[0].class.byte_len, bytes.len());
    assert_eq!(read_first_raw_image_bytes(&restored_image), bytes);
}

/// Release retained heap pages when image restore fails after retention.
#[test]
fn test_restore_full_heap_image_releases_retained_pages_on_failure() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let (mut heap, layout_ids) = heap_space_with_empty_layouts(allocator.clone(), &options, &[3]);
    heap.allocate_bytes(&[1, 2, 3], layout_ids[0])
        .expect("heap allocation should succeed");

    let image = heap.image().expect("heap image should capture");
    let image =
        image.with_size_classes(SizeClassTable::new([8]).expect("size classes should validate"));

    // failed restore should not leave shared retains behind
    assert_eq!(heap.borrowed_bytes(), 0);

    let error =
        HeapSpace::from_image(allocator, &image).expect_err("heap restore should fail loudly");

    assert_eq!(error, HeapError::InvalidSizeClass { class_bytes: 16 });
    assert_eq!(heap.borrowed_bytes(), 0);
}

/// Release retained heap pages when fork fails after retention.
#[test]
fn test_fork_heap_space_releases_retained_pages_on_failure() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let (mut heap, layout_ids) = heap_space_with_empty_layouts(allocator.clone(), &options, &[3]);
    heap.allocate_bytes(&[1, 2, 3], layout_ids[0])
        .expect("heap allocation should succeed");
    heap.small.size_classes = SizeClassTable::new([8]).expect("size classes should validate");

    // failed fork should not leave shared retains behind
    assert_eq!(heap.borrowed_bytes(), 0);

    let error = heap.fork().expect_err("heap fork should fail loudly");

    assert_eq!(error, HeapError::InvalidSizeClass { class_bytes: 16 });
    assert_eq!(heap.borrowed_bytes(), 0);
}

/// Release retained raw pages when image restore fails after retention.
#[test]
fn test_restore_raw_image_releases_retained_pages_on_failure() {
    let options = HeapOptions::local();
    let allocator = test_allocator(&options);
    let mut raw = RawSpace::with_options(allocator.clone(), &options)
        .expect("explicit raw options should build");
    raw.allocate_bytes(&[1, 2, 3])
        .expect("raw allocation should succeed");

    let image = raw.image();
    let image =
        image.with_size_classes(SizeClassTable::new([8]).expect("size classes should validate"));

    // failed restore should not leave shared retains behind
    assert_eq!(raw.borrowed_bytes(), 0);

    let error =
        RawSpace::from_image(allocator, &image).expect_err("raw restore should fail loudly");

    assert_eq!(error, HeapError::InvalidSizeClass { class_bytes: 16 });
    assert_eq!(raw.borrowed_bytes(), 0);
}

/// Release retained raw pages when fork fails after retention.
#[test]
fn test_fork_raw_space_releases_retained_pages_on_failure() {
    let options = HeapOptions::local();
    let allocator = test_allocator(&options);
    let mut raw = RawSpace::with_options(allocator.clone(), &options)
        .expect("explicit raw options should build");
    raw.allocate_bytes(&[1, 2, 3])
        .expect("raw allocation should succeed");
    raw.small.size_classes = crate::SizeClassTable::new([8]).expect("size classes should validate");

    // failed fork should not leave shared retains behind
    assert_eq!(raw.borrowed_bytes(), 0);

    let error = raw.fork().expect_err("raw fork should fail loudly");

    assert_eq!(error, HeapError::InvalidSizeClass { class_bytes: 16 });
    assert_eq!(raw.borrowed_bytes(), 0);
}

/// Release heap retains when full-heap image restore fails in raw space.
#[test]
fn test_restore_heap_image_releases_retained_pages_on_failure() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        ..HeapOptions::local()
    };
    let (mut test_heap, layout_ids) = test_heap_with_empty_layouts(options, &[3]);
    let heap = &mut test_heap.heap;
    heap.allocate_heap_bytes(&[1, 2, 3], layout_ids[0])
        .expect("heap allocation should succeed");
    heap.allocate_raw_bytes(&[4, 5, 6])
        .expect("raw allocation should succeed");

    let image = heap.image().expect("heap image should capture");
    let image = HeapImage::new(
        image.allocator().clone(),
        image.options().clone(),
        image.heap().clone(),
        image
            .raw()
            .clone()
            .with_size_classes(SizeClassTable::new([8]).expect("size classes should validate")),
    );

    // failed restore should not leave shared retains behind
    assert_eq!(heap.usage().borrowed_bytes(), 0);

    let error = crate::Heap::from_image(&image).expect_err("heap restore should fail loudly");

    assert_eq!(error, HeapError::InvalidSizeClass { class_bytes: 16 });
    assert_eq!(heap.usage().borrowed_bytes(), 0);
}

/// Release heap retains when full-heap fork fails in raw space.
#[test]
fn test_fork_heap_releases_retained_pages_on_failure() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        ..HeapOptions::local()
    };
    let (mut test_heap, layout_ids) = test_heap_with_empty_layouts(options, &[3]);
    let heap = &mut test_heap.heap;
    heap.allocate_heap_bytes(&[1, 2, 3], layout_ids[0])
        .expect("heap allocation should succeed");
    heap.allocate_raw_bytes(&[4, 5, 6])
        .expect("raw allocation should succeed");
    heap.raw.small.size_classes = SizeClassTable::new([8]).expect("size classes should validate");

    // failed fork should not leave shared retains behind
    assert_eq!(heap.usage().borrowed_bytes(), 0);

    let error = heap.fork().expect_err("heap fork should fail loudly");

    assert_eq!(error, HeapError::InvalidSizeClass { class_bytes: 16 });
    assert_eq!(heap.usage().borrowed_bytes(), 0);
}
