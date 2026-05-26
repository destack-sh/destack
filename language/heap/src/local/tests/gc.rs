use std::sync::Arc;

use crate::local::space::{HeapPlace, YoungPlace};
use crate::{
    Allocator, GcKind, GcOptions, GcProgress, Heap, HeapError, HeapOptions, HeapReference,
    HeapSpace, Payload, SharedHeapReference, SizeClassTable, TestLayout, test_allocator,
    test_layout, test_layouts,
};
use destack_mir::TraceMap;

use super::read_mapped_bytes;

/// Build one heap whose pacer triggers immediately in step-driven tests.
fn test_heap(layouts: &[(usize, TraceMap)]) -> (Heap, Vec<TestLayout>) {
    let options = HeapOptions {
        gc: GcOptions {
            growth_percent: 10,
            trigger_percent: 75,
            soft_limit_bytes: None,
            minimum_heap_bytes: Some(0),
            minimum_work_bytes: crate::DEFAULT_GC_MINIMUM_WORK_BYTES,
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
    matches!(
        heap.place(reference),
        Some(HeapPlace::Young(YoungPlace::Range { .. }))
            | Some(HeapPlace::Young(YoungPlace::Slot(_)))
    )
}

/// Read one heap reference from mapped heap bytes.
fn read_heap_reference(heap: &HeapSpace, reference: HeapReference) -> HeapReference {
    let address = heap.base_address() + reference.offset();
    let bytes = read_mapped_bytes(address, HeapReference::BYTE_LEN);
    let bits = usize::from_le_bytes(bytes.try_into().expect("heap reference should fit"));

    HeapReference::from_bits(bits)
}

/// Promote reachable young allocations and free unreachable young-space state.
#[test]
fn test_collect_minor_promotes_reachable_entries() {
    let options = tiny_heap_options();
    let allocator = test_allocator(&options);
    let layout = test_layout(3, TraceMap::empty());
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let reachable = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1, 2, 3]),
        )
        .expect("heap allocation should succeed");
    let unreachable = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[4, 5, 6]),
        )
        .expect("heap allocation should succeed");
    let mut roots = [reachable];

    assert!(is_young(&heap, reachable));
    assert!(is_young(&heap, unreachable));

    let stats = heap
        .collect_minor(&mut roots)
        .expect("young collection should succeed");

    assert_eq!(stats.freed_allocations, 1);
    assert_ne!(roots[0], reachable);
    assert!(!heap.is_live(unreachable));
    assert!(!heap.is_live(reachable));
    assert!(heap.is_live(roots[0]));
    assert!(!is_young(&heap, roots[0]));
    let address = heap.base_address() + roots[0].offset();

    let bytes = read_mapped_bytes(address, 3);

    assert_eq!(bytes, &[1, 2, 3]);

    let image = heap.image().expect("heap image should capture");
    let young = image.young();

    assert_eq!(
        young.pages().len(),
        young.capacity_bytes().div_ceil(young.page_bytes())
    );
}

/// Promote reachable fixed-size young run slots.
#[test]
fn test_collect_minor_promotes_reachable_noscan_runs() {
    let options = HeapOptions::local();
    let allocator = test_allocator(&options);
    let layout = test_layout(4, TraceMap::empty());
    let mut heap =
        Heap::with_allocator_limits_and_options(allocator, crate::HeapLimits::default(), options)
            .expect("heap should build");
    let reference = heap
        .allocate_zeroed(&heap.allocation_plan(layout.allocation()))
        .expect("young no-scan allocation should succeed");
    let mut roots = [reference];

    assert!(matches!(
        heap.heap.place(reference),
        Some(HeapPlace::Young(YoungPlace::Slot(_)))
    ));

    let stats = heap
        .collect_minor(&mut roots)
        .expect("young collection should succeed");

    assert_eq!(stats.freed_allocations, 0);
    assert_ne!(roots[0], reference);
    assert!(!heap.heap.is_live(reference));
    assert!(heap.heap.is_live(roots[0]));
    assert!(!is_young(&heap.heap, roots[0]));
    let address = heap.heap.base_address() + roots[0].offset();

    let bytes = read_mapped_bytes(address, 8);

    assert_eq!(bytes, &[0; 8]);
}

