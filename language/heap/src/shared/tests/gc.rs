use std::sync::Arc;

use destack_memory::MemoryMap;
use destack_mir::{TraceMap, TraceTable};

use crate::{
    AllocationCache, AllocationShape, DEFAULT_GC_MINIMUM_WORK_BYTES, DropId, DropReference,
    GcAdvance, GcCollector, GcOptions, GcPhase, HeapAllocationError, HeapError, Payload,
    SharedHeap, SharedHeapLimits, SharedHeapOptions, SharedHeapReference, SharedMarkWorker,
    SizeClassTable, TestLayout, shared_trace_map, test_layout, test_layouts,
};

use super::{
    TestHeapPlan, TestTraceTable, read_mapped_bytes, test_allocate, trace_view, write_mapped_bytes,
};

/// Build one shared heap whose pacer starts immediately in step-driven tests.
fn test_shared_heap(
    layouts: &[(usize, TraceMap)],
) -> (
    SharedHeap,
    AllocationCache,
    SharedMarkWorker,
    Vec<TestLayout>,
) {
    let options = SharedHeapOptions {
        gc: GcOptions {
            growth_percent: 0,
            trigger_percent: 75,
            soft_limit_bytes: None,
            minimum_heap_bytes: Some(0),
            minimum_work_bytes: DEFAULT_GC_MINIMUM_WORK_BYTES,
        },
        ..SharedHeapOptions::default()
    };
    let layouts = test_layouts(layouts);

    let memory = Arc::new(
        MemoryMap::reserve(1024 * 1024 * 1024, options.page_size_bytes)
            .expect("test World memory should reserve"),
    );
    let heap = SharedHeap::new(memory, SharedHeapLimits::default(), options)
        .expect("shared heap should build");

    let memory = heap.allocation_cache();
    let worker = heap.register_mark_worker();

    (heap, memory, worker, layouts)
}

/// Publish worker-local shared blocks before a direct heap collection.
fn flush_shared_cache(shared: &SharedHeap, cache: &mut AllocationCache) {
    shared.flush_allocation_cache(cache);
}

/// Reject one zero-size shared heap block.
#[test]
fn test_allocate_shared_rejects_zero_size_layout() {
    // build one shared heap with an invalid zero-size layout
    let (shared, mut memory, worker, layout_ids) = test_shared_heap(&[(0, TraceMap::empty())]);
    let layout = &layout_ids[0];
    let shape = layout.block();

    // reject zero-size shared heap objects loudly
    let error = shared
        .allocate_payload(
            &worker,
            &mut memory,
            &shared.test_allocation_plan(&shape),
            Payload::Bytes(&[]),
            trace_view(),
        )
        .expect_err("shared heap block should reject zero-size layouts");

    assert_eq!(
        error,
        HeapError::invalid_allocation(HeapAllocationError::ZeroSize)
    );
}

/// Keep worker-cursor blocks mutator-live while deferring accounting until memory flush.
#[test]
fn test_allocate_shared_zeroed_worker_cache_defers_accounting() {
    // allocate two zeroed slots from a worker-local cursor
    let (shared, mut memory, worker, layout_ids) = test_shared_heap(&[(8, TraceMap::empty())]);
    let shape = layout_ids[0].block();
    let layout = shared.test_allocation_plan(&shape);

    let first = shared
        .allocate_payload(&worker, &mut memory, &layout, Payload::Zeroed, trace_view())
        .expect("first shared block should succeed");
    let second = shared
        .allocate_payload(&worker, &mut memory, &layout, Payload::Zeroed, trace_view())
        .expect("second shared block should succeed");

    // worker-local slots are mutator-live before global publication
    assert!(memory.contains_heap_reference(first));
    assert!(memory.contains_heap_reference(second));
    assert!(!shared.is_heap_live(first));
    assert!(!shared.is_heap_live(second));
    assert_eq!(shared.heap_allocation_count(), 0);

    shared.flush_allocation_cache(&mut memory);

    // flushing publishes worker-local accounting
    assert!(shared.is_heap_live(first));
    assert!(shared.is_heap_live(second));
    assert_eq!(shared.heap_allocation_count(), 2);
}

