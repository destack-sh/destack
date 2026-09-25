use crate::local::gc::Phase;
use crate::local::storage::{HeapPlace, HeapStorage};
use crate::{
    AllocationShape, DEFAULT_GC_MINIMUM_WORK_BYTES, DropId, DropReference, GcAdvance, GcCollector,
    GcOptions, GcPhase, Heap, HeapError, HeapOptions, HeapReference, HeapResult, Payload,
    ReferenceRange, Release, RootSlot, SharedHeapReference, SizeClassTable, TestLayout,
    local_trace_map, shared_trace_map, test_layout, test_layouts, visit_heap_references,
    visit_heap_root_slots,
};
use tspp_mir::{Lifetime, Reference, TraceMap, TraceTable};

use super::{
    TestHeapPlan, TestTraceTable, read_mapped_bytes, test_heap, test_memory, test_storage,
    trace_view,
};

/// Build one heap whose pacer triggers immediately in step-driven tests.
fn pacing_heap(layouts: &[(usize, TraceMap)]) -> (Heap, Vec<TestLayout>) {
    let options = HeapOptions {
        gc: GcOptions {
            growth_percent: 10,
            trigger_percent: 75,
            soft_limit_bytes: None,
            minimum_heap_bytes: Some(0),
            minimum_work_bytes: DEFAULT_GC_MINIMUM_WORK_BYTES,
        },
        ..HeapOptions::local()
    };

    (test_heap(options), test_layouts(layouts))
}

/// Build heap options with tiny spans for focused GC tests.
fn tiny_heap_options() -> HeapOptions {
    HeapOptions {
        heap_small_size_bytes: 32,
        size_classes: SizeClassTable::new([16, 24, 32]).expect("size classes should validate"),
        ..HeapOptions::local()
    }
}

/// Read one heap reference from mapped heap bytes.
fn read_heap_reference(heap: &HeapStorage, reference: HeapReference) -> HeapReference {
    let address = heap.base_address() + reference.offset();
    let bytes = read_mapped_bytes(address, HeapReference::BYTE_LEN);
    let bits = usize::from_le_bytes(bytes.try_into().expect("heap reference should fit"));

    HeapReference::from_bits(bits)
}

/// Return the span index for one slot reference.
fn slot_span_index(heap: &HeapStorage, reference: HeapReference) -> usize {
    let Some(HeapPlace::Slot(slot)) = heap.place(reference) else {
        panic!("reference should point to a span slot");
    };

    slot.span_index()
}

/// Visit mutable test roots as heap root slots.
fn visit_roots(
    roots: &mut [HeapReference],
    visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
) -> HeapResult<()> {
    visit_heap_references(roots, visit)
}

/// Keep the rooted slot in place and free the unreachable one.
#[test]
fn test_collect_full_frees_unreachable_slots_in_place() {
    let options = tiny_heap_options();
    let layout = test_layout(3, TraceMap::empty());
    let mut heap = test_storage(&options);

    // root 1,2,3 and leave 4,5,6 unreachable
    let reachable = heap.test_allocate(layout.block(), Payload::Bytes(&[1, 2, 3]));
    let unreachable = heap.test_allocate(layout.block(), Payload::Bytes(&[4, 5, 6]));
    let mut roots = [reachable];

    let stats = heap
        .collect_full(
            &mut |visit| visit_roots(&mut roots, visit),
            trace_view(),
            &mut |_| Ok(()),
        )
        .expect("full collection should succeed");

    // the rooted block stays at its address, the unreachable block is gone
    assert_eq!(stats.freed_allocations, 1);
    assert_eq!(roots[0], reachable);
    assert!(heap.is_live(reachable));
    assert!(!heap.is_live(unreachable));

    let bytes = read_mapped_bytes(heap.base_address() + reachable.offset(), 3);

    assert_eq!(bytes, &[1, 2, 3]);
}

/// Reserve repeated allocations of one class from its cursor span.
#[test]
fn test_reserve_slot_reuses_the_class_span() {
    let first_map = local_trace_map(&[0]);
    let second_map = local_trace_map(&[8]);
    let mut trace_table = TraceTable::new();
    let first_trace_id = trace_table.insert(first_map.clone());
    let second_trace_id = trace_table.insert(second_map.clone());
    let options = HeapOptions::local();
    let mut heap = test_heap(options);
    let first_shape = AllocationShape::new(16, 1, Some(first_trace_id), first_map);
    let second_shape = AllocationShape::new(16, 1, Some(second_trace_id), second_map);
    let first_site = heap.options().allocation_plan(&first_shape);
    let second_site = heap.options().allocation_plan(&second_shape);

    // allocate from two same-size traced classes
    let first = heap
        .allocate_zeroed(first_site, &first_shape.trace_map)
        .expect("first local block should succeed");
    let second = heap
        .allocate_zeroed(second_site, &second_shape.trace_map)
        .expect("second local block should succeed");
    let first_again = heap
        .allocate_zeroed(first_site, &first_shape.trace_map)
        .expect("first trace class should still have its span");

    // each class keeps its own span and returns to it
    assert_eq!(
        slot_span_index(&heap.storage, first_again),
        slot_span_index(&heap.storage, first)
    );
    assert_ne!(
        slot_span_index(&heap.storage, second),
        slot_span_index(&heap.storage, first)
    );
}

