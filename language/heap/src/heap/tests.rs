use destack_core::SnapshotCodec;
use std::sync::Arc;

use crate::{
    Heap, HeapCaptureError, HeapLimits, LayoutId, MANAGED_PAGE_CAPACITY, ManagedHeap,
    ManagedReference, RAW_PAGE_CAPACITY, RawAllocationStorage, RawHeap, RawPointer, Value,
};

const CELL_PAGE_SPILL_ALLOCATIONS: usize = MANAGED_PAGE_CAPACITY + 8;
const RAW_PAGE_SPILL_ALLOCATIONS: usize = RAW_PAGE_CAPACITY + 8;

/// Allocate managed references across page boundaries while preserving stable ids.
#[test]
fn test_allocate_managed_allocations_across_pages() {
    let mut heap = ManagedHeap::new();
    let mut last_handle = ManagedReference::NULL;

    // cross the first managed-page boundary
    for index in 0..(MANAGED_PAGE_CAPACITY + 4) {
        last_handle = heap
            .allocate_single_checked(Value::int64(index as i64), |_| Ok(()))
            .expect("managed allocation should succeed");
    }

    // ensure ids continue monotonically across pages
    assert_eq!(last_handle.id(), (MANAGED_PAGE_CAPACITY + 4) as u64);
    assert!(heap.get(last_handle).is_some());
}

/// Allocate raw allocations across page boundaries while preserving stable ids.
#[test]
fn test_allocate_raw_allocations_across_pages() {
    let mut heap = RawHeap::new();
    let mut last_pointer = RawPointer::NULL;

    // cross the first raw-page boundary
    for index in 0..(RAW_PAGE_CAPACITY + 4) {
        last_pointer = heap
            .allocate_with_bytes_checked(&[index as u8], |_| Ok(()))
            .expect("raw allocation should succeed");
    }

    // ensure ids continue monotonically across pages
    assert_eq!(last_pointer.id(), (RAW_PAGE_CAPACITY + 4) as u64);
    assert!(heap.get(last_pointer).is_some());
}

/// Capture and restore managed heap state across managed-page boundaries.
#[test]
fn test_roundtrip_managed_heap_image() {
    let mut heap = ManagedHeap::new();
    let mut handles = Vec::new();

    // spill into a second managed page
    for index in 0..CELL_PAGE_SPILL_ALLOCATIONS {
        let handle = heap
            .allocate_single_checked(Value::int64(index as i64), |_| Ok(()))
            .expect("managed allocation should succeed");
        handles.push(handle);
    }

    // free and reuse one slot to preserve allocator state
    let freed_handle = handles[10];
    heap.free(freed_handle);
    let reused_handle = heap
        .allocate_single_checked(Value::int64(999), |_| Ok(()))
        .expect("managed allocation should succeed");
    assert_eq!(reused_handle.id(), freed_handle.id());

    let image = heap.image(None).expect("heap image should capture");
    let mut restored = ManagedHeap::from_image(&image);

    // verify preserved live allocations and allocator state
    assert_eq!(restored.allocation_count(), heap.allocation_count());
    assert_eq!(restored.heap_bytes(), heap.heap_bytes());
    assert_eq!(restored.gc_state().cycles, heap.gc_state().cycles);
    assert_eq!(
        restored.get_slot(reused_handle, 0),
        Some(&Value::int64(999))
    );
    assert_eq!(restored.get_slot(handles[0], 0), Some(&Value::int64(0)));

    // images and restored heaps share managed-page images until mutation
    let restored_image = restored.image(None).expect("heap image should capture");
    assert!(Arc::ptr_eq(
        &image.pages.get(0).unwrap().page,
        &restored_image.pages.get(0).unwrap().page
    ));
    assert!(Arc::ptr_eq(
        &image.pages.get(0).unwrap().layout_ids,
        &restored_image.pages.get(0).unwrap().layout_ids
    ));

    // marking a restored reference should not detach shared managed-page contents
    restored.mark_reference(handles[0]);
    let restored_page_image = restored.page_mut(0).image();
    assert!(Arc::ptr_eq(
        &image.pages.get(0).unwrap().page,
        &restored_page_image.page
    ));
    assert!(Arc::ptr_eq(
        &image.pages.get(0).unwrap().layout_ids,
        &restored_page_image.layout_ids
    ));

    // clear the synthetic mark work before the next image capture
    restored.clear_mark_queue();

    // mutating a restored managed page should detach only that page
    let _ = restored.set_slot(handles[0], 0, Value::int64(-1));
    let restored_image = restored.image(None).expect("heap image should capture");
    assert!(!Arc::ptr_eq(
        &image.pages.get(0).unwrap().page,
        &restored_image.pages.get(0).unwrap().page
    ));
}