/// Miss specialized shared zeroed block when the active cache has a different trace id.
#[test]
fn test_reserve_shared_zeroed_misses_different_trace_class() {
    let first_map = TraceMap::Fixed {
        local_offsets: Box::new([0]),
        shared_offsets: Box::new([]),
        frame_offsets: Box::new([]),
        borrow_offsets: Box::default(),
    };
    let second_map = TraceMap::Fixed {
        local_offsets: Box::new([]),
        shared_offsets: Box::new([0]),
        frame_offsets: Box::new([]),
        borrow_offsets: Box::default(),
    };
    let mut trace_table = TraceTable::new();
    let first_trace_id = trace_table.insert(first_map.clone());
    let second_trace_id = trace_table.insert(second_map.clone());
    let trace_table = TestTraceTable::from_mir(&trace_table);
    let trace_view = trace_table.view();
    let (shared, mut memory, worker, _) = test_shared_heap(&[]);
    let first_shape = AllocationShape::new(8, 1, Some(first_trace_id), first_map);
    let second_shape = AllocationShape::new(8, 1, Some(second_trace_id), second_map);
    let first_site = shared.options().allocation_plan(&first_shape);
    let second_site = shared.options().allocation_plan(&second_shape);
    let second_small = second_site
        .class
        .as_small()
        .expect("second site should be small");

    // prime one worker cache with the first trace class
    let _first = shared
        .allocate_zeroed(
            &worker,
            &mut memory,
            first_site,
            &first_shape.trace_map,
            trace_view,
        )
        .expect("first shared block should succeed");

    // reject cached cursor reuse across trace classes
    let second = shared.reserve_small_from_cache(&mut memory, second_small);

    assert_eq!(second, None);
}

/// Keep byte-initialized worker-cursor blocks mutator-live while deferring accounting.
#[test]
fn test_allocate_shared_bytes_worker_cache_defers_accounting() {
    // allocate two byte-initialized slots from a worker-local cursor
    let (shared, mut memory, worker, layout_ids) = test_shared_heap(&[(8, TraceMap::empty())]);
    let shape = layout_ids[0].block();
    let layout = shared.test_allocation_plan(&shape);

    let first = shared
        .allocate_payload(
            &worker,
            &mut memory,
            &layout,
            Payload::Bytes(&[1, 2, 3, 4, 5, 6, 7, 8]),
            trace_view(),
        )
        .expect("first shared block should succeed");
    let second = shared
        .allocate_payload(
            &worker,
            &mut memory,
            &layout,
            Payload::Bytes(&[8, 7, 6, 5, 4, 3, 2, 1]),
            trace_view(),
        )
        .expect("second shared block should succeed");

    // worker-local slots are mutator-live before global publication
    assert!(memory.contains_heap_reference(first));
    assert!(memory.contains_heap_reference(second));
    assert!(!shared.is_heap_live(first));
    assert!(!shared.is_heap_live(second));
    assert_eq!(shared.heap_allocation_count(), 0);

    shared.flush_allocation_cache(&mut memory);

    // flushing publishes worker-local accounting
    assert!(shared.is_heap_live(first));
    assert!(shared.is_heap_live(second));
    assert_eq!(shared.heap_allocation_count(), 2);
}

/// Free unreachable shared heap blocks and record the completed shared GC cycle.
#[test]
fn test_collect_shared_frees_unreachable_entries() {
    // root 1,2,3 and leave 4,5,6 unreachable
    let (shared, mut memory, worker, layout_ids) = test_shared_heap(&[(3, TraceMap::empty())]);
    let layout = &layout_ids[0];
    let reachable = test_allocate(
        &shared,
        &worker,
        &mut memory,
        layout.block(),
        Payload::Bytes(&[1, 2, 3]),
    );
    let unreachable = test_allocate(
        &shared,
        &worker,
        &mut memory,
        layout.block(),
        Payload::Bytes(&[4, 5, 6]),
    );

    // publish worker-local slots before direct full collection
    flush_shared_cache(&shared, &mut memory);

    // collect the shared heap from the explicit root set
    let stats = shared
        .collect_full(&[reachable], trace_view(), &mut |_| Ok::<(), HeapError>(()))
        .expect("shared collection should succeed");

    // free the unreachable entry and preserve root bytes
    assert_eq!(stats.freed_allocations, 1);
    assert!(!shared.is_heap_live(unreachable));
    let address = shared.heap_base_address() + reachable.offset();

    // shared small slots read as full slot width
    let bytes = read_mapped_bytes(address, 8);

    assert_eq!(bytes, &[1, 2, 3, 0, 0, 0, 0, 0]);

    // record the completed shared collection
    assert_eq!(shared.gc_state().completed_cycles, 1);
    assert_eq!(shared.gc_state().last_collector, Some(GcCollector::Shared));
    assert_eq!(shared.gc_state().last_stats, Some(stats));
}