/// Clear recycled young pages before zeroed allocation reuses them.
#[test]
fn test_collect_minor_recycles_young_zeroed_bytes() {
    let options = HeapOptions::local();
    let allocator = test_allocator(&options);
    let layout = test_layout(8, TraceMap::empty());
    let mut heap =
        Heap::with_allocator_limits_and_options(allocator, crate::HeapLimits::default(), options)
            .expect("heap should build");
    let reference = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[9; 8]),
        )
        .expect("heap allocation should succeed");
    let mut roots = [];

    assert!(is_young(&heap.heap, reference));

    heap.collect_minor(&mut roots)
        .expect("young collection should succeed");

    let reference = heap
        .allocate_zeroed(&heap.allocation_plan(layout.allocation()))
        .expect("zeroed heap allocation should succeed");
    let address = heap.heap.base_address() + reference.offset();

    let bytes = read_mapped_bytes(address, 8);

    assert_eq!(bytes, &[0; 8]);
}

/// Promote young allocations reached through traced young references.
#[test]
fn test_collect_minor_promotes_reachable_child_entries() {
    let options = tiny_heap_options();
    let allocator = test_allocator(&options);
    let trace_map = TraceMap::Fixed {
        local_offsets: vec![0].into_boxed_slice(),
        shared_offsets: Vec::new().into_boxed_slice(),
    };
    let layouts = test_layouts(&[(2, TraceMap::empty()), (8, trace_map.clone())]);
    let [child_layout, parent_layout]: [TestLayout; 2] =
        layouts.try_into().expect("test layouts should match");
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let child = heap
        .allocate(
            &heap.allocation_plan(child_layout.allocation()),
            Payload::Bytes(&[0xC1, 0x1D]),
        )
        .expect("heap allocation should succeed");
    let parent = heap
        .allocate(
            &heap.allocation_plan(parent_layout.allocation()),
            Payload::Bytes(&child.bits().to_le_bytes()),
        )
        .expect("heap allocation should succeed");
    let mut roots = [parent];

    let stats = heap
        .collect_minor(&mut roots)
        .expect("young collection should succeed");

    assert_eq!(stats.freed_allocations, 0);
    assert_ne!(roots[0], parent);
    assert!(!heap.is_live(parent));
    assert!(heap.is_live(roots[0]));
    assert!(!is_young(&heap, roots[0]));

    let traced_child = read_heap_reference(&heap, roots[0]);

    assert_ne!(traced_child, child);
    assert!(!heap.is_live(child));
    assert!(heap.is_live(traced_child));
    assert!(!is_young(&heap, traced_child));
    let child_address = heap.base_address() + traced_child.offset();

    let bytes = read_mapped_bytes(child_address, 2);

    assert_eq!(bytes, &[0xC1, 0x1D]);
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
    let layout = test_layout(3, TraceMap::empty());
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let reachable = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1, 2, 3]),
        )
        .expect("heap allocation should succeed");
    let mut roots = [reachable];

    let stats = heap
        .collect_minor(&mut roots)
        .expect("young collection should succeed");

    assert_eq!(heap.gc_state().completed_cycles, 1);
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Minor));
    assert_eq!(heap.gc_state().last_stats, Some(stats));
}

