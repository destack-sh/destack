use destack_core::SnapshotCodec;

use crate::managed::ManagedLocation;
use crate::{
    Heap, HeapCaptureError, HeapLayoutOptions, HeapLimits, LayoutId, ManagedReference,
    ManagedSpace, RawPointer, RawSpace, ReferenceMap, SharedBudget, SharedLimits, SharedSpace,
    Value,
};

/// Trace compact managed references at fixed byte offsets.
#[test]
fn test_reference_offsets_trace_handle32_payloads() {
    let first = ManagedReference::new(0x0102_0304);
    let second = ManagedReference::new(0x1122_3344);
    let bytes = [
        0xAA, 0xBB, 0xCC, 0xDD, 0x04, 0x03, 0x02, 0x01, 0x44, 0x33, 0x22, 0x11,
    ];
    let map = ReferenceMap::ReferenceOffsets {
        offsets: vec![4, 8],
    };
    let mut traced = Vec::new();

    // follow compact handles without assuming u64 payload words
    map.for_each_reference(&bytes, 4, |reference| traced.push(reference));

    assert_eq!(traced, vec![first, second]);
}

/// Trace compact managed references across repeated elements.
#[test]
fn test_repeated_reference_offsets_trace_handle32_payloads() {
    let first = ManagedReference::new(0x0102_0304);
    let second = ManagedReference::new(0x1122_3344);
    let bytes = [
        0x10, 0x20, 0x30, 0x40, 0x04, 0x03, 0x02, 0x01, 0x50, 0x60, 0x70, 0x80, 0x44, 0x33, 0x22,
        0x11,
    ];
    let map = ReferenceMap::RepeatedReferenceOffsets {
        count: 2,
        element_size: 8,
        offsets: vec![4],
    };
    let mut traced = Vec::new();

    // follow compact handles using the declared repeated stride
    map.for_each_reference(&bytes, 4, |reference| traced.push(reference));

    assert_eq!(traced, vec![first, second]);
}

/// Trace managed references across chunked large-allocation windows without flattening payloads.
#[test]
fn test_collect_handles_traces_chunked_large_allocation_references() {
    let layout = HeapLayoutOptions {
        page_bytes: 4,
        ..HeapLayoutOptions::default()
    };
    let mut managed = ManagedSpace::with_layout(&layout);
    let child = managed.allocate_bytes(&[0xAA], ReferenceMap::empty(), None);
    let mut parent_bytes = vec![0u8; 24];
    let child_bits = child.bits().to_le_bytes();

    // write one reference so it crosses chunk boundaries during tracing
    parent_bytes[10..18].copy_from_slice(&child_bits);

    let parent = managed.allocate_bytes(
        &parent_bytes,
        ReferenceMap::ReferenceOffsets { offsets: vec![10] },
        None,
    );

    // keep only the parent live and ensure the child edge is still traced
    let stats = managed.collect_handles([parent]);

    assert_eq!(stats.freed_allocations, 0);
    assert!(managed.is_allocated(child));
}

/// Reject unsupported traced managed-reference widths loudly.
#[test]
#[should_panic(expected = "unsupported managed reference width for tracing")]
fn test_reference_offsets_reject_unsupported_tracing_width() {
    let bytes = [0xAA, 0xBB, 0xCC, 0xDD];
    let map = ReferenceMap::ReferenceOffsets { offsets: vec![0] };

    map.for_each_reference(&bytes, 3, |_| {});
}

/// Reject unsupported managed-reference widths at heap construction.
#[test]
#[should_panic(expected = "unsupported managed reference width for heap layout")]
fn test_heap_rejects_invalid_managed_reference_width() {
    let layout = HeapLayoutOptions {
        managed_reference_bytes: 3,
        ..HeapLayoutOptions::default()
    };

    let _ = Heap::with_limits_and_layout(HeapLimits::default(), layout);
}

/// Reject unsupported managed-reference widths when restoring a heap snapshot.
#[test]
#[should_panic(expected = "unsupported managed reference width for heap layout")]
fn test_heap_snapshot_rejects_invalid_managed_reference_width() {
    let mut heap = Heap::new();
    let mut snapshot = heap.snapshot().expect("heap snapshot should capture");
    snapshot.managed.managed_reference_bytes = 3;

    let _ = Heap::from_snapshot(&snapshot);
}