/// Clear one reused shared small heap slot before writing a shorter payload.
#[test]
fn test_collect_shared_clears_reused_small_slot_tail() {
    // force small shared slot reuse
    let options = SharedHeapOptions {
        heap_small_size_bytes: 16,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..SharedHeapOptions::default()
    };
    let memory = Arc::new(
        MemoryMap::reserve(1024 * 1024 * 1024, options.page_size_bytes)
            .expect("test World memory should reserve"),
    );
    let shared = SharedHeap::new(memory, SharedHeapLimits::default(), options)
        .expect("shared heap should build");
    let mut memory = shared.allocation_cache();
    let worker = shared.register_mark_worker();
    let full_layout = test_layout(8, TraceMap::empty());
    let short_layout = test_layout(1, TraceMap::empty());

    // allocate a full slot and a live short slot
    let first = test_allocate(
        &shared,
        &worker,
        &mut memory,
        full_layout.block(),
        Payload::Bytes(&[0xAA; 8]),
    );
    let second = test_allocate(
        &shared,
        &worker,
        &mut memory,
        short_layout.block(),
        Payload::Bytes(&[0xBB]),
    );

    // collect with only the short slot rooted so the full slot is freed
    flush_shared_cache(&shared, &mut memory);

    shared
        .collect_full(&[second], trace_view(), &mut |_| Ok::<(), HeapError>(()))
        .expect("shared collection should succeed");
    assert!(!shared.is_heap_live(first));

    // reuse the freed slot with a shorter payload
    let reused = test_allocate(
        &shared,
        &worker,
        &mut memory,
        short_layout.block(),
        Payload::Bytes(&[0xCC]),
    );

    assert!(shared.is_heap_live(second));
    assert!(shared.is_heap_live(reused));
    let address = shared.heap_base_address() + reused.offset();

    // reused slot tail bytes should be cleared
    let bytes = read_mapped_bytes(address, 8);

    assert_eq!(bytes, &[0xCC, 0, 0, 0, 0, 0, 0, 0]);
}

/// Trace shared child references through one shared heap payload.
#[test]
fn test_collect_shared_keeps_reachable_children() {
    // parent traces one shared child reference at offset zero
    let trace_map = shared_trace_map(&[0]);
    let (shared, mut memory, worker, layout_ids) =
        test_shared_heap(&[(2, TraceMap::empty()), (8, trace_map.clone())]);
    let child_layout = &layout_ids[0];
    let parent_layout = &layout_ids[1];

    // root the parent and make the child reachable only through payload bytes
    let child = test_allocate(
        &shared,
        &worker,
        &mut memory,
        child_layout.block(),
        Payload::Bytes(&[0xC1, 0x1D]),
    );
    let parent = test_allocate(
        &shared,
        &worker,
        &mut memory,
        parent_layout.block(),
        Payload::Bytes(&child.bits().to_le_bytes()),
    );

    // publish worker-local slots before direct full collection
    flush_shared_cache(&shared, &mut memory);

    // collect through the parent root
    let stats = shared
        .collect_full(&[parent], trace_view(), &mut |_| Ok::<(), HeapError>(()))
        .expect("shared collection should succeed");

    // retain the child through the parent payload
    assert_eq!(stats.freed_allocations, 0);
    assert!(shared.is_heap_live(child));
    let address = shared.heap_base_address() + child.offset();

    // preserve child payload bytes
    let bytes = read_mapped_bytes(address, 8);

    assert_eq!(bytes, &[0xC1, 0x1D, 0, 0, 0, 0, 0, 0]);
}

/// Trace shared child references through table-backed shared small spans.
#[test]
fn test_collect_shared_keeps_table_traced_small_children() {
    // store the parent trace map in the explicit trace table
    let child_layout = test_layout(2, TraceMap::empty());
    let parent_map = shared_trace_map(&[0]);
    let mut trace_table = TraceTable::new();
    let parent_trace_id = trace_table.insert(parent_map.clone());
    let trace_table = TestTraceTable::from_mir(&trace_table);
    let trace_view = trace_table.view();
    let parent_layout = AllocationShape::new(8, 1, Some(parent_trace_id), parent_map);
    let (shared, mut memory, worker, _) = test_shared_heap(&[]);
    let child_shape = child_layout.block();

    // root the parent and make the child reachable through table-backed metadata
    let child = shared
        .allocate_payload(
            &worker,
            &mut memory,
            &shared.test_allocation_plan(&child_shape),
            Payload::Bytes(&[0xC1, 0x1D]),
            trace_view,
        )
        .expect("child block should succeed");
    let parent = shared
        .allocate_payload(
            &worker,
            &mut memory,
            &shared.test_allocation_plan(&parent_layout),
            Payload::Bytes(&child.bits().to_le_bytes()),
            trace_view,
        )
        .expect("parent block should succeed");

    // publish worker-local slots before direct full collection
    flush_shared_cache(&shared, &mut memory);

    // collect using the trace table that owns the parent map
    let stats = shared
        .collect_full(&[parent], trace_view, &mut |_| Ok::<(), HeapError>(()))
        .expect("shared collection should succeed");

    // retain the child through table-backed trace metadata
    assert_eq!(stats.freed_allocations, 0);
    assert!(shared.is_heap_live(child));
}