/// Pinning one young reference should preserve it in place.
#[test]
fn test_pin_preserves_young_reference() {
    let options = tiny_heap_options();
    let allocator = test_allocator(&options);
    let layout = test_layout(3, TraceMap::empty());
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let reference = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1, 2, 3]),
        )
        .expect("heap allocation should succeed");

    assert!(is_young(&heap, reference));

    let reference = heap.pin(reference).expect("pin should succeed");

    assert_eq!(
        heap.collector.pins.references().collect::<Vec<_>>(),
        vec![reference]
    );
    assert!(is_young(&heap, reference));
    let address = heap.base_address() + reference.offset();

    let bytes = read_mapped_bytes(address, 16);

    assert_eq!(bytes, &[1, 2, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
}

/// Pinning an interior young reference preserves its byte offset in place.
#[test]
fn test_pin_preserves_interior_young_reference() {
    let options = tiny_heap_options();
    let allocator = test_allocator(&options);
    let layout = test_layout(8, TraceMap::empty());
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let reference = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1, 2, 3, 4, 5, 6, 7, 8]),
        )
        .expect("heap allocation should succeed");
    let interior = HeapReference::new(reference.offset() + 3);

    let interior = heap.pin(interior).expect("pin should succeed");
    let location = heap
        .resolve_location(interior)
        .expect("interior pin should resolve");

    assert_eq!(location.byte_offset, 3);
    assert_eq!(
        heap.collector.pins.references().collect::<Vec<_>>(),
        vec![location.base]
    );
    assert!(is_young(&heap, interior));

    heap.unpin(interior).expect("interior unpin should succeed");

    assert!(heap.collector.pins.references().next().is_none());
}

/// Minor collection promotes interior young roots without losing their offset.
#[test]
fn test_collect_minor_promotes_interior_roots() {
    let options = tiny_heap_options();
    let allocator = test_allocator(&options);
    let layout = test_layout(8, TraceMap::empty());
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let reference = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1, 2, 3, 4, 5, 6, 7, 8]),
        )
        .expect("heap allocation should succeed");
    let mut roots = [HeapReference::new(reference.offset() + 5)];

    heap.collect_minor(&mut roots)
        .expect("young collection should succeed");
    let location = heap
        .resolve_location(roots[0])
        .expect("interior root should resolve");

    assert_eq!(location.byte_offset, 5);
    assert_ne!(roots[0], HeapReference::new(reference.offset() + 5));
    assert!(!heap.is_live(reference));
    assert!(heap.is_live(location.base));
    assert!(!is_young(&heap, location.base));
}

/// Pinned roots should stay in place while young children promote.
#[test]
fn test_collect_minor_traces_pinned_roots() {
    let options = tiny_heap_options();
    let allocator = test_allocator(&options);
    let trace_map = TraceMap::Fixed {
        local_offsets: vec![0].into_boxed_slice(),
        shared_offsets: Vec::new().into_boxed_slice(),
    };
    let layouts = test_layouts(&[(2, TraceMap::empty()), (8, trace_map.clone())]);
    let [child_layout, parent_layout]: [TestLayout; 2] =
        layouts.try_into().expect("test layouts should match");
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let child = heap
        .allocate(
            &heap.allocation_plan(child_layout.allocation()),
            Payload::Bytes(&[0xC1, 0x1D]),
        )
        .expect("heap allocation should succeed");
    let parent = heap
        .allocate(
            &heap.allocation_plan(parent_layout.allocation()),
            Payload::Bytes(&child.bits().to_le_bytes()),
        )
        .expect("heap allocation should succeed");
    let parent = heap.pin(parent).expect("pin should succeed");
    let mut roots = [];

    let stats = heap
        .collect_minor(&mut roots)
        .expect("young collection should succeed");

    assert_eq!(stats.freed_allocations, 0);
    assert_eq!(
        heap.collector.pins.references().collect::<Vec<_>>(),
        vec![parent]
    );
    assert!(is_young(&heap, parent));
    assert!(heap.is_live(parent));

    let traced_child = read_heap_reference(&heap, parent);

    assert_ne!(traced_child, child);
    assert!(!heap.is_live(child));
    assert!(heap.is_live(traced_child));
    assert!(!is_young(&heap, traced_child));
}

