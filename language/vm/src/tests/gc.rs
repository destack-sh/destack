use crate::Word;
use crate::tests::create_test_heap;
use destack_heap::{AllocationPlan, Heap, HeapError, HeapReference, Payload};
use destack_mir::ReferenceMap;

/// Allocate one managed cell for tests.
fn allocate(heap: &mut Heap) -> HeapReference {
    let reference_map = ReferenceMap::empty();
    let plan = AllocationPlan::new(1, 1, &reference_map);
    let layout = heap.allocation_layout(plan);

    heap.allocate(&layout, Payload::Bytes(&[0]))
        .expect("heap allocation should succeed")
}

/// Allocate one managed cell with word contents for tests.
fn allocate_with_values(heap: &mut Heap, values: Vec<Word>) -> HeapReference {
    let mut bytes = Vec::with_capacity(values.len() * Word::BYTE_LEN);
    let mut offsets = Vec::new();
    for (index, value) in values.into_iter().enumerate() {
        if heap.is_heap_live(value.as_heap_reference()) {
            offsets.push((index * Word::BYTE_LEN) as u32);
        }

        bytes.extend_from_slice(&value.to_byte_array());
    }

    let reference_map = if offsets.is_empty() {
        ReferenceMap::empty()
    } else {
        ReferenceMap::Direct {
            local_offsets: offsets.into_boxed_slice(),
            shared_offsets: Vec::new().into_boxed_slice(),
        }
    };
    let plan = AllocationPlan::new(bytes.len(), Word::BYTE_LEN, &reference_map);
    let layout = heap.allocation_layout(plan);

    heap.allocate(&layout, Payload::Bytes(&bytes))
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

/// Read managed heap bytes for GC assertions.
fn read_cell_bytes(heap: &Heap, reference: HeapReference, byte_len: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; byte_len];
    let address = heap.heap_base_address() + reference.offset();
    unsafe {
        std::ptr::copy_nonoverlapping(address as *const u8, bytes.as_mut_ptr(), byte_len);
    }

    bytes
}

/// Write managed heap bytes for GC assertions.
fn write_cell_bytes(heap: &mut Heap, reference: HeapReference, bytes: &[u8]) {
    heap.write_barrier(reference, 0, bytes.len())
        .expect("heap barrier should record");
    let address = heap.heap_base_address() + reference.offset();
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len());
    }
}

/// Decode one heap reference from the first word.
fn decode_first_heap_reference(bytes: &[u8]) -> HeapReference {
    let bits = u64::from_le_bytes(
        bytes[..HeapReference::BYTE_LEN]
            .try_into()
            .expect("reference word should fit"),
    );

    HeapReference::from_bits(bits as usize)
}

/// Assert that one promoted cell starts with the exact requested bytes.
fn assert_cell_prefix(heap: &Heap, reference: HeapReference, expected: &[u8]) {
    let bytes = read_cell_bytes(heap, reference, expected.len());

    assert!(bytes.starts_with(expected));
}

/// Zero-byte heap allocations are rejected.
#[test]
fn test_reject_zero_byte_heap_allocation() {
    let mut heap = create_test_heap();
    let reference_map = ReferenceMap::empty();
    let plan = AllocationPlan::new(0, 1, &reference_map);
    let layout = heap.allocation_layout(plan);

    let result = heap.allocate(&layout, Payload::Zeroed);

    assert_eq!(result, Err(HeapError::ZeroSizeAllocation));
}

/// Garbage collection removes cells not reachable from roots.
#[test]
fn test_gc_collects_unreachable() {
    let mut heap = create_test_heap();

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
    let mut heap = create_test_heap();

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
    let mut heap = create_test_heap();

    let child2 = allocate(&mut heap);
    let child1 = allocate_with_values(&mut heap, vec![Word::heap_reference(child2)]);
    let root = allocate_with_values(&mut heap, vec![Word::heap_reference(child1)]);
    let _unreachable = allocate(&mut heap);
    let mut roots = [root];

    assert_eq!(allocation_count(&heap), 4);

    heap.collect_full(&mut roots)
        .expect("heap collection should succeed");
    let rewritten_root = roots[0];
    let rewritten_root_bytes = read_cell_bytes(&heap, rewritten_root, Word::BYTE_LEN);
    let rewritten_child1 = decode_first_heap_reference(&rewritten_root_bytes);
    let rewritten_child1_bytes = read_cell_bytes(&heap, rewritten_child1, Word::BYTE_LEN);
    let rewritten_child2 = decode_first_heap_reference(&rewritten_child1_bytes);

    assert_eq!(allocation_count(&heap), 3);
    assert_ne!(rewritten_root, root);
    assert_ne!(rewritten_child1, child1);
    assert_ne!(rewritten_child2, child2);
    assert!(!contains(&heap, root));
    assert!(!contains(&heap, child1));
    assert!(!contains(&heap, child2));
    assert!(contains(&heap, rewritten_root));
    assert_cell_prefix(&heap, rewritten_child2, &[0]);
}

