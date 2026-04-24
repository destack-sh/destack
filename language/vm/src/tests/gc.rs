use crate::Value;
use crate::tests::create_empty_test_heap;
use destack_heap::{AllocationLayout, Heap, HeapError, HeapReference, Payload};
use destack_mir::ReferenceMap;

/// Allocate one zero-byte managed cell for tests.
fn allocate_empty(heap: &mut Heap) -> HeapReference {
    let reference_map = ReferenceMap::empty();
    let layout = AllocationLayout::new(0, &reference_map);

    heap.allocate(layout, Payload::Zeroed)
        .expect("heap allocation should succeed")
}

/// Allocate one managed cell for tests.
fn allocate(heap: &mut Heap) -> HeapReference {
    let reference_map = ReferenceMap::empty();
    let layout = AllocationLayout::new(1, &reference_map);

    heap.allocate(layout, Payload::Bytes(&[0]))
        .expect("heap allocation should succeed")
}

/// Allocate one managed cell with values for tests.
fn allocate_with_values(heap: &mut Heap, values: Vec<Value>) -> HeapReference {
    let mut bytes = Vec::with_capacity(values.len() * Value::BYTE_LEN);
    let mut offsets = Vec::new();

    // encode one explicit value-backed payload
    for (index, value) in values.into_iter().enumerate() {
        if value.as_heap_reference().is_some() {
            offsets.push((index * Value::BYTE_LEN) as u32);
        }

        bytes.extend_from_slice(&value.to_byte_array());
    }

    let reference_map = if offsets.is_empty() {
        ReferenceMap::empty()
    } else {
        ReferenceMap::Reference {
            local_offsets: offsets.into_boxed_slice(),
            shared_offsets: Vec::new().into_boxed_slice(),
        }
    };
    let layout = AllocationLayout::new(bytes.len(), &reference_map);

    heap.allocate(layout, Payload::Bytes(&bytes))
        .expect("heap allocation should succeed")
}

/// Return whether one managed cell exists.
fn contains(heap: &Heap, reference: HeapReference) -> bool {
    heap.is_heap_live(reference)
}

/// Return the heap allocation count for tests.
fn allocation_count(heap: &Heap) -> usize {
    heap.heap_allocation_count()
}

/// Decode one heap reference from the first packed value lane.
fn decode_first_heap_reference(bytes: &[u8]) -> HeapReference {
    let bits = u64::from_le_bytes(
        bytes[..HeapReference::BYTE_LEN]
            .try_into()
            .expect("reference payload should fit"),
    );

    HeapReference::from_bits(bits as usize)
}

/// Assert that one promoted payload starts with the exact requested bytes.
fn assert_payload_prefix(heap: &Heap, reference: HeapReference, expected: &[u8]) {
    let bytes = heap
        .read_heap_bytes(reference)
        .expect("promoted payload should read");

    assert!(bytes.starts_with(expected));
}

/// Zero-byte heap allocations still use distinct references.
#[test]
fn test_zero_byte_heap_allocations_use_distinct_references() {
    let mut heap = create_empty_test_heap();

    let first = allocate_empty(&mut heap);
    let second = allocate_empty(&mut heap);
    let third = allocate_empty(&mut heap);

    assert_ne!(first, second);
    assert_ne!(first, third);
    assert_ne!(second, third);
}

/// Garbage collection removes cells not reachable from roots.
#[test]
fn test_gc_collects_unreachable() {
    let mut heap = create_empty_test_heap();

    let handle1 = allocate(&mut heap);
    let handle2 = allocate(&mut heap);
    let _handle3 = allocate(&mut heap);
    let mut roots = [handle1, handle2];

    assert_eq!(allocation_count(&heap), 3);

    heap.collect_full(&mut roots)
        .expect("heap collection should succeed");

    assert_eq!(allocation_count(&heap), 2);
    assert!(contains(&heap, roots[0]));
    assert!(contains(&heap, roots[1]));
}

/// Garbage collection preserves all cells directly referenced as roots.
#[test]
fn test_gc_preserves_reachable() {
    let mut heap = create_empty_test_heap();

    let handle1 = allocate(&mut heap);
    let handle2 = allocate(&mut heap);
    let mut roots = [handle1, handle2];

    heap.collect_full(&mut roots)
        .expect("heap collection should succeed");

    assert_eq!(allocation_count(&heap), 2);
    assert!(contains(&heap, roots[0]));
    assert!(contains(&heap, roots[1]));
}