/// Keep mature dirty cards while they still point at pinned young children.
#[test]
fn test_collect_minor_retains_dirty_card_for_pinned_young_child() {
    let options = HeapOptions {
        max_heap_young_allocation_bytes: 8,
        ..tiny_heap_options()
    };
    let allocator = test_allocator(&options);
    let trace_map = TraceMap::Fixed {
        local_offsets: vec![0].into_boxed_slice(),
        shared_offsets: Vec::new().into_boxed_slice(),
    };
    let layouts = test_layouts(&[(4, TraceMap::empty()), (16, trace_map.clone())]);
    let [child_layout, parent_layout]: [TestLayout; 2] =
        layouts.try_into().expect("test layouts should match");
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let child = heap
        .allocate(
            &heap.allocation_plan(child_layout.allocation()),
            Payload::Zeroed,
        )
        .expect("young child allocation should succeed");
    let child = heap.pin(child).expect("pin should succeed");
    let mut parent_bytes = [0u8; 16];
    parent_bytes[..HeapReference::BYTE_LEN].copy_from_slice(&child.bits().to_le_bytes());
    let parent = heap
        .allocate(
            &heap.allocation_plan(parent_layout.allocation()),
            Payload::Bytes(&parent_bytes),
        )
        .expect("mature parent allocation should succeed");
    let mut roots = [parent];

    assert!(is_young(&heap, child));
    assert!(!is_young(&heap, parent));

    heap.collect_minor(&mut roots)
        .expect("young collection should keep pinned child");

    assert_eq!(read_heap_reference(&heap, roots[0]), child);
    assert!(heap.is_live(child));
    assert!(is_young(&heap, child));

    heap.unpin(child).expect("unpin should succeed");
    heap.collect_minor(&mut roots)
        .expect("young collection should rescan retained card");

    let promoted_child = read_heap_reference(&heap, roots[0]);

    assert_ne!(promoted_child, child);
    assert!(!heap.is_live(child));
    assert!(heap.is_live(promoted_child));
    assert!(!is_young(&heap, promoted_child));
}

/// Pinned mature references should stay live during full collection without explicit roots.
#[test]
fn test_collect_full_traces_pinned_roots() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let layout = test_layout(3, TraceMap::empty());
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let reference = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1, 2, 3]),
        )
        .expect("heap allocation should succeed");
    let reference = heap.pin(reference).expect("pin should succeed");
    let mut roots = [];

    let stats = heap
        .collect_full(&mut roots)
        .expect("full collection should succeed");

    assert_eq!(stats.freed_allocations, 0);
    assert!(heap.is_live(reference));
    let address = heap.base_address() + reference.offset();

    let bytes = read_mapped_bytes(address, 16);

    assert_eq!(bytes, &[1, 2, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
}

/// Trace shared roots through the full shared-reference width.
#[test]
fn test_scan_shared_roots_uses_shared_reference_width() {
    let options = HeapOptions::local();
    let allocator = test_allocator(&options);
    let trace_map = TraceMap::Fixed {
        local_offsets: Vec::new().into_boxed_slice(),
        shared_offsets: vec![0].into_boxed_slice(),
    };
    let layout = test_layout(8, trace_map);
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let shared = SharedHeapReference::new(7);
    let local = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&shared.bits().to_le_bytes()),
        )
        .expect("heap allocation should succeed");
    let mut roots = Vec::new();

    heap.start_shared_edge_scan();

    let scanned_bytes = heap
        .scan_shared_references(&mut roots, options.page_bytes)
        .expect("shared root scan should succeed");

    assert_eq!(scanned_bytes, 8);
    assert_eq!(roots, vec![shared]);
    assert!(heap.shared_edge_scan_idle());

    heap.finish_shared_edge_scan();

    assert!(heap.is_live(local));
}

