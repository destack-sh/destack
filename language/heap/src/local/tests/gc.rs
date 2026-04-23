use crate::local::space::HeapStorage;
use crate::{
    GcKind, GcOptions, Heap, HeapError, HeapOptions, HeapReference, HeapSpace, SharedHeapReference,
    test_allocator,
};
use destack_mir::LayoutTrace;

/// Build one local heap whose pacer triggers immediately in step-driven tests.
fn test_heap() -> Heap {
    let options = HeapOptions {
        gc: GcOptions {
            growth_percent: 10,
            trigger_percent: 75,
            soft_limit_bytes: None,
            minimum_heap_bytes: Some(0),
        },
        ..HeapOptions::local()
    };

    Heap::with_limits_and_options(crate::HeapLimits::default(), options)
        .expect("pacing heap should build")
}

/// Report whether one heap reference is currently young.
fn is_young(heap: &HeapSpace, reference: HeapReference) -> bool {
    matches!(heap.location(reference), Some(HeapStorage::Young(_)))
}

/// Report whether one heap reference is currently mature.
fn is_mature(heap: &HeapSpace, reference: HeapReference) -> bool {
    matches!(
        heap.location(reference),
        Some(HeapStorage::Small(_)) | Some(HeapStorage::Large(_))
    )
}

/// Promote reachable young entries and clear unreachable young-space state.
#[test]
fn test_collect_minor_promotes_reachable_entries() {
    let layout = HeapOptions {
        heap_small_bytes: 32,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&layout);
    let mut heap =
        HeapSpace::with_options(allocator, &layout).expect("explicit heap options should build");
    let reachable = heap
        .allocate_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("heap allocation should succeed");
    let unreachable = heap
        .allocate_bytes(&[4, 5, 6], LayoutTrace::empty(), None)
        .expect("heap allocation should succeed");
    let mut roots = [reachable];

    assert!(is_young(&heap, reachable));
    assert!(is_young(&heap, unreachable));

    let stats = heap
        .collect_minor(&mut roots)
        .expect("young collection should succeed");
    let reachable = roots[0];

    assert_eq!(stats.freed_allocations, 1);
    assert_eq!(heap.read_bytes(reachable), Ok(vec![1, 2, 3]));
    assert!(!heap.is_live(unreachable));
    assert!(is_mature(&heap, reachable));

    let image = heap.image().expect("heap image should capture");
    let young = image.young();

    assert_eq!(young.next_offset(), 0);
    assert_eq!(
        young.pages().len(),
        young.capacity_bytes().div_ceil(young.page_bytes())
    );
}

/// Promote young entries reached through traced young references.
#[test]
fn test_collect_minor_promotes_reachable_child_entries() {
    let layout = HeapOptions {
        heap_small_bytes: 32,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&layout);
    let mut heap =
        HeapSpace::with_options(allocator, &layout).expect("explicit heap options should build");
    let trace = LayoutTrace::Reference {
        local_offsets: vec![0].into_boxed_slice(),
        shared_offsets: Vec::new().into_boxed_slice(),
    };
    let child = heap
        .allocate_bytes(&[0xC1, 0x1D], LayoutTrace::empty(), None)
        .expect("heap allocation should succeed");
    let parent = heap
        .allocate_bytes(&child.bits().to_le_bytes(), trace, None)
        .expect("heap allocation should succeed");
    let mut roots = [parent];

    let stats = heap
        .collect_minor(&mut roots)
        .expect("young collection should succeed");
    let parent = roots[0];

    assert_eq!(stats.freed_allocations, 0);
    assert!(is_mature(&heap, parent));

    let child_bytes = heap.read_bytes(parent).expect("parent read should succeed");
    let rewritten_child = HeapReference::from_bits(u64::from_le_bytes(
        child_bytes[..8]
            .try_into()
            .expect("child reference should fit"),
    ));

    assert_eq!(heap.read_bytes(rewritten_child), Ok(vec![0xC1, 0x1D]));
}

/// Reject invalid explicit young roots loudly.
#[test]
fn test_collect_minor_rejects_invalid_root() {
    let layout = HeapOptions::local();
    let allocator = test_allocator(&layout);
    let mut heap =
        HeapSpace::with_options(allocator, &layout).expect("explicit heap options should build");
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
    let layout = HeapOptions::local();
    let allocator = test_allocator(&layout);
    let mut heap =
        HeapSpace::with_options(allocator, &layout).expect("explicit heap options should build");
    let reachable = heap
        .allocate_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
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
    let layout = HeapOptions {
        heap_small_bytes: 32,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&layout);
    let mut heap =
        HeapSpace::with_options(allocator, &layout).expect("explicit heap options should build");
    let reference = heap
        .allocate_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("heap allocation should succeed");

    assert!(is_young(&heap, reference));

    let reference = heap.pin(reference).expect("pin should succeed");

    assert_eq!(heap.pins.references().collect::<Vec<_>>(), vec![reference]);
    assert_eq!(heap.read_bytes(reference), Ok(vec![1, 2, 3]));
    assert!(is_mature(&heap, reference));
}