/// Hand a freed slot of the class span back out on the fast path.
#[test]
fn test_reserve_small_reuses_a_freed_slot_of_the_class_span() {
    let options = HeapOptions::local();
    let mut heap = test_heap(options);
    let layout = test_layout(16, TraceMap::empty());
    let plan = heap.options().allocation_plan(&layout.block());
    let small_plan = plan.small_allocation().expect("plan should be small");

    // fill three slots, free the middle one
    let first = heap.test_allocate(layout.block(), Payload::Zeroed);
    let second = heap.test_allocate(layout.block(), Payload::Zeroed);
    let third = heap.test_allocate(layout.block(), Payload::Zeroed);
    heap.release(second).expect("heap free should succeed");

    // the fast path reserves the freed slot before advancing past the third
    let reserved = heap
        .reserve_small(small_plan)
        .expect("class span should have a free slot");

    assert_eq!(reserved, second);
    assert!(heap.is_live(first));
    assert!(heap.is_live(reserved));
    assert!(heap.is_live(third));
    assert_eq!(heap.heap_allocation_count(), 3);
}

/// Clear a reclaimed slot before zeroed allocation reuses it.
#[test]
fn test_collect_full_clears_a_reclaimed_slot_before_zeroed_reuse() {
    let options = HeapOptions::local();
    let layout = test_layout(8, TraceMap::empty());
    let mut heap = test_heap(options);
    let reference = heap.test_allocate(layout.block(), Payload::Bytes(&[9; 8]));
    let mut roots = [];

    // collect with no roots so the slot is reclaimed
    heap.collect_full(
        &mut |visit| visit_roots(&mut roots, visit),
        trace_view(),
        &mut |_| Ok(()),
    )
    .expect("full collection should succeed");
    assert!(!heap.is_live(reference));

    // reuse the slot through the zeroed path
    let reused = heap.test_allocate(layout.block(), Payload::Zeroed);
    let bytes = read_mapped_bytes(heap.storage.base_address() + reused.offset(), 8);

    assert_eq!(reused, reference);
    assert_eq!(bytes, &[0; 8]);
}

/// Keep children reached through traced payload references.
#[test]
fn test_collect_full_keeps_children_through_traced_references() {
    // parent traces one local child reference at offset zero
    let options = tiny_heap_options();
    let trace_map = local_trace_map(&[0]);
    let layouts = test_layouts(&[(2, TraceMap::empty()), (8, trace_map.clone())]);
    let [child_layout, parent_layout]: [TestLayout; 2] =
        layouts.try_into().expect("test layouts should match");
    let mut heap = test_storage(&options);

    // root the parent and make the child reachable only through payload bytes
    let child = heap.test_allocate(child_layout.block(), Payload::Bytes(&[0xC1, 0x1D]));
    let parent = heap.test_allocate(
        parent_layout.block(),
        Payload::Bytes(&child.bits().to_le_bytes()),
    );
    let mut roots = [parent];

    let stats = heap
        .collect_full(
            &mut |visit| visit_roots(&mut roots, visit),
            trace_view(),
            &mut |_| Ok(()),
        )
        .expect("full collection should succeed");

    // keep the reachable graph in place
    assert_eq!(stats.freed_allocations, 0);
    assert_eq!(roots[0], parent);
    assert_eq!(read_heap_reference(&heap, parent), child);
    assert!(heap.is_live(child));

    let bytes = read_mapped_bytes(heap.base_address() + child.offset(), 2);

    assert_eq!(bytes, &[0xC1, 0x1D]);
}

/// Keep children reached through table-backed slot trace metadata.
#[test]
fn test_collect_full_keeps_children_through_table_traced_slots() {
    // store the parent trace map in the explicit trace table
    let options = tiny_heap_options();
    let child_layout = test_layout(2, TraceMap::empty());
    let parent_map = local_trace_map(&[0]);
    let mut trace_table = TraceTable::new();
    let parent_trace_id = trace_table.insert(parent_map.clone());
    let trace_table = TestTraceTable::from_mir(&trace_table);
    let trace_view = trace_table.view();
    let parent_layout = AllocationShape::new(8, 1, Some(parent_trace_id), parent_map);
    let mut heap = test_storage(&options);

    // root the parent and make the child reachable only through table-backed metadata
    let child = heap.test_allocate(child_layout.block(), Payload::Bytes(&[0xC1, 0x1D]));
    let parent = heap.test_allocate(parent_layout, Payload::Bytes(&child.bits().to_le_bytes()));
    let mut roots = [parent];

    let stats = heap
        .collect_full(
            &mut |visit| visit_roots(&mut roots, visit),
            trace_view,
            &mut |_| Ok(()),
        )
        .expect("full collection should succeed");

    assert_eq!(stats.freed_allocations, 0);
    assert!(heap.is_live(parent));
    assert!(heap.is_live(child));
}

