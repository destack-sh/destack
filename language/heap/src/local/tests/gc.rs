use std::sync::Arc;

use crate::local::space::HeapPlace;
use crate::{
    Allocator, GcKind, GcOptions, GcProgress, Heap, HeapError, HeapOptions, HeapReference,
    HeapSpace, Payload, SharedHeapReference, SizeClassTable, TestLayout, test_allocator,
    test_layout, test_layouts,
};
use destack_mir::ReferenceMap;

/// Build one heap whose pacer triggers immediately in step-driven tests.
fn test_heap(layouts: &[(usize, ReferenceMap)]) -> (Heap, Vec<TestLayout>) {
    let options = HeapOptions {
        gc: GcOptions {
            growth_percent: 10,
            trigger_percent: 75,
            soft_limit_bytes: None,
            minimum_heap_bytes: Some(0),
        },
        ..HeapOptions::local()
    };
    let layouts = test_layouts(layouts);
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );
    let heap =
        Heap::with_allocator_limits_and_options(allocator, crate::HeapLimits::default(), options)
            .expect("pacing heap should build");

    (heap, layouts)
}

/// Build heap options with tiny mature spans for focused GC tests.
fn tiny_heap_options() -> HeapOptions {
    HeapOptions {
        heap_small_bytes: 32,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    }
}

/// Report whether one heap reference is currently young.
fn is_young(heap: &HeapSpace, reference: HeapReference) -> bool {
    matches!(heap.place(reference), Some(HeapPlace::Young { .. }))
}

/// Report whether one heap reference is currently mature.
fn is_mature(heap: &HeapSpace, reference: HeapReference) -> bool {
    matches!(
        heap.place(reference),
        Some(HeapPlace::Small(_)) | Some(HeapPlace::Large(_))
    )
}

/// Return the full bytes expected from one managed allocation slot.
fn expected_heap_bytes(heap: &HeapSpace, reference: HeapReference, payload: &[u8]) -> Vec<u8> {
    let byte_len = heap
        .byte_len(reference)
        .expect("heap byte length should exist");
    let mut expected = vec![0; byte_len];
    expected[..payload.len()].copy_from_slice(payload);

    expected
}

/// Promote reachable young allocations and clear unreachable young-space state.
#[test]
fn test_collect_minor_promotes_reachable_entries() {
    let options = tiny_heap_options();
    let allocator = test_allocator(&options);
    let layout = test_layout(3, ReferenceMap::empty());
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let reachable = heap
        .allocate(layout.allocation(), Payload::Bytes(&[1, 2, 3]))
        .expect("heap allocation should succeed");
    let unreachable = heap
        .allocate(layout.allocation(), Payload::Bytes(&[4, 5, 6]))
        .expect("heap allocation should succeed");
    let mut roots = [reachable];

    assert!(is_young(&heap, reachable));
    assert!(is_young(&heap, unreachable));

    let stats = heap
        .collect_minor(&mut roots)
        .expect("young collection should succeed");
    let reachable = roots[0];

    assert_eq!(stats.freed_allocations, 1);
    assert_eq!(
        heap.read_bytes(reachable),
        Ok(expected_heap_bytes(&heap, reachable, &[1, 2, 3]))
    );
    assert!(!heap.is_live(unreachable));
    assert!(is_mature(&heap, reachable));

    let image = heap.image().expect("heap image should capture");
    let young = image.young();

    assert_eq!(
        young.next_offset(),
        options.small_allocation_alignment_bytes
    );
    assert_eq!(
        young.pages().len(),
        young.capacity_bytes().div_ceil(young.page_bytes())
    );
}