/// Preserve stable managed ids across span growth and id reuse.
#[test]
fn test_allocate_managed_across_runs() {
    let layout = HeapLayoutOptions::default();
    let slot_count = layout.managed_small_bytes / layout.size_classes.classes[0].bytes;
    let mut managed = ManagedSpace::with_layout(&layout);
    let mut last = ManagedReference::NULL;

    // cross the first managed span boundary
    for index in 0..(slot_count + 4) {
        last = managed.allocate_bytes(&[index as u8], ReferenceMap::empty(), None);
    }

    // ensure stable ids grow monotonically
    assert_eq!(last.id(), (slot_count + 4) as u64);

    // free and reuse one id
    let reused = ManagedReference::new(7);
    assert!(managed.free(reused));
    let handle = managed.allocate_bytes(&[0xAB], ReferenceMap::empty(), None);

    assert_eq!(handle.id(), reused.id());
}

/// Preserve stable raw ids across span growth and id reuse.
#[test]
fn test_allocate_raw_across_runs() {
    let layout = HeapLayoutOptions::default();
    let slot_count = layout.raw_small_bytes / layout.size_classes.classes[0].bytes;
    let mut raw = RawSpace::with_layout(&layout);
    let mut last = RawPointer::NULL;

    // cross the first raw span boundary
    for index in 0..(slot_count + 4) {
        last = raw.allocate_bytes(&[index as u8]);
    }

    // ensure stable ids grow monotonically
    assert_eq!(last.id(), (slot_count + 4) as u64);

    // free and reuse one id
    let reused = RawPointer::new(11);
    assert!(raw.free(reused));
    let pointer = raw.allocate_bytes(&[0xCD]);

    assert_eq!(pointer.id(), reused.id());
}

/// Share unchanged managed span leaves across image roundtrips and detach on mutation.
#[test]
fn test_roundtrip_managed_space_image() {
    let layout = HeapLayoutOptions::default();
    let slot_count = layout.managed_small_bytes / layout.size_classes.classes[0].bytes;
    let mut managed = ManagedSpace::with_layout(&layout);
    let mut handles = Vec::new();

    // spill into a second managed span
    for index in 0..(slot_count + 4) {
        let handle = managed.allocate_bytes(&[index as u8], ReferenceMap::empty(), None);
        handles.push(handle);
    }

    // preserve layout metadata across image roundtrip
    assert!(managed.set_layout_id(handles[0], LayoutId::new(41)));

    let image = managed.image(None).expect("managed image should capture");
    let mut restored = ManagedSpace::from_image(&image);
    let restored_image = restored.image(None).expect("managed image should capture");

    assert_eq!(restored.layout_id(handles[0]), Some(LayoutId::new(41)));
    assert!(
        image
            .spans
            .first()
            .unwrap()
            .shares_storage_with(restored_image.spans.first().unwrap())
    );

    // mutating one span should detach only that span
    assert!(restored.set_byte(handles[0], 0, 0xFE));
    let mutated_image = restored.image(None).expect("managed image should capture");

    assert!(
        !image
            .spans
            .first()
            .unwrap()
            .shares_storage_with(mutated_image.spans.first().unwrap())
    );
    assert!(
        image
            .spans
            .get(1)
            .unwrap()
            .shares_storage_with(mutated_image.spans.get(1).unwrap())
    );
}

/// Preserve nominal managed type ids across image roundtrips.
#[test]
fn test_roundtrip_managed_space_preserves_type_ids() {
    let mut managed = ManagedSpace::with_layout(&HeapLayoutOptions::default());
    let handle = managed.allocate_bytes(&[0xAA], ReferenceMap::empty(), None);

    // stamp one nominal type id before capturing the image
    assert!(managed.set_type_id(handle, 42));

    let image = managed.image(None).expect("managed image should capture");
    let restored = ManagedSpace::from_image(&image);

    assert_eq!(restored.type_id(handle), Some(42));
}

/// Clear stale nominal type ids when one managed id is freed and reused.
#[test]
fn test_free_reused_managed_id_clears_type_id() {
    let mut managed = ManagedSpace::with_layout(&HeapLayoutOptions::default());
    let original = managed.allocate_bytes(&[0xAA], ReferenceMap::empty(), None);

    // stamp one nominal type id on the original allocation
    assert!(managed.set_type_id(original, 42));
    assert_eq!(managed.type_id(original), Some(42));

    // free the allocation so the stable id can be reused
    assert!(managed.free(original));
    assert_eq!(managed.type_id(original), None);

    let reused = managed.allocate_bytes(&[0xBB], ReferenceMap::empty(), None);

    // reused ids must not inherit stale nominal type metadata
    assert_eq!(reused.id(), original.id());
    assert_eq!(managed.type_id(reused), None);
}