/// Garbage collection follows reference chains to preserve indirectly reachable cells.
#[test]
fn test_gc_follows_references() {
    let mut heap = create_empty_test_heap();

    let child2 = allocate(&mut heap);
    let child1 = allocate_with_values(&mut heap, vec![Value::heap_reference(child2)]);
    let root = allocate_with_values(&mut heap, vec![Value::heap_reference(child1)]);
    let _unreachable = allocate(&mut heap);
    let mut roots = [root];

    assert_eq!(allocation_count(&heap), 4);

    heap.collect_full(&mut roots)
        .expect("heap collection should succeed");

    // full collection should rewrite young roots into stable storage
    let rewritten_root = roots[0];
    let rewritten_child1 = decode_first_heap_reference(
        &heap
            .read_heap_bytes(rewritten_root)
            .expect("rewritten root should read"),
    );
    let rewritten_child2 = decode_first_heap_reference(
        &heap
            .read_heap_bytes(rewritten_child1)
            .expect("rewritten child should read"),
    );

    assert_eq!(allocation_count(&heap), 3);
    assert_ne!(rewritten_root, root);
    assert_ne!(rewritten_child1, child1);
    assert_ne!(rewritten_child2, child2);
    assert!(!contains(&heap, root));
    assert!(!contains(&heap, child1));
    assert!(!contains(&heap, child2));
    assert!(contains(&heap, rewritten_root));
    assert_payload_prefix(&heap, rewritten_child2, &[0]);
}

/// Garbage collection correctly handles cyclic reference structures.
#[test]
fn test_gc_handles_cycles() {
    let mut heap = create_empty_test_heap();

    let reference_map = ReferenceMap::Reference {
        local_offsets: vec![0].into_boxed_slice(),
        shared_offsets: Vec::new().into_boxed_slice(),
    };
    let layout = AllocationLayout::new(Value::BYTE_LEN, &reference_map);
    let a = heap
        .allocate(layout, Payload::Zeroed)
        .expect("heap allocation should succeed");
    let b = heap
        .allocate(layout, Payload::Zeroed)
        .expect("heap allocation should succeed");

    heap.write_heap_bytes(a, 0, &Value::heap_reference(b).to_byte_array())
        .expect("managed byte write should succeed");
    heap.write_heap_bytes(b, 0, &Value::heap_reference(a).to_byte_array())
        .expect("managed byte write should succeed");

    let _unreachable1 = allocate(&mut heap);
    let _unreachable2 = allocate(&mut heap);
    let mut roots = [a];

    assert_eq!(allocation_count(&heap), 4);

    heap.collect_full(&mut roots)
        .expect("heap collection should succeed");

    // full collection should preserve the rewritten cycle exactly
    let rewritten_a = roots[0];
    let rewritten_b = decode_first_heap_reference(
        &heap
            .read_heap_bytes(rewritten_a)
            .expect("rewritten cycle head should read"),
    );
    let cycle_back = decode_first_heap_reference(
        &heap
            .read_heap_bytes(rewritten_b)
            .expect("rewritten cycle tail should read"),
    );

    assert_eq!(allocation_count(&heap), 2);
    assert_ne!(rewritten_a, a);
    assert_ne!(rewritten_b, b);
    assert!(!contains(&heap, a));
    assert!(!contains(&heap, b));
    assert!(contains(&heap, rewritten_a));
    assert!(contains(&heap, rewritten_b));
    assert_eq!(cycle_back, rewritten_a);
}

/// Garbage collection with no roots removes all heap allocations.
#[test]
fn test_gc_empty_roots() {
    let mut heap = create_empty_test_heap();

    allocate(&mut heap);
    allocate(&mut heap);
    allocate(&mut heap);

    assert_eq!(allocation_count(&heap), 3);

    heap.collect_full(&mut [])
        .expect("heap collection should succeed");

    assert_eq!(allocation_count(&heap), 0);
}