/// Keep a block reached only through an interior root at its address.
#[test]
fn test_collect_full_keeps_a_block_through_an_interior_root() {
    let options = tiny_heap_options();
    let layout = test_layout(8, TraceMap::empty());
    let mut heap = test_storage(&options);
    let reference = heap.test_allocate(layout.block(), Payload::Bytes(&[1, 2, 3, 4, 5, 6, 7, 8]));
    let interior = HeapReference::new(reference.offset() + 5);
    let mut roots = [interior];

    let stats = heap
        .collect_full(
            &mut |visit| visit_roots(&mut roots, visit),
            trace_view(),
            &mut |_| Ok(()),
        )
        .expect("full collection should succeed");

    // the interior root resolves to its block, which stays where it is
    assert_eq!(stats.freed_allocations, 0);
    assert_eq!(roots[0], interior);
    assert!(heap.is_live(reference));

    let bytes = read_mapped_bytes(heap.base_address() + reference.offset(), 8);

    assert_eq!(bytes, &[1, 2, 3, 4, 5, 6, 7, 8]);
}

/// Reject invalid explicit roots loudly.
#[test]
fn test_collect_full_rejects_invalid_root() {
    // build a heap with no block at offset 7
    let options = HeapOptions::local();
    let mut heap = test_storage(&options);
    let invalid = HeapReference::new(7);
    let mut roots = [invalid];

    // reject the invalid root before mutating collection state
    let error = heap
        .collect_full(
            &mut |visit| visit_roots(&mut roots, visit),
            trace_view(),
            &mut |_| Ok(()),
        )
        .expect_err("invalid roots should fail collection");

    assert_eq!(error, HeapError::invalid_heap_reference(invalid));
}

/// Record the completed cycle after one full collection.
#[test]
fn test_collect_full_updates_gc_state() {
    let options = HeapOptions::local();
    let layout = test_layout(3, TraceMap::empty());
    let mut heap = test_storage(&options);
    let reachable = heap.test_allocate(layout.block(), Payload::Bytes(&[1, 2, 3]));
    let mut roots = [reachable];

    let stats = heap
        .collect_full(
            &mut |visit| visit_roots(&mut roots, visit),
            trace_view(),
            &mut |_| Ok(()),
        )
        .expect("full collection should succeed");

    // record the completed collection in heap state
    assert_eq!(heap.gc_state().completed_cycles, 1);
    assert_eq!(heap.gc_state().last_collector, Some(GcCollector::Local));
    assert_eq!(heap.gc_state().last_stats, Some(stats));
}

/// Trace shared roots through the full shared-reference width.
#[test]
fn test_trace_shared_roots_uses_shared_reference_width() {
    // allocate one local object with one shared reference field
    let options = HeapOptions::local();
    let trace_map = shared_trace_map(&[0]);
    let layout = test_layout(8, trace_map);
    let mut heap = test_storage(&options);
    let shared = SharedHeapReference::new(7);
    let local = heap.test_allocate(layout.block(), Payload::Bytes(&shared.bits().to_le_bytes()));
    let mut roots = Vec::new();

    // scan the tracked local-to-shared root
    heap.start_shared_edge_scan();

    let scanned_bytes = heap
        .trace_shared_roots(&mut roots, options.page_size_bytes, trace_view())
        .expect("shared root scan should succeed");

    // account for the full shared-reference width
    assert_eq!(scanned_bytes, 8);
    assert_eq!(roots, vec![shared]);
    assert!(heap.shared_edge_scan_idle());

    heap.finish_shared_edge_scan();

    // local ownership remains independent from the shared-edge scan
    assert!(heap.is_live(local));
}