/// Keep one first survivor young and then promote it on the next young collection.
#[test]
fn test_collect_young_handles_retains_then_promotes_survivors() {
    let mut managed = ManagedSpace::with_layout(&HeapLayoutOptions::default());
    let handle = managed.allocate_bytes(&[0xAA], ReferenceMap::empty(), None);

    // first young collection should retain the survivor in young space
    let first_stats = managed.collect_young_handles([handle]);

    assert_eq!(first_stats.freed_allocations, 0);
    assert!(matches!(
        managed.location(handle),
        Some(ManagedLocation::Young(_))
    ));

    // second young collection should age the survivor into small space
    let second_stats = managed.collect_young_handles([handle]);

    assert_eq!(second_stats.freed_allocations, 0);
    assert!(matches!(
        managed.location(handle),
        Some(ManagedLocation::Small(_))
    ));
}

/// Free unreachable young allocations during one young collection.
#[test]
fn test_collect_young_handles_frees_unreachable_young() {
    let mut managed = ManagedSpace::with_layout(&HeapLayoutOptions::default());
    let live = managed.allocate_bytes(&[0xAA], ReferenceMap::empty(), None);
    let dead = managed.allocate_bytes(&[0xBB], ReferenceMap::empty(), None);

    // keep only one young allocation live
    let stats = managed.collect_young_handles([live]);

    assert_eq!(stats.freed_allocations, 1);
    assert!(managed.is_allocated(live));
    assert!(!managed.is_allocated(dead));
}

/// Preserve young reachability through remembered mature allocations during minor collection.
#[test]
fn test_collect_young_handles_traces_remembered_mature_edges() {
    let mut managed = ManagedSpace::with_layout(&HeapLayoutOptions::default());
    let parent =
        managed.allocate_zeroed(8, ReferenceMap::ReferenceOffsets { offsets: vec![0] }, None);

    // promote the parent into mature storage first
    let _ = managed.collect_young_handles([parent]);
    let _ = managed.collect_young_handles([parent]);
    assert!(matches!(
        managed.location(parent),
        Some(ManagedLocation::Small(_))
    ));

    let child = managed.allocate_bytes(&[0xCC], ReferenceMap::empty(), None);
    let child_bits = child.bits().to_le_bytes();

    // record one old to young edge through the mature parent payload
    assert!(managed.set_bytes(parent, 0, &child_bits));

    let stats = managed.collect_young_handles([]);

    assert_eq!(stats.freed_allocations, 0);
    assert!(managed.is_allocated(child));
}

/// Drain young allocations into durable storage before image capture.
#[test]
fn test_managed_image_drains_young_space() {
    let mut managed = ManagedSpace::with_layout(&HeapLayoutOptions::default());
    let handle = managed.allocate_bytes(&[0xDD], ReferenceMap::empty(), None);

    // capture should force young allocations into mature storage
    let image = managed.image(None).expect("managed image should capture");

    assert!(matches!(
        managed.location(handle),
        Some(ManagedLocation::Small(_))
    ));

    let restored = ManagedSpace::from_image(&image);

    assert_eq!(restored.bytes_to_vec(handle), Some(vec![0xDD]));
    assert!(matches!(
        restored.location(handle),
        Some(ManagedLocation::Small(_))
    ));
}

/// Share unchanged raw span leaves across image roundtrips and detach on mutation.
#[test]
fn test_roundtrip_raw_space_image() {
    let layout = HeapLayoutOptions::default();
    let slot_count = layout.raw_small_bytes / layout.size_classes.classes[0].bytes;
    let mut raw = RawSpace::with_layout(&layout);
    let mut pointers = Vec::new();

    // spill into a second raw span
    for index in 0..(slot_count + 4) {
        let pointer = raw.allocate_bytes(&[index as u8, 0xAA]);
        pointers.push(pointer);
    }

    let image = raw.image(None);
    let mut restored = RawSpace::from_image(&image);
    let restored_image = restored.image(None);

    assert!(
        image
            .spans
            .first()
            .unwrap()
            .shares_storage_with(restored_image.spans.first().unwrap())
    );

    // mutating one span should detach only that span
    assert!(restored.set_byte(pointers[0], 1, 0xFE));
    let mutated_image = restored.image(None);

    assert!(
        !image
            .spans
            .first()
            .unwrap()
            .shares_storage_with(mutated_image.spans.first().unwrap())
    );
    assert!(
        image
            .spans
            .get(1)
            .unwrap()
            .shares_storage_with(mutated_image.spans.get(1).unwrap())
    );
}