/// Garbage collection correctly handles cyclic reference structures.
#[test]
fn test_gc_handles_cycles() {
    let mut heap = create_test_heap();

    let reference_map = ReferenceMap::Direct {
        local_offsets: vec![0].into_boxed_slice(),
        shared_offsets: Vec::new().into_boxed_slice(),
    };
    let plan = AllocationPlan::new(Word::BYTE_LEN, Word::BYTE_LEN, &reference_map);
    let layout = heap.allocation_layout(plan);
    let a = heap
        .allocate(&layout, Payload::Zeroed)
        .expect("heap allocation should succeed");
    let b = heap
        .allocate(&layout, Payload::Zeroed)
        .expect("heap allocation should succeed");

    write_cell_bytes(&mut heap, a, &Word::heap_reference(b).to_byte_array());
    write_cell_bytes(&mut heap, b, &Word::heap_reference(a).to_byte_array());

    let _unreachable1 = allocate(&mut heap);
    let _unreachable2 = allocate(&mut heap);
    let mut roots = [a];

    assert_eq!(allocation_count(&heap), 4);

    heap.collect_full(&mut roots)
        .expect("heap collection should succeed");
    let rewritten_a = roots[0];
    let rewritten_a_bytes = read_cell_bytes(&heap, rewritten_a, Word::BYTE_LEN);
    let rewritten_b = decode_first_heap_reference(&rewritten_a_bytes);
    let rewritten_b_bytes = read_cell_bytes(&heap, rewritten_b, Word::BYTE_LEN);
    let cycle_back = decode_first_heap_reference(&rewritten_b_bytes);

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
    let mut heap = create_test_heap();

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
    let mut heap = create_test_heap();

    let shared = allocate(&mut heap);
    let holder1 = allocate_with_values(&mut heap, vec![Word::heap_reference(shared)]);
    let holder2 = allocate_with_values(&mut heap, vec![Word::heap_reference(shared)]);
    let mut roots = [holder1, holder2];

    assert_eq!(allocation_count(&heap), 3);

    heap.collect_full(&mut roots)
        .expect("heap collection should succeed");
    let first_holder_bytes = read_cell_bytes(&heap, roots[0], Word::BYTE_LEN);
    let second_holder_bytes = read_cell_bytes(&heap, roots[1], Word::BYTE_LEN);
    let rewritten_child1 = decode_first_heap_reference(&first_holder_bytes);
    let rewritten_child2 = decode_first_heap_reference(&second_holder_bytes);

    assert_eq!(allocation_count(&heap), 3);
    assert!(contains(&heap, roots[0]));
    assert!(contains(&heap, roots[1]));
    assert_ne!(rewritten_child1, shared);
    assert_eq!(rewritten_child1, rewritten_child2);
    assert!(!contains(&heap, shared));
    assert_cell_prefix(&heap, rewritten_child1, &[0]);
}

/// Garbage collection traces references nested inside heap values.
#[test]
fn test_gc_traces_nested_heap_references() {
    let mut heap = create_test_heap();

    let child = allocate(&mut heap);
    let inner = allocate_with_values(
        &mut heap,
        vec![Word::int32(42), Word::heap_reference(child)],
    );
    let parent = allocate_with_values(&mut heap, vec![Word::heap_reference(inner)]);

    let _unreachable = allocate(&mut heap);
    let mut roots = [parent];

    assert_eq!(allocation_count(&heap), 4);

    heap.collect_full(&mut roots)
        .expect("heap collection should succeed");
    let rewritten_parent = roots[0];
    let rewritten_parent_bytes = read_cell_bytes(&heap, rewritten_parent, Word::BYTE_LEN);
    let rewritten_inner = decode_first_heap_reference(&rewritten_parent_bytes);
    let inner_bytes = read_cell_bytes(&heap, rewritten_inner, 2 * Word::BYTE_LEN);
    let rewritten_child = decode_first_heap_reference(&inner_bytes[Word::BYTE_LEN..]);

    assert_eq!(allocation_count(&heap), 3);
    assert_ne!(rewritten_parent, parent);
    assert_ne!(rewritten_inner, inner);
    assert_ne!(rewritten_child, child);
    assert!(!contains(&heap, parent));
    assert!(!contains(&heap, inner));
    assert!(!contains(&heap, child));
    assert!(contains(&heap, rewritten_parent));
    assert_eq!(
        Word::from_byte_slice(&inner_bytes[..Word::BYTE_LEN]),
        Some(Word::int32(42))
    );
    assert_cell_prefix(&heap, rewritten_child, &[0]);
}

/// Garbage collection rejects invalid references in the roots list.
#[test]
fn test_gc_invalid_root_fails() {
    let mut heap = create_test_heap();

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
    let mut heap = create_test_heap();

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