/// Continue local-to-shared edge scans across large block pages.
#[test]
fn test_trace_shared_roots_scans_large_blocks_incrementally() {
    // build one large object with shared references on separate pages
    let options = HeapOptions::local();
    let page_size_bytes = options.page_size_bytes;
    let first_offset = 0usize;
    let second_offset = page_size_bytes;
    let small_limit = options
        .size_classes
        .max_small_allocation_bytes()
        .expect("default size classes should not be empty");
    let parent_byte_len = small_limit + second_offset + SharedHeapReference::BYTE_LEN;
    let second_offset = u32::try_from(second_offset).expect("page offset should fit uint32");
    let trace_map = shared_trace_map(&[first_offset as u32, second_offset]);
    let layout = test_layout(parent_byte_len, trace_map);
    let mut heap = test_storage(&options);
    let first_shared = SharedHeapReference::new(11);
    let second_shared = SharedHeapReference::new(22);
    let mut parent_bytes = vec![0; parent_byte_len];
    parent_bytes[first_offset..first_offset + SharedHeapReference::BYTE_LEN]
        .copy_from_slice(&first_shared.bits().to_le_bytes());
    parent_bytes[second_offset as usize..second_offset as usize + SharedHeapReference::BYTE_LEN]
        .copy_from_slice(&second_shared.bits().to_le_bytes());
    let parent = heap.test_allocate(layout.block(), Payload::Bytes(&parent_bytes));
    let mut roots = Vec::new();

    // scan at a one-page budget
    heap.start_shared_edge_scan();

    let first_scanned = heap
        .trace_shared_roots(&mut roots, 1, trace_view())
        .expect("first shared-root scan should succeed");

    // first step should discover only the first page reference
    assert_eq!(first_scanned, page_size_bytes);
    assert_eq!(roots, vec![first_shared]);
    assert!(!heap.shared_edge_scan_idle());

    let second_scanned = heap
        .trace_shared_roots(&mut roots, 1, trace_view())
        .expect("second shared-root scan should succeed");

    // second step should discover the second page reference
    assert_eq!(second_scanned, page_size_bytes);
    assert_eq!(roots, vec![first_shared, second_shared]);

    // drain the rest of the large block scan cursor
    while !heap.shared_edge_scan_idle() {
        let scanned_bytes = heap
            .trace_shared_roots(&mut roots, 1, trace_view())
            .expect("remaining shared-root scan should succeed");
        assert!(scanned_bytes > 0);
    }

    heap.finish_shared_edge_scan();

    // scanning shared edges should not affect local object liveness
    assert!(heap.is_live(parent));
}

/// Keep unscanned shared-edge roots stable when earlier roots are freed.
#[test]
fn test_trace_shared_roots_survives_active_root_removal() {
    // allocate three local objects that each contain one shared edge
    let options = HeapOptions::local();
    let trace_map = shared_trace_map(&[0]);
    let layout = test_layout(8, trace_map);
    let mut heap = test_storage(&options);
    let first_shared = SharedHeapReference::new(11);
    let second_shared = SharedHeapReference::new(22);
    let third_shared = SharedHeapReference::new(33);
    let first_local = heap.test_allocate(
        layout.block(),
        Payload::Bytes(&first_shared.bits().to_le_bytes()),
    );
    let _second_local = heap.test_allocate(
        layout.block(),
        Payload::Bytes(&second_shared.bits().to_le_bytes()),
    );
    let _third_local = heap.test_allocate(
        layout.block(),
        Payload::Bytes(&third_shared.bits().to_le_bytes()),
    );
    let mut roots = Vec::new();

    // start a cursor-based local-to-shared scan
    heap.start_shared_edge_scan();

    // scan the first root, then remove it while the cursor points past it
    let first_work = heap
        .trace_shared_roots(&mut roots, 1, trace_view())
        .expect("first shared root scan should succeed");
    heap.free(first_local)
        .expect("freeing scanned root should succeed");

    let remaining_work = heap
        .trace_shared_roots(&mut roots, usize::MAX, trace_view())
        .expect("remaining shared root scan should succeed");

    // retain pending roots even when earlier tracked roots are removed
    roots.sort();
    assert_eq!(first_work, 8);
    assert_eq!(remaining_work, 16);
    assert_eq!(roots, vec![first_shared, second_shared, third_shared]);

    heap.finish_shared_edge_scan();
}

/// Stay idle when no local pressure or explicit request exists.
#[test]
fn test_step_collection_stays_idle_without_request() {
    // build an empty heap with no pressure
    let options = HeapOptions::local();
    let mut heap = test_heap(options);
    let mut roots = [];

    // no request and no pressure should produce no work
    let budget_bytes = heap.take_collection_budget_bytes();
    let progress = heap
        .step_collection(
            &mut |visit| visit_roots(&mut roots, visit),
            budget_bytes,
            trace_view(),
        )
        .expect("collection step should succeed");

    // keep the collector idle
    assert_eq!(progress, GcAdvance::Idle);
}

/// Report one unreachable allocation before reclaiming it.
#[test]
fn test_step_collection_drops_an_allocation_before_reclamation() {
    let options = HeapOptions::local();
    let mut heap = test_heap(options);
    let shape = test_layout(16, TraceMap::empty())
        .block()
        .with_drop(DropId::from_index(1))
        .expect("Drop plan should build");
    let reference = heap.test_allocate(shape, Payload::Zeroed);
    let mut roots = [];

    // request one cycle and advance to its drop callback
    heap.request_gc();
    let drop = loop {
        let progress = heap
            .step_collection(
                &mut |visit| visit_roots(&mut roots, visit),
                DEFAULT_GC_MINIMUM_WORK_BYTES,
                trace_view(),
            )
            .expect("collection step should succeed");
        if let GcAdvance::Drop(drop) = progress {
            break drop;
        }
    };

    // preserve the allocation until its callback has run
    assert_eq!(drop.collector, GcCollector::Local);
    assert_eq!(drop.reference, DropReference::Local(reference));
    assert_eq!(drop.drop, DropId::from_index(1));
    assert!(heap.is_live(reference));
    heap.complete_drop(drop.reference)
        .expect("allocation Drop should complete");

    // reclaim the allocation after its destructor completes
    loop {
        let progress = heap
            .step_collection(
                &mut |visit| visit_roots(&mut roots, visit),
                DEFAULT_GC_MINIMUM_WORK_BYTES,
                trace_view(),
            )
            .expect("collection step should succeed");
        if progress.completed_stats().is_some() {
            break;
        }
    }

    assert!(!heap.is_live(reference));
}

