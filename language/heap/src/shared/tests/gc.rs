use crate::{
    GcCycle, GcKind, GcPacing, HeapError, HeapOptions, LayoutId, SharedGcPhase, SharedHeap,
    SharedManagedReference,
};
use destack_mir::LayoutTrace;

/// Build one shared heap whose pacer starts immediately in step-driven tests.
fn pacing_shared_heap() -> SharedHeap {
    let options = HeapOptions {
        gc: GcPacing {
            growth_percent: 0,
            trigger_percent: 75,
            soft_limit_bytes: None,
            minimum_heap_bytes: Some(0),
        },
        ..HeapOptions::shared()
    };

    SharedHeap::with_options(options)
}

/// Free unreachable shared managed entries and record the completed shared GC cycle.
#[test]
fn test_collect_shared_frees_unreachable_entries() {
    let mut shared = SharedHeap::new();
    let reachable = shared
        .allocate_managed_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("shared managed allocation should succeed");
    let unreachable = shared
        .allocate_managed_bytes(&[4, 5, 6], LayoutTrace::empty(), None)
        .expect("shared managed allocation should succeed");

    let stats = shared
        .collect_full([reachable])
        .expect("shared collection should succeed");

    assert_eq!(stats.freed_allocations, 1);
    assert_eq!(shared.read_managed_bytes(reachable), Ok(vec![1, 2, 3]));
    assert!(!shared.is_managed_live(unreachable));
    assert_eq!(shared.gc_state().completed_cycles, 1);
    assert_eq!(
        shared.gc_state().last_cycle,
        Some(GcCycle {
            kind: GcKind::Full,
            stats,
        })
    );
}

/// Trace shared child references through one shared managed payload.
#[test]
fn test_collect_shared_keeps_reachable_children() {
    let mut shared = SharedHeap::new();
    let child = shared
        .allocate_managed_bytes(&[0xC1, 0x1D], LayoutTrace::empty(), None)
        .expect("shared managed allocation should succeed");
    let parent = shared
        .allocate_managed_bytes(
            &child.bits().to_le_bytes(),
            LayoutTrace::Reference {
                local_offsets: Vec::new().into_boxed_slice(),
                shared_offsets: vec![0].into_boxed_slice(),
            },
            None,
        )
        .expect("shared managed allocation should succeed");

    let stats = shared
        .collect_full([parent])
        .expect("shared collection should succeed");

    assert_eq!(stats.freed_allocations, 0);
    assert_eq!(shared.read_managed_bytes(child), Ok(vec![0xC1, 0x1D]));
}

/// Reject invalid explicit shared managed roots.
#[test]
fn test_collect_shared_rejects_invalid_root() {
    let mut shared = SharedHeap::new();
    let invalid = SharedManagedReference::new(7);

    let error = shared
        .collect_full([invalid])
        .expect_err("invalid shared roots should fail collection");

    assert_eq!(
        error,
        HeapError::InvalidSharedManagedReference { reference: invalid }
    );
}

/// Preserve shared managed collector state across shared-heap images.
#[test]
fn test_shared_managed_gc_state_roundtrips_through_image() {
    let mut shared = SharedHeap::new();
    let reference = shared
        .allocate_managed_bytes(
            &SharedManagedReference::NULL.bits().to_le_bytes(),
            LayoutTrace::empty(),
            None,
        )
        .expect("shared managed allocation should succeed");
    shared
        .set_managed_layout_id(reference, LayoutId::new(41))
        .expect("shared managed storage layout id should update");
    let stats = shared
        .collect_full([reference])
        .expect("shared collection should succeed");
    let arena = shared.arena.clone();
    let image = shared.image();
    let restored =
        SharedHeap::from_image_with_arena(arena, &image).expect("shared image should restore");

    assert_eq!(
        restored.gc_state().last_cycle,
        Some(GcCycle {
            kind: GcKind::Full,
            stats,
        })
    );
    assert_eq!(
        restored.managed_layout_id(reference),
        Ok(Some(LayoutId::new(41)))
    );
}

/// Keep a child written during mark through the shared write barrier.
#[test]
fn test_collect_shared_barrier_keeps_written_child() {
    let mut shared = pacing_shared_heap();
    let child = shared
        .allocate_managed_bytes(&[0xC1, 0x1D], LayoutTrace::empty(), None)
        .expect("shared managed allocation should succeed");
    let parent = shared
        .allocate_managed_zeroed(
            SharedManagedReference::BYTE_LEN,
            LayoutTrace::Reference {
                local_offsets: Vec::new().into_boxed_slice(),
                shared_offsets: vec![0].into_boxed_slice(),
            },
            None,
        )
        .expect("shared managed allocation should succeed");

    shared
        .gc_step(&[parent], false, 1)
        .expect("shared collection step should succeed");
    assert_eq!(shared.gc_phase(), SharedGcPhase::Mark);

    shared
        .write_managed_bytes(parent, 0, &child.bits().to_le_bytes())
        .expect("shared managed write should succeed");
    shared
        .write_barrier(parent, 0, SharedManagedReference::BYTE_LEN)
        .expect("shared barrier should succeed");

    while shared
        .gc_step(&[parent], true, 1)
        .expect("shared collection step should succeed")
        .is_none()
    {}

    assert!(shared.is_managed_live(child));
}

/// Require explicit mark termination before shared sweep begins.
#[test]
fn test_collect_shared_requires_explicit_mark_finish() {
    let mut shared = pacing_shared_heap();
    let reachable = shared
        .allocate_managed_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("shared managed allocation should succeed");
    let unreachable = shared
        .allocate_managed_bytes(&[4, 5, 6], LayoutTrace::empty(), None)
        .expect("shared managed allocation should succeed");

    shared
        .gc_step(&[reachable], false, 1)
        .expect("shared collection step should succeed");

    assert_eq!(shared.gc_phase(), SharedGcPhase::Mark);
    assert!(shared.is_managed_live(unreachable));

    while shared
        .gc_step(&[reachable], true, 1)
        .expect("shared collection step should succeed")
        .is_none()
    {}

    assert_eq!(shared.gc_phase(), SharedGcPhase::Idle);
    assert!(!shared.is_managed_live(unreachable));
}

/// Stay idle when no shared pressure or explicit request exists.
#[test]
fn test_shared_gc_step_stays_idle_without_request() {
    let mut shared = SharedHeap::new();

    let stats = shared
        .gc_step(&[], true, 1)
        .expect("shared gc step should succeed");

    assert_eq!(stats, None);
    assert_eq!(shared.gc_phase(), SharedGcPhase::Idle);
}

/// Honor one explicit shared collection request below the pacing trigger.
#[test]
fn test_shared_gc_step_honors_manual_request() {
    let mut shared = SharedHeap::new();
    let reachable = shared
        .allocate_managed_bytes(&[1, 2, 3], LayoutTrace::empty(), None)
        .expect("shared managed allocation should succeed");

    shared.request_gc();

    shared
        .gc_step(&[reachable], false, 1)
        .expect("shared gc step should succeed");

    assert_eq!(shared.gc_phase(), SharedGcPhase::Mark);
}
