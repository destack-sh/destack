use std::sync::Arc;

use crate::local::space::{HeapPlace, YoungPlace};
use crate::{
    AllocationShape, Allocator, GcKind, GcOptions, GcProgress, Heap, HeapError, HeapOptions,
    HeapReference, HeapResult, HeapSpace, Payload, RootSlot, SharedHeapReference, SizeClassTable,
    TestLayout, test_allocator, test_layout, test_layouts, visit_heap_references,
};
use destack_mir::{TraceMap, TraceTable};

use super::{read_mapped_bytes, trace_table};

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

/// Read one heap reference from mapped heap bytes.
fn read_heap_reference(heap: &HeapSpace, reference: HeapReference) -> HeapReference {
    let address = heap.base_address() + reference.offset();
    let bytes = read_mapped_bytes(address, HeapReference::BYTE_LEN);
    let bits = usize::from_le_bytes(bytes.try_into().expect("heap reference should fit"));

    HeapReference::from_bits(bits)
}

/// Visit mutable test roots as heap root slots.
fn visit_roots(
    roots: &mut [HeapReference],
    visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
) -> HeapResult<()> {
    visit_heap_references(roots, visit)
}

/// Promote reachable young allocations and free unreachable young-space state.
#[test]
fn test_collect_minor_promotes_reachable_entries() {
    // build a tiny mature space so promotion is easy to observe
    let options = tiny_heap_options();
    let allocator = test_allocator(&options);
    let layout = test_layout(3, TraceMap::empty());
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");

    // root 1,2,3 and leave 4,5,6 unreachable
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

    // verify both allocations begin in the nursery
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
        young.pages().len(),
        young.capacity_bytes().div_ceil(young.page_bytes())
    );
}