/// Garbage collection preserves cells referenced by multiple holders.
#[test]
fn test_gc_multiple_references_to_same_cell() {
    let mut heap = create_empty_test_heap();

    let shared = allocate(&mut heap);
    let holder1 = allocate_with_values(&mut heap, vec![Value::heap_reference(shared)]);
    let holder2 = allocate_with_values(&mut heap, vec![Value::heap_reference(shared)]);
    let mut roots = [holder1, holder2];

    assert_eq!(allocation_count(&heap), 3);

    heap.collect_full(&mut roots)
        .expect("heap collection should succeed");

    // both rewritten holders should still agree on one rewritten child
    let rewritten_child1 = decode_first_heap_reference(
        &heap
            .read_heap_bytes(roots[0])
            .expect("first rewritten holder should read"),
    );
    let rewritten_child2 = decode_first_heap_reference(
        &heap
            .read_heap_bytes(roots[1])
            .expect("second rewritten holder should read"),
    );

    assert_eq!(allocation_count(&heap), 3);
    assert!(contains(&heap, roots[0]));
    assert!(contains(&heap, roots[1]));
    assert_ne!(rewritten_child1, shared);
    assert_eq!(rewritten_child1, rewritten_child2);
    assert!(!contains(&heap, shared));
    assert_payload_prefix(&heap, rewritten_child1, &[0]);
}

/// Garbage collection traces references nested inside aggregate values.
#[test]
fn test_gc_handles_aggregates() {
    let mut heap = create_empty_test_heap();

    let child = allocate(&mut heap);
    let inner_agg = allocate_with_values(
        &mut heap,
        vec![Value::int32(42), Value::heap_reference(child)],
    );
    let parent = allocate_with_values(&mut heap, vec![Value::heap_reference(inner_agg)]);

    let _unreachable = allocate(&mut heap);
    let mut roots = [parent];

    assert_eq!(allocation_count(&heap), 4);

    heap.collect_full(&mut roots)
        .expect("heap collection should succeed");

    // rewritten aggregate links should still decode the nested child
    let rewritten_parent = roots[0];
    let rewritten_inner = decode_first_heap_reference(
        &heap
            .read_heap_bytes(rewritten_parent)
            .expect("rewritten parent should read"),
    );
    let inner_bytes = heap
        .read_heap_bytes(rewritten_inner)
        .expect("rewritten aggregate payload should read");
    let rewritten_child = decode_first_heap_reference(&inner_bytes[Value::BYTE_LEN..]);

    assert_eq!(allocation_count(&heap), 3);
    assert_ne!(rewritten_parent, parent);
    assert_ne!(rewritten_inner, inner_agg);
    assert_ne!(rewritten_child, child);
    assert!(!contains(&heap, parent));
    assert!(!contains(&heap, inner_agg));
    assert!(!contains(&heap, child));
    assert!(contains(&heap, rewritten_parent));
    assert_eq!(
        Value::from_byte_slice(&inner_bytes[..Value::BYTE_LEN]),
        Some(Value::int32(42))
    );
    assert_payload_prefix(&heap, rewritten_child, &[0]);
}

/// Garbage collection rejects invalid references in the roots list.
#[test]
fn test_gc_invalid_root_fails() {
    let mut heap = create_empty_test_heap();

    let valid = allocate(&mut heap);
    let invalid = HeapReference::new(9999);

    assert_eq!(allocation_count(&heap), 1);

    let error = heap
        .collect_full(&mut [valid, invalid])
        .expect_err("invalid roots should fail collection");

    assert_eq!(
        error,
        HeapError::InvalidHeapReference { reference: invalid }
    );
    assert_eq!(allocation_count(&heap), 1);
    assert!(contains(&heap, valid));
}

/// Repeated garbage collections correctly remove newly allocated garbage.
#[test]
fn test_gc_repeated_collection() {
    let mut heap = create_empty_test_heap();

    let root = allocate(&mut heap);
    let _garbage = allocate(&mut heap);
    let mut roots = [root];

    heap.collect_full(&mut roots)
        .expect("heap collection should succeed");
    assert_eq!(allocation_count(&heap), 1);

    let _more_garbage = allocate(&mut heap);
    let _even_more = allocate(&mut heap);

    heap.collect_full(&mut roots)
        .expect("heap collection should succeed");
    assert_eq!(allocation_count(&heap), 1);
}