/// Bound shared small-span mark work by marked slots.
#[test]
fn test_collect_shared_scans_small_spans_incrementally() {
    // build two parent-child pairs in shared small spans
    let trace_map = shared_trace_map(&[0]);
    let (shared, mut memory, worker, layout_ids) =
        test_shared_heap(&[(2, TraceMap::empty()), (8, trace_map.clone())]);
    let child_layout = &layout_ids[0];
    let parent_layout = &layout_ids[1];

    // make each child reachable through exactly one rooted parent
    let first_child = test_allocate(
        &shared,
        &worker,
        &mut memory,
        child_layout.block(),
        Payload::Bytes(&[0xC1, 0x1D]),
    );
    let second_child = test_allocate(
        &shared,
        &worker,
        &mut memory,
        child_layout.block(),
        Payload::Bytes(&[0xC2, 0x1D]),
    );
    let first_parent = test_allocate(
        &shared,
        &worker,
        &mut memory,
        parent_layout.block(),
        Payload::Bytes(&first_child.bits().to_le_bytes()),
    );
    let second_parent = test_allocate(
        &shared,
        &worker,
        &mut memory,
        parent_layout.block(),
        Payload::Bytes(&second_child.bits().to_le_bytes()),
    );

    // publish worker-local slots before manual stepping
    flush_shared_cache(&shared, &mut memory);

    // start collection and run only one tiny mark step
    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    shared
        .step_collection(&[first_parent, second_parent], true, 1, trace_view())
        .expect("shared collection step should succeed");

    assert_eq!(shared.gc_phase(), GcPhase::Mark);
    assert!(!shared.mark_idle());

    // drain the remaining bounded mark and sweep work
    while shared
        .step_collection(&[first_parent, second_parent], true, 1, trace_view())
        .expect("shared collection step should succeed")
        .completed_stats()
        .is_none()
    {}

    // retain both children reached through small-span parents
    assert!(shared.is_heap_live(first_child));
    assert!(shared.is_heap_live(second_child));
}

/// Scan large shared blocks incrementally by memory page.
#[test]
fn test_collect_shared_scans_large_blocks_incrementally() {
    // build one large parent with child references on separate pages
    let options = SharedHeapOptions::default();
    let first_offset = 0usize;
    let second_offset = options.page_size_bytes;
    let parent_byte_len = second_offset + SharedHeapReference::BYTE_LEN;
    let second_offset = u32::try_from(second_offset).expect("page offset should fit uint32");
    let trace_map = shared_trace_map(&[first_offset as u32, second_offset]);
    let (shared, mut memory, worker, layout_ids) =
        test_shared_heap(&[(2, TraceMap::empty()), (parent_byte_len, trace_map)]);
    let child_layout = &layout_ids[0];
    let parent_layout = &layout_ids[1];

    // allocate two children and encode both references into the large parent
    let first_child = test_allocate(
        &shared,
        &worker,
        &mut memory,
        child_layout.block(),
        Payload::Bytes(&[0xC1, 0x1D]),
    );
    let second_child = test_allocate(
        &shared,
        &worker,
        &mut memory,
        child_layout.block(),
        Payload::Bytes(&[0xC2, 0x1D]),
    );
    let mut parent_bytes = vec![0; parent_byte_len];
    parent_bytes[first_offset..first_offset + SharedHeapReference::BYTE_LEN]
        .copy_from_slice(&first_child.bits().to_le_bytes());
    parent_bytes[second_offset as usize..second_offset as usize + SharedHeapReference::BYTE_LEN]
        .copy_from_slice(&second_child.bits().to_le_bytes());
    let parent = test_allocate(
        &shared,
        &worker,
        &mut memory,
        parent_layout.block(),
        Payload::Bytes(&parent_bytes),
    );

    // publish worker-local slots before manual stepping
    flush_shared_cache(&shared, &mut memory);

    // start collection and scan only the first page of the large parent
    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    shared
        .step_collection(&[parent], true, 1, trace_view())
        .expect("shared collection step should succeed");
    assert_eq!(shared.gc_phase(), GcPhase::Mark);

    // drain the remaining bounded mark and sweep work
    while shared
        .step_collection(&[parent], true, 1, trace_view())
        .expect("shared collection step should succeed")
        .completed_stats()
        .is_none()
    {}

    // retain both children reached through the large parent
    assert!(shared.is_heap_live(first_child));
    assert!(shared.is_heap_live(second_child));
}

