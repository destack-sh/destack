use std::sync::Arc;

use crate::managed::ManagedLocation;
use crate::{
    Arena, Heap, HeapImage, HeapLayout, HeapLimits, LayoutId, ManagedReference, ManagedSpace,
    RawPointer, RawSpace, ReferenceMap, SharedSpace, Value,
};

/// Create one shared arena for one explicit heap layout.
fn arena(layout: &HeapLayout) -> Arc<Arena> {
    Arc::new(Arena::with_page_bytes(layout.page_bytes))
}

/// Trace compact managed references at fixed byte offsets.
#[test]
fn test_reference_offsets_trace_reference32_payloads() {
    let first = ManagedReference::new(0x0102_0304);
    let second = ManagedReference::new(0x1122_3344);
    let bytes = [
        0xAA, 0xBB, 0xCC, 0xDD, 0x04, 0x03, 0x02, 0x01, 0x44, 0x33, 0x22, 0x11,
    ];
    let map = ReferenceMap::ReferenceOffsets {
        offsets: vec![4, 8].into_boxed_slice(),
    };
    let mut traced = Vec::new();

    // follow compact references without assuming u64 payload words
    map.trace_references(&bytes, 4, |reference| traced.push(reference))
        .expect("reference tracing should succeed");

    assert_eq!(traced, vec![first, second]);
}

/// Trace compact managed references across repeated elements.
#[test]
fn test_repeated_reference_offsets_trace_reference32_payloads() {
    let first = ManagedReference::new(0x0102_0304);
    let second = ManagedReference::new(0x1122_3344);
    let bytes = [
        0x10, 0x20, 0x30, 0x40, 0x04, 0x03, 0x02, 0x01, 0x50, 0x60, 0x70, 0x80, 0x44, 0x33, 0x22,
        0x11,
    ];
    let map = ReferenceMap::RepeatedReferenceOffsets {
        count: 2,
        element_size: 8,
        offsets: vec![4].into_boxed_slice(),
    };
    let mut traced = Vec::new();

    // follow compact references using the declared repeated stride
    map.trace_references(&bytes, 4, |reference| traced.push(reference))
        .expect("reference tracing should succeed");

    assert_eq!(traced, vec![first, second]);
}

/// Reject unsupported traced managed-reference widths loudly.
#[test]
fn test_reference_offsets_reject_unsupported_tracing_width() {
    let bytes = [0xAA, 0xBB, 0xCC, 0xDD];
    let map = ReferenceMap::ReferenceOffsets {
        offsets: vec![0].into_boxed_slice(),
    };

    let error = map
        .trace_references(&bytes, 3, |_| {})
        .expect_err("unsupported tracing widths should fail");

    assert_eq!(
        error.to_string(),
        "unsupported managed reference width for tracing: 3"
    );
}

/// Reject unsupported managed-reference widths at heap construction.
#[test]
fn test_heap_rejects_invalid_managed_reference_width() {
    let layout = HeapLayout {
        managed_reference_bytes: 3,
        ..HeapLayout::default()
    };

    let error = Heap::with_limits_and_layout(HeapLimits::default(), layout)
        .expect_err("invalid heap layout should fail loudly");

    assert_eq!(
        error.to_string(),
        "unsupported managed reference width for heap layout: 3"
    );
}

/// Reject unsupported managed-reference widths when restoring a heap snapshot.
#[test]
fn test_heap_snapshot_rejects_invalid_managed_reference_width() {
    let mut heap = Heap::new();
    let mut snapshot = heap.snapshot().expect("heap snapshot should capture");

    // invalid managed reference widths must still fail at restore time
    snapshot.managed.managed_reference_bytes = 3;

    let error = Heap::from_snapshot(&snapshot)
        .expect_err("invalid heap snapshot layout should fail loudly");

    assert_eq!(
        error.to_string(),
        "unsupported managed reference width for heap layout: 3"
    );
}

