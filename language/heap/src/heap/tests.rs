use std::sync::Arc;

use crate::{
    HEAP_PAGE_CAPACITY, ManagedHeap, ManagedPointer, RawCellStorage, RawHeap, RawPointer, Value,
};

const PAGE_SPILL_ALLOCATIONS: usize = HEAP_PAGE_CAPACITY + 8;

/// Allocate managed cells across page boundaries while preserving stable ids.
#[test]
fn test_allocate_managed_cells_across_pages() {
    let mut heap = ManagedHeap::new();
    let mut last_handle = ManagedPointer::NULL;

    // cross the first page boundary
    for index in 0..(HEAP_PAGE_CAPACITY + 4) {
        last_handle = heap.allocate_single(Value::int64(index as i64));
    }

    // ensure ids continue monotonically across pages
    assert_eq!(last_handle.id(), (HEAP_PAGE_CAPACITY + 4) as u64);
    assert!(heap.get(last_handle).is_some());
}

/// Allocate raw cells across page boundaries while preserving stable ids.
#[test]
fn test_allocate_raw_cells_across_pages() {
    let mut heap = RawHeap::new();
    let mut last_pointer = RawPointer::NULL;

    // cross the first page boundary
    for index in 0..(HEAP_PAGE_CAPACITY + 4) {
        last_pointer = heap.allocate_with_bytes(&[index as u8]);
    }

    // ensure ids continue monotonically across pages
    assert_eq!(last_pointer.id(), (HEAP_PAGE_CAPACITY + 4) as u64);
    assert!(heap.get(last_pointer).is_some());
}

/// Capture and restore managed heap state across page boundaries.
#[test]
fn test_roundtrip_managed_heap_snapshot() {
    let mut heap = ManagedHeap::new();
    let mut handles = Vec::new();

    // spill into a second page
    for index in 0..PAGE_SPILL_ALLOCATIONS {
        let handle = heap.allocate_single(Value::int64(index as i64));
        handles.push(handle);
    }

    // free and reuse one slot to preserve allocator state
    let freed_handle = handles[10];
    heap.free(freed_handle);
    let reused_handle = heap.allocate_single(Value::int64(999));
    assert_eq!(reused_handle.id(), freed_handle.id());

    let snapshot = heap.snapshot();
    let mut restored = ManagedHeap::restore(&snapshot);

    // verify preserved live cells and allocator state
    assert_eq!(restored.cell_count(), heap.cell_count());
    assert_eq!(restored.heap_bytes(), heap.heap_bytes());
    assert_eq!(restored.gc_state().cycles, heap.gc_state().cycles);
    assert_eq!(
        restored.get(reused_handle).unwrap().slots.get(0),
        Some(&Value::int64(999))
    );
    assert_eq!(
        restored.get(handles[0]).unwrap().slots.get(0),
        Some(&Value::int64(0))
    );

    // snapshots and restored heaps share page images until mutation
    let restored_snapshot = restored.snapshot();
    assert!(Arc::ptr_eq(&snapshot.pages[0], &restored_snapshot.pages[0]));

    // mutating a restored page should detach only that page
    let _ = restored.set_slot(handles[0], 0, Value::int64(-1));
    let restored_snapshot = restored.snapshot();
    assert!(!Arc::ptr_eq(
        &snapshot.pages[0],
        &restored_snapshot.pages[0]
    ));
}

/// Capture and restore raw heap state across page boundaries.
#[test]
fn test_roundtrip_raw_heap_snapshot() {
    let mut heap = RawHeap::new();
    let mut pointers = Vec::new();

    // spill into a second page
    for index in 0..PAGE_SPILL_ALLOCATIONS {
        let pointer = heap.allocate_with_bytes(&[index as u8, 0xAA]);
        pointers.push(pointer);
    }

    // free and reuse one slot to preserve allocator state
    let freed_pointer = pointers[17];
    assert!(heap.free(freed_pointer));
    let reused_pointer = heap.allocate_with_bytes(&[0xFE, 0xED]);
    assert_eq!(reused_pointer.id(), freed_pointer.id());

    let snapshot = heap.snapshot();
    let restored = RawHeap::restore(&snapshot);

    // verify preserved live cells and allocator state
    assert_eq!(restored.cell_count(), heap.cell_count());
    let storage = &restored.get(reused_pointer).unwrap().storage;
    match storage {
        RawCellStorage::Bytes(bytes) => assert_eq!(bytes.as_slice(), &[0xFE, 0xED]),
        _ => panic!("expected raw byte storage"),
    }
}