/// Reject invalid explicit shared heap roots.
#[test]
fn test_collect_shared_rejects_invalid_root() {
    // build a shared heap with no block at offset 7
    let options = SharedHeapOptions::default();
    let memory = Arc::new(
        MemoryMap::reserve(1024 * 1024 * 1024, options.page_size_bytes)
            .expect("test World memory should reserve"),
    );
    let shared = SharedHeap::new(memory, SharedHeapLimits::default(), options)
        .expect("shared heap should build");
    let invalid = SharedHeapReference::new(7);

    // reject the invalid root before mutating collection state
    let error = shared
        .collect_full(&[invalid], trace_view(), &mut |_| Ok::<(), HeapError>(()))
        .expect_err("invalid shared roots should fail collection");

    assert_eq!(error, HeapError::invalid_shared_heap_reference(invalid));
}

/// Preserve shared heap collector state across shared-heap images.
#[test]
fn test_shared_heap_gc_state_roundtrips_through_image() {
    // allocate one rooted shared object and run a full collection
    let layout = test_layout(8, TraceMap::empty());
    let options = SharedHeapOptions::default();
    let memory = Arc::new(
        MemoryMap::reserve(1024 * 1024 * 1024, options.page_size_bytes)
            .expect("test World memory should reserve"),
    );
    let shared = SharedHeap::new(memory.clone(), SharedHeapLimits::default(), options.clone())
        .expect("shared heap should build");
    let mut memory = shared.allocation_cache();
    let worker = shared.register_mark_worker();
    let reference = test_allocate(
        &shared,
        &worker,
        &mut memory,
        layout.block(),
        Payload::Bytes(&SharedHeapReference::NULL.bits().to_le_bytes()),
    );
    flush_shared_cache(&shared, &mut memory);

    // capture the heap image after collection
    let stats = shared
        .collect_full(&[reference], trace_view(), &mut |_| Ok::<(), HeapError>(()))
        .expect("shared collection should succeed");
    let memory_image = shared
        .storage
        .memory
        .capture()
        .expect("memory image should capture");
    let image = shared.image();
    let restored_memory = Arc::new(memory_image.restore().expect("memory should restore"));
    let restored =
        SharedHeap::from_image_with_limits(&image, restored_memory, SharedHeapLimits::default())
            .expect("shared image should restore");

    // restored state should preserve the last completed cycle
    assert_eq!(
        restored.gc_state().last_collector,
        Some(GcCollector::Shared)
    );
    assert_eq!(restored.gc_state().last_stats, Some(stats));
}

/// Keep a child written during mark through the shared write barrier.
#[test]
fn test_collect_shared_barrier_keeps_written_child() {
    // parent traces one shared child reference at offset zero
    let trace_map = shared_trace_map(&[0]);
    let (shared, mut memory, worker, layout_ids) =
        test_shared_heap(&[(2, TraceMap::empty()), (8, trace_map.clone())]);
    let child_layout = &layout_ids[0];
    let parent_layout = &layout_ids[1];

    // allocate a child and a zeroed parent before marking
    let child = test_allocate(
        &shared,
        &worker,
        &mut memory,
        child_layout.block(),
        Payload::Bytes(&[0xC1, 0x1D]),
    );
    let parent = test_allocate(
        &shared,
        &worker,
        &mut memory,
        parent_layout.block(),
        Payload::Zeroed,
    );

    // publish worker-local slots before manual stepping
    flush_shared_cache(&shared, &mut memory);

    // start marking without allowing mark termination
    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    shared
        .step_collection(&[parent], false, 1, trace_view())
        .expect("shared collection step should succeed");
    assert_eq!(shared.gc_phase(), GcPhase::Mark);

    // publish the child edge through the write barrier before writing bytes
    shared
        .write_barrier_bytes(parent, 0, &child.bits().to_le_bytes(), trace_view())
        .expect("shared heap write barrier should record");
    let address = shared.heap_base_address() + parent.offset();

    write_mapped_bytes(address, &child.bits().to_le_bytes());

    // finish the collection after the barrier-published edge
    while shared
        .step_collection(&[parent], true, 1, trace_view())
        .expect("shared collection step should succeed")
        .completed_stats()
        .is_none()
    {}

    // retain the child written during mark
    assert!(shared.is_heap_live(child));
}

