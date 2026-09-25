use crate::local::storage::HeapPlace;
use crate::{
    AllocationShape, HeapAllocationError, HeapError, HeapOptions, HeapReference, Payload,
    SizeClassTable, test_aligned_layout, test_layout,
};
use tspp_mir::{DiscriminantField, TraceMap, VariantEncoding, VariantTrace};

use super::{TestHeapPlan, read_mapped_bytes, test_heap, test_storage, trace_view};

/// Keep allocation-specific trace maps on dedicated large blocks.
#[test]
fn test_allocate_heap_preserves_dynamic_trace_map() {
    let trace_map = TraceMap::Repeated {
        count: 2,
        stride: 8,
        element: Box::new(TraceMap::Fixed {
            local_offsets: vec![0].into_boxed_slice(),
            shared_offsets: vec![].into_boxed_slice(),
            frame_offsets: vec![].into_boxed_slice(),
            borrow_offsets: vec![].into(),
        }),
    };
    let shape = AllocationShape::new(16, 8, None, trace_map.clone());
    let mut heap = test_heap(HeapOptions::local());
    let plan = heap.options().allocation_plan(&shape);

    let reference = heap
        .allocate_zeroed(plan, &trace_map)
        .expect("dynamic trace allocation should succeed");
    let place = heap
        .storage
        .place(reference)
        .expect("dynamic trace allocation should be live");
    let decoded = heap
        .trace_map(reference, trace_view())
        .expect("dynamic trace metadata should resolve");

    assert!(matches!(place, HeapPlace::LargeBlock(_)));
    assert_eq!(decoded, trace_map);
}

/// Reject one zero-size heap block.
#[test]
fn test_allocate_heap_rejects_zero_size_layout() {
    // build a valid heap with an invalid block layout
    let options = HeapOptions::local();
    let layout = test_layout(0, TraceMap::empty());
    let shape = layout.block();
    let mut heap = test_storage(&options);

    // reject zero-size heap objects loudly
    let error = heap
        .allocate(&heap.test_allocation_plan(&shape), Payload::Bytes(&[]))
        .expect_err("heap block should reject zero-size layouts");

    assert_eq!(
        error,
        HeapError::invalid_allocation(HeapAllocationError::ZeroSize)
    );
}

/// Reclaim one freed heap block and allow another block.
#[test]
fn test_free_heap_reclaims_live_allocation() {
    // allocate one live heap payload
    let options = HeapOptions::local();
    let layout = test_layout(1, TraceMap::empty());
    let mut heap = test_storage(&options);
    let reference = heap.test_allocate(layout.block(), Payload::Bytes(&[0xAB]));

    // free the block and retire its old reference
    assert!(heap.is_live(reference));
    heap.free(reference).expect("heap free should succeed");
    assert!(!heap.is_live(reference));

    // allocate again to prove the heap remains usable
    let next_reference = heap.test_allocate(layout.block(), Payload::Bytes(&[0xCD]));

    assert!(heap.is_live(next_reference));
}

