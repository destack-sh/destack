use std::sync::Arc;

use crate::{
    Allocator, GcKind, GcOptions, GcProgress, HeapError, HeapOptions, Payload, SharedAllocator,
    SharedGcPhase, SharedGcWorker, SharedHeap, SharedHeapLimits, SharedHeapReference,
    SizeClassTable, TestLayout, test_layout, test_layouts,
};
use destack_mir::TraceMap;

use super::{read_mapped_bytes, write_mapped_bytes};

/// Build one shared heap whose pacer starts immediately in step-driven tests.
fn test_shared_heap(
    layouts: &[(usize, TraceMap)],
) -> (SharedHeap, SharedAllocator, SharedGcWorker, Vec<TestLayout>) {
    let options = HeapOptions {
        gc: GcOptions {
            growth_percent: 0,
            trigger_percent: 75,
            soft_limit_bytes: None,
            minimum_heap_bytes: Some(0),
            minimum_work_bytes: crate::DEFAULT_GC_MINIMUM_WORK_BYTES,
        },
        ..HeapOptions::shared()
    };
    let layouts = test_layouts(layouts);

    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );
    let heap = SharedHeap::with_allocator_limits_and_options(
        allocator,
        SharedHeapLimits::default(),
        options,
    )
    .expect("shared heap should build");

    let allocator = heap.allocator();
    let worker = heap.register_collector_worker();

    (heap, allocator, worker, layouts)
}

/// Publish worker-local shared allocations before a direct heap collection.
fn flush_shared_allocator(shared: &SharedHeap, allocator: &mut SharedAllocator) {
    shared.flush_allocator(allocator);
}

/// Reject one zero-size shared managed heap allocation.
#[test]
fn test_allocate_shared_rejects_zero_size_layout() {
    let (shared, mut allocator, worker, layout_ids) = test_shared_heap(&[(0, TraceMap::empty())]);
    let layout = &layout_ids[0];

    let error = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            Payload::Bytes(&[]),
        )
        .expect_err("shared heap allocation should reject zero-size layouts");

    assert_eq!(error, HeapError::ZeroSizeAllocation);
}

/// Publish shared worker-run allocations before flushing the allocator.
#[test]
fn test_allocate_shared_zeroed_worker_run_is_live_before_flush() {
    let (shared, mut allocator, worker, layout_ids) = test_shared_heap(&[(8, TraceMap::empty())]);
    let layout = shared.allocation_plan(layout_ids[0].allocation());

    let first = shared
        .allocate_zeroed(&worker, &mut allocator, &layout)
        .expect("first shared allocation should succeed");
    let second = shared
        .allocate_zeroed(&worker, &mut allocator, &layout)
        .expect("second shared allocation should succeed");

    assert!(shared.is_heap_live(first));
    assert!(shared.is_heap_live(second));
    assert_eq!(shared.heap_allocation_count(), 2);
}

/// Publish byte-initialized shared worker-run allocations before flushing the allocator.
#[test]
fn test_allocate_shared_bytes_worker_run_is_live_before_flush() {
    let (shared, mut allocator, worker, layout_ids) = test_shared_heap(&[(8, TraceMap::empty())]);
    let layout = shared.allocation_plan(layout_ids[0].allocation());

    let first = shared
        .allocate_bytes(&worker, &mut allocator, &layout, &[1, 2, 3, 4, 5, 6, 7, 8])
        .expect("first shared allocation should succeed");
    let second = shared
        .allocate_bytes(&worker, &mut allocator, &layout, &[8, 7, 6, 5, 4, 3, 2, 1])
        .expect("second shared allocation should succeed");

    assert!(shared.is_heap_live(first));
    assert!(shared.is_heap_live(second));
    assert_eq!(shared.heap_allocation_count(), 2);
}

/// Free unreachable shared heap allocations and record the completed shared GC cycle.
#[test]
fn test_collect_shared_frees_unreachable_entries() {
    let (shared, mut allocator, worker, layout_ids) = test_shared_heap(&[(3, TraceMap::empty())]);
    let layout = &layout_ids[0];
    let reachable = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1, 2, 3]),
        )
        .expect("shared heap allocation should succeed");
    let unreachable = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            Payload::Bytes(&[4, 5, 6]),
        )
        .expect("shared heap allocation should succeed");

    flush_shared_allocator(&shared, &mut allocator);

    let stats = shared
        .collect_full(&[reachable])
        .expect("shared collection should succeed");

    assert_eq!(stats.freed_allocations, 1);
    assert!(!shared.is_heap_live(unreachable));
    let address = shared.heap_base_address() + reachable.offset();

    let bytes = read_mapped_bytes(address, 8);

    assert_eq!(bytes, &[1, 2, 3, 0, 0, 0, 0, 0]);
    assert_eq!(shared.gc_state().completed_cycles, 1);
    assert_eq!(shared.gc_state().last_kind, Some(GcKind::Full));
    assert_eq!(shared.gc_state().last_stats, Some(stats));
}