/// Keep one block created during mark alive through the active cycle.
#[test]
fn test_collect_shared_keeps_allocation_created_during_mark() {
    // seed one rooted shared block
    let (shared, mut memory, worker, layout_ids) = test_shared_heap(&[(3, TraceMap::empty())]);
    let layout = &layout_ids[0];
    let root = test_allocate(
        &shared,
        &worker,
        &mut memory,
        layout.block(),
        Payload::Bytes(&[1, 2, 3]),
    );

    flush_shared_cache(&shared, &mut memory);

    // start marking from the root
    shared.request_gc();
    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    shared
        .step_collection(&[root], false, 1, trace_view())
        .expect("shared collection step should succeed");
    assert_eq!(shared.gc_phase(), GcPhase::Mark);

    // allocate after mark has started
    let late = test_allocate(
        &shared,
        &worker,
        &mut memory,
        layout.block(),
        Payload::Bytes(&[7, 8, 9]),
    );

    // drain the active cycle
    while shared
        .step_collection(&[root], true, 1, trace_view())
        .expect("shared collection step should succeed")
        .completed_stats()
        .is_none()
    {}

    // keep both the root and the block created during mark
    assert!(shared.is_heap_live(root));
    assert!(shared.is_heap_live(late));
}

/// Keep a shared block created during mark after publishing one worker-local cursor.
#[test]
fn test_collect_shared_keeps_cache_allocation_created_during_mark() {
    // seed one block that will become unreachable
    let (shared, mut memory, worker, layout_ids) = test_shared_heap(&[(3, TraceMap::empty())]);
    let layout = &layout_ids[0];
    let seed = test_allocate(
        &shared,
        &worker,
        &mut memory,
        layout.block(),
        Payload::Bytes(&[1, 2, 3]),
    );

    // start marking without flushing the worker-local cache
    shared.request_gc();
    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    // allocate into a worker-local cursor during active marking
    let late = test_allocate(
        &shared,
        &worker,
        &mut memory,
        layout.block(),
        Payload::Bytes(&[7, 8, 9]),
    );

    // drain the active cycle with no explicit roots
    while shared
        .step_collection(&[], true, 1, trace_view())
        .expect("shared collection step should succeed")
        .completed_stats()
        .is_none()
    {}

    // free the unrooted seed but keep the block created during mark
    assert!(!shared.is_heap_live(seed));
    assert!(shared.is_heap_live(late));
    let address = shared.heap_base_address() + late.offset();

    // inspect the late block payload
    let bytes = read_mapped_bytes(address, 8);

    assert_eq!(bytes, &[7, 8, 9, 0, 0, 0, 0, 0]);
}

/// Keep one block created while block assists sweep.
#[test]
fn test_allocate_shared_assists_sweep_before_returning() {
    // seed one root and one unreachable block
    let (shared, mut memory, worker, layout_ids) = test_shared_heap(&[(3, TraceMap::empty())]);
    let layout = &layout_ids[0];
    let root = test_allocate(
        &shared,
        &worker,
        &mut memory,
        layout.block(),
        Payload::Bytes(&[1, 2, 3]),
    );
    let unreachable = test_allocate(
        &shared,
        &worker,
        &mut memory,
        layout.block(),
        Payload::Bytes(&[4, 5, 6]),
    );

    flush_shared_cache(&shared, &mut memory);

    // advance the requested cycle into sweep
    shared.request_gc();
    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    while shared.gc_phase() == GcPhase::Mark {
        shared
            .step_collection(&[root], true, 1, trace_view())
            .expect("shared collection step should succeed");
    }

    // finish the empty Drop pass before testing allocation during sweep
    while shared.gc_phase() == GcPhase::Drop {
        shared
            .step_collection(&[root], true, 1, trace_view())
            .expect("shared Drop step should succeed");
    }

    assert_eq!(shared.gc_phase(), GcPhase::Sweep);
    let completed_cycles = shared.gc_state().completed_cycles;

    // block assist should service sweep before returning
    let late = test_allocate(
        &shared,
        &worker,
        &mut memory,
        layout.block(),
        Payload::Bytes(&[7, 8, 9]),
    );

    // drain any remaining sweep work
    while shared.gc_phase() != GcPhase::Idle {
        shared
            .step_collection(&[], true, 1, trace_view())
            .expect("shared collection step should succeed");
    }

    // keep the root and the late block
    assert!(shared.gc_state().completed_cycles > completed_cycles);
    assert!(shared.is_heap_live(root));
    assert!(shared.is_heap_live(late));

    // the assist may reclaim and immediately reuse the dead slot
    if late != unreachable {
        assert!(!shared.is_heap_live(unreachable));
    }
}

