use crate::memory::{Heap, HeapHandle, Value};

/// Garbage collection removes cells not reachable from roots.
#[test]
fn test_gc_collects_unreachable() {
    let mut heap = Heap::new();

    // allocate some cells
    let handle1 = heap.allocate();
    let handle2 = heap.allocate();
    let _handle3 = heap.allocate();

    assert_eq!(heap.cell_count(), 3);

    // only keep handle1 and handle2 as roots
    heap.collect(&[handle1, handle2]);

    // handle3 should be collected
    assert_eq!(heap.cell_count(), 2);
    assert!(heap.get(handle1).is_some());
    assert!(heap.get(handle2).is_some());
}

/// Garbage collection preserves all cells directly referenced as roots.
#[test]
fn test_gc_preserves_reachable() {
    let mut heap = Heap::new();

    let handle1 = heap.allocate();
    let handle2 = heap.allocate();

    heap.collect(&[handle1, handle2]);

    assert_eq!(heap.cell_count(), 2);
    assert!(heap.get(handle1).is_some());
    assert!(heap.get(handle2).is_some());
}

/// Garbage collection follows reference chains to preserve indirectly reachable cells.
#[test]
fn test_gc_follows_references() {
    let mut heap = Heap::new();

    // create a chain: root -> child1 -> child2
    let child2 = heap.allocate();
    let child1 = heap.allocate_with_values(vec![Value::ManagedReference(child2)]);
    let root = heap.allocate_with_values(vec![Value::ManagedReference(child1)]);

    // also create an unreachable cell
    let _unreachable = heap.allocate();

    assert_eq!(heap.cell_count(), 4);

    // only root is in the roots list, but child1 and child2 should be preserved
    heap.collect(&[root]);

    assert_eq!(heap.cell_count(), 3);
    assert!(heap.get(root).is_some());
    assert!(heap.get(child1).is_some());
    assert!(heap.get(child2).is_some());
}

/// Garbage collection correctly handles cyclic reference structures.
#[test]
fn test_gc_handles_cycles() {
    let mut heap = Heap::new();

    // create a cycle: a -> b -> a
    let a = heap.allocate();
    let b = heap.allocate();

    // set up the cycle
    heap.get_mut(a).unwrap().slots.push(Value::ManagedReference(b));
    heap.get_mut(b).unwrap().slots.push(Value::ManagedReference(a));

    // create unreachable cells
    let _unreachable1 = heap.allocate();
    let _unreachable2 = heap.allocate();

    assert_eq!(heap.cell_count(), 4);

    // collect with only 'a' as root
    heap.collect(&[a]);

    // cycle should be preserved, unreachable should be collected
    assert_eq!(heap.cell_count(), 2);
    assert!(heap.get(a).is_some());
    assert!(heap.get(b).is_some());
}

/// Garbage collection with no roots removes all heap cells.
#[test]
fn test_gc_empty_roots() {
    let mut heap = Heap::new();

    heap.allocate();
    heap.allocate();
    heap.allocate();

    assert_eq!(heap.cell_count(), 3);

    // no roots = collect everything
    heap.collect(&[]);

    assert_eq!(heap.cell_count(), 0);
}

/// Garbage collection preserves cells referenced by multiple holders.
#[test]
fn test_gc_multiple_references_to_same_cell() {
    let mut heap = Heap::new();

    let shared = heap.allocate();
    let holder1 = heap.allocate_with_values(vec![Value::ManagedReference(shared)]);
    let holder2 = heap.allocate_with_values(vec![Value::ManagedReference(shared)]);

    assert_eq!(heap.cell_count(), 3);

    // both holders reference the same shared cell
    heap.collect(&[holder1, holder2]);

    assert_eq!(heap.cell_count(), 3);
    assert!(heap.get(shared).is_some());
    assert!(heap.get(holder1).is_some());
    assert!(heap.get(holder2).is_some());
}

/// Garbage collection traces references nested inside aggregate values.
#[test]
fn test_gc_handles_aggregates() {
    let mut heap = Heap::new();

    let child = heap.allocate();
    // put a reference inside an aggregate value
    let parent = heap.allocate_with_values(vec![Value::Aggregate(
        vec![Value::int32(42), Value::ManagedReference(child)].into_boxed_slice(),
    )]);

    let _unreachable = heap.allocate();

    assert_eq!(heap.cell_count(), 3);

    heap.collect(&[parent]);

    // parent and child should be preserved
    assert_eq!(heap.cell_count(), 2);
    assert!(heap.get(parent).is_some());
    assert!(heap.get(child).is_some());
}

/// Garbage collection ignores invalid handles in the roots list.
#[test]
fn test_gc_invalid_root_ignored() {
    let mut heap = Heap::new();

    let valid = heap.allocate();

    // create an invalid handle
    let invalid = HeapHandle::new(9999);

    assert_eq!(heap.cell_count(), 1);

    // gc should not crash with invalid roots
    heap.collect(&[valid, invalid]);

    assert_eq!(heap.cell_count(), 1);
    assert!(heap.get(valid).is_some());
}

/// Repeated garbage collections correctly remove newly allocated garbage.
#[test]
fn test_gc_repeated_collection() {
    let mut heap = Heap::new();

    let root = heap.allocate();
    let _garbage = heap.allocate();

    heap.collect(&[root]);
    assert_eq!(heap.cell_count(), 1);

    // allocate more garbage
    let _more_garbage = heap.allocate();
    let _even_more = heap.allocate();

    heap.collect(&[root]);
    assert_eq!(heap.cell_count(), 1);
}