/// Preserve stable managed ids across allocation growth and id reuse.
#[test]
fn test_allocate_managed_ids_reuse_after_free() {
    let layout = HeapLayout::default();
    let mut managed = ManagedSpace::with_layout(arena(&layout), &layout)
        .expect("default managed layout should build");
    let mut last = ManagedReference::NULL;

    // grow the reference table beyond one short run
    for index in 0..12 {
        last = managed.allocate_bytes(&[index as u8], ReferenceMap::empty(), None);
    }

    assert_eq!(last.id(), 12);

    // freed ids should be reused directly
    let reused = ManagedReference::new(7);
    assert!(managed.free(reused));

    let reference = managed.allocate_bytes(&[0xAB], ReferenceMap::empty(), None);

    assert_eq!(reference.id(), reused.id());
}

/// Preserve stable raw ids across allocation growth and id reuse.
#[test]
fn test_allocate_raw_ids_reuse_after_free() {
    let layout = HeapLayout::default();
    let mut raw = RawSpace::with_layout(arena(&layout), &layout);
    let mut last = RawPointer::NULL;

    // grow the pointer table beyond one short run
    for index in 0..12 {
        last = raw.allocate_bytes(&[index as u8]);
    }

    assert_eq!(last.id(), 12);

    // freed ids should be reused directly
    let reused = RawPointer::new(7);
    assert!(raw.free(reused));

    let pointer = raw.allocate_bytes(&[0xCD]);

    assert_eq!(pointer.id(), reused.id());
}

/// Preserve managed metadata across image roundtrips and detach only touched allocations.
#[test]
fn test_roundtrip_managed_space_image() {
    let layout = HeapLayout {
        managed_young_bytes: 0,
        managed_small_bytes: 32,
        page_bytes: 4,
        ..HeapLayout::default()
    };
    let arena = arena(&layout);
    let mut managed = ManagedSpace::with_layout(arena.clone(), &layout)
        .expect("explicit managed layout should build");

    // capture two allocations so only one has to detach later
    let first_bytes = vec![1; 5000];
    let second_bytes = vec![2; 5000];
    let first = managed.allocate_bytes(&first_bytes, ReferenceMap::empty(), None);
    let _second = managed.allocate_bytes(&second_bytes, ReferenceMap::empty(), None);
    assert!(managed.set_layout_id(first, LayoutId::new(41)));
    assert!(managed.set_type_id(first, 42));

    let image = managed.image().expect("managed image should capture");
    let mut restored = ManagedSpace::from_image(arena.clone(), &image)
        .expect("managed image layout should restore");
    let restored_image = restored.image().expect("managed image should capture");

    // restored metadata should match and untouched pages should still share
    assert_eq!(restored.layout_id(first), Some(LayoutId::new(41)));
    assert_eq!(restored.type_id(first), Some(42));
    assert!(Arc::ptr_eq(restored.arena(), &arena));
    assert_eq!(
        image.allocations()[0].pages,
        restored_image.allocations()[0].pages
    );
    assert_eq!(
        image.allocations()[1].pages,
        restored_image.allocations()[1].pages
    );

    // mutating one allocation should detach only that allocation
    assert!(restored.set_byte(first, 0, 0xFE));
    let mutated_image = restored.image().expect("managed image should capture");

    assert_ne!(
        image.allocations()[0].pages,
        mutated_image.allocations()[0].pages
    );
    assert_eq!(
        image.allocations()[1].pages,
        mutated_image.allocations()[1].pages
    );
}

/// Clear stale nominal type ids when one managed id is freed and reused.
#[test]
fn test_free_reused_managed_id_clears_type_id() {
    let layout = HeapLayout::default();
    let mut managed = ManagedSpace::with_layout(arena(&layout), &layout)
        .expect("default managed layout should build");
    let original = managed.allocate_bytes(&[0xAA], ReferenceMap::empty(), None);

    // assign one nominal type id before freeing
    assert!(managed.set_type_id(original, 42));
    assert_eq!(managed.type_id(original), Some(42));

    assert!(managed.free(original));
    assert_eq!(managed.type_id(original), None);

    let reused = managed.allocate_bytes(&[0xBB], ReferenceMap::empty(), None);

    assert_eq!(reused.id(), original.id());
    assert_eq!(managed.type_id(reused), None);
}

