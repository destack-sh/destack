use crate::local::gc::Phase;
use crate::local::storage::{HeapPlace, HeapStorage};
use crate::{
    AllocationShape, GcKind, GcOptions, GcProgress, Heap, HeapError, HeapOptions, HeapReference,
    HeapResult, Payload, RootSlot, SharedHeapReference, SizeClassTable, TestLayout,
    local_trace_map, shared_trace_map, test_layout, test_layouts, visit_heap_references,
};
use destack_mir::{TraceMap, TraceTable};

use super::{
    TestHeapPlan, allocation_site, read_mapped_bytes, test_heap, test_storage, trace_table,
    write_mapped_bytes,
};

/// Build one heap whose pacer triggers immediately in step-driven tests.
fn pacing_heap(layouts: &[(usize, TraceMap)]) -> (Heap, Vec<TestLayout>) {
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

    (test_heap(options), test_layouts(layouts))
}

/// Build heap options with tiny mature spans for focused GC tests.
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

/// Return the young span index for one fixed-size young block.
fn young_slot_span_index(heap: &HeapStorage, reference: HeapReference) -> usize {
    let Some(HeapPlace::YoungSlot(slot)) = heap.place(reference) else {
        panic!("reference should point to a young span slot");
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

/// Promote reachable young blocks and free unreachable young space state.
#[test]
fn test_collect_minor_promotes_reachable_entries() {
    // build a tiny mature space so promotion is easy to observe
    let options = tiny_heap_options();
    let layout = test_layout(3, TraceMap::empty());
    let mut heap = test_storage(&options);

    // root 1,2,3 and leave 4,5,6 unreachable
    let reachable = heap.test_allocate(layout.block(), Payload::Bytes(&[1, 2, 3]));
    let unreachable = heap.test_allocate(layout.block(), Payload::Bytes(&[4, 5, 6]));
    let mut roots = [reachable];

    // verify both blocks begin in the nursery
    assert!(heap.is_young(reachable));
    assert!(heap.is_young(unreachable));

    // collect the nursery from the explicit root set
    let stats = heap
        .collect_minor(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect("young collection should succeed");

    // promote the reachable payload and drop the unreachable payload
    assert_eq!(stats.freed_allocations, 1);
    assert_ne!(roots[0], reachable);
    assert!(!heap.is_live(unreachable));
    assert!(!heap.is_live(reachable));
    assert!(heap.is_live(roots[0]));
    assert!(!heap.is_young(roots[0]));

    // preserve the rooted payload bytes through promotion
    let address = heap.base_address() + roots[0].offset();

    let bytes = read_mapped_bytes(address, 3);

    assert_eq!(bytes, &[1, 2, 3]);

    // keep the reserved nursery pages represented in heap images
    let image = heap.image().expect("heap image should capture");
    let young = image.young();

    assert_eq!(
        young.bytes().len().div_ceil(young.page_size_bytes()),
        young.capacity_bytes().div_ceil(young.page_size_bytes())
    );
}

/// Promote reachable fixed-size young span slots.
#[test]
fn test_collect_minor_promotes_reachable_noscan_spans() {
    // use the default young span path for fixed-size no-scan payloads
    let options = HeapOptions::local();
    let layout = test_layout(4, TraceMap::empty());
    let mut heap = test_heap(options);

    // root the single zeroed span slot
    let reference = heap.test_allocate(layout.block(), Payload::Zeroed);
    let mut roots = [reference];

    // verify the block used the young fixed-size span path
    assert!(matches!(
        heap.storage.place(reference),
        Some(HeapPlace::YoungSlot(_))
    ));

    // collect the nursery from the fixed-size span root
    let stats = heap
        .collect_minor(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect("young collection should succeed");

    // promote the live span slot without freeing anything
    assert_eq!(stats.freed_allocations, 0);
    assert_ne!(roots[0], reference);
    assert!(!heap.storage.is_live(reference));
    assert!(heap.storage.is_live(roots[0]));
    assert!(!heap.is_young(roots[0]));

    // preserve the zeroed slot bytes through promotion
    let address = heap.storage.base_address() + roots[0].offset();

    let bytes = read_mapped_bytes(address, 8);

    assert_eq!(bytes, &[0; 8]);
}

/// Reuse exact young spans after another trace class becomes active.
#[test]
fn test_reserve_local_young_span_uses_exact_trace_cache() {
    let first_map = local_trace_map(&[0]);
    let second_map = local_trace_map(&[8]);
    let mut trace_table = TraceTable::new();
    let first_trace_id = trace_table.insert(first_map.clone());
    let second_trace_id = trace_table.insert(second_map.clone());
    let options = HeapOptions::local();
    let mut heap = test_heap(options);
    let first_shape = AllocationShape::new(16, 1, Some(first_trace_id), &first_map);
    let second_shape = AllocationShape::new(16, 1, Some(second_trace_id), &second_map);
    let first_site = allocation_site(heap.options(), first_shape);
    let second_site = allocation_site(heap.options(), second_shape);

    // allocate from two same-size traced classes
    let first = heap
        .allocate_zeroed(first_site, &first_map)
        .expect("first local block should succeed");
    let _second = heap
        .allocate_zeroed(second_site, &second_map)
        .expect("second local block should succeed");
    let first_again = heap
        .allocate_zeroed(first_site, &first_map)
        .expect("first trace class should still have a cached span");

    // reuse the first trace class span instead of allocating a third span
    let first_span = young_slot_span_index(&heap.storage, first);
    let first_again_span = young_slot_span_index(&heap.storage, first_again);

    assert_eq!(first_again_span, first_span);
}

/// Clear recycled young pages before zeroed block reuses them.
#[test]
fn test_collect_minor_recycles_young_zeroed_bytes() {
    // allocate nonzero bytes in young space
    let options = HeapOptions::local();
    let layout = test_layout(8, TraceMap::empty());
    let mut heap = test_heap(options);
    let reference = heap.test_allocate(layout.block(), Payload::Bytes(&[9; 8]));
    let mut roots = [];

    assert!(heap.is_young(reference));

    // collect with no roots so the young page can be recycled
    heap.collect_minor(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect("young collection should succeed");

    // reuse young space through the zeroed path
    let reference = heap.test_allocate(layout.block(), Payload::Zeroed);
    let address = heap.storage.base_address() + reference.offset();

    // recycled bytes should be cleared before reuse
    let bytes = read_mapped_bytes(address, 8);

    assert_eq!(bytes, &[0; 8]);
}

/// Promote young blocks reached through traced young references.
#[test]
fn test_collect_minor_promotes_reachable_child_entries() {
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

    // collect and rewrite both parent and child references
    let stats = heap
        .collect_minor(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect("young collection should succeed");

    // promote the reachable graph without freeing anything
    assert_eq!(stats.freed_allocations, 0);
    assert_ne!(roots[0], parent);
    assert!(!heap.is_live(parent));
    assert!(heap.is_live(roots[0]));
    assert!(!heap.is_young(roots[0]));

    let traced_child = read_heap_reference(&heap, roots[0]);

    // update the parent payload to the promoted child reference
    assert_ne!(traced_child, child);
    assert!(!heap.is_live(child));
    assert!(heap.is_live(traced_child));
    assert!(!heap.is_young(traced_child));
    let child_address = heap.base_address() + traced_child.offset();

    // preserve the child payload bytes through promotion
    let bytes = read_mapped_bytes(child_address, 2);

    assert_eq!(bytes, &[0xC1, 0x1D]);
}

/// Promote young span slots with table-backed trace metadata.
#[test]
fn test_collect_minor_promotes_table_traced_young_span_slots() {
    // store the parent trace map in the explicit trace table
    let options = tiny_heap_options();
    let child_layout = test_layout(2, TraceMap::empty());
    let parent_map = local_trace_map(&[0]);
    let mut trace_table = TraceTable::new();
    let parent_trace_id = trace_table.insert(parent_map.clone());
    let parent_layout = AllocationShape::new(8, 1, Some(parent_trace_id), &parent_map);
    let mut heap = test_storage(&options);

    // root the parent and make the child reachable only through table-backed metadata
    let child = heap.test_allocate(child_layout.block(), Payload::Bytes(&[0xC1, 0x1D]));
    let parent = heap.test_allocate(parent_layout, Payload::Bytes(&child.bits().to_le_bytes()));
    let mut roots = [parent];

    assert!(heap.is_young(parent));

    // collect using the trace table that owns the parent map
    let stats = heap
        .collect_minor(&mut |visit| visit_roots(&mut roots, visit), &trace_table)
        .expect("young collection should succeed");

    // promote the parent and preserve its traced child edge
    assert_eq!(stats.freed_allocations, 0);
    assert_ne!(roots[0], parent);
    assert!(!heap.is_live(parent));
    assert!(heap.is_live(roots[0]));

    let traced_child = read_heap_reference(&heap, roots[0]);

    // update the parent payload to the promoted child reference
    assert_ne!(traced_child, child);
    assert!(!heap.is_live(child));
    assert!(heap.is_live(traced_child));
}

/// Reject invalid explicit young roots loudly.
#[test]
fn test_collect_minor_rejects_invalid_root() {
    // build a heap with no block at offset 7
    let options = HeapOptions::local();
    let mut heap = test_storage(&options);
    let invalid = HeapReference::new(7);
    let mut roots = [invalid];

    // reject the invalid root before mutating collection state
    let error = heap
        .collect_minor(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect_err("invalid roots should fail collection");

    assert_eq!(error, HeapError::invalid_heap_reference(invalid));
}

/// Record the completed young GC cycle after one local young collection.
#[test]
fn test_collect_minor_updates_gc_state() {
    // allocate one reachable young payload
    let options = HeapOptions::local();
    let layout = test_layout(3, TraceMap::empty());
    let mut heap = test_storage(&options);
    let reachable = heap.test_allocate(layout.block(), Payload::Bytes(&[1, 2, 3]));
    let mut roots = [reachable];

    // span one complete minor collection
    let stats = heap
        .collect_minor(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect("young collection should succeed");

    // record the completed collection in heap state
    assert_eq!(heap.gc_state().completed_cycles, 1);
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Minor));
    assert_eq!(heap.gc_state().last_stats, Some(stats));
}

/// Pinning one young reference should preserve it in storage.
#[test]
fn test_pin_preserves_young_reference() {
    // allocate one young payload
    let options = tiny_heap_options();
    let layout = test_layout(3, TraceMap::empty());
    let mut heap = test_storage(&options);
    let reference = heap.test_allocate(layout.block(), Payload::Bytes(&[1, 2, 3]));

    assert!(heap.is_young(reference));

    // pinning a young reference keeps the same base block live and immobile
    let reference = heap.pin(reference).expect("pin should succeed");

    assert_eq!(
        heap.collector.pins.references().collect::<Vec<_>>(),
        vec![reference]
    );
    assert!(heap.is_young(reference));
    let address = heap.base_address() + reference.offset();

    // pinned payload bytes stay in the original young slot
    let bytes = read_mapped_bytes(address, 16);

    assert_eq!(bytes, &[1, 2, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
}

/// Pinning an interior young reference preserves its byte offset in storage.
#[test]
fn test_pin_preserves_interior_young_reference() {
    // allocate one young payload and point inside it
    let options = tiny_heap_options();
    let layout = test_layout(8, TraceMap::empty());
    let mut heap = test_storage(&options);
    let reference = heap.test_allocate(layout.block(), Payload::Bytes(&[1, 2, 3, 4, 5, 6, 7, 8]));
    let interior = HeapReference::new(reference.offset() + 3);

    // pinning the interior reference records the base block
    let interior = heap.pin(interior).expect("pin should succeed");
    let extent = heap
        .resolve_extent(interior)
        .expect("interior pin should resolve");

    assert_eq!(extent.byte_offset, 3);
    assert_eq!(
        heap.collector.pins.references().collect::<Vec<_>>(),
        vec![extent.base]
    );
    assert!(heap.is_young(interior));

    // unpinning through the same interior reference clears the pin set
    heap.unpin(interior).expect("interior unpin should succeed");

    assert!(heap.collector.pins.references().next().is_none());
}

/// Minor collection promotes interior young roots without losing their offset.
#[test]
fn test_collect_minor_promotes_interior_roots() {
    // allocate one young payload and root an interior address
    let options = tiny_heap_options();
    let layout = test_layout(8, TraceMap::empty());
    let mut heap = test_storage(&options);
    let reference = heap.test_allocate(layout.block(), Payload::Bytes(&[1, 2, 3, 4, 5, 6, 7, 8]));
    let mut roots = [HeapReference::new(reference.offset() + 5)];

    // collection should rewrite the interior root, not just the base
    heap.collect_minor(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect("young collection should succeed");
    let extent = heap
        .resolve_extent(roots[0])
        .expect("interior root should resolve");

    // preserve the interior byte offset after promotion
    assert_eq!(extent.byte_offset, 5);
    assert_ne!(roots[0], HeapReference::new(reference.offset() + 5));
    assert!(!heap.is_live(reference));
    assert!(heap.is_live(extent.base));
    assert!(!heap.is_young(extent.base));
}

/// Pinned roots should stay in storage while young children promote.
#[test]
fn test_collect_minor_traces_pinned_roots() {
    // parent traces one local child reference at offset zero
    let options = tiny_heap_options();
    let trace_map = local_trace_map(&[0]);
    let layouts = test_layouts(&[(2, TraceMap::empty()), (8, trace_map.clone())]);
    let [child_layout, parent_layout]: [TestLayout; 2] =
        layouts.try_into().expect("test layouts should match");
    let mut heap = test_storage(&options);

    // pin the parent and leave the explicit root set empty
    let child = heap.test_allocate(child_layout.block(), Payload::Bytes(&[0xC1, 0x1D]));
    let parent = heap.test_allocate(
        parent_layout.block(),
        Payload::Bytes(&child.bits().to_le_bytes()),
    );
    let parent = heap.pin(parent).expect("pin should succeed");
    let mut roots = [];

    // pinned roots should seed minor marking
    let stats = heap
        .collect_minor(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect("young collection should succeed");

    // keep the pinned parent in storage while promoting the child
    assert_eq!(stats.freed_allocations, 0);
    assert_eq!(
        heap.collector.pins.references().collect::<Vec<_>>(),
        vec![parent]
    );
    assert!(heap.is_young(parent));
    assert!(heap.is_live(parent));

    let traced_child = read_heap_reference(&heap, parent);

    // rewrite the pinned parent payload to the promoted child
    assert_ne!(traced_child, child);
    assert!(!heap.is_live(child));
    assert!(heap.is_live(traced_child));
    assert!(!heap.is_young(traced_child));
}

/// Keep mature dirty cards while they still point at pinned young children.
#[test]
fn test_collect_minor_retains_dirty_card_for_pinned_young_child() {
    // force the parent into mature space and keep the child young
    let options = HeapOptions {
        max_heap_young_allocation_size_bytes: 8,
        ..tiny_heap_options()
    };
    let trace_map = local_trace_map(&[0]);
    let layouts = test_layouts(&[(4, TraceMap::empty()), (16, trace_map.clone())]);
    let [child_layout, parent_layout]: [TestLayout; 2] =
        layouts.try_into().expect("test layouts should match");
    let mut heap = test_storage(&options);

    // pin the child so the parent dirty card still points into young space
    let child = heap.test_allocate(child_layout.block(), Payload::Zeroed);
    let child = heap.pin(child).expect("pin should succeed");
    let mut parent_bytes = [0u8; 16];
    parent_bytes[..HeapReference::BYTE_LEN].copy_from_slice(&child.bits().to_le_bytes());
    let parent = heap.test_allocate(parent_layout.block(), Payload::Bytes(&parent_bytes));
    let mut roots = [parent];

    assert!(heap.is_young(child));
    assert!(!heap.is_young(parent));

    // first minor collection should retain the dirty card for the pinned child
    heap.collect_minor(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect("young collection should keep pinned child");

    assert_eq!(read_heap_reference(&heap, roots[0]), child);
    assert!(heap.is_live(child));
    assert!(heap.is_young(child));

    // after unpinning, the retained dirty card should let minor collection promote the child
    heap.unpin(child).expect("unpin should succeed");
    heap.collect_minor(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect("young collection should rescan retained card");

    let promoted_child = read_heap_reference(&heap, roots[0]);

    // rewrite the mature parent payload to the promoted child
    assert_ne!(promoted_child, child);
    assert!(!heap.is_live(child));
    assert!(heap.is_live(promoted_child));
    assert!(!heap.is_young(promoted_child));
}

/// Pinned mature references should stay live during full collection without explicit roots.
#[test]
fn test_collect_full_traces_pinned_roots() {
    // allocate one mature object and pin it
    let options = HeapOptions {
        heap_young_size_bytes: 0,
        ..HeapOptions::local()
    };
    let layout = test_layout(3, TraceMap::empty());
    let mut heap = test_storage(&options);
    let reference = heap.test_allocate(layout.block(), Payload::Bytes(&[1, 2, 3]));
    let reference = heap.pin(reference).expect("pin should succeed");
    let mut roots = [];

    // full collection should trace pins even without explicit roots
    let stats = heap
        .collect_full(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect("full collection should succeed");

    // keep the pinned payload live
    assert_eq!(stats.freed_allocations, 0);
    assert!(heap.is_live(reference));
    let address = heap.base_address() + reference.offset();

    // preserve mature payload bytes
    let bytes = read_mapped_bytes(address, 16);

    assert_eq!(bytes, &[1, 2, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
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
        .trace_shared_roots(&mut roots, options.page_size_bytes, trace_table())
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
    let options = HeapOptions {
        heap_young_size_bytes: 0,
        ..HeapOptions::local()
    };
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
        .trace_shared_roots(&mut roots, 1, trace_table())
        .expect("first shared-root scan should succeed");

    // first step should discover only the first page reference
    assert_eq!(first_scanned, page_size_bytes);
    assert_eq!(roots, vec![first_shared]);
    assert!(!heap.shared_edge_scan_idle());

    let second_scanned = heap
        .trace_shared_roots(&mut roots, 1, trace_table())
        .expect("second shared-root scan should succeed");

    // second step should discover the second page reference
    assert_eq!(second_scanned, page_size_bytes);
    assert_eq!(roots, vec![first_shared, second_shared]);

    // drain the rest of the large block scan cursor
    while !heap.shared_edge_scan_idle() {
        let scanned_bytes = heap
            .trace_shared_roots(&mut roots, 1, trace_table())
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
        .trace_shared_roots(&mut roots, 1, trace_table())
        .expect("first shared root scan should succeed");
    heap.free(first_local)
        .expect("freeing scanned root should succeed");

    let remaining_work = heap
        .trace_shared_roots(&mut roots, usize::MAX, trace_table())
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
            trace_table(),
        )
        .expect("collection step should succeed");

    // keep the collector idle
    assert_eq!(progress, GcProgress::Idle);
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
            trace_table(),
        )
        .expect("collection step should succeed");
    let stats = progress
        .completed_stats()
        .expect("pressure should request one cycle");
    let root = roots[0];

    // preserve the rooted block and record a full cycle
    assert_eq!(stats.freed_allocations, 0);
    assert!(heap.is_heap_live(root));
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Full));
}

/// Run one minor cycle after young space occupancy crosses the configured trigger.
#[test]
fn test_step_collection_runs_minor_after_young_occupancy() {
    // configure a tiny nursery and a young trigger below full occupancy
    let options = HeapOptions {
        gc: GcOptions {
            growth_percent: 100,
            trigger_percent: 75,
            soft_limit_bytes: None,
            minimum_heap_bytes: Some(1024 * 1024),
            minimum_work_bytes: crate::DEFAULT_GC_MINIMUM_WORK_BYTES,
        },
        heap_young_size_bytes: 64,
        max_heap_young_allocation_size_bytes: 16,
        ..HeapOptions::local()
    };
    let mut heap = test_heap(options);
    let layout = test_layout(16, TraceMap::empty());
    let mut roots = Vec::new();

    // fill the young prefix to the configured trigger
    for _ in 0..3 {
        let reference = heap.test_allocate(layout.block(), Payload::Bytes(&[1; 16]));
        roots.push(reference);
    }

    // young occupancy should request and complete one minor collection
    let budget_bytes = heap.take_collection_budget_bytes();
    let progress = heap
        .step_collection(
            &mut |visit| visit_roots(&mut roots, visit),
            budget_bytes,
            trace_table(),
        )
        .expect("collection step should succeed");

    // record a minor cycle
    assert_eq!(progress.completed_stats().map(|_| ()), Some(()));
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Minor));
}

/// Bound minor collection work at one safepoint.
#[test]
fn test_step_collection_bounds_minor_at_safepoint() {
    // configure a tiny nursery and a young trigger below full occupancy
    let options = HeapOptions {
        gc: GcOptions {
            growth_percent: 100,
            trigger_percent: 75,
            soft_limit_bytes: None,
            minimum_heap_bytes: Some(1024 * 1024),
            minimum_work_bytes: crate::DEFAULT_GC_MINIMUM_WORK_BYTES,
        },
        heap_young_size_bytes: 64,
        max_heap_young_allocation_size_bytes: 16,
        ..HeapOptions::local()
    };
    let mut heap = test_heap(options);
    let layout = test_layout(16, TraceMap::empty());
    let mut roots = Vec::new();

    // fill young space past the trigger
    for _ in 0..3 {
        let reference = heap.test_allocate(layout.block(), Payload::Bytes(&[1; 16]));
        roots.push(reference);
    }

    // minor collection should respect the caller budget
    let progress = heap
        .step_collection(
            &mut |visit| visit_roots(&mut roots, visit),
            1,
            trace_table(),
        )
        .expect("small-budget collection should succeed");

    // leave the cycle active
    assert_eq!(progress, GcProgress::Active);
    assert_eq!(heap.gc_state().last_kind, None);
}

/// Avoid expanding local collection budgets to cover a large nursery.
#[test]
fn test_collect_budget_does_not_expand_to_large_nursery() {
    // configure a nursery larger than the minimum work quantum
    let options = HeapOptions {
        gc: GcOptions {
            growth_percent: 100,
            trigger_percent: 75,
            soft_limit_bytes: None,
            minimum_heap_bytes: Some(1024 * 1024),
            minimum_work_bytes: 64,
        },
        heap_young_size_bytes: 1024,
        max_heap_young_allocation_size_bytes: 256,
        ..HeapOptions::local()
    };
    let mut heap = test_heap(options);
    let layout = test_layout(256, TraceMap::empty());

    // fill young space beyond one configured safepoint quantum
    for _ in 0..3 {
        let _reference = heap.test_allocate(layout.block(), Payload::Bytes(&[1; 256]));
    }

    // budget stays zero until explicit safepoint collection is chosen
    let budget_bytes = heap.take_collection_budget_bytes();

    assert_eq!(budget_bytes, 0);
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
    heap.request_full_gc();

    let budget_bytes = heap.take_collection_budget_bytes();
    let progress = heap
        .step_collection(
            &mut |visit| visit_roots(&mut roots, visit),
            budget_bytes,
            trace_table(),
        )
        .expect("collection step should succeed");
    let stats = progress
        .completed_stats()
        .expect("manual request should span one cycle");

    // keep the root and record a full collection
    assert_eq!(stats.freed_allocations, 0);
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Full));
}

/// Continue one local major collection across bounded safepoint work.
#[test]
fn test_step_major_gc_spreads_full_cycle() {
    // allocate one rooted object and one unreachable object in mature space
    let options = HeapOptions {
        heap_young_size_bytes: 0,
        ..HeapOptions::local()
    };
    let layout = test_layout(1, TraceMap::empty());
    let mut heap = test_storage(&options);
    let root = heap.test_allocate(layout.block(), Payload::Bytes(&[1]));
    let garbage = heap.test_allocate(layout.block(), Payload::Bytes(&[2]));
    let mut roots = [root];

    // start a major cycle and stop after the first bounded step
    heap.start_major_gc(&mut |visit| visit_roots(&mut roots, visit))
        .expect("major collection should start");

    let first = heap
        .step_major_gc(
            &mut |visit| visit_roots(&mut roots, visit),
            1,
            trace_table(),
        )
        .expect("major step should succeed");

    // first step should leave the major cycle active
    assert_eq!(first, GcProgress::Active);
    assert!(heap.major_gc_active());

    // finish the bounded major cycle
    let stats = heap
        .drain_major_gc(
            &mut |visit| visit_roots(&mut roots, visit),
            1,
            trace_table(),
        )
        .expect("major drain should succeed");

    // reclaim only the unreachable mature block
    assert_eq!(stats.freed_allocations, 1);
    assert!(heap.is_live(root));
    assert!(!heap.is_live(garbage));
    assert!(!heap.major_gc_active());
}

/// Keep young no-scan span slots allocated during an active major cycle.
#[test]
fn test_step_major_gc_keeps_young_noscan_span_allocated_during_cycle() {
    // start an empty major cycle before the mutator allocates again
    let options = HeapOptions::local();
    let layout = test_layout(4, TraceMap::empty());
    let mut heap = test_storage(&options);
    let mut roots = [];

    heap.start_major_gc(&mut |visit| visit_roots(&mut roots, visit))
        .expect("major collection should start");

    // allocate through the young no-scan span path while the cycle is active
    let reference = heap.test_allocate(layout.block(), Payload::Zeroed);

    assert!(matches!(
        heap.place(reference),
        Some(HeapPlace::YoungSlot(_))
    ));

    // drain the active cycle
    let stats = heap
        .drain_major_gc(
            &mut |visit| visit_roots(&mut roots, visit),
            1,
            trace_table(),
        )
        .expect("major drain should succeed");

    // preserve post-cycle blocks by treating them as black
    assert_eq!(stats.freed_allocations, 0);
    assert!(heap.is_live(reference));
    assert!(!heap.major_gc_active());
}

/// Continue local major marking across large block pages.
#[test]
fn test_step_major_gc_scans_large_blocks_incrementally() {
    // build one large object with local references on separate pages
    let options = HeapOptions {
        heap_young_size_bytes: 0,
        ..HeapOptions::local()
    };
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

    // start major marking from the large parent root
    heap.start_major_gc(&mut |visit| visit_roots(&mut roots, visit))
        .expect("major collection should start");

    let first = heap
        .step_major_gc(
            &mut |visit| visit_roots(&mut roots, visit),
            1,
            trace_table(),
        )
        .expect("major step should succeed");

    // first step should not scan the entire large block
    assert_eq!(first, GcProgress::Active);
    assert!(heap.major_gc_active());

    // finish the bounded scan and sweep
    let stats = heap
        .drain_major_gc(
            &mut |visit| visit_roots(&mut roots, visit),
            1,
            trace_table(),
        )
        .expect("major drain should succeed");

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
    heap.collect_full(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect("full collection should succeed");

    assert_eq!(heap.allocation_count(), 1);

    let more_garbage = heap.test_allocate(layout.block(), Payload::Bytes(&[3]));
    let even_more = heap.test_allocate(layout.block(), Payload::Bytes(&[4]));

    // later blocks should still enter young space
    assert!(heap.is_young(more_garbage));
    assert!(heap.is_young(even_more));

    // the next minor cycle should clear the unreachable nursery
    heap.collect_minor(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect("minor collection should succeed");

    assert_eq!(heap.allocation_count(), 1);
}

/// Keep rooted young allocations made during an active minor sweep.
// FUGU #Broken: young allocations made during an active minor sweep are never marked and get swept
#[test]
fn test_step_young_gc_keeps_rooted_allocation_during_sweep() {
    // young range layouts carry one local reference and no table id
    let options = HeapOptions::local();
    let range_map = local_trace_map(&[0]);
    let layout = AllocationShape::new(8, 1, None, &range_map);
    let mut heap = test_storage(&options);

    // root one survivor and leave several ranges unreachable for sweep work
    let survivor = heap.test_allocate(layout, Payload::Zeroed);
    for _ in 0..12 {
        heap.test_allocate(layout, Payload::Zeroed);
    }
    let mut roots = vec![survivor];

    // drive the incremental cycle into its sweep phase
    heap.start_young_gc()
        .expect("young collection should start");
    for _ in 0..10_000 {
        if heap.collector.minor_phase == Phase::Sweep {
            break;
        }

        heap.step_young_gc(
            &mut |visit| visit_roots(&mut roots, visit),
            1,
            trace_table(),
        )
        .expect("young step should succeed");
    }
    assert_eq!(heap.collector.minor_phase, Phase::Sweep);

    // allocate one rooted block while the sweep is still active
    let late = heap.test_allocate(layout, Payload::Zeroed);
    roots.push(late);

    // drain the active cycle
    heap.drain_young_gc(
        &mut |visit| visit_roots(&mut roots, visit),
        usize::MAX,
        trace_table(),
    )
    .expect("young drain should succeed");

    // keep both rooted blocks alive through the cycle
    assert!(heap.is_live(roots[0]));
    assert!(heap.is_live(roots[1]));
}

/// Keep fast-path no-scan reservations made during an active major sweep.
// FUGU #Broken: fast-path no-scan reservations skip the allocate-black marking that the slow path performs during major cycles
#[test]
fn test_reserve_small_noscan_keeps_allocation_during_major_sweep() {
    // build one full heap so the public fast reservation path is available
    let options = HeapOptions::local();
    let mut heap = test_heap(options);
    let layout = test_layout(16, TraceMap::empty());
    let site = allocation_site(heap.options(), layout.block());
    let small_site = site.small_site().expect("site should be small");
    let mut roots: Vec<HeapReference> = Vec::new();

    // prime the young allocation cursor with enough slots to keep sweep busy
    for _ in 0..10 {
        heap.test_allocate(layout.block(), Payload::Zeroed);
    }

    // drive an empty major cycle into its sweep phase
    heap.storage
        .start_major_gc(&mut |visit| visit_roots(&mut roots, visit))
        .expect("major collection should start");
    heap.storage
        .step_major_gc(
            &mut |visit| visit_roots(&mut roots, visit),
            1,
            trace_table(),
        )
        .expect("major step should succeed");
    assert_eq!(heap.storage.collector.major_phase, Phase::Sweep);

    // reserve one no-scan block through the public fast path mid-sweep
    let reference = heap
        .reserve_small_noscan(small_site)
        .expect("fast reservation should succeed");

    // drain the active cycle
    heap.storage
        .drain_major_gc(
            &mut |visit| visit_roots(&mut roots, visit),
            usize::MAX,
            trace_table(),
        )
        .expect("major drain should succeed");

    // keep the mid-sweep reservation alive through the cycle
    assert!(heap.is_heap_live(reference));
}

/// Rewrite pinned young span-slot payloads after their children promote.
// FUGU #Broken: promotion rewrites young range payloads but not pinned young span-slot payloads
#[test]
fn test_collect_minor_rewrites_pinned_young_slot_payload() {
    // store the parent trace map in the explicit trace table
    let options = tiny_heap_options();
    let child_layout = test_layout(2, TraceMap::empty());
    let parent_map = local_trace_map(&[0]);
    let mut trace_table = TraceTable::new();
    let parent_trace_id = trace_table.insert(parent_map.clone());
    let parent_layout = AllocationShape::new(8, 1, Some(parent_trace_id), &parent_map);
    let mut heap = test_storage(&options);

    // pin the parent so it survives in place while the child promotes
    let child = heap.test_allocate(child_layout.block(), Payload::Bytes(&[0xC1, 0x1D]));
    let parent = heap.test_allocate(parent_layout, Payload::Bytes(&child.bits().to_le_bytes()));
    heap.pin(parent).expect("parent pin should succeed");
    let mut roots = [parent];

    assert!(heap.is_young(parent));
    assert!(heap.is_young(child));

    // collect the nursery while the parent stays pinned
    heap.collect_minor(&mut |visit| visit_roots(&mut roots, visit), &trace_table)
        .expect("young collection should succeed");

    // keep the pinned parent in place
    assert_eq!(roots[0], parent);
    assert!(heap.is_live(parent));
    assert!(heap.is_young(parent));

    // rewrite the pinned payload to the promoted child
    let traced_child = read_heap_reference(&heap, parent);

    assert_ne!(traced_child, child);
    assert!(heap.is_live(traced_child));
    assert!(!heap.is_young(traced_child));
}

/// Recycle a filled default-size nursery through paced step collection.
// FUGU #Broken: the paced minor request gate is unsatisfiable for nurseries larger than the safepoint work quantum
#[test]
fn test_step_collection_recycles_filled_default_nursery() {
    // use unmodified production defaults
    let options = HeapOptions::local();
    let trigger_bytes = options.heap_young_size_bytes * options.gc.trigger_percent as usize / 100;
    let mut heap = test_heap(options);
    let layout = test_layout(
        crate::DEFAULT_MAX_HEAP_YOUNG_ALLOCATION_SIZE_BYTES,
        TraceMap::empty(),
    );
    let mut roots: Vec<HeapReference> = Vec::new();

    // fill the nursery past the configured young trigger
    while heap.storage.young.used_bytes() < trigger_bytes {
        heap.test_allocate(layout.block(), Payload::Zeroed);
    }

    // drive paced collection until it goes quiescent
    for _ in 0..10_000 {
        let budget_bytes = heap.take_collection_budget_bytes();
        if budget_bytes == 0 {
            break;
        }

        heap.step_collection(
            &mut |visit| visit_roots(&mut roots, visit),
            budget_bytes,
            trace_table(),
        )
        .expect("collection step should succeed");
    }

    // recycle the unreachable nursery through one paced minor cycle
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Minor));
    assert_eq!(heap.storage.young.used_bytes(), 0);
}

/// Reuse young spans across no-scan classes that differ only by trace id.
#[test]
fn test_reserve_young_spans_reuse_across_noscan_trace_ids() {
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
    let first_layout = AllocationShape::new(16, 1, Some(first_trace_id), &first_map);
    let second_layout = AllocationShape::new(16, 1, Some(second_trace_id), &second_map);
    let mut heap = test_storage(&options);

    // allocate alternately between the two classes
    for _ in 0..2 {
        heap.test_allocate(first_layout, Payload::Zeroed);
        heap.test_allocate(second_layout, Payload::Zeroed);
    }

    // share young spans instead of allocating one span per allocation
    assert!(
        heap.young.spans.len() <= 2,
        "expected at most two young spans, found {}",
        heap.young.spans.len()
    );
}

/// Rescan mature cards dirtied after their extent was passed by an active minor cycle.
// FUGU #Broken: cards dirtied on an already-passed queued extent are not rescanned within the active minor cycle
#[test]
fn test_collect_minor_rescans_card_dirtied_after_extent_scan() {
    // one mature slot spans two remembered cards with one reference in each
    let options = HeapOptions {
        size_classes: SizeClassTable::new([512]).expect("size classes should validate"),
        heap_small_size_bytes: 1024,
        max_heap_young_allocation_size_bytes: 16,
        ..HeapOptions::local()
    };
    let mature_map = local_trace_map(&[0, 256]);
    let mut trace_table = TraceTable::new();
    let mature_trace_id = trace_table.insert(mature_map.clone());
    let mature_layout = AllocationShape::new(512, 1, Some(mature_trace_id), &mature_map);
    let chain_map = local_trace_map(&[0]);
    let chain_layout = AllocationShape::new(8, 1, None, &chain_map);
    let young_layout = test_layout(8, TraceMap::empty());
    let mut heap = test_storage(&options);

    // keep one young child reachable only through the first mature card
    let first_child = heap.test_allocate(young_layout.block(), Payload::Zeroed);
    let second_child = heap.test_allocate(young_layout.block(), Payload::Zeroed);
    let mature = heap.test_allocate(mature_layout, Payload::Zeroed);
    let mature_address = heap.base_address() + mature.offset();
    write_mapped_bytes(mature_address, &first_child.bits().to_le_bytes());
    heap.write_barrier(mature, 0, HeapReference::BYTE_LEN, &trace_table)
        .expect("first card barrier should succeed");

    // root a young chain so marking stays busy after the dirty cards drain
    let mut chain_head = heap.test_allocate(chain_layout, Payload::Zeroed);
    for _ in 0..8 {
        chain_head = heap.test_allocate(
            chain_layout,
            Payload::Bytes(&chain_head.bits().to_le_bytes()),
        );
    }
    let mut roots = vec![chain_head];

    // dirty the second mature card only after its extent was passed
    heap.start_young_gc()
        .expect("young collection should start");
    let mut wrote_late_reference = false;
    for _ in 0..10_000 {
        let dirty_drained =
            heap.collector.young_dirty_extent_cursor >= heap.collector.dirty_extents.len();
        if !wrote_late_reference && dirty_drained && heap.collector.minor_phase == Phase::Mark {
            write_mapped_bytes(mature_address + 256, &second_child.bits().to_le_bytes());
            heap.write_barrier(mature, 256, HeapReference::BYTE_LEN, &trace_table)
                .expect("second card barrier should succeed");
            wrote_late_reference = true;
        }

        let progress = heap
            .step_young_gc(&mut |visit| visit_roots(&mut roots, visit), 1, &trace_table)
            .expect("young step should succeed");
        if progress.completed_stats().is_some() {
            break;
        }
    }
    assert!(wrote_late_reference);

    // keep both mature-referenced children alive through the cycle
    let first_forwarded = read_heap_reference(&heap, mature);
    let second_forwarded = {
        let bytes = read_mapped_bytes(mature_address + 256, HeapReference::BYTE_LEN);
        let bits = usize::from_le_bytes(bytes.try_into().expect("heap reference should fit"));

        HeapReference::from_bits(bits)
    };

    assert!(heap.is_live(first_forwarded));
    assert!(heap.is_live(second_forwarded));
}