/// Capture and restore raw heap state across raw-page boundaries.
#[test]
fn test_roundtrip_raw_heap_image() {
    let mut heap = RawHeap::new();
    let mut pointers = Vec::new();

    // spill into a second raw page
    for index in 0..RAW_PAGE_SPILL_ALLOCATIONS {
        let pointer = heap
            .allocate_with_bytes_checked(&[index as u8, 0xAA], |_| Ok(()))
            .expect("raw allocation should succeed");
        pointers.push(pointer);
    }

    // free and reuse one slot to preserve allocator state
    let freed_pointer = pointers[17];
    assert!(heap.free(freed_pointer));
    let reused_pointer = heap
        .allocate_with_bytes_checked(&[0xFE, 0xED], |_| Ok(()))
        .expect("raw allocation should succeed");
    assert_eq!(reused_pointer.id(), freed_pointer.id());

    let image = heap.image(None);
    let restored = RawHeap::from_image(&image);

    // verify preserved live allocations and allocator state
    assert_eq!(restored.allocation_count(), heap.allocation_count());
    let storage = &restored.get(reused_pointer).unwrap().storage;
    match storage {
        RawAllocationStorage::Bytes(_) => {
            assert_eq!(restored.bytes(reused_pointer), Some(&[0xFE, 0xED][..]))
        }
        _ => panic!("expected raw byte storage"),
    }
}

/// Preserve managed layouts across heap image roundtrips.
#[test]
fn test_roundtrip_layout_ids() {
    let mut heap = ManagedHeap::new();
    let handle = heap
        .allocate_single_checked(Value::int64(7), |_| Ok(()))
        .expect("managed allocation should succeed");

    // attach one durable managed layout before capture
    assert!(heap.set_layout_id(handle, LayoutId::new(41)));

    let image = heap.image(None).expect("heap image should capture");
    let restored = ManagedHeap::from_image(&image);

    // managed layout metadata should survive the image roundtrip
    assert_eq!(restored.layout_id(handle), Some(LayoutId::new(41)));
}

/// Reject heap capture while managed pins are active.
#[test]
fn test_reject_managed_heap_image_with_active_pins() {
    let mut heap = ManagedHeap::new();
    let handle = heap
        .allocate_single_checked(Value::int64(1), |_| Ok(()))
        .expect("managed allocation should succeed");

    // active pins should block durable heap capture
    assert!(heap.pin(handle));
    assert_eq!(
        heap.image(None).unwrap_err(),
        HeapCaptureError::PinnedManagedReferences
    );

    // releasing the pin should allow capture again
    assert!(heap.unpin(handle));
    assert!(heap.image(None).is_ok());
}

/// Share unchanged heap leaves across full heap image captures and snapshots.
#[test]
fn test_roundtrip_heap_image_and_snapshot_shares_leaves() {
    let mut heap = Heap::default();

    // populate both managed and raw leaves
    let managed = heap
        .allocate_managed_single(Value::int64(7))
        .expect("managed allocation should succeed");
    let raw = heap
        .allocate_raw_bytes(&[0xCA, 0xFE, 0xBA, 0xBE])
        .expect("raw allocation should succeed");

    let image = heap.image().expect("heap image should capture");
    let snapshot = Heap::encode_snapshot(&image).expect("heap snapshot should encode");
    let decoded = Heap::decode_snapshot(&snapshot).expect("heap snapshot should decode");
    let restored = Heap::from_image(&image);

    // decoded snapshots preserve durable payload accounting
    assert_eq!(decoded.heap_bytes(), image.heap_bytes());
    assert_eq!(decoded.leaf_count(), image.leaf_count());

    // restored images still share the same immutable leaves
    let mut restored = restored;
    let restored_image = restored.image().expect("heap image should capture");
    assert!(
        image
            .managed_page(0)
            .unwrap()
            .shares_storage_with(restored_image.managed_page(0).unwrap())
    );

    // raw spans survive the full image and snapshot roundtrip
    let from_snapshot = Heap::from_snapshot(&snapshot);
    assert_eq!(
        from_snapshot.raw_bytes(raw),
        Some(&[0xCA, 0xFE, 0xBA, 0xBE][..])
    );
    assert_eq!(
        from_snapshot.managed_slot(managed, 0),
        Some(&Value::int64(7))
    );
}

/// Detach only the touched raw span on mutation after capture.
#[test]
fn test_detach_only_mutated_raw_byte_span() {
    let mut heap = Heap::default();
    let first = heap
        .allocate_raw_bytes(&[1, 2, 3])
        .expect("raw allocation should succeed");
    let second = heap
        .allocate_raw_bytes(&[4, 5, 6])
        .expect("raw allocation should succeed");

    let image = heap.image().expect("heap image should capture");

    // mutate one raw span after the image is shared
    assert!(heap.set_raw_byte(first, 1, 9));
    let mutated = heap.image().expect("heap image should capture");

    // only the touched byte span should detach
    assert!(!image.raw_span_shares_with(&mutated, 0));
    assert!(image.raw_span_shares_with(&mutated, 1));

    assert_eq!(heap.raw_bytes(first), Some(&[1, 9, 3][..]));
    assert_eq!(heap.raw_bytes(second), Some(&[4, 5, 6][..]));
}