/// Share unchanged raw allocations across image roundtrips and detach only touched allocations.
#[test]
fn test_roundtrip_raw_space_image() {
    let layout = HeapLayout {
        page_bytes: 4,
        ..HeapLayout::default()
    };
    let arena = arena(&layout);
    let mut raw = RawSpace::with_layout(arena.clone(), &layout);

    // capture two allocations so only one has to detach later
    let first_bytes = vec![1; 5000];
    let second_bytes = vec![2; 5000];
    let first = raw.allocate_bytes(&first_bytes);
    let second = raw.allocate_bytes(&second_bytes);

    let image = raw.image();
    let mut restored = RawSpace::from_image(arena.clone(), &image);
    let restored_image = restored.image();

    // untouched pages should still share after restore
    assert!(Arc::ptr_eq(restored.arena(), &arena));
    assert_eq!(
        image.allocations()[0].pages,
        restored_image.allocations()[0].pages
    );
    assert_eq!(
        image.allocations()[1].pages,
        restored_image.allocations()[1].pages
    );

    // mutating one allocation should detach only that allocation
    assert!(restored.set_byte(first, 1, 0xFE));
    let mutated_image = restored.image();

    assert_ne!(
        image.allocations()[0].pages,
        mutated_image.allocations()[0].pages
    );
    assert_eq!(
        image.allocations()[1].pages,
        mutated_image.allocations()[1].pages
    );
    let mut expected_first = first_bytes;
    expected_first[1] = 0xFE;

    assert_eq!(restored.bytes(first), Some(expected_first));
    assert_eq!(restored.bytes(second), Some(second_bytes));
}

/// Share unchanged shared allocations across image roundtrips and detach only touched regions.
#[test]
fn test_roundtrip_shared_space_image() {
    let mut shared = SharedSpace::with_page_bytes(4);

    // capture two regions so only one has to detach later
    let first = shared.allocate_bytes(&[1, 2, 3, 4, 5, 6]);
    let second = shared.allocate_bytes(&[7, 8, 9, 10, 11, 12]);
    let image = shared.image();
    let mut restored = SharedSpace::from_image_with_arena(shared.arena.clone(), &image);
    let restored_image = restored.image();

    // untouched pages should still share after restore
    assert_eq!(
        image.region(0).unwrap().pages,
        restored_image.region(0).unwrap().pages
    );
    assert_eq!(
        image.region(1).unwrap().pages,
        restored_image.region(1).unwrap().pages
    );

    // mutating one region should detach only that region
    assert!(restored.replace_bytes(first, &[9, 2, 3, 4, 5, 6]));
    let mutated_image = restored.image();

    assert_ne!(
        image.region(0).unwrap().pages,
        mutated_image.region(0).unwrap().pages
    );
    assert_eq!(
        image.region(1).unwrap().pages,
        mutated_image.region(1).unwrap().pages
    );
    assert_eq!(restored.bytes_to_vec(first), Some(vec![9, 2, 3, 4, 5, 6]));
    assert_eq!(
        restored.bytes_to_vec(second),
        Some(vec![7, 8, 9, 10, 11, 12])
    );
}