/// Promote young allocations reached through traced young references.
#[test]
fn test_collect_minor_promotes_reachable_child_entries() {
    let options = tiny_heap_options();
    let allocator = test_allocator(&options);
    let reference_map = ReferenceMap::Reference {
        local_offsets: vec![0].into_boxed_slice(),
        shared_offsets: Vec::new().into_boxed_slice(),
    };
    let layouts = test_layouts(&[(2, ReferenceMap::empty()), (8, reference_map.clone())]);
    let [child_layout, parent_layout]: [TestLayout; 2] =
        layouts.try_into().expect("test layouts should match");
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let child = heap
        .allocate(child_layout.allocation(), Payload::Bytes(&[0xC1, 0x1D]))
        .expect("heap allocation should succeed");
    let parent = heap
        .allocate(
            parent_layout.allocation(),
            Payload::Bytes(&child.bits().to_le_bytes()),
        )
        .expect("heap allocation should succeed");
    let mut roots = [parent];

    let stats = heap
        .collect_minor(&mut roots)
        .expect("young collection should succeed");
    let parent = roots[0];

    assert_eq!(stats.freed_allocations, 0);
    assert!(is_mature(&heap, parent));

    let child_bytes = heap.read_bytes(parent).expect("parent read should succeed");
    let rewritten_child = HeapReference::from_bits(usize::from_le_bytes(
        child_bytes[..HeapReference::BYTE_LEN]
            .try_into()
            .expect("child reference should fit"),
    ));

    assert_eq!(
        heap.read_bytes(rewritten_child),
        Ok(expected_heap_bytes(&heap, rewritten_child, &[0xC1, 0x1D]))
    );
}

/// Reject invalid explicit young roots loudly.
#[test]
fn test_collect_minor_rejects_invalid_root() {
    let options = HeapOptions::local();
    let allocator = test_allocator(&options);
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let invalid = HeapReference::new(7);
    let mut roots = [invalid];

    let error = heap
        .collect_minor(&mut roots)
        .expect_err("invalid roots should fail collection");

    assert_eq!(
        error,
        HeapError::InvalidHeapReference { reference: invalid }
    );
}

/// Record the completed young GC cycle after one local young collection.
#[test]
fn test_collect_minor_updates_gc_state() {
    let options = HeapOptions::local();
    let allocator = test_allocator(&options);
    let layout = test_layout(3, ReferenceMap::empty());
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let reachable = heap
        .allocate(layout.allocation(), Payload::Bytes(&[1, 2, 3]))
        .expect("heap allocation should succeed");
    let mut roots = [reachable];

    let stats = heap
        .collect_minor(&mut roots)
        .expect("young collection should succeed");

    assert_eq!(heap.gc_state().completed_cycles, 1);
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Minor));
    assert_eq!(heap.gc_state().last_stats, Some(stats));
}

/// Pinning one young reference should tenure it immediately.
#[test]
fn test_pin_promotes_young_reference() {
    let options = tiny_heap_options();
    let allocator = test_allocator(&options);
    let layout = test_layout(3, ReferenceMap::empty());
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let reference = heap
        .allocate(layout.allocation(), Payload::Bytes(&[1, 2, 3]))
        .expect("heap allocation should succeed");

    assert!(is_young(&heap, reference));

    let reference = heap.pin(reference).expect("pin should succeed");

    assert_eq!(heap.pins.references().collect::<Vec<_>>(), vec![reference]);
    assert_eq!(
        heap.read_bytes(reference),
        Ok(expected_heap_bytes(&heap, reference, &[1, 2, 3]))
    );
    assert!(is_mature(&heap, reference));
}