/// Continue local-to-shared edge scans across large allocation pages.
#[test]
fn test_scan_shared_roots_scans_large_allocations_incrementally() {
    let options = HeapOptions {
        heap_young_bytes: 0,
        ..HeapOptions::local()
    };
    let allocator = test_allocator(&options);
    let page_bytes = allocator.page_bytes();
    let first_offset = 0usize;
    let second_offset = page_bytes;
    let small_limit = options
        .size_classes
        .max_small_allocation_bytes()
        .expect("default size classes should not be empty");
    let parent_byte_len = small_limit + second_offset + SharedHeapReference::BYTE_LEN;
    let second_offset = u32::try_from(second_offset).expect("page offset should fit uint32");
    let trace_map = TraceMap::Fixed {
        local_offsets: Vec::new().into_boxed_slice(),
        shared_offsets: vec![first_offset as u32, second_offset].into_boxed_slice(),
    };
    let layout = test_layout(parent_byte_len, trace_map);
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let first_shared = SharedHeapReference::new(11);
    let second_shared = SharedHeapReference::new(22);
    let mut parent_bytes = vec![0; parent_byte_len];
    parent_bytes[first_offset..first_offset + SharedHeapReference::BYTE_LEN]
        .copy_from_slice(&first_shared.bits().to_le_bytes());
    parent_bytes[second_offset as usize..second_offset as usize + SharedHeapReference::BYTE_LEN]
        .copy_from_slice(&second_shared.bits().to_le_bytes());
    let parent = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&parent_bytes),
        )
        .expect("heap allocation should succeed");
    let mut roots = Vec::new();

    heap.start_shared_edge_scan();

    let first_scanned = heap
        .scan_shared_references(&mut roots, 1)
        .expect("first shared-root scan should succeed");

    assert_eq!(first_scanned, page_bytes);
    assert_eq!(roots, vec![first_shared]);
    assert!(!heap.shared_edge_scan_idle());

    let second_scanned = heap
        .scan_shared_references(&mut roots, 1)
        .expect("second shared-root scan should succeed");

    assert_eq!(second_scanned, page_bytes);
    assert_eq!(roots, vec![first_shared, second_shared]);

    while !heap.shared_edge_scan_idle() {
        let scanned_bytes = heap
            .scan_shared_references(&mut roots, 1)
            .expect("remaining shared-root scan should succeed");
        assert!(scanned_bytes > 0);
    }

    heap.finish_shared_edge_scan();

    assert!(heap.is_live(parent));
}

/// Keep unscanned shared-edge roots stable when earlier roots are freed.
#[test]
fn test_scan_shared_roots_survives_active_root_removal() {
    let options = HeapOptions::local();
    let allocator = test_allocator(&options);
    let trace_map = TraceMap::Fixed {
        local_offsets: Vec::new().into_boxed_slice(),
        shared_offsets: vec![0].into_boxed_slice(),
    };
    let layout = test_layout(8, trace_map);
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let first_shared = SharedHeapReference::new(11);
    let second_shared = SharedHeapReference::new(22);
    let third_shared = SharedHeapReference::new(33);
    let first_local = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&first_shared.bits().to_le_bytes()),
        )
        .expect("heap allocation should succeed");
    let _second_local = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&second_shared.bits().to_le_bytes()),
        )
        .expect("heap allocation should succeed");
    let _third_local = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&third_shared.bits().to_le_bytes()),
        )
        .expect("heap allocation should succeed");
    let mut roots = Vec::new();

    heap.start_shared_edge_scan();

    // scan the first root, then remove it while the cursor points past it
    let first_work = heap
        .scan_shared_references(&mut roots, 1)
        .expect("first shared root scan should succeed");
    heap.free(first_local)
        .expect("freeing scanned root should succeed");

    let remaining_work = heap
        .scan_shared_references(&mut roots, usize::MAX)
        .expect("remaining shared root scan should succeed");

    assert_eq!(first_work, 8);
    assert_eq!(remaining_work, 16);
    assert_eq!(roots, vec![first_shared, second_shared, third_shared]);

    heap.finish_shared_edge_scan();
}

/// Stay idle when no local pressure or explicit request exists.
#[test]
fn test_collect_step_stays_idle_without_request() {
    let options = HeapOptions::local();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );
    let mut heap =
        Heap::with_allocator_limits_and_options(allocator, crate::HeapLimits::default(), options)
            .expect("heap should build");
    let mut roots = [];

    let budget_bytes = heap.take_collection_budget_bytes();
    let progress = heap
        .collect_step(&mut roots, budget_bytes)
        .expect("collection step should succeed");

    assert_eq!(progress, GcProgress::Idle);
}

