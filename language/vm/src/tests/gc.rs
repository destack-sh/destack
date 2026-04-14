use destack_heap::{Heap, ManagedReference, ReferenceMap, Value};

/// Allocate one empty managed cell for tests.
fn allocate(heap: &mut Heap) -> ManagedReference {
    heap.allocate_managed_zeroed(0, destack_heap::ReferenceMap::empty(), None)
        .expect("managed allocation should succeed")
}

/// Allocate one managed cell with values for tests.
fn allocate_with_values(heap: &mut Heap, values: Vec<Value>) -> ManagedReference {
    let mut bytes = Vec::with_capacity(values.len() * Value::BYTE_LEN);
    let mut offsets = Vec::new();

    // encode one explicit value-backed payload
    for (index, value) in values.into_iter().enumerate() {
        if value.as_managed_reference().is_some() {
            offsets.push((index * Value::BYTE_LEN) as u32);
        }

        bytes.extend_from_slice(&value.to_byte_array());
    }

    let reference_map = if offsets.is_empty() {
        ReferenceMap::empty()
    } else {
        ReferenceMap::ValueOffsets {
            offsets: offsets.into_boxed_slice(),
        }
    };

    heap.allocate_managed_bytes(&bytes, reference_map, None)
        .expect("managed allocation should succeed")
}

/// Return whether one managed cell exists.
fn contains(heap: &Heap, reference: ManagedReference) -> bool {
    heap.is_managed_allocated(reference)
}

/// Return the managed allocation count for tests.
fn allocation_count(heap: &Heap) -> usize {
    heap.managed_allocation_count()
}

/// Garbage collection removes cells not reachable from roots.
#[test]
fn test_gc_collects_unreachable() {
    let mut heap = Heap::new();

    let handle1 = allocate(&mut heap);
    let handle2 = allocate(&mut heap);
    let _handle3 = allocate(&mut heap);

    assert_eq!(allocation_count(&heap), 3);

    heap.collect_managed_references([handle1, handle2])
        .expect("managed collection should succeed");

    assert_eq!(allocation_count(&heap), 2);
    assert!(contains(&heap, handle1));
    assert!(contains(&heap, handle2));
}

/// Garbage collection preserves all cells directly referenced as roots.
#[test]
fn test_gc_preserves_reachable() {
    let mut heap = Heap::new();

    let handle1 = allocate(&mut heap);
    let handle2 = allocate(&mut heap);

    heap.collect_managed_references([handle1, handle2])
        .expect("managed collection should succeed");

    assert_eq!(allocation_count(&heap), 2);
    assert!(contains(&heap, handle1));
    assert!(contains(&heap, handle2));
}

/// Garbage collection follows reference chains to preserve indirectly reachable cells.
#[test]
fn test_gc_follows_references() {
    let mut heap = Heap::new();

    let child2 = allocate(&mut heap);
    let child1 = allocate_with_values(&mut heap, vec![Value::managed_reference(child2)]);
    let root = allocate_with_values(&mut heap, vec![Value::managed_reference(child1)]);
    let _unreachable = allocate(&mut heap);

    assert_eq!(allocation_count(&heap), 4);

    heap.collect_managed_references([root])
        .expect("managed collection should succeed");

    assert_eq!(allocation_count(&heap), 3);
    assert!(contains(&heap, root));
    assert!(contains(&heap, child1));
    assert!(contains(&heap, child2));
}

/// Garbage collection correctly handles cyclic reference structures.
#[test]
fn test_gc_handles_cycles() {
    let mut heap = Heap::new();

    let a = heap
        .allocate_managed_zeroed(
            Value::BYTE_LEN,
            ReferenceMap::ValueOffsets {
                offsets: vec![0].into_boxed_slice(),
            },
            None,
        )
        .expect("managed allocation should succeed");
    let b = heap
        .allocate_managed_zeroed(
            Value::BYTE_LEN,
            ReferenceMap::ValueOffsets {
                offsets: vec![0].into_boxed_slice(),
            },
            None,
        )
        .expect("managed allocation should succeed");

    assert!(heap.set_managed_bytes(a, 0, &Value::managed_reference(b).to_byte_array()));
    assert!(heap.set_managed_bytes(b, 0, &Value::managed_reference(a).to_byte_array()));

    let _unreachable1 = allocate(&mut heap);
    let _unreachable2 = allocate(&mut heap);

    assert_eq!(allocation_count(&heap), 4);

    heap.collect_managed_references([a])
        .expect("managed collection should succeed");

    assert_eq!(allocation_count(&heap), 2);
    assert!(contains(&heap, a));
    assert!(contains(&heap, b));
}

/// Garbage collection with no roots removes all managed allocations.
#[test]
fn test_gc_empty_roots() {
    let mut heap = Heap::new();

    allocate(&mut heap);
    allocate(&mut heap);
    allocate(&mut heap);

    assert_eq!(allocation_count(&heap), 3);

    heap.collect_managed_references([])
        .expect("managed collection should succeed");

    assert_eq!(allocation_count(&heap), 0);
}

/// Garbage collection preserves cells referenced by multiple holders.
#[test]
fn test_gc_multiple_references_to_same_cell() {
    let mut heap = Heap::new();

    let shared = allocate(&mut heap);
    let holder1 = allocate_with_values(&mut heap, vec![Value::managed_reference(shared)]);
    let holder2 = allocate_with_values(&mut heap, vec![Value::managed_reference(shared)]);

    assert_eq!(allocation_count(&heap), 3);

    heap.collect_managed_references([holder1, holder2])
        .expect("managed collection should succeed");

    assert_eq!(allocation_count(&heap), 3);
    assert!(contains(&heap, shared));
    assert!(contains(&heap, holder1));
    assert!(contains(&heap, holder2));
}

/// Garbage collection traces references nested inside aggregate values.
#[test]
fn test_gc_handles_aggregates() {
    let mut heap = Heap::new();

    let child = allocate(&mut heap);
    let inner_agg = allocate_with_values(
        &mut heap,
        vec![Value::int32(42), Value::managed_reference(child)],
    );
    let parent = allocate_with_values(&mut heap, vec![Value::managed_reference(inner_agg)]);

    let _unreachable = allocate(&mut heap);

    assert_eq!(allocation_count(&heap), 4);

    heap.collect_managed_references([parent])
        .expect("managed collection should succeed");

    assert_eq!(allocation_count(&heap), 3);
    assert!(contains(&heap, parent));
    assert!(contains(&heap, inner_agg));
    assert!(contains(&heap, child));
}

/// Garbage collection ignores invalid references in the roots list.
#[test]
fn test_gc_invalid_root_ignored() {
    let mut heap = Heap::new();

    let valid = allocate(&mut heap);
    let invalid = ManagedReference::new(9999);

    assert_eq!(allocation_count(&heap), 1);

    heap.collect_managed_references([valid, invalid])
        .expect("managed collection should succeed");

    assert_eq!(allocation_count(&heap), 1);
    assert!(contains(&heap, valid));
}

/// Repeated garbage collections correctly remove newly allocated garbage.
#[test]
fn test_gc_repeated_collection() {
    let mut heap = Heap::new();

    let root = allocate(&mut heap);
    let _garbage = allocate(&mut heap);

    heap.collect_managed_references([root])
        .expect("managed collection should succeed");
    assert_eq!(allocation_count(&heap), 1);

    let _more_garbage = allocate(&mut heap);
    let _even_more = allocate(&mut heap);

    heap.collect_managed_references([root])
        .expect("managed collection should succeed");
    assert_eq!(allocation_count(&heap), 1);
}