/// Pinned mature roots should keep young children alive during minor collection.
#[test]
fn test_collect_minor_traces_pinned_roots() {
    let options = tiny_heap_options();
    let allocator = test_allocator(&options);
    let reference_map = ReferenceMap::Reference {
        local_offsets: vec![0].into_boxed_slice(),
        shared_offsets: Vec::new().into_boxed_slice(),
    };
    let layouts = test_layouts(&[(2, ReferenceMap::empty()), (8, reference_map.clone())]);
    let [child_layout, parent_layout]: [TestLayout; 2] =
        layouts.try_into().expect("test layouts should match");
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let child = heap
        .allocate(child_layout.allocation(), Payload::Bytes(&[0xC1, 0x1D]))
        .expect("heap allocation should succeed");
    let parent = heap
        .allocate(
            parent_layout.allocation(),
            Payload::Bytes(&child.bits().to_le_bytes()),
        )
        .expect("heap allocation should succeed");
    let parent = heap.pin(parent).expect("pin should succeed");
    let mut roots = [];

    let stats = heap
        .collect_minor(&mut roots)
        .expect("young collection should succeed");

    assert_eq!(stats.freed_allocations, 0);
    assert!(is_mature(&heap, parent));

    let child_bytes = heap.read_bytes(parent).expect("parent read should succeed");
    let rewritten_child = HeapReference::from_bits(usize::from_le_bytes(
        child_bytes[..HeapReference::BYTE_LEN]
            .try_into()
            .expect("child reference should fit"),
    ));

    assert_eq!(
        heap.read_bytes(rewritten_child),
        Ok(expected_heap_bytes(&heap, rewritten_child, &[0xC1, 0x1D]))
    );
    assert!(is_mature(&heap, rewritten_child));
}

/// Pinned mature references should stay live during full collection without explicit roots.
#[test]
fn test_collect_full_traces_pinned_roots() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let layout = test_layout(3, ReferenceMap::empty());
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let reference = heap
        .allocate(layout.allocation(), Payload::Bytes(&[1, 2, 3]))
        .expect("heap allocation should succeed");
    let reference = heap.pin(reference).expect("pin should succeed");
    let mut roots = [];

    let stats = heap
        .collect_full(&mut roots)
        .expect("full collection should succeed");

    assert_eq!(stats.freed_allocations, 0);
    assert!(heap.is_live(reference));
    assert_eq!(
        heap.read_bytes(reference),
        Ok(expected_heap_bytes(&heap, reference, &[1, 2, 3]))
    );
}

/// Trace shared roots through the full shared-reference width.
#[test]
fn test_scan_shared_roots_uses_shared_reference_width() {
    let options = HeapOptions::local();
    let allocator = test_allocator(&options);
    let reference_map = ReferenceMap::Reference {
        local_offsets: Vec::new().into_boxed_slice(),
        shared_offsets: vec![0].into_boxed_slice(),
    };
    let layout = test_layout(8, reference_map);
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let shared = SharedHeapReference::new(7);
    let local = heap
        .allocate(
            layout.allocation(),
            Payload::Bytes(&shared.bits().to_le_bytes()),
        )
        .expect("heap allocation should succeed");
    let mut roots = Vec::new();

    heap.start_shared_edge_scan();

    let work_done = heap
        .scan_shared_edge_step(&mut roots, 1)
        .expect("shared root scan should succeed");

    assert_eq!(work_done, 1);
    assert_eq!(roots, vec![shared]);
    assert!(heap.shared_edge_scan_idle());

    heap.finish_shared_edge_scan();

    assert!(heap.is_live(local));
}