/// Promote reachable fixed-size young run slots.
#[test]
fn test_collect_minor_promotes_reachable_noscan_runs() {
    // use the default young run path for fixed-size no-scan payloads
    let options = HeapOptions::local();
    let allocator = test_allocator(&options);
    let layout = test_layout(4, TraceMap::empty());
    let mut heap =
        Heap::with_allocator_limits_and_options(allocator, crate::HeapLimits::default(), options)
            .expect("heap should build");

    // root the single zeroed run slot
    let reference = heap
        .allocate_zeroed(&heap.allocation_plan(layout.allocation()))
        .expect("young no-scan allocation should succeed");
    let mut roots = [reference];

    // verify the allocation used the young fixed-size run path
    assert!(matches!(
        heap.heap.place(reference),
        Some(HeapPlace::Young(YoungPlace::Slot(_)))
    ));

    // collect the nursery from the fixed-size run root
    let stats = heap
        .collect_minor(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect("young collection should succeed");

    // promote the live run slot without freeing anything
    assert_eq!(stats.freed_allocations, 0);
    assert_ne!(roots[0], reference);
    assert!(!heap.heap.is_live(reference));
    assert!(heap.heap.is_live(roots[0]));
    assert!(!heap.is_young(roots[0]));

    // preserve the zeroed slot bytes through promotion
    let address = heap.heap.base_address() + roots[0].offset();

    let bytes = read_mapped_bytes(address, 8);

    assert_eq!(bytes, &[0; 8]);
}

/// Clear recycled young pages before zeroed allocation reuses them.
#[test]
fn test_collect_minor_recycles_young_zeroed_bytes() {
    // allocate nonzero bytes in young space
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

    assert!(heap.is_young(reference));

    // collect with no roots so the young page can be recycled
    heap.collect_minor(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect("young collection should succeed");

    // reuse young space through the zeroed path
    let reference = heap
        .allocate_zeroed(&heap.allocation_plan(layout.allocation()))
        .expect("zeroed heap allocation should succeed");
    let address = heap.heap.base_address() + reference.offset();

    // recycled bytes should be cleared before reuse
    let bytes = read_mapped_bytes(address, 8);

    assert_eq!(bytes, &[0; 8]);
}

/// Promote young allocations reached through traced young references.
#[test]
fn test_collect_minor_promotes_reachable_child_entries() {
    // parent traces one local child reference at offset zero
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

    // root the parent and make the child reachable only through payload bytes
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

/// Promote young run slots with table-backed trace metadata.
#[test]
fn test_collect_minor_promotes_table_traced_young_run_slots() {
    // store the parent trace map in the explicit trace table
    let options = tiny_heap_options();
    let allocator = test_allocator(&options);
    let child_layout = test_layout(2, TraceMap::empty());
    let parent_map = TraceMap::Fixed {
        local_offsets: vec![0].into_boxed_slice(),
        shared_offsets: Vec::new().into_boxed_slice(),
    };
    let mut trace_table = TraceTable::new();
    let parent_trace_id = trace_table.insert(parent_map.clone());
    let parent_layout = AllocationShape::new(8, 1, Some(parent_trace_id), &parent_map);
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");

    // root the parent and make the child reachable only through table-backed metadata
    let child = heap
        .allocate(
            &heap.allocation_plan(child_layout.allocation()),
            Payload::Bytes(&[0xC1, 0x1D]),
        )
        .expect("child allocation should succeed");
    let parent = heap
        .allocate(
            &heap.allocation_plan(parent_layout),
            Payload::Bytes(&child.bits().to_le_bytes()),
        )
        .expect("parent allocation should succeed");
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
    // build a heap with no allocation at offset 7
    let options = HeapOptions::local();
    let allocator = test_allocator(&options);
    let mut heap =
        HeapSpace::with_options(allocator, &options).expect("explicit heap options should build");
    let invalid = HeapReference::new(7);
    let mut roots = [invalid];

    // reject the invalid root before mutating collection state
    let error = heap
        .collect_minor(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect_err("invalid roots should fail collection");

    assert_eq!(
        error,
        HeapError::InvalidHeapReference { reference: invalid }
    );
}

/// Record the completed young GC cycle after one local young collection.
#[test]
fn test_collect_minor_updates_gc_state() {
    // allocate one reachable young payload
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

    // run one complete minor collection
    let stats = heap
        .collect_minor(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect("young collection should succeed");

    // record the completed collection in heap state
    assert_eq!(heap.gc_state().completed_cycles, 1);
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Minor));
    assert_eq!(heap.gc_state().last_stats, Some(stats));
}

/// Pinning one young reference should preserve it in place.
#[test]
fn test_pin_preserves_young_reference() {
    // allocate one young payload
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

    assert!(heap.is_young(reference));

    // pinning a young reference keeps the same base allocation live and immobile
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

/// Pinning an interior young reference preserves its byte offset in place.
#[test]
fn test_pin_preserves_interior_young_reference() {
    // allocate one young payload and point inside it
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

    // pinning the interior reference records the base allocation
    let interior = heap.pin(interior).expect("pin should succeed");
    let location = heap
        .resolve_location(interior)
        .expect("interior pin should resolve");

    assert_eq!(location.byte_offset, 3);
    assert_eq!(
        heap.collector.pins.references().collect::<Vec<_>>(),
        vec![location.base]
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

    // collection should rewrite the interior root, not just the base
    heap.collect_minor(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect("young collection should succeed");
    let location = heap
        .resolve_location(roots[0])
        .expect("interior root should resolve");

    // preserve the interior byte offset after promotion
    assert_eq!(location.byte_offset, 5);
    assert_ne!(roots[0], HeapReference::new(reference.offset() + 5));
    assert!(!heap.is_live(reference));
    assert!(heap.is_live(location.base));
    assert!(!heap.is_young(location.base));
}

/// Pinned roots should stay in place while young children promote.
#[test]
fn test_collect_minor_traces_pinned_roots() {
    // parent traces one local child reference at offset zero
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

    // pin the parent and leave the explicit root set empty
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

    // pinned roots should seed minor marking
    let stats = heap
        .collect_minor(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect("young collection should succeed");

    // keep the pinned parent in place while promoting the child
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

    // pin the child so the parent dirty card still points into young space
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
fn test_scan_shared_roots_uses_shared_reference_width() {
    // allocate one local object with one shared reference field
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

    // scan the tracked local-to-shared root
    heap.start_shared_edge_scan();

    let scanned_bytes = heap
        .scan_shared_references(&mut roots, options.page_bytes, trace_table())
        .expect("shared root scan should succeed");

    // account for the full shared-reference width
    assert_eq!(scanned_bytes, 8);
    assert_eq!(roots, vec![shared]);
    assert!(heap.shared_edge_scan_idle());

    heap.finish_shared_edge_scan();

    // local ownership remains independent from the shared-edge scan
    assert!(heap.is_live(local));
}

/// Continue local-to-shared edge scans across large allocation pages.
#[test]
fn test_scan_shared_roots_scans_large_allocations_incrementally() {
    // build one large object with shared references on separate pages
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

    // scan at a one-page budget
    heap.start_shared_edge_scan();

    let first_scanned = heap
        .scan_shared_references(&mut roots, 1, trace_table())
        .expect("first shared-root scan should succeed");

    // first step should discover only the first page reference
    assert_eq!(first_scanned, page_bytes);
    assert_eq!(roots, vec![first_shared]);
    assert!(!heap.shared_edge_scan_idle());

    let second_scanned = heap
        .scan_shared_references(&mut roots, 1, trace_table())
        .expect("second shared-root scan should succeed");

    // second step should discover the second page reference
    assert_eq!(second_scanned, page_bytes);
    assert_eq!(roots, vec![first_shared, second_shared]);

    // drain the rest of the large allocation scan cursor
    while !heap.shared_edge_scan_idle() {
        let scanned_bytes = heap
            .scan_shared_references(&mut roots, 1, trace_table())
            .expect("remaining shared-root scan should succeed");
        assert!(scanned_bytes > 0);
    }

    heap.finish_shared_edge_scan();

    // scanning shared edges should not affect local object liveness
    assert!(heap.is_live(parent));
}

/// Keep unscanned shared-edge roots stable when earlier roots are freed.
#[test]
fn test_scan_shared_roots_survives_active_root_removal() {
    // allocate three local objects that each contain one shared edge
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

    // start a cursor-based local-to-shared scan
    heap.start_shared_edge_scan();

    // scan the first root, then remove it while the cursor points past it
    let first_work = heap
        .scan_shared_references(&mut roots, 1, trace_table())
        .expect("first shared root scan should succeed");
    heap.free(first_local)
        .expect("freeing scanned root should succeed");

    let remaining_work = heap
        .scan_shared_references(&mut roots, usize::MAX, trace_table())
        .expect("remaining shared root scan should succeed");

    // retain pending roots even when earlier tracked roots are removed
    assert_eq!(first_work, 8);
    assert_eq!(remaining_work, 16);
    assert_eq!(roots, vec![first_shared, second_shared, third_shared]);

    heap.finish_shared_edge_scan();
}

/// Stay idle when no local pressure or explicit request exists.
#[test]
fn test_collect_step_stays_idle_without_request() {
    // build an empty heap with no pressure
    let options = HeapOptions::local();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );
    let mut heap =
        Heap::with_allocator_limits_and_options(allocator, crate::HeapLimits::default(), options)
            .expect("heap should build");
    let mut roots = [];

    // no request and no pressure should produce no work
    let budget_bytes = heap.take_collection_budget_bytes();
    let progress = heap
        .collect_step(
            &mut |visit| visit_roots(&mut roots, visit),
            budget_bytes,
            trace_table(),
        )
        .expect("collection step should succeed");

    // keep the collector idle
    assert_eq!(progress, GcProgress::Idle);
}

/// Run one full bounded cycle after heap allocation pressure.
#[test]
fn test_collect_step_runs_full_after_pressure() {
    // configure the pacer to trigger after one allocation
    let (mut heap, layout_ids) = test_heap(&[(64, TraceMap::empty())]);
    let layout = &layout_ids[0];
    let root = heap
        .allocate(
            &heap.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1; 64]),
        )
        .expect("heap allocation should succeed");
    let mut roots = [root];

    // collection budget should service the pressure-triggered full cycle
    let budget_bytes = heap.take_collection_budget_bytes();
    let progress = heap
        .collect_step(
            &mut |visit| visit_roots(&mut roots, visit),
            budget_bytes,
            trace_table(),
        )
        .expect("collection step should succeed");
    let stats = progress
        .completed_stats()
        .expect("pressure should request one cycle");
    let root = roots[0];

    // preserve the rooted allocation and record a full cycle
    assert_eq!(stats.freed_allocations, 0);
    assert!(heap.is_heap_live(root));
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Full));
}

/// Run one minor cycle after young-space occupancy crosses the configured trigger.
#[test]
fn test_collect_step_runs_minor_after_young_occupancy() {
    // configure a tiny nursery and a young trigger below full occupancy
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

    // young occupancy should request and complete one minor collection
    let budget_bytes = heap.take_collection_budget_bytes();
    let progress = heap
        .collect_step(
            &mut |visit| visit_roots(&mut roots, visit),
            budget_bytes,
            trace_table(),
        )
        .expect("collection step should succeed");

    // record a minor cycle
    assert_eq!(progress.completed_stats().map(|_| ()), Some(()));
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Minor));
}

/// Drain minor collection at one safepoint even with a small caller budget.
#[test]
fn test_collect_step_drains_minor_at_safepoint() {
    // configure a tiny nursery and a young trigger below full occupancy
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

    // minor collection must drain even with a tiny caller budget because it moves objects
    let progress = heap
        .collect_step(
            &mut |visit| visit_roots(&mut roots, visit),
            1,
            trace_table(),
        )
        .expect("small-budget collection should succeed");

    // record a complete minor cycle
    assert_eq!(progress.completed_stats().map(|_| ()), Some(()));
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Minor));
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

    // budget stays zero until explicit safepoint collection is chosen
    let budget_bytes = heap.take_collection_budget_bytes();

    assert_eq!(budget_bytes, 0);
}