/// Surface repeated values individually in reverse acquisition order.
#[test]
fn test_step_collection_drops_repeated_values_incrementally() {
    let options = HeapOptions::local();
    let mut heap = test_heap(options.clone());
    let element = AllocationShape::new(16, 8, None, TraceMap::empty())
        .with_drop(DropId::from_index(2))
        .expect("Drop plan should build");
    let element = options.allocation_plan(&element);
    let shape = element
        .repeat(&TraceMap::empty(), 3)
        .expect("repeated allocation should build");
    let reference = heap.test_allocate(shape, Payload::Zeroed);
    let mut roots = [];
    let mut dropped = Vec::new();

    // collect every value-level Drop request before physical reclamation
    heap.request_gc();
    loop {
        let budget_bytes = DEFAULT_GC_MINIMUM_WORK_BYTES + dropped.len();
        let progress = heap
            .step_collection(
                &mut |visit| visit_roots(&mut roots, visit),
                budget_bytes,
                trace_view(),
            )
            .expect("collection step should succeed");

        if let GcAdvance::Drop(drop) = progress {
            assert_eq!(drop.budget_bytes, budget_bytes);
            dropped.push(drop.reference);
            heap.complete_drop(drop.reference)
                .expect("repeated value Drop should complete");
        }
        if progress.completed_stats().is_some() {
            break;
        }
    }

    assert_eq!(
        dropped,
        [
            DropReference::Local(HeapReference::new(reference.offset() + 32)),
            DropReference::Local(HeapReference::new(reference.offset() + 16)),
            DropReference::Local(reference),
        ]
    );
    assert!(!heap.is_live(reference));
}

/// Hand back the drop plan of an unretained allocation, freeing it once the caller ran it.
#[test]
fn test_release_hands_back_the_plan_of_an_unretained_allocation() {
    let options = HeapOptions::local();
    let mut heap = test_heap(options);
    let shape = test_layout(16, TraceMap::empty())
        .block()
        .with_drop(DropId::from_index(1))
        .expect("Drop plan should build");
    let reference = heap.test_allocate(shape, Payload::Zeroed);

    // destroy the value before the storage returns
    let Release::Destroy { plan, byte_len } =
        heap.release(reference).expect("release should succeed")
    else {
        panic!("an unretained allocation hands back its plan");
    };
    assert_eq!(plan.drop, DropId::from_index(1));
    assert_eq!(
        plan.value_count(byte_len)
            .expect("plan should cover the block"),
        1
    );
    assert!(heap.is_live(reference));

    heap.free(reference).expect("free should succeed");
    assert!(!heap.is_live(reference));
}

/// Free an unretained allocation without a plan immediately.
#[test]
fn test_release_frees_an_unretained_allocation_without_a_plan() {
    let options = HeapOptions::local();
    let mut heap = test_heap(options);
    let layout = test_layout(16, TraceMap::empty());
    let reference = heap.test_allocate(layout.block(), Payload::Zeroed);

    assert_eq!(
        heap.release(reference).expect("release should succeed"),
        Release::Freed
    );
    assert!(!heap.is_live(reference));
}

/// Leave a retained allocation to the collector, which runs its drop plan once.
#[test]
fn test_release_defers_a_retained_allocation_to_the_collector() {
    let options = HeapOptions::local();
    let mut heap = test_heap(options);
    let trace_map = local_trace_map(&[0]);
    let layouts = test_layouts(&[(16, TraceMap::empty()), (8, trace_map)]);
    let [child_layout, parent_layout]: [TestLayout; 2] =
        layouts.try_into().expect("test layouts should match");
    let child_shape = child_layout
        .block()
        .with_drop(DropId::from_index(1))
        .expect("Drop plan should build");

    // store the child in a heap block so heap storage retains it
    let child = heap.test_allocate(child_shape, Payload::Zeroed);
    let _parent = heap.test_allocate(
        parent_layout.block(),
        Payload::Bytes(&child.bits().to_le_bytes()),
    );
    assert_eq!(
        heap.release(child).expect("release should succeed"),
        Release::Retained
    );
    assert!(heap.is_live(child));

    // collect with no roots: the child is destroyed once, then reclaimed
    let mut roots = [];
    let mut dropped = Vec::new();
    heap.collect_full(
        &mut |visit| visit_roots(&mut roots, visit),
        trace_view(),
        &mut |drop| {
            dropped.push(drop.reference);

            Ok(())
        },
    )
    .expect("full collection should succeed");

    assert_eq!(dropped, [DropReference::Local(child)]);
    assert!(!heap.is_live(child));
}