/// Keep unscanned shared-edge roots stable when earlier roots are freed.
#[test]
fn test_scan_shared_roots_survives_active_root_removal() {
    let options = HeapOptions::local();
    let allocator = test_allocator(&options);
    let reference_map = ReferenceMap::Reference {
        local_offsets: Vec::new().into_boxed_slice(),
        shared_offsets: vec![0].into_boxed_slice(),
    };
    let layout = test_layout(8, reference_map);
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let first_shared = SharedHeapReference::new(11);
    let second_shared = SharedHeapReference::new(22);
    let third_shared = SharedHeapReference::new(33);
    let first_local = heap
        .allocate(
            layout.allocation(),
            Payload::Bytes(&first_shared.bits().to_le_bytes()),
        )
        .expect("heap allocation should succeed");
    let _second_local = heap
        .allocate(
            layout.allocation(),
            Payload::Bytes(&second_shared.bits().to_le_bytes()),
        )
        .expect("heap allocation should succeed");
    let _third_local = heap
        .allocate(
            layout.allocation(),
            Payload::Bytes(&third_shared.bits().to_le_bytes()),
        )
        .expect("heap allocation should succeed");
    let mut roots = Vec::new();

    heap.start_shared_edge_scan();

    // scan the first root, then remove it while the cursor points past it
    let first_work = heap
        .scan_shared_edge_step(&mut roots, 1)
        .expect("first shared root scan should succeed");
    heap.free(first_local)
        .expect("freeing scanned root should succeed");

    let remaining_work = heap
        .scan_shared_edge_step(&mut roots, usize::MAX)
        .expect("remaining shared root scan should succeed");

    assert_eq!(first_work, 1);
    assert_eq!(remaining_work, 2);
    assert_eq!(roots, vec![first_shared, second_shared, third_shared]);

    heap.finish_shared_edge_scan();
}

/// Stay idle when no local pressure or explicit request exists.
#[test]
fn test_heap_gc_step_stays_idle_without_request() {
    let options = HeapOptions::local();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );
    let mut heap =
        Heap::with_allocator_limits_and_options(allocator, crate::HeapLimits::default(), options)
            .expect("heap should build");
    let mut roots = [];

    let progress = heap.gc_step(&mut roots).expect("gc step should succeed");

    assert_eq!(progress, GcProgress::Idle);
}

/// Run one minor cycle after heap allocation pressure.
#[test]
fn test_heap_gc_step_runs_minor_after_pressure() {
    let (mut heap, layout_ids) = test_heap(&[(64, ReferenceMap::empty())]);
    let layout = &layout_ids[0];
    let root = heap
        .allocate(layout.allocation(), Payload::Bytes(&[1; 64]))
        .expect("heap allocation should succeed");
    let mut roots = [root];

    let progress = heap.gc_step(&mut roots).expect("gc step should succeed");
    let stats = progress
        .completed_stats()
        .expect("pressure should request one cycle");
    let root = roots[0];

    assert_eq!(stats.freed_allocations, 0);
    assert!(heap.is_heap_live(root));
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Minor));
}

/// Honor one explicit full local collection request below the pacing trigger.
#[test]
fn test_heap_gc_step_honors_manual_full_request() {
    let layout = test_layout(3, ReferenceMap::empty());
    let options = HeapOptions::local();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );
    let mut heap =
        Heap::with_allocator_limits_and_options(allocator, crate::HeapLimits::default(), options)
            .expect("heap should build");
    let root = heap
        .allocate(layout.allocation(), Payload::Bytes(&[1, 2, 3]))
        .expect("heap allocation should succeed");
    let mut roots = [root];

    heap.request_full_gc();

    let progress = heap.gc_step(&mut roots).expect("gc step should succeed");
    let stats = progress
        .completed_stats()
        .expect("manual request should run one cycle");

    assert_eq!(stats.freed_allocations, 0);
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Full));
}

/// Continue one local major collection across bounded safepoint work.
#[test]
fn test_step_major_gc_spreads_full_cycle() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let layout = test_layout(1, ReferenceMap::empty());
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let root = heap
        .allocate(layout.allocation(), Payload::Bytes(&[1]))
        .expect("heap allocation should succeed");
    let garbage = heap
        .allocate(layout.allocation(), Payload::Bytes(&[2]))
        .expect("heap allocation should succeed");
    let mut roots = [root];

    heap.start_major_gc(&mut roots)
        .expect("major collection should start");

    let first = heap
        .step_major_gc(&mut roots, 1)
        .expect("major step should succeed");

    assert_eq!(first, GcProgress::Active);
    assert!(heap.major_gc_active());

    let stats = loop {
        let progress = heap
            .step_major_gc(&mut roots, 1)
            .expect("major step should succeed");
        if let Some(stats) = progress.completed_stats() {
            break stats;
        }
    };

    assert_eq!(stats.freed_allocations, 1);
    assert!(heap.is_live(root));
    assert!(!heap.is_live(garbage));
    assert!(!heap.major_gc_active());
}