/// Honor one explicit full local collection request below the pacing trigger.
#[test]
fn test_collect_step_honors_manual_full_request() {
    // allocate one root below the pacing trigger
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

    // explicit requests should bypass pressure checks
    heap.request_full_gc();

    let budget_bytes = heap.take_collection_budget_bytes();
    let progress = heap
        .collect_step(
            &mut |visit| visit_roots(&mut roots, visit),
            budget_bytes,
            trace_table(),
        )
        .expect("collection step should succeed");
    let stats = progress
        .completed_stats()
        .expect("manual request should run one cycle");

    // keep the root and record a full collection
    assert_eq!(stats.freed_allocations, 0);
    assert_eq!(heap.gc_state().last_kind, Some(GcKind::Full));
}

/// Continue one local major collection across bounded safepoint work.
#[test]
fn test_step_major_gc_spreads_full_cycle() {
    // allocate one rooted object and one unreachable object in mature space
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
    let stats = loop {
        let progress = heap
            .step_major_gc(
                &mut |visit| visit_roots(&mut roots, visit),
                1,
                trace_table(),
            )
            .expect("major step should succeed");
        if let Some(stats) = progress.completed_stats() {
            break stats;
        }
    };

    // reclaim only the unreachable mature allocation
    assert_eq!(stats.freed_allocations, 1);
    assert!(heap.is_live(root));
    assert!(!heap.is_live(garbage));
    assert!(!heap.major_gc_active());
}