/// Require explicit mark termination before shared sweep begins.
#[test]
fn test_collect_shared_requires_explicit_mark_finish() {
    // root 1,2,3 and leave 4,5,6 unreachable
    let (shared, mut memory, worker, layout_ids) = test_shared_heap(&[(3, TraceMap::empty())]);
    let layout = &layout_ids[0];
    let reachable = test_allocate(
        &shared,
        &worker,
        &mut memory,
        layout.block(),
        Payload::Bytes(&[1, 2, 3]),
    );
    let unreachable = test_allocate(
        &shared,
        &worker,
        &mut memory,
        layout.block(),
        Payload::Bytes(&[4, 5, 6]),
    );

    flush_shared_cache(&shared, &mut memory);

    // start marking and disallow mark termination for the first step
    shared.request_gc();
    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    shared
        .step_collection(&[reachable], false, 1, trace_view())
        .expect("shared collection step should succeed");

    // unreachable block should remain live until sweep is allowed
    assert_eq!(shared.gc_phase(), GcPhase::Mark);
    assert!(shared.is_heap_live(unreachable));

    // allowing mark termination should finish the cycle and free garbage
    while shared
        .step_collection(&[reachable], true, 1, trace_view())
        .expect("shared collection step should succeed")
        .completed_stats()
        .is_none()
    {}

    // finish in idle with the unreachable block reclaimed
    assert_eq!(shared.gc_phase(), GcPhase::Idle);
    assert!(!shared.is_heap_live(unreachable));
}

/// Stay idle when no shared pressure or explicit request exists.
#[test]
fn test_step_collection_stays_idle_without_request() {
    // build an empty shared heap with no pressure
    let options = SharedHeapOptions::default();
    let memory = Arc::new(
        MemoryMap::reserve(1024 * 1024 * 1024, options.page_size_bytes)
            .expect("test World memory should reserve"),
    );
    let shared = SharedHeap::new(memory, SharedHeapLimits::default(), options)
        .expect("shared heap should build");

    // no request and no pressure should produce no work
    let progress = shared
        .step_collection(&[], true, 1, trace_view())
        .expect("shared collection step should succeed");

    // keep the collector idle
    assert_eq!(progress, GcAdvance::Idle);
    assert_eq!(shared.gc_phase(), GcPhase::Idle);
}

/// Surface one unreachable shared allocation before reclaiming it.
#[test]
fn test_step_collection_drops_shared_allocation_before_reclamation() {
    let (shared, mut cache, worker, layout_ids) = test_shared_heap(&[(16, TraceMap::empty())]);
    let shape = layout_ids[0]
        .block()
        .with_drop(DropId::from_index(0))
        .expect("Drop plan should build");
    let first = test_allocate(&shared, &worker, &mut cache, shape.clone(), Payload::Zeroed);
    let second = test_allocate(&shared, &worker, &mut cache, shape, Payload::Zeroed);
    flush_shared_cache(&shared, &mut cache);

    // request one cycle and finish marking
    shared.request_gc();
    assert!(shared.start_gc().expect("shared collection should start"));
    while shared.gc_phase() == GcPhase::Mark {
        shared
            .step_collection(&[], true, DEFAULT_GC_MINIMUM_WORK_BYTES, trace_view())
            .expect("shared mark step should succeed");
    }
    assert_eq!(shared.gc_phase(), GcPhase::Drop);

    // leave Drop work for a runtime worker
    let coordinator = shared
        .step_collection(&[], true, DEFAULT_GC_MINIMUM_WORK_BYTES, trace_view())
        .expect("shared coordinator step should succeed");
    assert_eq!(coordinator, GcAdvance::Idle);
    assert!(shared.is_heap_live(first));
    assert!(shared.is_heap_live(second));

    // claim one allocation while both remain live
    let first_progress = shared
        .step_collection_for_worker(
            Some(&worker),
            &[],
            true,
            DEFAULT_GC_MINIMUM_WORK_BYTES,
            trace_view(),
        )
        .expect("shared reclamation step should succeed");
    let GcAdvance::Drop(first_drop) = first_progress else {
        panic!("shared allocation should require Drop: {first_progress:?}");
    };

    assert_eq!(first_drop.collector, GcCollector::Shared);
    assert_eq!(first_drop.reference, DropReference::Shared(first));
    assert_eq!(first_drop.drop, DropId::from_index(0));
    assert!(shared.is_heap_live(first));
    assert!(shared.is_heap_live(second));

    // keep allocations made by a destructor black for the active cycle
    let survivor_shape = AllocationShape::new(16, 8, None, TraceMap::empty());
    let survivor = test_allocate(
        &shared,
        &worker,
        &mut cache,
        survivor_shape,
        Payload::Zeroed,
    );

    // reject another claim until the first Drop completes
    let blocked = shared
        .step_collection_for_worker(
            Some(&worker),
            &[],
            true,
            DEFAULT_GC_MINIMUM_WORK_BYTES,
            trace_view(),
        )
        .expect("shared reclamation step should succeed");
    assert_eq!(blocked, GcAdvance::Idle);
    shared
        .complete_drop(first_drop.reference)
        .expect("first shared Drop should complete");

    // claim and complete the second allocation
    let second_progress = shared
        .step_collection_for_worker(
            Some(&worker),
            &[],
            true,
            DEFAULT_GC_MINIMUM_WORK_BYTES,
            trace_view(),
        )
        .expect("shared reclamation step should succeed");
    let GcAdvance::Drop(second_drop) = second_progress else {
        panic!("second shared allocation should require Drop: {second_progress:?}");
    };

    assert_eq!(second_drop.reference, DropReference::Shared(second));
    shared
        .complete_drop(second_drop.reference)
        .expect("second shared Drop should complete");

    // finish reclamation after the callback boundary
    while shared
        .step_collection(&[], true, DEFAULT_GC_MINIMUM_WORK_BYTES, trace_view())
        .expect("shared sweep step should succeed")
        .completed_stats()
        .is_none()
    {}

    assert!(!shared.is_heap_live(first));
    assert!(!shared.is_heap_live(second));
    assert!(shared.is_heap_live(survivor));
}