/// Free a retained allocation whose values moved out, leaving it to the collector without its drop plan.
#[test]
fn test_free_leaves_a_retained_allocation_to_the_collector_without_its_plan() {
    let options = HeapOptions::local();
    let mut heap = test_heap(options);
    let trace_map = local_trace_map(&[0]);
    let layouts = test_layouts(&[(16, TraceMap::empty()), (8, trace_map)]);
    let [child_layout, parent_layout]: [TestLayout; 2] =
        layouts.try_into().expect("test layouts should match");
    let child_shape = child_layout
        .block()
        .with_drop(DropId::from_index(1))
        .expect("Drop plan should build");

    // store the child in a heap block, then move its values out
    let child = heap.test_allocate(child_shape, Payload::Zeroed);
    let _parent = heap.test_allocate(
        parent_layout.block(),
        Payload::Bytes(&child.bits().to_le_bytes()),
    );
    heap.free(child).expect("free should succeed");
    assert!(heap.is_live(child));

    // collect with no roots: the child is reclaimed without a drop callback
    let mut roots = [];
    let mut dropped = Vec::new();
    heap.collect_full(
        &mut |visit| visit_roots(&mut roots, visit),
        trace_view(),
        &mut |drop| {
            dropped.push(drop.reference);

            Ok(())
        },
    )
    .expect("full collection should succeed");

    assert_eq!(dropped, []);
    assert!(!heap.is_live(child));
}

/// Run one full bounded cycle after heap block pressure.
#[test]
fn test_step_collection_runs_full_after_pressure() {
    // configure the pacer to trigger after one block
    let (mut heap, layout_ids) = pacing_heap(&[(64, TraceMap::empty())]);
    let layout = &layout_ids[0];
    let root = heap.test_allocate(layout.block(), Payload::Bytes(&[1; 64]));
    let mut roots = [root];

    // collection budget should service the pressure-triggered full cycle
    let budget_bytes = heap.take_collection_budget_bytes();
    let progress = heap
        .step_collection(
            &mut |visit| visit_roots(&mut roots, visit),
            budget_bytes,
            trace_view(),
        )
        .expect("collection step should succeed");
    let stats = progress
        .completed_stats()
        .expect("pressure should request one cycle");
    let root = roots[0];

    // preserve the rooted block and record a full cycle
    assert_eq!(stats.freed_allocations, 0);
    assert!(heap.is_live(root));
    assert_eq!(heap.gc_state().last_collector, Some(GcCollector::Local));
}

/// Honor one explicit full local collection request below the pacing trigger.
#[test]
fn test_step_collection_honors_manual_full_request() {
    // allocate one root below the pacing trigger
    let layout = test_layout(3, TraceMap::empty());
    let options = HeapOptions::local();
    let mut heap = test_heap(options);
    let root = heap.test_allocate(layout.block(), Payload::Bytes(&[1, 2, 3]));
    let mut roots = [root];

    // explicit requests should bypass pressure checks
    heap.request_gc();

    let budget_bytes = heap.take_collection_budget_bytes();
    let progress = heap
        .step_collection(
            &mut |visit| visit_roots(&mut roots, visit),
            budget_bytes,
            trace_view(),
        )
        .expect("collection step should succeed");
    let stats = progress
        .completed_stats()
        .expect("manual request should span one cycle");

    // keep the root and record a full collection
    assert_eq!(stats.freed_allocations, 0);
    assert_eq!(heap.gc_state().last_collector, Some(GcCollector::Local));
}

/// Continue one collection across bounded safepoint work.
#[test]
fn test_step_gc_spreads_one_cycle() {
    // allocate one rooted object and one unreachable object
    let options = HeapOptions::local();
    let layout = test_layout(1, TraceMap::empty());
    let mut heap = test_storage(&options);
    let root = heap.test_allocate(layout.block(), Payload::Bytes(&[1]));
    let garbage = heap.test_allocate(layout.block(), Payload::Bytes(&[2]));
    let mut roots = [root];

    // start a cycle and stop after the first bounded step
    heap.start_gc(&mut |visit| visit_roots(&mut roots, visit))
        .expect("collection should start");

    let first = heap
        .step_gc(&mut |visit| visit_roots(&mut roots, visit), 1, trace_view())
        .expect("step should succeed");

    // first step should leave the cycle active
    assert!(matches!(
        first,
        GcAdvance::Stepped(step)
            if step.collector == GcCollector::Local && step.phase == GcPhase::Mark
    ));
    assert!(heap.collector.is_collecting());

    // finish the bounded cycle
    let stats = heap
        .drain_gc(
            &mut |visit| visit_roots(&mut roots, visit),
            1,
            trace_view(),
            &mut |_| Ok(()),
        )
        .expect("drain should succeed");

    // reclaim only the unreachable block
    assert_eq!(stats.freed_allocations, 1);
    assert!(heap.is_live(root));
    assert!(!heap.is_live(garbage));
    assert!(!heap.collector.is_collecting());
}