/// Clear one reused shared small heap slot before writing a shorter payload.
#[test]
fn test_collect_shared_clears_reused_small_slot_tail() {
    let options = HeapOptions {
        heap_small_bytes: 16,
        size_classes: SizeClassTable::new([8]).expect("size classes should validate"),
        ..HeapOptions::shared()
    };
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );
    let shared = SharedHeap::with_allocator_limits_and_options(
        allocator,
        SharedHeapLimits::default(),
        options,
    )
    .expect("shared heap should build");
    let mut allocator = shared.allocator();
    let worker = shared.register_collector_worker();
    let full_layout = test_layout(8, TraceMap::empty());
    let short_layout = test_layout(1, TraceMap::empty());

    let first = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(full_layout.allocation()),
            Payload::Bytes(&[0xAA; 8]),
        )
        .expect("shared heap allocation should succeed");
    let second = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(short_layout.allocation()),
            Payload::Bytes(&[0xBB]),
        )
        .expect("shared heap allocation should succeed");

    flush_shared_allocator(&shared, &mut allocator);

    shared
        .collect_full(&[second])
        .expect("shared collection should succeed");
    assert!(!shared.is_heap_live(first));

    let reused = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(short_layout.allocation()),
            Payload::Bytes(&[0xCC]),
        )
        .expect("shared heap allocation should succeed");

    assert!(shared.is_heap_live(second));
    assert!(shared.is_heap_live(reused));
    let address = shared.heap_base_address() + reused.offset();

    let bytes = read_mapped_bytes(address, 8);

    assert_eq!(bytes, &[0xCC, 0, 0, 0, 0, 0, 0, 0]);
}

/// Trace shared child references through one shared heap payload.
#[test]
fn test_collect_shared_keeps_reachable_children() {
    let trace_map = TraceMap::Fixed {
        local_offsets: Vec::new().into_boxed_slice(),
        shared_offsets: vec![0].into_boxed_slice(),
    };
    let (shared, mut allocator, worker, layout_ids) =
        test_shared_heap(&[(2, TraceMap::empty()), (8, trace_map.clone())]);
    let child_layout = &layout_ids[0];
    let parent_layout = &layout_ids[1];
    let child = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(child_layout.allocation()),
            Payload::Bytes(&[0xC1, 0x1D]),
        )
        .expect("shared heap allocation should succeed");
    let parent = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(parent_layout.allocation()),
            Payload::Bytes(&child.bits().to_le_bytes()),
        )
        .expect("shared heap allocation should succeed");

    flush_shared_allocator(&shared, &mut allocator);

    let stats = shared
        .collect_full(&[parent])
        .expect("shared collection should succeed");

    assert_eq!(stats.freed_allocations, 0);
    assert!(shared.is_heap_live(child));
    let address = shared.heap_base_address() + child.offset();

    let bytes = read_mapped_bytes(address, 8);

    assert_eq!(bytes, &[0xC1, 0x1D, 0, 0, 0, 0, 0, 0]);
}

/// Bound shared small-span mark work by marked slots.
#[test]
fn test_collect_shared_scans_small_spans_incrementally() {
    let trace_map = TraceMap::Fixed {
        local_offsets: Vec::new().into_boxed_slice(),
        shared_offsets: vec![0].into_boxed_slice(),
    };
    let (shared, mut allocator, worker, layout_ids) =
        test_shared_heap(&[(2, TraceMap::empty()), (8, trace_map.clone())]);
    let child_layout = &layout_ids[0];
    let parent_layout = &layout_ids[1];
    let first_child = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(child_layout.allocation()),
            Payload::Bytes(&[0xC1, 0x1D]),
        )
        .expect("shared heap allocation should succeed");
    let second_child = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(child_layout.allocation()),
            Payload::Bytes(&[0xC2, 0x1D]),
        )
        .expect("shared heap allocation should succeed");
    let first_parent = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(parent_layout.allocation()),
            Payload::Bytes(&first_child.bits().to_le_bytes()),
        )
        .expect("shared heap allocation should succeed");
    let second_parent = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(parent_layout.allocation()),
            Payload::Bytes(&second_child.bits().to_le_bytes()),
        )
        .expect("shared heap allocation should succeed");

    flush_shared_allocator(&shared, &mut allocator);

    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    shared
        .collect_step(&[first_parent, second_parent], true, 1)
        .expect("shared collection step should succeed");

    assert_eq!(shared.gc_phase(), SharedGcPhase::Mark);
    assert!(!shared.mark_idle());

    while shared
        .collect_step(&[first_parent, second_parent], true, 1)
        .expect("shared collection step should succeed")
        .completed_stats()
        .is_none()
    {}

    assert!(shared.is_heap_live(first_child));
    assert!(shared.is_heap_live(second_child));
}