/// Run one full bounded cycle after heap allocation pressure.
#[test]
fn test_collect_step_runs_full_after_pressure() {
    let (mut heap, layout_ids) = test_heap(&[(64, TraceMap::empty())]);
    let layout = &layout_ids[0];
    let root = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1; 64]),
        )
        .expect("heap allocation should succeed");
    let mut roots = [root];

    let budget_bytes = heap.take_collection_budget_bytes();
    let progress = heap
        .collect_step(&mut roots, budget_bytes)
        .expect("collection step should succeed");
    let stats = progress
        .completed_stats()
        .expect("pressure should request one cycle");
    let root = roots[0];

    assert_eq!(stats.freed_allocations, 0);
    assert!(heap.is_heap_live(root));
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Full));
}

/// Run one minor cycle after young-space occupancy crosses the configured trigger.
#[test]
fn test_collect_step_runs_minor_after_young_occupancy() {
    let options = HeapOptions {
        gc: GcOptions {
            growth_percent: 100,
            trigger_percent: 75,
            soft_limit_bytes: None,
            minimum_heap_bytes: Some(1024 * 1024),
            minimum_work_bytes: crate::DEFAULT_GC_MINIMUM_WORK_BYTES,
        },
        heap_young_bytes: 64,
        max_heap_young_allocation_bytes: 16,
        ..HeapOptions::local()
    };
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );
    let mut heap =
        Heap::with_allocator_limits_and_options(allocator, crate::HeapLimits::default(), options)
            .expect("heap should build");
    let layout = test_layout(16, TraceMap::empty());
    let mut roots = Vec::new();

    // fill the young prefix to the configured trigger
    for _ in 0..3 {
        let reference = heap
            .allocate(
                &heap.allocation_plan(layout.allocation()),
                Payload::Bytes(&[1; 16]),
            )
            .expect("heap allocation should succeed");
        roots.push(reference);
    }

    let budget_bytes = heap.take_collection_budget_bytes();
    let progress = heap
        .collect_step(&mut roots, budget_bytes)
        .expect("collection step should succeed");

    assert_eq!(progress.completed_stats().map(|_| ()), Some(()));
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Minor));
}

/// Drain minor collection at one safepoint even with a small caller budget.
#[test]
fn test_collect_step_drains_minor_at_safepoint() {
    let options = HeapOptions {
        gc: GcOptions {
            growth_percent: 100,
            trigger_percent: 75,
            soft_limit_bytes: None,
            minimum_heap_bytes: Some(1024 * 1024),
            minimum_work_bytes: crate::DEFAULT_GC_MINIMUM_WORK_BYTES,
        },
        heap_young_bytes: 64,
        max_heap_young_allocation_bytes: 16,
        ..HeapOptions::local()
    };
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );
    let mut heap =
        Heap::with_allocator_limits_and_options(allocator, crate::HeapLimits::default(), options)
            .expect("heap should build");
    let layout = test_layout(16, TraceMap::empty());
    let mut roots = Vec::new();

    // fill young space past the trigger
    for _ in 0..3 {
        let reference = heap
            .allocate(
                &heap.allocation_plan(layout.allocation()),
                Payload::Bytes(&[1; 16]),
            )
            .expect("heap allocation should succeed");
        roots.push(reference);
    }

    let progress = heap
        .collect_step(&mut roots, 1)
        .expect("small-budget collection should succeed");

    assert_eq!(progress.completed_stats().map(|_| ()), Some(()));
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Minor));
}