/// Keep a slot allocated during an active cycle, born marked.
#[test]
fn test_step_gc_keeps_a_slot_allocated_during_cycle() {
    // start an empty cycle before the mutator allocates again
    let options = HeapOptions::local();
    let layout = test_layout(4, TraceMap::empty());
    let mut heap = test_storage(&options);
    let mut roots = [];

    heap.start_gc(&mut |visit| visit_roots(&mut roots, visit))
        .expect("collection should start");

    // allocate while the cycle is active
    let reference = heap.test_allocate(layout.block(), Payload::Zeroed);

    assert!(matches!(heap.place(reference), Some(HeapPlace::Slot(_))));

    // drain the active cycle
    let stats = heap
        .drain_gc(
            &mut |visit| visit_roots(&mut roots, visit),
            1,
            trace_view(),
            &mut |_| Ok(()),
        )
        .expect("drain should succeed");

    // preserve blocks published into the cycle by treating them as black
    assert_eq!(stats.freed_allocations, 0);
    assert!(heap.is_live(reference));
    assert!(!heap.collector.is_collecting());
}

/// Continue marking across large block pages.
#[test]
fn test_step_gc_scans_large_blocks_incrementally() {
    // build one large object with local references on separate pages
    let options = HeapOptions::local();
    let first_offset = 0usize;
    let second_offset = options.page_size_bytes;
    let parent_byte_len = second_offset + HeapReference::BYTE_LEN;
    let second_offset = u32::try_from(second_offset).expect("page offset should fit uint32");
    let trace_map = local_trace_map(&[first_offset as u32, second_offset]);

    let layouts = test_layouts(&[(2, TraceMap::empty()), (parent_byte_len, trace_map)]);
    let child_layout = &layouts[0];
    let parent_layout = &layouts[1];
    let mut heap = test_storage(&options);
    let first_child = heap.test_allocate(child_layout.block(), Payload::Bytes(&[0xC1, 0x1D]));
    let second_child = heap.test_allocate(child_layout.block(), Payload::Bytes(&[0xC2, 0x1D]));
    let mut parent_bytes = vec![0; parent_byte_len];
    parent_bytes[first_offset..first_offset + HeapReference::BYTE_LEN]
        .copy_from_slice(&first_child.bits().to_le_bytes());
    parent_bytes[second_offset as usize..second_offset as usize + HeapReference::BYTE_LEN]
        .copy_from_slice(&second_child.bits().to_le_bytes());
    let parent = heap.test_allocate(parent_layout.block(), Payload::Bytes(&parent_bytes));
    let mut roots = [parent];

    // start marking from the large parent root
    heap.start_gc(&mut |visit| visit_roots(&mut roots, visit))
        .expect("collection should start");

    let first = heap
        .step_gc(&mut |visit| visit_roots(&mut roots, visit), 1, trace_view())
        .expect("step should succeed");

    // first step should not scan the entire large block
    assert!(matches!(
        first,
        GcAdvance::Stepped(step)
            if step.collector == GcCollector::Local && step.phase == GcPhase::Mark
    ));
    assert!(heap.collector.is_collecting());

    // finish the bounded scan and sweep
    let stats = heap
        .drain_gc(
            &mut |visit| visit_roots(&mut roots, visit),
            1,
            trace_view(),
            &mut |_| Ok(()),
        )
        .expect("drain should succeed");

    // retain both children reached through the large parent
    assert_eq!(stats.freed_allocations, 0);
    assert!(heap.is_live(first_child));
    assert!(heap.is_live(second_child));
}

/// Repeated full collection should free later unreachable blocks too.
#[test]
fn test_collect_full_reclaims_later_unreachable_allocations() {
    // seed one root and one unreachable block
    let options = HeapOptions::local();
    let layout = test_layout(1, TraceMap::empty());
    let mut heap = test_storage(&options);
    let root = heap.test_allocate(layout.block(), Payload::Bytes(&[1]));
    let _garbage = heap.test_allocate(layout.block(), Payload::Bytes(&[2]));
    let mut roots = [root];

    // the first full cycle should leave only the explicit root
    heap.collect_full(
        &mut |visit| visit_roots(&mut roots, visit),
        trace_view(),
        &mut |_| Ok(()),
    )
    .expect("full collection should succeed");

    assert_eq!(heap.allocation_count(), 1);

    let _second_garbage = heap.test_allocate(layout.block(), Payload::Bytes(&[3]));
    let _third_garbage = heap.test_allocate(layout.block(), Payload::Bytes(&[4]));

    // the next cycle should clear the later unreachable blocks
    heap.collect_full(
        &mut |visit| visit_roots(&mut roots, visit),
        trace_view(),
        &mut |_| Ok(()),
    )
    .expect("full collection should succeed");

    assert_eq!(heap.allocation_count(), 1);
}