/// Preserve heap images and snapshots across the full heap root.
#[test]
fn test_roundtrip_heap_image_and_snapshot() {
    let mut heap = Heap::default();
    let managed_bytes = Value::int64(7).to_byte_array();
    let managed = heap
        .allocate_managed_bytes(&managed_bytes, ReferenceMap::empty(), None)
        .expect("managed allocation should succeed");
    let raw = heap
        .allocate_raw_bytes(&[0xCA, 0xFE, 0xBA, 0xBE])
        .expect("raw allocation should succeed");

    // capture both the in-memory and serialized roots
    let image = heap.image().expect("heap image should capture");
    let snapshot = heap.snapshot().expect("heap snapshot should capture");
    let decoded = HeapImage::from_snapshot(&snapshot);
    let restored = Heap::from_snapshot(&snapshot).expect("heap snapshot layout should restore");

    assert_eq!(
        decoded.local_allocated_bytes(),
        image.local_allocated_bytes()
    );
    assert_eq!(decoded.page_count(), image.page_count());
    assert_eq!(
        restored.managed_bytes(managed).as_deref(),
        Some(managed_bytes.as_slice())
    );
    assert_eq!(
        restored.raw_bytes(raw).as_deref(),
        Some(&[0xCA, 0xFE, 0xBA, 0xBE][..])
    );
}

/// Reject one managed allocation when the active-byte limit would be exceeded.
#[test]
fn test_reject_managed_allocation_when_limit_exceeded() {
    let mut heap = Heap::new();
    let baseline = heap.usage().managed.active_bytes;
    heap.set_limits(HeapLimits {
        max_bytes: None,
        managed: crate::ManagedLimits {
            max_bytes: Some(baseline),
        },
        raw: crate::RawLimits { max_bytes: None },
    })
    .expect("baseline managed heap should fit its current active-byte limit");

    let error = heap
        .allocate_managed_bytes(&[1], ReferenceMap::empty(), None)
        .expect_err("managed allocation should be rejected");

    assert_eq!(error.scope, crate::HeapLimitScope::Managed);
    assert_eq!(heap.managed_allocation_count(), 0);
    assert_eq!(heap.usage().managed.active_bytes, baseline);
}

/// Reject one raw allocation when the active-byte limit would be exceeded.
#[test]
fn test_reject_raw_allocation_when_limit_exceeded() {
    let mut heap = Heap::new();
    let baseline = heap.usage().raw.active_bytes;
    heap.set_limits(HeapLimits {
        max_bytes: None,
        managed: crate::ManagedLimits { max_bytes: None },
        raw: crate::RawLimits {
            max_bytes: Some(baseline),
        },
    })
    .expect("baseline raw heap should fit its current active-byte limit");

    let error = heap
        .allocate_raw_bytes(&[1])
        .expect_err("raw allocation should be rejected");

    assert_eq!(error.scope, crate::HeapLimitScope::Raw);
    assert_eq!(heap.raw().allocation_count(), 0);
    assert_eq!(heap.usage().raw.active_bytes, baseline);
}