/// Continue local major marking across large allocation pages.
#[test]
fn test_step_major_gc_scans_large_allocations_incrementally() {
    // build one large object with local references on separate pages
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

    // first step should not scan the entire large allocation
    assert_eq!(first, GcProgress::Active);
    assert!(heap.major_gc_active());

    // finish the bounded scan and sweep
    let stats = loop {
        let progress = heap
            .step_major_gc(
                &mut |visit| visit_roots(&mut roots, visit),
                1,
                trace_table(),
            )
            .expect("major step should succeed");
        if let Some(stats) = progress.completed_stats() {
            break stats;
        }
    };

    // retain both children reached through the large parent
    assert_eq!(stats.freed_allocations, 0);
    assert!(heap.is_live(first_child));
    assert!(heap.is_live(second_child));
}

/// Repeated full collection should free later unreachable allocations too.
#[test]
fn test_collect_full_reclaims_later_unreachable_allocations() {
    // seed one root and one unreachable allocation
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
    heap.collect_full(&mut |visit| visit_roots(&mut roots, visit), trace_table())
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
    assert!(heap.is_young(more_garbage));
    assert!(heap.is_young(even_more));

    // the next minor cycle should clear the unreachable nursery
    heap.collect_minor(&mut |visit| visit_roots(&mut roots, visit), trace_table())
        .expect("minor collection should succeed");

    assert_eq!(heap.allocation_count(), 1);
}