/// Close the fast path and keep slow-path allocations live during an active sweep.
#[test]
fn test_reserve_small_closes_during_an_active_cycle() {
    // build one full heap so the public fast reservation path is available
    let options = HeapOptions::local();
    let mut heap = test_heap(options);
    let layout = test_layout(16, TraceMap::empty());
    let plan = heap.options().allocation_plan(&layout.block());
    let small_plan = plan.small_allocation().expect("plan should be small");
    let mut roots: Vec<HeapReference> = Vec::new();

    // fill enough slots to keep sweep busy
    for _ in 0..10 {
        heap.test_allocate(layout.block(), Payload::Zeroed);
    }

    // drive an empty cycle through Drop into sweep
    heap.storage
        .start_gc(&mut |visit| visit_roots(&mut roots, visit))
        .expect("collection should start");
    while heap.storage.collector.phase != Phase::Sweep {
        heap.storage
            .step_gc(&mut |visit| visit_roots(&mut roots, visit), 1, trace_view())
            .expect("step should succeed");
    }
    assert_eq!(heap.storage.collector.phase, Phase::Sweep);

    // close the fast path while the cycle is active
    assert!(heap.reserve_small(small_plan).is_none());

    // publish the block through the slow path instead
    let reference = heap
        .allocate_zeroed(plan, &layout.trace_map)
        .expect("slow-path block should allocate");

    // drain the active cycle
    let stats = heap
        .storage
        .drain_gc(
            &mut |visit| visit_roots(&mut roots, visit),
            usize::MAX,
            trace_view(),
            &mut |_| Ok(()),
        )
        .expect("drain should succeed");

    // free only the unrooted primers and keep the mid-sweep block
    assert_eq!(stats.freed_allocations, 10);
    assert!(heap.is_live(reference));
}

/// Share one span across no-scan classes that differ only by trace id.
#[test]
fn test_reserve_slot_shares_a_span_across_noscan_trace_ids() {
    // two no-scan layouts share one size class under different trace ids
    let options = HeapOptions::local();
    let first_map = TraceMap::Empty;
    let second_map = TraceMap::Nested {
        byte_offset: 0,
        map: Box::new(TraceMap::Empty),
    };
    let mut trace_table = TraceTable::new();
    let first_trace_id = trace_table.insert(first_map.clone());
    let second_trace_id = trace_table.insert(second_map.clone());
    let first_layout = AllocationShape::new(16, 1, Some(first_trace_id), first_map);
    let second_layout = AllocationShape::new(16, 1, Some(second_trace_id), second_map);
    let mut heap = test_storage(&options);

    // allocate alternately between the two classes
    for _ in 0..2 {
        heap.test_allocate(first_layout.clone(), Payload::Zeroed);
        heap.test_allocate(second_layout.clone(), Payload::Zeroed);
    }

    assert_eq!(heap.small.spans.len(), 1);
}

/// Keep an object that a borrow root points into.
#[test]
fn test_collect_full_keeps_an_object_reached_by_an_interior_borrow() {
    let options = tiny_heap_options();
    let layout = test_layout(16, TraceMap::empty());
    let mut heap = test_storage(&options);
    let object = heap.test_allocate(layout.block(), Payload::Zeroed);

    // root one generic borrow at the object's second word
    let mut frame = (object.offset() + 8).to_le_bytes();
    let borrow = TraceMap::reference(Reference::Borrowed, &Lifetime::bound(0));
    let stats = heap
        .collect_full(
            &mut |visit| visit_heap_root_slots(&borrow, 0, &mut frame, ReferenceRange::All, visit),
            trace_view(),
            &mut |_| Ok(()),
        )
        .expect("full collection should succeed");

    assert_eq!(stats.freed_allocations, 0);
    assert!(heap.is_live(object));
}

/// Skip a borrow root that addresses memory outside the heap.
#[test]
fn test_collect_full_skips_a_borrow_root_outside_the_heap() {
    let options = tiny_heap_options();
    let memory = test_memory(options.page_size_bytes);
    let mut heap = HeapStorage::new(memory.clone(), &options).expect("heap should build");
    let layout = test_layout(16, TraceMap::empty());
    let unreachable = heap.test_allocate(layout.block(), Payload::Zeroed);

    // root one generic borrow into frame storage beside the heap
    let frame_range = memory
        .allocate(64, 8)
        .expect("frame storage should allocate");
    let mut frame = frame_range.offset.to_le_bytes();
    let borrow = TraceMap::reference(Reference::Borrowed, &Lifetime::bound(0));
    let stats = heap
        .collect_full(
            &mut |visit| visit_heap_root_slots(&borrow, 0, &mut frame, ReferenceRange::All, visit),
            trace_view(),
            &mut |_| Ok(()),
        )
        .expect("full collection should skip the frame borrow");

    assert_eq!(stats.freed_allocations, 1);
    assert!(!heap.is_live(unreachable));
}