/// Detach one managed allocation after restoring a shared heap image.
#[test]
fn test_heap_managed_write_detaches_only_touched_allocation() {
    let layout = HeapLayout {
        managed_young_bytes: 0,
        managed_small_bytes: 32,
        ..HeapLayout::default()
    };
    let mut heap = Heap::with_limits_and_layout(HeapLimits::default(), layout)
        .expect("explicit heap layout should build");
    let first_bytes = vec![0xAA; 5000];
    let second_bytes = vec![0xBB; 5000];
    let first = heap
        .allocate_managed_bytes(&first_bytes, ReferenceMap::empty(), None)
        .expect("managed allocation should succeed");
    let second = heap
        .allocate_managed_bytes(&second_bytes, ReferenceMap::empty(), None)
        .expect("managed allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored = Heap::from_image(&image).expect("heap image layout should restore");

    // mutating one allocation should detach only that allocation
    assert!(restored.set_managed_bytes(first, 0, &[0xCC]));
    let mutated_image = restored.image().expect("heap image should capture");

    assert_ne!(
        image.managed().allocations()[0].pages,
        mutated_image.managed().allocations()[0].pages
    );
    assert_eq!(
        image.managed().allocations()[1].pages,
        mutated_image.managed().allocations()[1].pages
    );
    let mut expected_first = first_bytes;
    expected_first[0] = 0xCC;

    assert_eq!(
        restored.managed_bytes(first).as_deref(),
        Some(expected_first.as_slice())
    );
    assert_eq!(
        restored.managed_bytes(second).as_deref(),
        Some(second_bytes.as_slice())
    );
}

/// Detach only the touched page inside one shared managed allocation.
#[test]
fn test_heap_managed_write_detaches_only_touched_page() {
    let layout = HeapLayout {
        managed_young_bytes: 0,
        managed_small_bytes: 32,
        ..HeapLayout::default()
    };
    let mut heap = Heap::with_limits_and_layout(HeapLimits::default(), layout)
        .expect("explicit heap layout should build");
    let bytes = vec![0xAA; 9000];
    let reference = heap
        .allocate_managed_bytes(&bytes, ReferenceMap::empty(), None)
        .expect("managed allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut restored = Heap::from_image(&image).expect("heap image layout should restore");

    // mutating one page should leave the untouched pages shared
    assert!(restored.set_managed_bytes(reference, 4096, &[0xCC]));
    let mutated_image = restored.image().expect("heap image should capture");
    let original_pages = &image.managed().allocations()[0].pages;
    let mutated_pages = &mutated_image.managed().allocations()[0].pages;

    assert_eq!(original_pages.page(0), mutated_pages.page(0));
    assert_ne!(original_pages.page(1), mutated_pages.page(1));
    assert_eq!(original_pages.page(2), mutated_pages.page(2));
}

/// Preserve arena sharing for managed small-space spans across image roundtrips.
#[test]
fn test_roundtrip_managed_small_space_image() {
    let layout = HeapLayout {
        managed_young_bytes: 0,
        page_bytes: 4,
        ..HeapLayout::default()
    };
    let arena = arena(&layout);
    let mut managed = ManagedSpace::with_layout(arena.clone(), &layout)
        .expect("explicit managed layout should build");

    // small allocations should stay in spans and share those span pages after restore
    let first = managed.allocate_bytes(&[1, 2, 3], ReferenceMap::empty(), None);
    let _second = managed.allocate_bytes(&[4, 5, 6], ReferenceMap::empty(), None);
    let image = managed.image().expect("managed image should capture");
    let mut restored = ManagedSpace::from_image(arena.clone(), &image)
        .expect("managed image layout should restore");
    let restored_image = restored.image().expect("managed image should capture");

    assert_eq!(image.spans()[0].pages, restored_image.spans()[0].pages);

    // mutating one small allocation should detach the touched span
    assert!(restored.set_byte(first, 1, 0xFE));
    let mutated_image = restored.image().expect("managed image should capture");

    assert_ne!(image.spans()[0].pages, mutated_image.spans()[0].pages);
    assert_eq!(restored.bytes(first), Some(vec![1, 0xFE, 3]));
}

/// Preserve arena sharing for managed young-space allocations across image roundtrips.
#[test]
fn test_roundtrip_managed_young_space_image() {
    let layout = HeapLayout {
        page_bytes: 4,
        ..HeapLayout::default()
    };
    let arena = arena(&layout);
    let mut managed = ManagedSpace::with_layout(arena.clone(), &layout)
        .expect("explicit managed layout should build");

    // young allocations should share nursery pages after restore
    let first = managed.allocate_bytes(&[1, 2, 3], ReferenceMap::empty(), None);
    let _second = managed.allocate_bytes(&[4, 5, 6], ReferenceMap::empty(), None);
    let image = managed.image().expect("managed image should capture");
    let mut restored = ManagedSpace::from_image(arena.clone(), &image)
        .expect("managed image layout should restore");
    let restored_image = restored.image().expect("managed image should capture");

    assert_eq!(image.young().pages(), restored_image.young().pages());

    // mutating one young allocation should detach the nursery pages
    assert!(restored.set_byte(first, 1, 0xFE));
    let mutated_image = restored.image().expect("managed image should capture");

    assert_ne!(image.young().pages(), mutated_image.young().pages());
    assert_eq!(restored.bytes(first), Some(vec![1, 0xFE, 3]));
}

/// Promote reachable young allocations and clear unreachable nursery state.
#[test]
fn test_collect_young_references_promotes_reachable_allocations() {
    let layout = HeapLayout {
        managed_small_bytes: 32,
        ..HeapLayout::default()
    };
    let arena = arena(&layout);
    let mut managed =
        ManagedSpace::with_layout(arena, &layout).expect("explicit managed layout should build");

    // allocate one reachable and one unreachable young allocation
    let reachable = managed.allocate_bytes(&[1, 2, 3], ReferenceMap::empty(), None);
    let unreachable = managed.allocate_bytes(&[4, 5, 6], ReferenceMap::empty(), None);

    assert!(matches!(
        managed.location(reachable),
        Some(ManagedLocation::Young(_))
    ));
    assert!(matches!(
        managed.location(unreachable),
        Some(ManagedLocation::Young(_))
    ));

    // collect against the reachable root
    let stats = managed
        .collect_young_references([reachable])
        .expect("young collection should succeed");

    assert_eq!(stats.freed_allocations, 1);
    assert_eq!(managed.bytes(reachable), Some(vec![1, 2, 3]));
    assert!(!managed.is_allocated(unreachable));
    assert!(matches!(
        managed.location(reachable),
        Some(ManagedLocation::Small(_)) | Some(ManagedLocation::Large(_))
    ));
    // the nursery should reset to an empty cursor over a fresh fixed reservation
    let image = managed.image().expect("managed image should capture");
    let young = image.young();

    assert_eq!(young.next_offset(), 0);
    assert_eq!(
        young.pages().len(),
        young.capacity_bytes().div_ceil(young.page_bytes())
    );
}

/// Preserve arena sharing for raw small-space spans across image roundtrips.
#[test]
fn test_roundtrip_raw_small_space_image() {
    let layout = HeapLayout {
        page_bytes: 4,
        ..HeapLayout::default()
    };
    let arena = arena(&layout);
    let mut raw = RawSpace::with_layout(arena.clone(), &layout);

    // small allocations should stay in spans and share those span pages after restore
    let first = raw.allocate_bytes(&[1, 2, 3]);
    let _second = raw.allocate_bytes(&[4, 5, 6]);
    let image = raw.image();
    let mut restored = RawSpace::from_image(arena.clone(), &image);
    let restored_image = restored.image();

    assert_eq!(image.spans()[0].pages, restored_image.spans()[0].pages);

    // mutating one small allocation should detach the touched span
    assert!(restored.set_byte(first, 1, 0xFE));
    let mutated_image = restored.image();

    assert_ne!(image.spans()[0].pages, mutated_image.spans()[0].pages);
    assert_eq!(restored.bytes(first), Some(vec![1, 0xFE, 3]));
}

/// Reject one raw replace when byte growth would exceed the active-byte limit.
#[test]
fn test_reject_raw_replace_when_limit_exceeded() {
    let mut heap = Heap::new();
    let pointer = heap
        .allocate_raw_bytes(&[0xAA])
        .expect("raw allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut heap = Heap::from_image(&image).expect("heap image layout should restore");
    let baseline = heap.usage().raw.active_bytes;
    heap.set_limits(HeapLimits {
        max_bytes: None,
        managed: crate::ManagedLimits { max_bytes: None },
        raw: crate::RawLimits {
            max_bytes: Some(baseline),
        },
    })
    .expect("baseline raw heap should fit its current active-byte limit");

    let error = heap
        .replace_raw_bytes(pointer, &[0xBB, 0xCC])
        .expect_err("raw replace should be rejected");

    assert_eq!(error.scope, crate::HeapLimitScope::Raw);
    assert_eq!(heap.raw_bytes(pointer).as_deref(), Some(&[0xAA][..]));
    assert_eq!(heap.usage().raw.active_bytes, baseline);
}