/// Share unchanged shared chunks across image roundtrips and detach on mutation.
#[test]
fn test_roundtrip_shared_space_image() {
    let mut shared = SharedSpace::with_page_bytes(4);
    let mut budget = SharedBudget::new(SharedLimits::default(), 0);
    let pointer = shared
        .allocate_bytes(&[1, 2, 3, 4, 5, 6], &mut budget)
        .unwrap();

    let image = shared.image(None);
    let mut restored = SharedSpace::from_image(&image);
    let restored_image = restored.image(None);

    assert!(
        image
            .region(0)
            .unwrap()
            .shares_storage_with(restored_image.region(0).unwrap())
    );

    let mut restored_budget = SharedBudget::new(SharedLimits::default(), restored.active_bytes());

    assert!(restored.set_byte(pointer, 1, 9, &mut restored_budget));
    let mutated_image = restored.image(None);

    assert!(
        !image
            .region(0)
            .unwrap()
            .shares_storage_with(mutated_image.region(0).unwrap())
    );
}

/// Reject managed-space capture while one pin is still active.
#[test]
fn test_reject_managed_image_with_active_pins() {
    let mut managed = ManagedSpace::new();
    let handle = managed.allocate_bytes(&[1], ReferenceMap::empty(), None);

    assert!(managed.pin(handle));
    assert_eq!(
        managed.image(None).unwrap_err(),
        HeapCaptureError::PinnedManagedReferences
    );

    assert!(managed.unpin(handle));
    assert!(managed.image(None).is_ok());
}

/// Preserve heap images and snapshots across the full heap wrapper.
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

    let image = heap.image().expect("heap image should capture");
    let snapshot = Heap::encode_snapshot(&image).expect("heap snapshot should encode");
    let decoded = Heap::decode_snapshot(&snapshot).expect("heap snapshot should decode");
    let restored = Heap::from_snapshot(&snapshot);

    assert_eq!(
        decoded.local_allocated_bytes(),
        image.local_allocated_bytes()
    );
    assert_eq!(decoded.leaf_count(), image.leaf_count());
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

/// Reject one managed write when CoW detachment would exceed the active-byte limit.
#[test]
fn test_reject_managed_write_when_limit_exceeded() {
    let mut heap = Heap::new();
    let handle = heap
        .allocate_managed_bytes(&[0xAA], ReferenceMap::empty(), None)
        .expect("managed allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut heap = Heap::from_image(&image);
    let baseline = heap.usage().managed.active_bytes;
    heap.set_limits(HeapLimits {
        max_bytes: None,
        managed: crate::ManagedLimits {
            max_bytes: Some(baseline),
        },
        raw: crate::RawLimits { max_bytes: None },
    })
    .expect("baseline managed heap should fit its current active-byte limit");

    let updated = heap.set_managed_byte(handle, 0, 0xBB);

    assert!(!updated);
    assert_eq!(heap.managed_bytes(handle).as_deref(), Some(&[0xAA][..]));
    assert_eq!(heap.usage().managed.active_bytes, baseline);
}

/// Reject one raw replace when CoW detachment would exceed the active-byte limit.
#[test]
fn test_reject_raw_replace_when_limit_exceeded() {
    let mut heap = Heap::new();
    let pointer = heap
        .allocate_raw_bytes(&[0xAA])
        .expect("raw allocation should succeed");
    let image = heap.image().expect("heap image should capture");
    let mut heap = Heap::from_image(&image);
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
        .replace_raw_bytes(pointer, &[0xBB])
        .expect_err("raw replace should be rejected");

    assert_eq!(error.scope, crate::HeapLimitScope::Raw);
    assert_eq!(heap.raw_bytes(pointer).as_deref(), Some(&[0xAA][..]));
    assert_eq!(heap.usage().raw.active_bytes, baseline);
}