/// Clear one reused small heap slot before writing a shorter payload.
#[test]
fn test_allocate_heap_clears_reused_small_slot_tail() {
    // force small mature block reuse
    let options = HeapOptions {
        heap_small_size_bytes: 16,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let full_layout = test_layout(8, TraceMap::empty());
    let short_layout = test_layout(1, TraceMap::empty());
    let mut heap = test_storage(&options);

    let first = heap.test_allocate(full_layout.block(), Payload::Bytes(&[0xAA; 8]));
    let second = heap.test_allocate(short_layout.block(), Payload::Bytes(&[0xBB]));

    // free the full slot so a shorter payload can reuse it
    heap.free(first).expect("heap free should succeed");

    let reused = heap.test_allocate(short_layout.block(), Payload::Bytes(&[0xCC]));

    assert!(heap.is_live(second));
    assert!(heap.is_live(reused));
    let address = heap.base_address() + reused.offset();

    // reused slot tail bytes should be cleared
    let bytes = read_mapped_bytes(address, 8);

    assert_eq!(bytes, &[0xCC, 0, 0, 0, 0, 0, 0, 0]);
}

/// Keep variant trace maps on block records.
#[test]
fn test_allocate_heap_routes_variant_trace_map_to_large() {
    // variant trace maps require dedicated block records
    let options = HeapOptions {
        heap_small_size_bytes: 64,
        size_classes: SizeClassTable::new([16]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let trace_map = TraceMap::Variant {
        encoding: VariantEncoding::Direct {
            field: DiscriminantField::scalar(0, 1),
        },
        cases: vec![VariantTrace {
            discriminant: 0u128.into(),
            payload_offset: 8,
            map: TraceMap::Fixed {
                local_offsets: vec![0].into_boxed_slice(),
                shared_offsets: Box::default(),
                frame_offsets: Box::default(),
                borrow_offsets: Box::default(),
            },
        }]
        .into_boxed_slice(),
    };
    let layout = test_layout(16, trace_map);
    let mut heap = test_storage(&options);

    let reference = heap.test_allocate(layout.block(), Payload::Zeroed);

    // variant payloads should take the large path
    assert!(matches!(
        heap.place(reference),
        Some(HeapPlace::LargeBlock(_))
    ));
}

/// Keep over-aligned blocks on aligned slots.
#[test]
fn test_allocate_heap_honors_layout_alignment() {
    // use a size class that can satisfy 16-byte alignment
    let options = HeapOptions {
        heap_small_size_bytes: 64,
        size_classes: SizeClassTable::new([8, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let layout = test_aligned_layout(17, 16, TraceMap::empty());
    let mut heap = test_storage(&options);

    let first = heap.test_allocate(layout.block(), Payload::Bytes(&[0xAA; 17]));
    let second = heap.test_allocate(layout.block(), Payload::Bytes(&[0xBB; 17]));

    // every returned base should satisfy the layout alignment
    assert_eq!(first.offset() % 16, 0);
    assert_eq!(second.offset() % 16, 0);
    assert_eq!(second.offset() - first.offset(), 32);
}

/// Reject one write that crosses block bounds from an interior reference.
#[test]
fn test_write_heap_rejects_interior_reference_crossing_bounds() {
    // allocate one small payload and form an interior reference near the end
    let options = HeapOptions {
        heap_small_size_bytes: 8,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let layout = test_layout(1, TraceMap::empty());
    let mut heap = test_storage(&options);
    let reference = heap.test_allocate(layout.block(), Payload::Bytes(&[0xAB]));
    let reference = reference.add_bytes(7);

    // writing two bytes from offset 7 crosses the block boundary
    let error = heap
        .write_barrier(reference, 0, 2, trace_view())
        .expect_err("heap write should reject bounds crossing");

    assert_eq!(
        error,
        HeapError::InvalidByteRange {
            start: 7,
            len: 2,
            capacity: 8
        }
    );
}

/// Reclaim one freed heap large block and allow another large block.
#[test]
fn test_free_heap_reclaims_large_block() {
    // force blocks larger than the local small span classes
    let options = HeapOptions {
        heap_small_size_bytes: 32,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    };
    let large_byte_len = options
        .size_classes
        .max_small_allocation_bytes()
        .expect("size class table should not be empty")
        + 1;
    let layout = test_layout(large_byte_len, TraceMap::empty());
    let mut heap = test_storage(&options);
    let first = heap.test_allocate(layout.block(), Payload::Bytes(&vec![0xAB; large_byte_len]));

    assert!(matches!(heap.place(first), Some(HeapPlace::LargeBlock(_))));

    // free the large block before allocating another one
    heap.free(first).expect("heap large free should succeed");

    let second = heap.test_allocate(layout.block(), Payload::Bytes(&vec![0xCD; large_byte_len]));

    // large block placement should stay on the large path
    assert!(matches!(heap.place(second), Some(HeapPlace::LargeBlock(_))));
}

/// Reject one invalid heap reference loudly.
#[test]
fn test_free_heap_rejects_invalid_reference() {
    // build a heap with no block at offset 7
    let options = HeapOptions::local();
    let mut heap = test_storage(&options);

    // reject the unknown address
    let reference = HeapReference::new(7);
    let error = heap.free(reference).expect_err("heap free should fail");

    assert_eq!(error, HeapError::invalid_heap_reference(reference));
}