/// Use a dedicated large managed span for larger managed payloads.
#[test]
fn test_capture_large_managed_span_as_dedicated_leaf() {
    let mut heap = ManagedHeap::with_large_span_values(5);
    let values = vec![
        Value::int64(1),
        Value::int64(2),
        Value::int64(3),
        Value::int64(4),
        Value::int64(5),
        Value::int64(6),
    ];
    let handle = heap
        .allocate_with_values_checked(values, |_| Ok(()))
        .expect("managed allocation should succeed");

    let image = heap.image(None).expect("heap image should capture");
    assert_eq!(image.large_spans.len(), 1);
    assert_eq!(image.free_large_span_ids.len(), 0);

    // restored images should share the same large managed span leaf
    let mut restored = ManagedHeap::from_image(&image);
    let restored_image = restored.image(None).expect("heap image should capture");
    assert!(Arc::ptr_eq(
        image.large_spans.get(0).unwrap(),
        restored_image.large_spans.get(0).unwrap()
    ));

    // mutating the large managed span should detach only that leaf
    assert!(restored.set_slot(handle, 2, Value::int64(99)));
    let mutated_image = restored.image(None).expect("heap image should capture");
    assert!(!Arc::ptr_eq(
        image.large_spans.get(0).unwrap(),
        mutated_image.large_spans.get(0).unwrap()
    ));
}

/// Use a dedicated large raw span for larger byte payloads.
#[test]
fn test_capture_large_raw_span_as_dedicated_leaf() {
    let mut heap = RawHeap::with_large_span_bytes(4);
    let pointer = heap
        .allocate_with_bytes_checked(&[1, 2, 3, 4, 5], |_| Ok(()))
        .expect("raw allocation should succeed");

    let image = heap.image(None);
    assert_eq!(image.large_spans.len(), 1);
    assert_eq!(image.free_large_span_ids.len(), 0);

    // restored images should share the same large raw span leaf
    let mut restored = RawHeap::from_image(&image);
    let restored_image = restored.image(None);
    assert!(Arc::ptr_eq(
        image.large_spans.get(0).unwrap(),
        restored_image.large_spans.get(0).unwrap()
    ));

    // mutating the large raw span should detach only that leaf
    assert!(restored.set_byte(pointer, 1, 9));
    let mutated_image = restored.image(None);
    assert!(!Arc::ptr_eq(
        image.large_spans.get(0).unwrap(),
        mutated_image.large_spans.get(0).unwrap()
    ));
}

/// Preserve explicit heap span thresholds across image capture and restore.
#[test]
fn test_roundtrip_heap_span_thresholds() {
    let mut heap = Heap::with_limits_and_large_span_thresholds(HeapLimits::default(), 9, 33);

    let image = heap.image().expect("heap image should capture");
    let restored = Heap::from_image(&image);

    assert_eq!(restored.managed_large_span_value_threshold(), 9);
    assert_eq!(restored.raw_large_span_byte_threshold(), 33);
}

/// Reject one managed external allocation when combined retained-byte admission exceeds the limit.
#[test]
fn test_reject_managed_external_allocation_when_combined_delta_exceeds_limit() {
    let managed = ManagedHeap::new();
    let span_delta = managed.span_allocate_delta(8);
    let mut heap = Heap::new();
    let baseline_usage = heap.usage();
    let max_managed_bytes = baseline_usage
        .managed
        .retained_bytes
        .saturating_add(span_delta as u64);

    heap.set_limits(HeapLimits {
        max_bytes: None,
        max_managed_bytes: Some(max_managed_bytes),
        max_raw_bytes: None,
    })
    .expect("baseline heap usage should fit configured limits");

    let error = heap
        .allocate_managed_slots(8)
        .expect_err("combined span and location admission should fail");

    assert_eq!(error.scope, crate::HeapLimitScope::Managed);
}

/// Reject one invalid limit update without mutating the current heap limits.
#[test]
fn test_reject_invalid_limit_update_without_mutating_heap_limits() {
    let mut heap = Heap::new();

    heap.allocate_raw_bytes(b"baseline")
        .expect("baseline raw allocation should succeed");

    let result = heap.set_limits(HeapLimits {
        max_bytes: Some(1),
        max_managed_bytes: None,
        max_raw_bytes: None,
    });

    assert!(result.is_err(), "invalid limit update should be rejected");

    heap.allocate_raw_bytes(&[0xAB; 256])
        .expect("rejected limit update should not replace existing limits");
}