/// Continue local major marking across large allocation pages.
#[test]
fn test_step_major_gc_scans_large_allocations_incrementally() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        ..HeapOptions::local()
    };
    let first_offset = 0usize;
    let second_offset = options.page_bytes;
    let parent_byte_len = second_offset + HeapReference::BYTE_LEN;
    let second_offset = u32::try_from(second_offset).expect("page offset should fit uint32");
    let reference_map = ReferenceMap::Reference {
        local_offsets: vec![first_offset as u32, second_offset].into_boxed_slice(),
        shared_offsets: Vec::new().into_boxed_slice(),
    };
    let allocator = test_allocator(&options);

    let layouts = test_layouts(&[(2, ReferenceMap::empty()), (parent_byte_len, reference_map)]);
    let child_layout = &layouts[0];
    let parent_layout = &layouts[1];
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let first_child = heap
        .allocate(child_layout.allocation(), Payload::Bytes(&[0xC1, 0x1D]))
        .expect("heap allocation should succeed");
    let second_child = heap
        .allocate(child_layout.allocation(), Payload::Bytes(&[0xC2, 0x1D]))
        .expect("heap allocation should succeed");
    let mut parent_bytes = vec![0; parent_byte_len];
    parent_bytes[first_offset..first_offset + HeapReference::BYTE_LEN]
        .copy_from_slice(&first_child.bits().to_le_bytes());
    parent_bytes[second_offset as usize..second_offset as usize + HeapReference::BYTE_LEN]
        .copy_from_slice(&second_child.bits().to_le_bytes());
    let parent = heap
        .allocate(parent_layout.allocation(), Payload::Bytes(&parent_bytes))
        .expect("heap allocation should succeed");
    let mut roots = [parent];

    heap.start_major_gc(&mut roots)
        .expect("major collection should start");

    let first = heap
        .step_major_gc(&mut roots, 1)
        .expect("major step should succeed");

    assert_eq!(first, GcProgress::Active);
    assert!(heap.major_gc_active());

    let stats = loop {
        let progress = heap
            .step_major_gc(&mut roots, 1)
            .expect("major step should succeed");
        if let Some(stats) = progress.completed_stats() {
            break stats;
        }
    };

    assert_eq!(stats.freed_allocations, 0);
    assert!(heap.is_live(first_child));
    assert!(heap.is_live(second_child));
}

/// Repeated full collection should free later unreachable allocations too.
#[test]
fn test_collect_full_reclaims_later_unreachable_allocations() {
    let options = HeapOptions::local();
    let allocator = test_allocator(&options);
    let layout = test_layout(1, ReferenceMap::empty());
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let root = heap
        .allocate(layout.allocation(), Payload::Bytes(&[1]))
        .expect("heap allocation should succeed");
    let _garbage = heap
        .allocate(layout.allocation(), Payload::Bytes(&[2]))
        .expect("heap allocation should succeed");
    let mut roots = [root];

    // the first full cycle should leave only the explicit root
    heap.collect_full(&mut roots)
        .expect("full collection should succeed");

    assert_eq!(heap.allocation_count(), 1);

    let more_garbage = heap
        .allocate(layout.allocation(), Payload::Bytes(&[3]))
        .expect("heap allocation should succeed");
    let even_more = heap
        .allocate(layout.allocation(), Payload::Bytes(&[4]))
        .expect("heap allocation should succeed");

    // later allocations should still enter young space
    assert!(is_young(&heap, more_garbage));
    assert!(is_young(&heap, even_more));

    // the next minor cycle should clear the unreachable nursery
    heap.collect_minor(&mut roots)
        .expect("minor collection should succeed");

    assert_eq!(heap.allocation_count(), 1);
}