/// Pinned mature roots should keep young children alive during minor collection.
#[test]
fn test_collect_minor_traces_pinned_roots() {
    let layout = HeapOptions {
        heap_small_bytes: 32,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&layout);
    let mut heap =
        HeapSpace::with_options(allocator, &layout).expect("explicit heap options should build");
    let trace = LayoutTrace::Reference {
        local_offsets: vec![0].into_boxed_slice(),
        shared_offsets: Vec::new().into_boxed_slice(),
    };
    let child = heap
        .allocate_bytes(&[0xC1, 0x1D], LayoutTrace::empty(), None)
        .expect("heap allocation should succeed");
    let parent = heap
        .allocate_bytes(&child.bits().to_le_bytes(), trace, None)
        .expect("heap allocation should succeed");
    let parent = heap.pin(parent).expect("pin should succeed");
    let mut roots = [];

    let stats = heap
        .collect_minor(&mut roots)
        .expect("young collection should succeed");

    assert_eq!(stats.freed_allocations, 0);
    assert!(is_mature(&heap, parent));

    let child_bytes = heap.read_bytes(parent).expect("parent read should succeed");
    let rewritten_child = HeapReference::from_bits(u64::from_le_bytes(
        child_bytes[..8]
            .try_into()
            .expect("child reference should fit"),
    ));

    assert_eq!(heap.read_bytes(rewritten_child), Ok(vec![0xC1, 0x1D]));
    assert!(is_mature(&heap, rewritten_child));
}

/// Pinned mature references should stay live during full collection without explicit roots.
#[test]
fn test_collect_full_traces_pinned_roots() {
    let layout = HeapOptions {
        heap_young_bytes: 0,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&layout);
    let mut heap =
        HeapSpace::with_options(allocator, &layout).expect("explicit heap options should build");
    let reference = heap
        .allocate_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("heap allocation should succeed");
    let reference = heap.pin(reference).expect("pin should succeed");
    let mut roots = [];

    let stats = heap
        .collect_full(&mut roots)
        .expect("full collection should succeed");

    assert_eq!(stats.freed_allocations, 0);
    assert!(heap.is_live(reference));
    assert_eq!(heap.read_bytes(reference), Ok(vec![1, 2, 3]));
}

/// Trace shared roots through the full shared-reference width.
#[test]
fn test_scan_shared_roots_uses_shared_reference_width() {
    let options = HeapOptions::local();
    let allocator = test_allocator(&options);
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let trace = LayoutTrace::Reference {
        local_offsets: Vec::new().into_boxed_slice(),
        shared_offsets: vec![0].into_boxed_slice(),
    };
    let shared = SharedHeapReference::new(7);
    let local = heap
        .allocate_bytes(&shared.bits().to_le_bytes(), trace, None)
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

/// Stay idle when no local pressure or explicit request exists.
#[test]
fn test_heap_gc_step_stays_idle_without_request() {
    let mut heap = Heap::new().expect("heap should build");
    let mut roots = [];

    let stats = heap.gc_step(&mut roots).expect("gc step should succeed");

    assert_eq!(stats, None);
}

/// Run one minor cycle after local heap allocation pressure.
#[test]
fn test_heap_gc_step_runs_minor_after_pressure() {
    let mut heap = test_heap();
    let root = heap
        .allocate_heap_bytes(&vec![1; 64], LayoutTrace::empty(), None)
        .expect("heap allocation should succeed");
    let mut roots = [root];

    let stats = heap
        .gc_step(&mut roots)
        .expect("gc step should succeed")
        .expect("pressure should request one cycle");
    let root = roots[0];

    assert_eq!(stats.freed_allocations, 0);
    assert!(heap.is_heap_live(root));
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Minor));
}

/// Honor one explicit full local collection request below the pacing trigger.
#[test]
fn test_heap_gc_step_honors_manual_full_request() {
    let mut heap = Heap::new().expect("heap should build");
    let root = heap
        .allocate_heap_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("heap allocation should succeed");
    let mut roots = [root];

    heap.request_full_gc();

    let stats = heap
        .gc_step(&mut roots)
        .expect("gc step should succeed")
        .expect("manual request should run one cycle");

    assert_eq!(stats.freed_allocations, 0);
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Full));
}

/// Repeated full collection should free later unreachable allocations too.
#[test]
fn test_collect_full_reclaims_later_unreachable_allocations() {
    let layout = HeapOptions::local();
    let allocator = test_allocator(&layout);
    let mut heap =
        HeapSpace::with_options(allocator, &layout).expect("explicit heap options should build");
    let root = heap
        .allocate_bytes(&[1], LayoutTrace::empty(), None)
        .expect("heap allocation should succeed");
    let _garbage = heap
        .allocate_bytes(&[2], LayoutTrace::empty(), None)
        .expect("heap allocation should succeed");
    let mut roots = [root];

    // the first full cycle should leave only the explicit root
    heap.collect_full(&mut roots)
        .expect("full collection should succeed");

    assert_eq!(heap.allocation_count(), 1);

    let more_garbage = heap
        .allocate_bytes(&[3], LayoutTrace::empty(), None)
        .expect("heap allocation should succeed");
    let even_more = heap
        .allocate_bytes(&[4], LayoutTrace::empty(), None)
        .expect("heap allocation should succeed");

    // later allocations should still enter young space
    assert!(is_young(&heap, more_garbage));
    assert!(is_young(&heap, even_more));

    // the next minor cycle should clear the unreachable nursery
    heap.collect_minor(&mut roots)
        .expect("minor collection should succeed");

    assert_eq!(heap.allocation_count(), 1);
}