/// Scan large shared allocations incrementally by allocator page.
#[test]
fn test_collect_shared_scans_large_allocations_incrementally() {
    let options = HeapOptions::shared();
    let first_offset = 0usize;
    let second_offset = options.page_bytes;
    let parent_byte_len = second_offset + SharedHeapReference::BYTE_LEN;
    let second_offset = u32::try_from(second_offset).expect("page offset should fit uint32");
    let trace_map = TraceMap::Fixed {
        local_offsets: Vec::new().into_boxed_slice(),
        shared_offsets: vec![first_offset as u32, second_offset].into_boxed_slice(),
    };
    let (shared, mut allocator, worker, layout_ids) =
        test_shared_heap(&[(2, TraceMap::empty()), (parent_byte_len, trace_map)]);
    let child_layout = &layout_ids[0];
    let parent_layout = &layout_ids[1];
    let first_child = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(child_layout.allocation()),
            Payload::Bytes(&[0xC1, 0x1D]),
        )
        .expect("shared heap allocation should succeed");
    let second_child = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(child_layout.allocation()),
            Payload::Bytes(&[0xC2, 0x1D]),
        )
        .expect("shared heap allocation should succeed");
    let mut parent_bytes = vec![0; parent_byte_len];
    parent_bytes[first_offset..first_offset + SharedHeapReference::BYTE_LEN]
        .copy_from_slice(&first_child.bits().to_le_bytes());
    parent_bytes[second_offset as usize..second_offset as usize + SharedHeapReference::BYTE_LEN]
        .copy_from_slice(&second_child.bits().to_le_bytes());
    let parent = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(parent_layout.allocation()),
            Payload::Bytes(&parent_bytes),
        )
        .expect("shared heap allocation should succeed");

    flush_shared_allocator(&shared, &mut allocator);

    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    shared
        .collect_step(&[parent], true, 1)
        .expect("shared collection step should succeed");
    assert_eq!(shared.gc_phase(), SharedGcPhase::Mark);

    while shared
        .collect_step(&[parent], true, 1)
        .expect("shared collection step should succeed")
        .completed_stats()
        .is_none()
    {}

    assert!(shared.is_heap_live(first_child));
    assert!(shared.is_heap_live(second_child));
}

/// Reject invalid explicit shared heap roots.
#[test]
fn test_collect_shared_rejects_invalid_root() {
    let options = HeapOptions::shared();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );
    let shared = SharedHeap::with_allocator_limits_and_options(
        allocator,
        SharedHeapLimits::default(),
        options,
    )
    .expect("shared heap should build");
    let invalid = SharedHeapReference::new(7);

    let error = shared
        .collect_full(&[invalid])
        .expect_err("invalid shared roots should fail collection");

    assert_eq!(
        error,
        HeapError::InvalidSharedHeapReference { reference: invalid }
    );
}

/// Preserve shared heap collector state across shared-heap images.
#[test]
fn test_shared_heap_gc_state_roundtrips_through_image() {
    let layout = test_layout(8, TraceMap::empty());
    let options = HeapOptions::shared();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );
    let shared = SharedHeap::with_allocator_limits_and_options(
        allocator.clone(),
        SharedHeapLimits::default(),
        options.clone(),
    )
    .expect("shared heap should build");
    let mut allocator = shared.allocator();
    let worker = shared.register_collector_worker();
    let reference = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            Payload::Bytes(&SharedHeapReference::NULL.bits().to_le_bytes()),
        )
        .expect("shared heap allocation should succeed");
    flush_shared_allocator(&shared, &mut allocator);

    let stats = shared
        .collect_full(&[reference])
        .expect("shared collection should succeed");
    let image = shared.image().expect("shared image should capture");
    let restored = SharedHeap::from_image_with_limits(&image, SharedHeapLimits::default())
        .expect("shared image should restore");

    assert_eq!(restored.gc_state().last_kind, Some(GcKind::Full));
    assert_eq!(restored.gc_state().last_stats, Some(stats));
}

