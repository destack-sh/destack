use crate::{
    GcKind, GcOptions, HeapError, HeapOptions, LayoutId, SharedGcPhase, SharedHeap,
    SharedHeapReference,
};
use destack_mir::LayoutTrace;

/// Build one shared heap whose pacer starts immediately in step-driven tests.
fn test_shared_heap() -> SharedHeap {
    let options = HeapOptions {
        gc: GcOptions {
            growth_percent: 0,
            trigger_percent: 75,
            soft_limit_bytes: None,
            minimum_heap_bytes: Some(0),
        },
        ..HeapOptions::shared()
    };

    SharedHeap::with_options(options)
}

/// Free unreachable shared heap entries and record the completed shared GC cycle.
#[test]
fn test_collect_shared_frees_unreachable_entries() {
    let shared = SharedHeap::new();
    let reachable = shared
        .allocate_heap_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("shared heap allocation should succeed");
    let unreachable = shared
        .allocate_heap_bytes(&[4, 5, 6], LayoutTrace::empty(), None)
        .expect("shared heap allocation should succeed");

    let stats = shared
        .collect_full([reachable])
        .expect("shared collection should succeed");

    assert_eq!(stats.freed_allocations, 1);
    assert_eq!(shared.read_heap_bytes(reachable), Ok(vec![1, 2, 3]));
    assert!(!shared.is_heap_live(unreachable));
    assert_eq!(shared.gc_state().completed_cycles, 1);
    assert_eq!(shared.gc_state().last_kind, Some(GcKind::Full));
    assert_eq!(shared.gc_state().last_stats, Some(stats));
}

/// Trace shared child references through one shared heap payload.
#[test]
fn test_collect_shared_keeps_reachable_children() {
    let shared = SharedHeap::new();
    let child = shared
        .allocate_heap_bytes(&[0xC1, 0x1D], LayoutTrace::empty(), None)
        .expect("shared heap allocation should succeed");
    let parent = shared
        .allocate_heap_bytes(
            &child.bits().to_le_bytes(),
            LayoutTrace::Reference {
                local_offsets: Vec::new().into_boxed_slice(),
                shared_offsets: vec![0].into_boxed_slice(),
            },
            None,
        )
        .expect("shared heap allocation should succeed");

    let stats = shared
        .collect_full([parent])
        .expect("shared collection should succeed");

    assert_eq!(stats.freed_allocations, 0);
    assert_eq!(shared.read_heap_bytes(child), Ok(vec![0xC1, 0x1D]));
}

/// Reject invalid explicit shared heap roots.
#[test]
fn test_collect_shared_rejects_invalid_root() {
    let shared = SharedHeap::new();
    let invalid = SharedHeapReference::new(7);

    let error = shared
        .collect_full([invalid])
        .expect_err("invalid shared roots should fail collection");

    assert_eq!(
        error,
        HeapError::InvalidSharedHeapReference { reference: invalid }
    );
}

/// Preserve shared heap collector state across shared-heap images.
#[test]
fn test_shared_heap_gc_state_roundtrips_through_image() {
    let shared = SharedHeap::new();
    let reference = shared
        .allocate_heap_bytes(
            &SharedHeapReference::NULL.bits().to_le_bytes(),
            LayoutTrace::empty(),
            None,
        )
        .expect("shared heap allocation should succeed");
    shared
        .set_heap_layout_id(reference, LayoutId::new(41))
        .expect("shared heap storage layout id should update");
    let stats = shared
        .collect_full([reference])
        .expect("shared collection should succeed");
    let allocator = shared.allocator.clone();
    let image = shared.image();
    let restored = SharedHeap::from_image_with_allocator(allocator, &image)
        .expect("shared image should restore");

    assert_eq!(restored.gc_state().last_kind, Some(GcKind::Full));
    assert_eq!(restored.gc_state().last_stats, Some(stats));
    assert_eq!(
        restored.heap_layout_id(reference),
        Ok(Some(LayoutId::new(41)))
    );
}

/// Keep a child written during mark through the shared write barrier.
#[test]
fn test_collect_shared_barrier_keeps_written_child() {
    let shared = test_shared_heap();
    let child = shared
        .allocate_heap_bytes(&[0xC1, 0x1D], LayoutTrace::empty(), None)
        .expect("shared heap allocation should succeed");
    let parent = shared
        .allocate_heap_zeroed(
            SharedHeapReference::BYTE_LEN,
            LayoutTrace::Reference {
                local_offsets: Vec::new().into_boxed_slice(),
                shared_offsets: vec![0].into_boxed_slice(),
            },
            None,
        )
        .expect("shared heap allocation should succeed");

    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    shared
        .gc_step(&[parent], false, 1)
        .expect("shared collection step should succeed");
    assert_eq!(shared.gc_phase(), SharedGcPhase::Mark);

    shared
        .write_heap_bytes(parent, 0, &child.bits().to_le_bytes())
        .expect("shared heap write should succeed");
    shared
        .write_barrier(parent, 0, SharedHeapReference::BYTE_LEN)
        .expect("shared barrier should succeed");

    while shared
        .gc_step(&[parent], true, 1)
        .expect("shared collection step should succeed")
        .is_none()
    {}

    assert!(shared.is_heap_live(child));
}