/// Surface repeated shared values individually in reverse acquisition order.
#[test]
fn test_step_collection_drops_repeated_shared_values_incrementally() {
    let (shared, mut cache, worker, _) = test_shared_heap(&[]);
    let options = SharedHeapOptions::default();
    let element = AllocationShape::new(16, 8, None, TraceMap::empty())
        .with_drop(DropId::from_index(1))
        .expect("Drop plan should build");
    let element = options.allocation_plan(&element);
    let shape = element
        .repeat(&TraceMap::empty(), 3)
        .expect("repeated allocation should build");
    let reference = test_allocate(&shared, &worker, &mut cache, shape, Payload::Zeroed);
    flush_shared_cache(&shared, &mut cache);

    // finish marking before selecting value-level Drop requests
    shared.request_gc();
    assert!(shared.start_gc().expect("shared collection should start"));
    while shared.gc_phase() == GcPhase::Mark {
        shared
            .step_collection(&[], true, DEFAULT_GC_MINIMUM_WORK_BYTES, trace_view())
            .expect("shared mark step should succeed");
    }

    // complete each repeated value while preserving its allocation
    let mut dropped = Vec::new();
    loop {
        let progress = shared
            .step_collection_for_worker(
                Some(&worker),
                &[],
                true,
                DEFAULT_GC_MINIMUM_WORK_BYTES,
                trace_view(),
            )
            .expect("shared reclamation step should succeed");
        if let GcAdvance::Drop(drop) = progress {
            dropped.push(drop.reference);
            assert!(shared.is_heap_live(reference));
            shared
                .complete_drop(drop.reference)
                .expect("shared repeated value Drop should complete");
        }
        if progress.completed_stats().is_some() {
            break;
        }
    }

    assert_eq!(
        dropped,
        [
            DropReference::Shared(SharedHeapReference::new(reference.offset() + 32)),
            DropReference::Shared(SharedHeapReference::new(reference.offset() + 16)),
            DropReference::Shared(reference),
        ]
    );
}

/// Honor one explicit shared collection request below the pacing trigger.
#[test]
fn test_step_collection_honors_manual_request() {
    // allocate one root below the pacing trigger
    let (shared, mut memory, worker, layout_ids) = test_shared_heap(&[(3, TraceMap::empty())]);
    let layout = &layout_ids[0];
    let reachable = test_allocate(
        &shared,
        &worker,
        &mut memory,
        layout.block(),
        Payload::Bytes(&[1, 2, 3]),
    );

    flush_shared_cache(&shared, &mut memory);

    // explicit requests should bypass pressure checks
    shared.request_gc();
    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    shared
        .step_collection(&[reachable], false, 1, trace_view())
        .expect("shared collection step should succeed");

    // first bounded step should enter mark
    assert_eq!(shared.gc_phase(), GcPhase::Mark);
}

/// Consume shared collector work from the active cycle budget.
#[test]
fn test_shared_gc_budget_consumes_cycle_work() {
    // allocate one object so a collection cycle has work
    let (shared, mut memory, worker, layout_ids) = test_shared_heap(&[(3, TraceMap::empty())]);
    let layout = &layout_ids[0];
    let _reference = test_allocate(
        &shared,
        &worker,
        &mut memory,
        layout.block(),
        Payload::Bytes(&[1, 2, 3]),
    );

    flush_shared_cache(&shared, &mut memory);

    // start one explicit shared cycle
    shared.request_gc();
    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    // budget request should consume pacer work
    let before = shared.gc_pacer();
    let budget_bytes = shared.take_collection_budget_bytes(1);
    let after = shared.gc_pacer();

    assert!(before.remaining_work_bytes > 0);
    assert!(budget_bytes > 0);
    assert!(after.remaining_work_bytes < before.remaining_work_bytes);
}