/// Restore shared heap collector state from one serialized snapshot.
#[test]
fn test_shared_heap_gc_state_roundtrips_through_snapshot() {
    let layout = test_layout(8, TraceMap::empty());
    let options = HeapOptions::shared();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );
    let shared = SharedHeap::with_allocator_limits_and_options(
        allocator,
        SharedHeapLimits::default(),
        options,
    )
    .expect("shared heap should build");
    let mut allocator = shared.allocator();
    let worker = shared.register_collector_worker();
    let reference = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            Payload::Bytes(&SharedHeapReference::NULL.bits().to_le_bytes()),
        )
        .expect("shared heap allocation should succeed");
    flush_shared_allocator(&shared, &mut allocator);

    let stats = shared
        .collect_full(&[reference])
        .expect("shared collection should succeed");
    let image = shared.image().expect("shared image should capture");
    let snapshot = image.snapshot().expect("shared snapshot should capture");
    let restored = SharedHeap::from_snapshot_with_limits(&snapshot, SharedHeapLimits::default())
        .expect("shared snapshot should restore");

    assert_eq!(restored.gc_state().last_kind, Some(GcKind::Full));
    assert_eq!(restored.gc_state().last_stats, Some(stats));
}

/// Keep a child written during mark through the shared write barrier.
#[test]
fn test_collect_shared_barrier_keeps_written_child() {
    let trace_map = TraceMap::Fixed {
        local_offsets: Vec::new().into_boxed_slice(),
        shared_offsets: vec![0].into_boxed_slice(),
    };
    let (shared, mut allocator, worker, layout_ids) =
        test_shared_heap(&[(2, TraceMap::empty()), (8, trace_map.clone())]);
    let child_layout = &layout_ids[0];
    let parent_layout = &layout_ids[1];
    let child = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(child_layout.allocation()),
            Payload::Bytes(&[0xC1, 0x1D]),
        )
        .expect("shared heap allocation should succeed");
    let parent = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(parent_layout.allocation()),
            Payload::Zeroed,
        )
        .expect("shared heap allocation should succeed");

    flush_shared_allocator(&shared, &mut allocator);

    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    shared
        .collect_step(&[parent], false, 1)
        .expect("shared collection step should succeed");
    assert_eq!(shared.gc_phase(), SharedGcPhase::Mark);

    shared
        .write_barrier_bytes(parent, 0, &child.bits().to_le_bytes())
        .expect("shared heap write barrier should record");
    let address = shared.heap_base_address() + parent.offset();

    write_mapped_bytes(address, &child.bits().to_le_bytes());

    while shared
        .collect_step(&[parent], true, 1)
        .expect("shared collection step should succeed")
        .completed_stats()
        .is_none()
    {}

    assert!(shared.is_heap_live(child));
}

/// Keep one allocation created during mark alive through the active cycle.
#[test]
fn test_collect_shared_keeps_allocation_created_during_mark() {
    let (shared, mut allocator, worker, layout_ids) = test_shared_heap(&[(3, TraceMap::empty())]);
    let layout = &layout_ids[0];
    let root = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1, 2, 3]),
        )
        .expect("shared heap allocation should succeed");

    flush_shared_allocator(&shared, &mut allocator);

    shared.request_gc();
    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    shared
        .collect_step(&[root], false, 1)
        .expect("shared collection step should succeed");
    assert_eq!(shared.gc_phase(), SharedGcPhase::Mark);

    let late = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            Payload::Bytes(&[7, 8, 9]),
        )
        .expect("shared heap allocation should succeed");

    while shared
        .collect_step(&[root], true, 1)
        .expect("shared collection step should succeed")
        .completed_stats()
        .is_none()
    {}

    assert!(shared.is_heap_live(root));
    assert!(shared.is_heap_live(late));
}

/// Keep a shared allocation created during mark after publishing one worker-local run.
#[test]
fn test_collect_shared_keeps_run_allocation_created_during_mark() {
    let (shared, mut allocator, worker, layout_ids) = test_shared_heap(&[(3, TraceMap::empty())]);
    let layout = &layout_ids[0];
    let seed = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1, 2, 3]),
        )
        .expect("shared heap allocation should succeed");

    shared.request_gc();
    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    let late = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            Payload::Bytes(&[7, 8, 9]),
        )
        .expect("shared heap allocation should succeed");

    while shared
        .collect_step(&[], true, 1)
        .expect("shared collection step should succeed")
        .completed_stats()
        .is_none()
    {}

    assert!(!shared.is_heap_live(seed));
    assert!(shared.is_heap_live(late));
    let address = shared.heap_base_address() + late.offset();

    // inspect the late allocation payload
    let bytes = read_mapped_bytes(address, 8);

    assert_eq!(bytes, &[7, 8, 9, 0, 0, 0, 0, 0]);
}

