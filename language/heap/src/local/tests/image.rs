use std::sync::Arc;

use crate::tests::test_arena;
use crate::{HeapError, HeapImage, HeapOptions, ManagedSpace, RawSpace, SizeClassTable};
use destack_mir::LayoutTrace;

use super::TestHeap;

/// Preserve managed metadata across image roundtrips and detach only touched entries.
#[test]
fn test_roundtrip_managed_space_image() {
    let layout = HeapOptions {
        managed_young_bytes: 0,
        managed_small_bytes: 32,
        page_bytes: 4,
        ..HeapOptions::local()
    };
    let arena = test_arena(&layout);
    let mut managed = ManagedSpace::with_options(arena.clone(), &layout)
        .expect("explicit managed layout should build");

    // capture two entries so only one has to detach later
    let first_bytes = vec![1; 5000];
    let second_bytes = vec![2; 5000];
    let first = managed
        .allocate_bytes(&first_bytes, LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");
    let _second = managed
        .allocate_bytes(&second_bytes, LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");
    managed
        .set_layout_id(first, crate::LayoutId::new(41))
        .expect("managed storage layout id should update");
    let image = managed.image().expect("managed image should capture");
    let mut restored = ManagedSpace::from_image(arena.clone(), &image)
        .expect("managed image layout should restore");
    let restored_image = restored.image().expect("managed image should capture");

    // restored metadata should match and untouched pages should still share
    assert_eq!(
        restored.layout_id(first),
        Ok(Some(crate::LayoutId::new(41)))
    );
    assert!(Arc::ptr_eq(restored.arena(), &arena));
    assert_eq!(image.entries()[0].pages, restored_image.entries()[0].pages);
    assert_eq!(image.entries()[1].pages, restored_image.entries()[1].pages);

    // mutating one allocation should detach only that allocation
    restored
        .write_byte(first, 0, 0xFE)
        .expect("managed byte write should succeed");
    let mutated_image = restored.image().expect("managed image should capture");

    assert_ne!(image.entries()[0].pages, mutated_image.entries()[0].pages);
    assert_eq!(image.entries()[1].pages, mutated_image.entries()[1].pages);
}

/// Share unchanged raw entries across image roundtrips and detach only touched entries.
#[test]
fn test_roundtrip_raw_space_image() {
    let layout = HeapOptions {
        page_bytes: 4,
        ..HeapOptions::local()
    };
    let arena = test_arena(&layout);
    let mut raw =
        RawSpace::with_options(arena.clone(), &layout).expect("explicit raw layout should build");

    // capture two entries so only one has to detach later
    let first_bytes = vec![1; 5000];
    let second_bytes = vec![2; 5000];
    let first = raw
        .allocate_bytes(&first_bytes)
        .expect("raw allocation should succeed");
    let second = raw
        .allocate_bytes(&second_bytes)
        .expect("raw allocation should succeed");

    let image = raw.image();
    let mut restored =
        RawSpace::from_image(arena.clone(), &image).expect("raw image should restore");
    let restored_image = restored.image();

    // untouched pages should still share after restore
    assert!(Arc::ptr_eq(restored.arena(), &arena));
    assert_eq!(image.entries()[0].pages, restored_image.entries()[0].pages);
    assert_eq!(image.entries()[1].pages, restored_image.entries()[1].pages);

    // mutating one allocation should detach only that allocation
    restored
        .set_byte(first, 1, 0xFE)
        .expect("managed byte write should succeed");
    let mutated_image = restored.image();

    assert_ne!(image.entries()[0].pages, mutated_image.entries()[0].pages);
    assert_eq!(image.entries()[1].pages, mutated_image.entries()[1].pages);
    let mut expected_first = first_bytes;
    expected_first[1] = 0xFE;

    assert_eq!(restored.read_bytes(first), Ok(expected_first));
    assert_eq!(restored.read_bytes(second), Ok(second_bytes));
}

/// Preserve heap images and forks across the full heap root.
#[test]
fn test_roundtrip_heap_image_and_fork() {
    let mut test_heap = TestHeap::new();
    let heap = &mut test_heap.heap;
    let managed_bytes = 7i64.to_le_bytes();
    let managed = heap
        .allocate_managed_bytes(&managed_bytes, LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");
    let raw = heap
        .allocate_raw_bytes(&[0xCA, 0xFE, 0xBA, 0xBE])
        .expect("raw allocation should succeed");

    // capture both the frozen root and the live fork
    let image = heap.image().expect("heap image should capture");
    let mut forked = heap.fork().expect("heap fork should retain live pages");
    let restored = crate::Heap::from_image(&image).expect("heap image layout should restore");

    assert_eq!(
        forked
            .image()
            .expect("heap image should capture")
            .local_allocated_bytes()
            .expect("heap image bytes should stay exact"),
        image
            .local_allocated_bytes()
            .expect("heap image bytes should stay exact")
    );
    assert_eq!(
        forked
            .image()
            .expect("heap image should capture")
            .page_count(),
        image.page_count()
    );
    assert_eq!(
        restored.read_managed_bytes(managed),
        Ok(managed_bytes.to_vec())
    );
    assert_eq!(
        restored.read_raw_bytes(raw),
        Ok(vec![0xCA, 0xFE, 0xBA, 0xBE])
    );
}

/// Detach one managed allocation after restoring a shared heap image.
#[test]
fn test_heap_managed_write_detaches_only_touched_allocation() {
    let mut test_heap = TestHeap::with_options(HeapOptions {
        managed_young_bytes: 0,
        managed_small_bytes: 32,
        ..HeapOptions::local()
    });
    let heap = &mut test_heap.heap;
    let first_bytes = vec![0xAA; 5000];
    let second_bytes = vec![0xBB; 5000];
    let first = heap
        .allocate_managed_bytes(&first_bytes, LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");
    let second = heap
        .allocate_managed_bytes(&second_bytes, LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored = crate::Heap::from_image(&image).expect("heap image layout should restore");

    // mutating one allocation should detach only that allocation
    restored
        .write_managed_bytes(first, 0, &[0xCC])
        .expect("managed bytes should update");
    let mutated_image = restored.image().expect("heap image should capture");

    assert_ne!(
        image.managed().entries()[0].pages,
        mutated_image.managed().entries()[0].pages
    );
    assert_eq!(
        image.managed().entries()[1].pages,
        mutated_image.managed().entries()[1].pages
    );
    let mut expected_first = first_bytes;
    expected_first[0] = 0xCC;

    assert_eq!(restored.read_managed_bytes(first), Ok(expected_first));
    assert_eq!(restored.read_managed_bytes(second), Ok(second_bytes));
}

/// Detach only the touched page inside one shared managed allocation.
#[test]
fn test_heap_managed_write_detaches_only_touched_page() {
    let mut test_heap = TestHeap::with_options(HeapOptions {
        managed_young_bytes: 0,
        managed_small_bytes: 32,
        ..HeapOptions::local()
    });
    let heap = &mut test_heap.heap;
    let bytes = vec![0xAA; 9000];
    let reference = heap
        .allocate_managed_bytes(&bytes, LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored = crate::Heap::from_image(&image).expect("heap image layout should restore");

    // mutating one page should leave the untouched pages shared
    restored
        .write_managed_bytes(reference, 4096, &[0xCC])
        .expect("managed bytes should update");
    let mutated_image = restored.image().expect("heap image should capture");
    let original_pages = &image.managed().entries()[0].pages;
    let mutated_pages = &mutated_image.managed().entries()[0].pages;

    assert_eq!(original_pages.page(0), mutated_pages.page(0));
    assert_ne!(original_pages.page(1), mutated_pages.page(1));
    assert_eq!(original_pages.page(2), mutated_pages.page(2));
}

/// Keep untouched pages shared when the write fits inline patch room.
#[test]
fn test_heap_managed_write_keeps_sharing_within_patch_room() {
    let mut test_heap = TestHeap::with_options(HeapOptions {
        managed_young_bytes: 0,
        managed_small_bytes: 32,
        page_bytes: 4096,
        ..HeapOptions::local()
    });
    let heap = &mut test_heap.heap;
    let bytes = vec![0xAA; 5 * 4096];
    let reference = heap
        .allocate_managed_bytes(&bytes, LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored = crate::Heap::from_image(&image).expect("heap image layout should restore");

    // mutating three pages should stay within inline patch room
    restored
        .write_managed_bytes(reference, 0, &vec![0xCC; 3 * 4096])
        .expect("managed bytes should update");
    let mutated_image = restored.image().expect("heap image should capture");
    let original_pages = &image.managed().entries()[0].pages;
    let mutated_pages = &mutated_image.managed().entries()[0].pages;

    assert_ne!(original_pages.page(0), mutated_pages.page(0));
    assert_ne!(original_pages.page(1), mutated_pages.page(1));
    assert_ne!(original_pages.page(2), mutated_pages.page(2));
    assert_eq!(original_pages.page(3), mutated_pages.page(3));
    assert_eq!(original_pages.page(4), mutated_pages.page(4));
}

/// Rebase the whole view when the write would overflow inline patch room.
#[test]
fn test_heap_managed_write_rebases_when_patch_room_overflows() {
    let mut test_heap = TestHeap::with_options(HeapOptions {
        managed_young_bytes: 0,
        managed_small_bytes: 32,
        page_bytes: 4096,
        ..HeapOptions::local()
    });
    let heap = &mut test_heap.heap;
    let bytes = vec![0xAA; 5 * 4096];
    let reference = heap
        .allocate_managed_bytes(&bytes, LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored = crate::Heap::from_image(&image).expect("heap image layout should restore");

    // mutating four pages should force one full rebase
    restored
        .write_managed_bytes(reference, 0, &vec![0xCC; 4 * 4096])
        .expect("managed bytes should update");
    let mutated_image = restored.image().expect("heap image should capture");
    let original_pages = &image.managed().entries()[0].pages;
    let mutated_pages = &mutated_image.managed().entries()[0].pages;

    assert_ne!(original_pages.page(0), mutated_pages.page(0));
    assert_ne!(original_pages.page(1), mutated_pages.page(1));
    assert_ne!(original_pages.page(2), mutated_pages.page(2));
    assert_ne!(original_pages.page(3), mutated_pages.page(3));
    assert_ne!(original_pages.page(4), mutated_pages.page(4));
}

/// Preserve arena sharing for managed small-space spans across image roundtrips.
#[test]
fn test_roundtrip_managed_small_space_image() {
    let layout = HeapOptions {
        managed_young_bytes: 0,
        page_bytes: 4,
        ..HeapOptions::local()
    };
    let arena = test_arena(&layout);
    let mut managed = ManagedSpace::with_options(arena.clone(), &layout)
        .expect("explicit managed layout should build");

    // small entries should stay in spans and share those span pages after restore
    let first = managed
        .allocate_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");
    let _second = managed
        .allocate_bytes(&[4, 5, 6], LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");
    let image = managed.image().expect("managed image should capture");
    let mut restored = ManagedSpace::from_image(arena.clone(), &image)
        .expect("managed image layout should restore");
    let restored_image = restored.image().expect("managed image should capture");

    assert_eq!(image.spans()[0].pages, restored_image.spans()[0].pages);

    // mutating one small allocation should detach the touched span
    restored
        .write_byte(first, 1, 0xFE)
        .expect("managed byte write should succeed");
    let mutated_image = restored.image().expect("managed image should capture");

    assert_ne!(image.spans()[0].pages, mutated_image.spans()[0].pages);
    assert_eq!(restored.read_bytes(first), Ok(vec![1, 0xFE, 3]));
}

/// Preserve arena sharing for managed young-space entries across image roundtrips.
#[test]
fn test_roundtrip_managed_young_space_image() {
    let layout = HeapOptions {
        page_bytes: 4,
        ..HeapOptions::local()
    };
    let arena = test_arena(&layout);
    let mut managed = ManagedSpace::with_options(arena.clone(), &layout)
        .expect("explicit managed layout should build");

    // young entries should share young-space pages after restore
    let first = managed
        .allocate_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");
    let _second = managed
        .allocate_bytes(&[4, 5, 6], LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");
    let image = managed.image().expect("managed image should capture");
    let mut restored = ManagedSpace::from_image(arena.clone(), &image)
        .expect("managed image layout should restore");
    let restored_image = restored.image().expect("managed image should capture");

    assert_eq!(image.young().pages(), restored_image.young().pages());

    // mutating one young entry should detach the young-space pages
    restored
        .write_byte(first, 1, 0xFE)
        .expect("managed byte write should succeed");
    let mutated_image = restored.image().expect("managed image should capture");

    assert_ne!(image.young().pages(), mutated_image.young().pages());
    assert_eq!(restored.read_bytes(first), Ok(vec![1, 0xFE, 3]));
}

/// Preserve arena sharing for raw small-space spans across image roundtrips.
#[test]
fn test_roundtrip_raw_small_space_image() {
    let layout = HeapOptions {
        page_bytes: 4,
        ..HeapOptions::local()
    };
    let arena = test_arena(&layout);
    let mut raw =
        RawSpace::with_options(arena.clone(), &layout).expect("explicit raw layout should build");

    // small entries should stay in spans and share those span pages after restore
    let first = raw
        .allocate_bytes(&[1, 2, 3])
        .expect("raw allocation should succeed");
    let _second = raw
        .allocate_bytes(&[4, 5, 6])
        .expect("raw allocation should succeed");
    let image = raw.image();
    let mut restored =
        RawSpace::from_image(arena.clone(), &image).expect("raw image should restore");
    let restored_image = restored.image();

    assert_eq!(image.spans()[0].pages, restored_image.spans()[0].pages);

    // mutating one small allocation should detach the touched span
    restored
        .set_byte(first, 1, 0xFE)
        .expect("raw byte write should succeed");
    let mutated_image = restored.image();

    assert_ne!(image.spans()[0].pages, mutated_image.spans()[0].pages);
    assert_eq!(restored.read_bytes(first), Ok(vec![1, 0xFE, 3]));
}

/// Roundtrip one raw span image with one size class above `u16::MAX`.
#[test]
fn test_roundtrip_raw_small_space_image_with_large_size_class() {
    let layout = HeapOptions {
        size_classes: SizeClassTable::new([70_000]).expect("size classes should validate"),
        raw_small_bytes: 70_000,
        managed_small_bytes: 70_000,
        managed_young_bytes: 0,
        max_managed_young_allocation_bytes: 0,
        ..HeapOptions::local()
    };
    let arena = test_arena(&layout);
    let mut raw =
        RawSpace::with_options(arena.clone(), &layout).expect("explicit raw layout should build");
    let bytes = vec![0xAB; 70_000];

    // one custom large size class should stay in small space without truncation
    let pointer = raw
        .allocate_bytes(&bytes)
        .expect("raw allocation should succeed");
    let image = raw.image();
    let restored = RawSpace::from_image(arena, &image).expect("raw image should restore");

    assert_eq!(image.spans()[0].lengths[0], bytes.len());
    assert_eq!(restored.read_bytes(pointer), Ok(bytes));
}

/// Release retained managed pages when image restore fails after retention.
#[test]
fn test_restore_managed_image_releases_retained_pages_on_failure() {
    let layout = HeapOptions {
        managed_young_bytes: 0,
        ..HeapOptions::local()
    };
    let arena = test_arena(&layout);
    let mut managed = ManagedSpace::with_options(arena.clone(), &layout)
        .expect("explicit managed layout should build");
    managed
        .allocate_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");

    let image = managed.image().expect("managed image should capture");
    let image =
        image.with_size_classes(SizeClassTable::new([8]).expect("size classes should validate"));

    // failed restore should not leave shared retains behind
    assert_eq!(managed.borrowed_bytes(), Ok(0));

    let error =
        ManagedSpace::from_image(arena, &image).expect_err("managed restore should fail loudly");

    assert_eq!(error, HeapError::InvalidSizeClass { class_bytes: 16 });
    assert_eq!(managed.borrowed_bytes(), Ok(0));
}

/// Release retained managed pages when fork fails after retention.
#[test]
fn test_fork_managed_space_releases_retained_pages_on_failure() {
    let layout = HeapOptions {
        managed_young_bytes: 0,
        ..HeapOptions::local()
    };
    let arena = test_arena(&layout);
    let mut managed = ManagedSpace::with_options(arena.clone(), &layout)
        .expect("explicit managed layout should build");
    managed
        .allocate_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");
    managed.small.size_classes = SizeClassTable::new([8]).expect("size classes should validate");

    // failed fork should not leave shared retains behind
    assert_eq!(managed.borrowed_bytes(), Ok(0));

    let error = managed.fork().expect_err("managed fork should fail loudly");

    assert_eq!(error, HeapError::InvalidSizeClass { class_bytes: 16 });
    assert_eq!(managed.borrowed_bytes(), Ok(0));
}

/// Release retained raw pages when image restore fails after retention.
#[test]
fn test_restore_raw_image_releases_retained_pages_on_failure() {
    let layout = HeapOptions::local();
    let arena = test_arena(&layout);
    let mut raw =
        RawSpace::with_options(arena.clone(), &layout).expect("explicit raw layout should build");
    raw.allocate_bytes(&[1, 2, 3])
        .expect("raw allocation should succeed");

    let image = raw.image();
    let image =
        image.with_size_classes(SizeClassTable::new([8]).expect("size classes should validate"));

    // failed restore should not leave shared retains behind
    assert_eq!(raw.borrowed_bytes(), Ok(0));

    let error = RawSpace::from_image(arena, &image).expect_err("raw restore should fail loudly");

    assert_eq!(error, HeapError::InvalidSizeClass { class_bytes: 16 });
    assert_eq!(raw.borrowed_bytes(), Ok(0));
}

/// Release retained raw pages when fork fails after retention.
#[test]
fn test_fork_raw_space_releases_retained_pages_on_failure() {
    let layout = HeapOptions::local();
    let arena = test_arena(&layout);
    let mut raw =
        RawSpace::with_options(arena.clone(), &layout).expect("explicit raw layout should build");
    raw.allocate_bytes(&[1, 2, 3])
        .expect("raw allocation should succeed");
    raw.small.size_classes = crate::SizeClassTable::new([8]).expect("size classes should validate");

    // failed fork should not leave shared retains behind
    assert_eq!(raw.borrowed_bytes(), Ok(0));

    let error = raw.fork().expect_err("raw fork should fail loudly");

    assert_eq!(error, HeapError::InvalidSizeClass { class_bytes: 16 });
    assert_eq!(raw.borrowed_bytes(), Ok(0));
}

/// Release managed retains when full-heap image restore fails in raw space.
#[test]
fn test_restore_heap_image_releases_retained_pages_on_failure() {
    let mut test_heap = TestHeap::with_options(HeapOptions {
        managed_young_bytes: 0,
        ..HeapOptions::local()
    });
    let heap = &mut test_heap.heap;
    heap.allocate_managed_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");
    heap.allocate_raw_bytes(&[4, 5, 6])
        .expect("raw allocation should succeed");

    let image = heap.image().expect("heap image should capture");
    let image = HeapImage::new(
        image.arena().clone(),
        image.options().clone(),
        image.managed().clone(),
        image
            .raw()
            .clone()
            .with_size_classes(SizeClassTable::new([8]).expect("size classes should validate")),
    );

    // failed restore should not leave shared retains behind
    assert_eq!(
        heap.usage()
            .expect("heap usage should resolve")
            .borrowed_bytes(),
        Ok(0)
    );

    let error = crate::Heap::from_image(&image).expect_err("heap restore should fail loudly");

    assert_eq!(error, HeapError::InvalidSizeClass { class_bytes: 16 });
    assert_eq!(
        heap.usage()
            .expect("heap usage should resolve")
            .borrowed_bytes(),
        Ok(0)
    );
}

/// Release managed retains when full-heap fork fails in raw space.
#[test]
fn test_fork_heap_releases_retained_pages_on_failure() {
    let mut test_heap = TestHeap::with_options(HeapOptions {
        managed_young_bytes: 0,
        ..HeapOptions::local()
    });
    let heap = &mut test_heap.heap;
    heap.allocate_managed_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("managed allocation should succeed");
    heap.allocate_raw_bytes(&[4, 5, 6])
        .expect("raw allocation should succeed");
    heap.raw.small.size_classes = SizeClassTable::new([8]).expect("size classes should validate");

    // failed fork should not leave shared retains behind
    assert_eq!(
        heap.usage()
            .expect("heap usage should resolve")
            .borrowed_bytes(),
        Ok(0)
    );

    let error = heap.fork().expect_err("heap fork should fail loudly");

    assert_eq!(error, HeapError::InvalidSizeClass { class_bytes: 16 });
    assert_eq!(
        heap.usage()
            .expect("heap usage should resolve")
            .borrowed_bytes(),
        Ok(0)
    );
}