/// Avoid expanding local collection budgets to cover a large nursery.
#[test]
fn test_collect_budget_does_not_expand_to_large_nursery() {
    let options = HeapOptions {
        gc: GcOptions {
            growth_percent: 100,
            trigger_percent: 75,
            soft_limit_bytes: None,
            minimum_heap_bytes: Some(1024 * 1024),
            minimum_work_bytes: 64,
        },
        heap_young_bytes: 1024,
        max_heap_young_allocation_bytes: 256,
        ..HeapOptions::local()
    };
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );
    let mut heap =
        Heap::with_allocator_limits_and_options(allocator, crate::HeapLimits::default(), options)
            .expect("heap should build");
    let layout = test_layout(256, TraceMap::empty());

    // fill young space beyond one configured safepoint quantum
    for _ in 0..3 {
        let _reference = heap
            .allocate(
                &heap.allocation_plan(layout.allocation()),
                Payload::Bytes(&[1; 256]),
            )
            .expect("heap allocation should succeed");
    }

    let budget_bytes = heap.take_collection_budget_bytes();

    assert_eq!(budget_bytes, 0);
}

/// Honor one explicit full local collection request below the pacing trigger.
#[test]
fn test_collect_step_honors_manual_full_request() {
    let layout = test_layout(3, TraceMap::empty());
    let options = HeapOptions::local();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );
    let mut heap =
        Heap::with_allocator_limits_and_options(allocator, crate::HeapLimits::default(), options)
            .expect("heap should build");
    let root = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1, 2, 3]),
        )
        .expect("heap allocation should succeed");
    let mut roots = [root];

    heap.request_full_gc();

    let budget_bytes = heap.take_collection_budget_bytes();
    let progress = heap
        .collect_step(&mut roots, budget_bytes)
        .expect("collection step should succeed");
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
    let layout = test_layout(1, TraceMap::empty());
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let root = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1]),
        )
        .expect("heap allocation should succeed");
    let garbage = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[2]),
        )
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
    let trace_map = TraceMap::Fixed {
        local_offsets: vec![first_offset as u32, second_offset].into_boxed_slice(),
        shared_offsets: Vec::new().into_boxed_slice(),
    };
    let allocator = test_allocator(&options);

    let layouts = test_layouts(&[(2, TraceMap::empty()), (parent_byte_len, trace_map)]);
    let child_layout = &layouts[0];
    let parent_layout = &layouts[1];
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let first_child = heap
        .allocate(
            &heap.allocation_plan(child_layout.allocation()),
            Payload::Bytes(&[0xC1, 0x1D]),
        )
        .expect("heap allocation should succeed");
    let second_child = heap
        .allocate(
            &heap.allocation_plan(child_layout.allocation()),
            Payload::Bytes(&[0xC2, 0x1D]),
        )
        .expect("heap allocation should succeed");
    let mut parent_bytes = vec![0; parent_byte_len];
    parent_bytes[first_offset..first_offset + HeapReference::BYTE_LEN]
        .copy_from_slice(&first_child.bits().to_le_bytes());
    parent_bytes[second_offset as usize..second_offset as usize + HeapReference::BYTE_LEN]
        .copy_from_slice(&second_child.bits().to_le_bytes());
    let parent = heap
        .allocate(
            &heap.allocation_plan(parent_layout.allocation()),
            Payload::Bytes(&parent_bytes),
        )
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
    let layout = test_layout(1, TraceMap::empty());
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let root = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1]),
        )
        .expect("heap allocation should succeed");
    let _garbage = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[2]),
        )
        .expect("heap allocation should succeed");
    let mut roots = [root];

    // the first full cycle should leave only the explicit root
    heap.collect_full(&mut roots)
        .expect("full collection should succeed");

    assert_eq!(heap.allocation_count(), 1);

    let more_garbage = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[3]),
        )
        .expect("heap allocation should succeed");
    let even_more = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[4]),
        )
        .expect("heap allocation should succeed");

    // later allocations should still enter young space
    assert!(is_young(&heap, more_garbage));
    assert!(is_young(&heap, even_more));

    // the next minor cycle should clear the unreachable nursery
    heap.collect_minor(&mut roots)
        .expect("minor collection should succeed");

    assert_eq!(heap.allocation_count(), 1);
}