/// Keep one allocation created while allocation assists sweep.
#[test]
fn test_allocate_shared_assists_sweep_before_returning() {
    let (shared, mut allocator, worker, layout_ids) = test_shared_heap(&[(3, TraceMap::empty())]);
    let layout = &layout_ids[0];
    let root = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1, 2, 3]),
        )
        .expect("shared heap allocation should succeed");
    let unreachable = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            Payload::Bytes(&[4, 5, 6]),
        )
        .expect("shared heap allocation should succeed");

    flush_shared_allocator(&shared, &mut allocator);

    shared.request_gc();
    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    while shared.gc_phase() == SharedGcPhase::Mark {
        shared
            .collect_step(&[root], true, 1)
            .expect("shared collection step should succeed");
    }

    assert_eq!(shared.gc_phase(), SharedGcPhase::Sweep);
    let completed_cycles = shared.gc_state().completed_cycles;

    let late = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            Payload::Bytes(&[7, 8, 9]),
        )
        .expect("shared heap allocation should succeed");

    while shared.gc_phase() != SharedGcPhase::Idle {
        shared
            .collect_step(&[], true, 1)
            .expect("shared collection step should succeed");
    }

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
    let (shared, mut allocator, worker, layout_ids) = test_shared_heap(&[(3, TraceMap::empty())]);
    let layout = &layout_ids[0];
    let reachable = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1, 2, 3]),
        )
        .expect("shared heap allocation should succeed");
    let unreachable = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            Payload::Bytes(&[4, 5, 6]),
        )
        .expect("shared heap allocation should succeed");

    flush_shared_allocator(&shared, &mut allocator);

    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    shared
        .collect_step(&[reachable], false, 1)
        .expect("shared collection step should succeed");

    assert_eq!(shared.gc_phase(), SharedGcPhase::Mark);
    assert!(shared.is_heap_live(unreachable));

    while shared
        .collect_step(&[reachable], true, 1)
        .expect("shared collection step should succeed")
        .completed_stats()
        .is_none()
    {}

    assert_eq!(shared.gc_phase(), SharedGcPhase::Idle);
    assert!(!shared.is_heap_live(unreachable));
}

/// Stay idle when no shared pressure or explicit request exists.
#[test]
fn test_collect_step_stays_idle_without_request() {
    let options = HeapOptions::shared();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("allocator should build"),
    );
    let shared = SharedHeap::with_allocator_limits_and_options(
        allocator,
        SharedHeapLimits::default(),
        options,
    )
    .expect("shared heap should build");

    let progress = shared
        .collect_step(&[], true, 1)
        .expect("shared collection step should succeed");

    assert_eq!(progress, GcProgress::Idle);
    assert_eq!(shared.gc_phase(), SharedGcPhase::Idle);
}

/// Honor one explicit shared collection request below the pacing trigger.
#[test]
fn test_collect_step_honors_manual_request() {
    let (shared, mut allocator, worker, layout_ids) = test_shared_heap(&[(3, TraceMap::empty())]);
    let layout = &layout_ids[0];
    let reachable = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1, 2, 3]),
        )
        .expect("shared heap allocation should succeed");

    flush_shared_allocator(&shared, &mut allocator);

    shared.request_gc();
    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    shared
        .collect_step(&[reachable], false, 1)
        .expect("shared collection step should succeed");

    assert_eq!(shared.gc_phase(), SharedGcPhase::Mark);
}

/// Consume shared collector work from the active cycle budget.
#[test]
fn test_shared_gc_budget_consumes_cycle_work() {
    let (shared, mut allocator, worker, layout_ids) = test_shared_heap(&[(3, TraceMap::empty())]);
    let layout = &layout_ids[0];
    let _reference = shared
        .allocate(
            &worker,
            &mut allocator,
            &shared.allocation_plan(layout.allocation()),
            Payload::Bytes(&[1, 2, 3]),
        )
        .expect("shared heap allocation should succeed");

    flush_shared_allocator(&shared, &mut allocator);

    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    let before = shared.gc_pacer();
    let budget_bytes = shared.take_collection_budget_bytes(1);
    let after = shared.gc_pacer();

    assert!(before.remaining_work_bytes > 0);
    assert!(budget_bytes > 0);
    assert!(after.remaining_work_bytes < before.remaining_work_bytes);
}