/// Keep a child published through one exact shared-reference barrier path.
#[test]
fn test_collect_shared_publish_edge_keeps_written_child() {
    let shared = test_shared_heap();
    let child = shared
        .allocate_heap_bytes(&[0xC1, 0x1D], LayoutTrace::empty(), None)
        .expect("shared heap allocation should succeed");
    let parent = shared
        .allocate_heap_zeroed(
            SharedHeapReference::BYTE_LEN,
            LayoutTrace::Reference {
                local_offsets: Vec::new().into_boxed_slice(),
                shared_offsets: vec![0].into_boxed_slice(),
            },
            None,
        )
        .expect("shared heap allocation should succeed");

    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    shared
        .gc_step(&[parent], false, 1)
        .expect("shared collection step should succeed");
    assert_eq!(shared.gc_phase(), SharedGcPhase::Mark);

    shared
        .write_heap_bytes(parent, 0, &child.bits().to_le_bytes())
        .expect("shared heap write should succeed");
    shared
        .publish_edge(child)
        .expect("shared exact publish should succeed");

    while shared
        .gc_step(&[parent], true, 1)
        .expect("shared collection step should succeed")
        .is_none()
    {}

    assert!(shared.is_heap_live(child));
}

/// Keep one allocation created during mark alive through the active cycle.
#[test]
fn test_collect_shared_keeps_allocation_created_during_mark() {
    let shared = test_shared_heap();
    let root = shared
        .allocate_heap_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("shared heap allocation should succeed");

    shared.request_gc();
    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    shared
        .gc_step(&[root], false, 1)
        .expect("shared collection step should succeed");
    assert_eq!(shared.gc_phase(), SharedGcPhase::Mark);

    let late = shared
        .allocate_heap_bytes(&[7, 8, 9], LayoutTrace::empty(), None)
        .expect("shared heap allocation should succeed");

    while shared
        .gc_step(&[root], true, 1)
        .expect("shared collection step should succeed")
        .is_none()
    {}

    assert!(shared.is_heap_live(root));
    assert!(shared.is_heap_live(late));
}

/// Keep one allocation created during sweep alive through the active cycle.
#[test]
fn test_collect_shared_keeps_allocation_created_during_sweep() {
    let shared = test_shared_heap();
    let root = shared
        .allocate_heap_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("shared heap allocation should succeed");
    let unreachable = shared
        .allocate_heap_bytes(&[4, 5, 6], LayoutTrace::empty(), None)
        .expect("shared heap allocation should succeed");

    shared.request_gc();
    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    while shared.gc_phase() == SharedGcPhase::Mark {
        shared
            .gc_step(&[root], true, 1)
            .expect("shared collection step should succeed");
    }

    assert_eq!(shared.gc_phase(), SharedGcPhase::Sweep);

    let late = shared
        .allocate_heap_bytes(&[7, 8, 9], LayoutTrace::empty(), None)
        .expect("shared heap allocation should succeed");

    while shared
        .gc_step(&[], true, 1)
        .expect("shared collection step should succeed")
        .is_none()
    {}

    assert!(shared.is_heap_live(root));
    assert!(shared.is_heap_live(late));
    assert!(!shared.is_heap_live(unreachable));
}

/// Require explicit mark termination before shared sweep begins.
#[test]
fn test_collect_shared_requires_explicit_mark_finish() {
    let shared = test_shared_heap();
    let reachable = shared
        .allocate_heap_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("shared heap allocation should succeed");
    let unreachable = shared
        .allocate_heap_bytes(&[4, 5, 6], LayoutTrace::empty(), None)
        .expect("shared heap allocation should succeed");

    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    shared
        .gc_step(&[reachable], false, 1)
        .expect("shared collection step should succeed");

    assert_eq!(shared.gc_phase(), SharedGcPhase::Mark);
    assert!(shared.is_heap_live(unreachable));

    while shared
        .gc_step(&[reachable], true, 1)
        .expect("shared collection step should succeed")
        .is_none()
    {}

    assert_eq!(shared.gc_phase(), SharedGcPhase::Idle);
    assert!(!shared.is_heap_live(unreachable));
}

/// Stay idle when no shared pressure or explicit request exists.
#[test]
fn test_shared_gc_step_stays_idle_without_request() {
    let shared = SharedHeap::new();

    let stats = shared
        .gc_step(&[], true, 1)
        .expect("shared gc step should succeed");

    assert_eq!(stats, None);
    assert_eq!(shared.gc_phase(), SharedGcPhase::Idle);
}

/// Honor one explicit shared collection request below the pacing trigger.
#[test]
fn test_shared_gc_step_honors_manual_request() {
    let shared = SharedHeap::new();
    let reachable = shared
        .allocate_heap_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("shared heap allocation should succeed");

    shared.request_gc();
    assert!(
        shared.start_gc().expect("shared collection should start"),
        "shared collection should become active"
    );

    shared
        .gc_step(&[reachable], false, 1)
        .expect("shared gc step should succeed");

    assert_eq!(shared.gc_phase(), SharedGcPhase::Mark);
}
