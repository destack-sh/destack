use destack_core::SnapshotCodec;

use crate::{
    Heap, HeapCaptureError, HeapLayoutOptions, HeapLimits, LayoutId, ManagedReference,
    ManagedSpace, RawPointer, RawSpace, ReferenceMap, SharedBudget, SharedLimits, SharedSpace,
    Value,
};

/// Preserve stable managed ids across run growth and id reuse.
#[test]
fn test_allocate_managed_across_runs() {
    let layout = HeapLayoutOptions::default();
    let slot_count = layout.managed_run_bytes / layout.size_classes.classes[0].bytes;
    let mut managed = ManagedSpace::with_layout(&layout);
    let mut last = ManagedReference::NULL;

    // cross the first managed run boundary
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

/// Preserve stable raw ids across run growth and id reuse.
#[test]
fn test_allocate_raw_across_runs() {
    let layout = HeapLayoutOptions::default();
    let slot_count = layout.raw_run_bytes / layout.size_classes.classes[0].bytes;
    let mut raw = RawSpace::with_layout(&layout);
    let mut last = RawPointer::NULL;

    // cross the first raw run boundary
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

/// Share unchanged managed run leaves across image roundtrips and detach on mutation.
#[test]
fn test_roundtrip_managed_space_image() {
    let layout = HeapLayoutOptions::default();
    let slot_count = layout.managed_run_bytes / layout.size_classes.classes[0].bytes;
    let mut managed = ManagedSpace::with_layout(&layout);
    let mut handles = Vec::new();

    // spill into a second managed run
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
            .runs
            .get(0)
            .unwrap()
            .shares_storage_with(restored_image.runs.get(0).unwrap())
    );

    // mutating one run should detach only that run
    assert!(restored.set_byte(handles[0], 0, 0xFE));
    let mutated_image = restored.image(None).expect("managed image should capture");

    assert!(
        !image
            .runs
            .get(0)
            .unwrap()
            .shares_storage_with(mutated_image.runs.get(0).unwrap())
    );
    assert!(
        image
            .runs
            .get(1)
            .unwrap()
            .shares_storage_with(mutated_image.runs.get(1).unwrap())
    );
}

/// Share unchanged raw run leaves across image roundtrips and detach on mutation.
#[test]
fn test_roundtrip_raw_space_image() {
    let layout = HeapLayoutOptions::default();
    let slot_count = layout.raw_run_bytes / layout.size_classes.classes[0].bytes;
    let mut raw = RawSpace::with_layout(&layout);
    let mut pointers = Vec::new();

    // spill into a second raw run
    for index in 0..(slot_count + 4) {
        let pointer = raw.allocate_bytes(&[index as u8, 0xAA]);
        pointers.push(pointer);
    }

    let image = raw.image(None);
    let mut restored = RawSpace::from_image(&image);
    let restored_image = restored.image(None);

    assert!(
        image
            .runs
            .get(0)
            .unwrap()
            .shares_storage_with(restored_image.runs.get(0).unwrap())
    );

    // mutating one run should detach only that run
    assert!(restored.set_byte(pointers[0], 1, 0xFE));
    let mutated_image = restored.image(None);

    assert!(
        !image
            .runs
            .get(0)
            .unwrap()
            .shares_storage_with(mutated_image.runs.get(0).unwrap())
    );
    assert!(
        image
            .runs
            .get(1)
            .unwrap()
            .shares_storage_with(mutated_image.runs.get(1).unwrap())
    );
}

/// Share unchanged shared chunks across image roundtrips and detach on mutation.
#[test]
fn test_roundtrip_shared_space_image() {
    let mut shared = SharedSpace::with_chunk_bytes(4);
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

    assert!(restored.set_byte(pointer, 1, 9));
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
    let managed = heap
        .allocate_packed_single(Value::int64(7))
        .expect("managed allocation should succeed");
    let raw = heap
        .allocate_raw_bytes(&[0xCA, 0xFE, 0xBA, 0xBE])
        .expect("raw allocation should succeed");

    let image = heap.image().expect("heap image should capture");
    let snapshot = Heap::encode_snapshot(&image).expect("heap snapshot should encode");
    let decoded = Heap::decode_snapshot(&snapshot).expect("heap snapshot should decode");
    let restored = Heap::from_snapshot(&snapshot);

    assert_eq!(
        decoded.local_allocation_bytes(),
        image.local_allocation_bytes()
    );
    assert_eq!(decoded.leaf_count(), image.leaf_count());
    assert_eq!(restored.packed_value_at(managed, 0), Some(Value::int64(7)));
    assert_eq!(
        restored.raw_bytes(raw).as_deref(),
        Some(&[0xCA, 0xFE, 0xBA, 0xBE][..])
    );
}

/// Reject one managed allocation when the retained-byte limit would be exceeded.
#[test]
fn test_reject_managed_allocation_when_limit_exceeded() {
    let mut heap = Heap::new();
    let baseline = heap.usage().managed.retained_bytes;
    heap.set_limits(HeapLimits {
        max_bytes: None,
        managed: crate::ManagedLimits {
            max_bytes: Some(baseline),
        },
        raw: crate::RawLimits { max_bytes: None },
    })
    .expect("baseline managed heap should fit its current retained-byte limit");

    let error = heap
        .allocate_managed_bytes(&[1], ReferenceMap::empty(), None)
        .expect_err("managed allocation should be rejected");

    assert_eq!(error.scope, crate::HeapLimitScope::Managed);
    assert_eq!(heap.managed_allocation_count(), 0);
}
