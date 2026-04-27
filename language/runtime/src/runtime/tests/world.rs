#![allow(clippy::arc_with_non_send_sync)]

use std::sync::Arc;

use destack_engine::Value;
use destack_heap as heap;
use destack_mir::ReferenceMap;
use destack_workspace::{
    ExecutionMode, RandomMode, RuntimeAccess, RuntimeIdentitySelector, RuntimeOptions,
    RuntimeSelector, RuntimeWorld, TimeMode,
};

use super::tests::{AllocatingEngine, TestEngine, TestRuntime, TestWorld};
use crate::host::{HostEventKind, Session};
use crate::platform::{ResourceEntry, ResourceId, ResourceKind};
use crate::runtime::bindings::BindingDescriptor;
use crate::runtime::engine::{Continuation, NativeContinuation};
use crate::runtime::memory::{RootSet, RootVisitor};
use crate::runtime::observe::{Observation, ObservationCategory, ObservationOptions};
use crate::runtime::policy::{
    Effect, Fault, FaultTarget, FaultType, Hook, Policy, Rule, RuleId, Trigger,
};
use crate::runtime::poller::{
    PollerEvent, PollerEventFlags, PollerEventMask, PollerEventPayload, PollerEventSource,
    PollerToken,
};
use crate::runtime::scheduler::{Runnable, Task, TaskId, TaskStatus};
use crate::runtime::time::WorldInstant;
use crate::runtime::trace::{Trace, TraceRecord, TraceSequence};
use crate::runtime::{
    BranchId, Command, Worker, WorkerId, World, WorldEdge, WorldEdgeKindDefinition, WorldEntity,
    WorldEntityKindDefinition, WorldResourceId,
};

/// Return one byte payload layout for runtime tests.
fn byte_layout<'a>(byte_len: usize, reference_map: &'a ReferenceMap) -> heap::AllocationLayout<'a> {
    heap::AllocationLayout::new(byte_len, reference_map)
}

/// Build runtime options with record mode enabled.
fn record_options() -> RuntimeOptions {
    let mut options = RuntimeOptions::default();
    options.set_execution_mode(ExecutionMode::Record);
    options
}

/// Ensures hard heap limits fail after one allocating entrypoint.
#[test]
fn test_runtime_heap_limits_fail_after_allocating_entrypoint() {
    let mut runtime = TestRuntime::with_options_and_engine(
        &RuntimeOptions::default(),
        AllocatingEngine {
            heap_values: 512,
            raw_bytes: 0,
        },
    );

    // set one hard limit just above bootstrap usage so the entrypoint allocation trips it
    let baseline_usage = runtime.heap_usage();
    let max_managed_bytes = baseline_usage.heap.retained_bytes + 4 * 1024;
    let max_total_bytes = baseline_usage.retained_bytes() + 1024 * 1024;
    runtime.set_heap_limits(heap::HeapLimits {
        max_bytes: Some(max_total_bytes),
        heap: heap::HeapSpaceLimits {
            max_bytes: Some(max_managed_bytes),
        },
        raw: heap::RawLimits { max_bytes: None },
    });

    // one allocating step should trip the configured hard limit
    let error = runtime
        .run_entrypoint()
        .expect_err("allocating entrypoint should exceed the hard heap limit");
    let error = error.as_ref();

    assert!(matches!(
        error,
        crate::diagnostic::RuntimeError::HeapLimitExceeded { scope, .. } if scope == "managed"
    ));
}

/// Ensures new worlds start on one real root branch.
#[test]
fn test_world_starts_on_root_branch() {
    // create one new world
    let world = World::from_options(&RuntimeOptions::default()).expect("world test should build");

    // the active branch should be the root branch
    assert_eq!(world.branch_id(), BranchId::new(0));
    assert_eq!(world.branch().name, "root");
    assert!(world.branch().labels.is_empty());
    assert_eq!(world.branch_ids(), vec![BranchId::new(0)]);
}

/// Ensures empty worlds can checkpoint, rewind, and fork exactly.
#[test]
fn test_world_checkpoint_and_fork_empty_world() {
    // create one new world
    let mut world =
        World::from_options(&RuntimeOptions::default()).expect("world test should build");

    // capture one empty-world checkpoint
    let checkpoint_id = world
        .checkpoint("steady")
        .expect("checkpoint should succeed");
    let checkpoint = world
        .checkpoint_info(checkpoint_id)
        .expect("checkpoint metadata should exist");
    let checkpoint_revision = world
        .revision_state(checkpoint.revision)
        .expect("checkpoint revision should exist");
    assert_eq!(checkpoint_revision.branch_id, BranchId::new(0));
    assert_eq!(checkpoint.name, "steady");

    // rewind should restore the same empty state
    world
        .rewind(checkpoint_id)
        .expect("rewind should restore the checkpoint");

    // forking should create one child world on one child branch
    let child = Arc::new(
        world
            .fork(checkpoint_id, "child")
            .expect("fork should succeed"),
    );
    assert_eq!(child.branch().name, "child");
    assert_eq!(child.branch_ids(), vec![BranchId::new(0), BranchId::new(1)]);
    assert_eq!(world.branch_ids(), vec![BranchId::new(0), BranchId::new(1)]);
}

/// Ensures checkpoints restore VM-backed runtime and heap state exactly.
#[test]
fn test_world_rewind_restores_vm_runtime_state() {
    // build one vm-backed runtime
    let options = RuntimeOptions::default();
    let mut test = TestWorld::new();
    let runtime_id = test.spawn_vm_runtime(&options);

    // create one baseline heap allocation before the checkpoint
    test.allocate_vm_heap_allocation(runtime_id);
    assert_eq!(test.vm_heap_allocation_count(runtime_id), 1);

    // capture one checkpoint at the baseline state
    let checkpoint_id = test
        .world_mut()
        .checkpoint("vm-steady")
        .expect("checkpoint should succeed");

    // mutate both world topology and vm heap after the checkpoint
    test.world_mut()
        .spawn_runtime(Vec::new(), &options, TestWorld::vm_engine())
        .expect("second runtime should spawn in world");
    test.allocate_vm_heap_allocation(runtime_id);
    assert_eq!(test.world().runtime_ids().len(), 2);
    assert_eq!(test.vm_heap_allocation_count(runtime_id), 2);

    // rewind back to the captured point
    test.world_mut()
        .rewind(checkpoint_id)
        .expect("rewind should restore the checkpoint");

    // the world and vm heap should both return to the checkpoint state
    assert_eq!(test.world().runtime_ids(), vec![runtime_id]);
    assert_eq!(test.vm_heap_allocation_count(runtime_id), 1);
}

/// Ensures runtime root collection keeps task-held local references alive.
#[test]
fn test_runtime_collect_roots_preserves_task_resume_heap_reference() {
    let options = RuntimeOptions::default();
    let mut test = TestWorld::new();
    let runtime_id = test.spawn_runtime(&options, TestEngine::default());

    // install one local root only through queued scheduler state
    let (root, garbage) = {
        let runtime = test
            .world_mut()
            .runtime_mut(runtime_id)
            .expect("runtime should exist");
        let worker_id = runtime.primary_worker_id();
        let worker = runtime
            .worker_mut(worker_id)
            .expect("runtime should keep its primary worker");
        let reference_map = ReferenceMap::empty();
        let layout = byte_layout(1, &reference_map);

        let root = worker
            .heap
            .allocate(layout, heap::Payload::Bytes(&[0xA1]))
            .expect("heap allocation should succeed");
        let root = worker
            .heap
            .pin_heap(root)
            .expect("pinning should keep the expected root address fixed");
        let garbage = worker
            .heap
            .allocate(layout, heap::Payload::Bytes(&[0xB2]))
            .expect("heap allocation should succeed");

        worker.event_loop.enqueue_task(Task {
            id: TaskId::new(1),
            runnable: Continuation::Native(NativeContinuation::new(7)),
            resume_value: Value::HeapReference(root),
            status: TaskStatus::Ready,
            priority: 0,
        });

        (root, garbage)
    };

    // collect through the runtime root path, not an ad hoc visitor
    let roots = {
        let runtime = test
            .world_mut()
            .runtime_mut(runtime_id)
            .expect("runtime should exist");
        let mut roots = RootSet::new();

        runtime
            .visit_roots(&mut RootVisitor::All(&mut roots))
            .expect("root collection should succeed");

        roots
    };

    {
        let runtime = test
            .world_mut()
            .runtime_mut(runtime_id)
            .expect("runtime should exist");
        let worker_id = runtime.primary_worker_id();
        let worker = runtime
            .worker_mut(worker_id)
            .expect("runtime should keep its primary worker");
        let mut heap_roots = roots.heap().to_vec();

        worker
            .heap
            .collect_full(&mut heap_roots)
            .expect("local collection should succeed");

        assert!(worker.heap.is_heap_live(root));
        assert!(!worker.heap.is_heap_live(garbage));
    }
}

/// Ensures heap handles keep host-retained local references alive.
#[test]
fn test_runtime_heap_handle_roots_local_reference() {
    let options = RuntimeOptions::default();
    let mut test = TestWorld::new();
    let runtime_id = test.spawn_runtime(&options, TestEngine::default());
    let reference_map = ReferenceMap::empty();
    let layout = byte_layout(1, &reference_map);

    // retain one local reference through the worker handle table
    let (handle, garbage) = {
        let runtime = test
            .world_mut()
            .runtime_mut(runtime_id)
            .expect("runtime should exist");
        let worker_id = runtime.primary_worker_id();
        let worker = runtime
            .worker_mut(worker_id)
            .expect("runtime should keep its primary worker");
        let root = worker
            .heap
            .allocate(layout, heap::Payload::Bytes(&[0xE5]))
            .expect("heap allocation should succeed");
        let garbage = worker
            .heap
            .allocate(layout, heap::Payload::Bytes(&[0xF6]))
            .expect("heap allocation should succeed");
        let handle = worker.retain_heap_reference(root);

        (handle, garbage)
    };

    // collect through the handle root slots so moving GC can rewrite the handle
    let retained = {
        let runtime = test
            .world_mut()
            .runtime_mut(runtime_id)
            .expect("runtime should exist");
        let worker_id = runtime.primary_worker_id();
        let worker = runtime
            .worker_mut(worker_id)
            .expect("runtime should keep its primary worker");
        let handles = &mut worker.handles;
        let heap = &mut worker.heap;
        let mut roots = |visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>| {
            handles.visit_root_slots(visit)
        };

        heap.collect_full(&mut roots)
            .expect("local collection should succeed");
        let retained = handles
            .reference(handle)
            .expect("handle should still resolve");

        assert!(heap.is_heap_live(retained));
        assert!(!heap.is_heap_live(garbage));

        retained
    };

    // releasing the handle removes the final root
    {
        let runtime = test
            .world_mut()
            .runtime_mut(runtime_id)
            .expect("runtime should exist");
        let worker_id = runtime.primary_worker_id();
        let worker = runtime
            .worker_mut(worker_id)
            .expect("runtime should keep its primary worker");
        worker
            .release_heap_handle(handle)
            .expect("handle release should succeed");
        worker
            .heap
            .collect_full(&mut ())
            .expect("local collection should succeed");

        assert!(!worker.heap.is_heap_live(retained));
    }
}

/// Ensures runtime root collection keeps task-held shared references alive.
#[test]
fn test_runtime_collect_roots_preserves_task_resume_shared_reference() {
    let options = RuntimeOptions::default();
    let mut test = TestWorld::new();
    let runtime_id = test.spawn_runtime(&options, TestEngine::default());
    let reference_map = ReferenceMap::empty();
    let layout = byte_layout(1, &reference_map);
    let mut allocator = test
        .world()
        .runtime(runtime_id)
        .expect("runtime should exist")
        .shared
        .shared()
        .allocator();

    // install one shared root only through queued scheduler state
    let root = test
        .world()
        .runtime(runtime_id)
        .expect("runtime should exist")
        .shared
        .shared()
        .allocate(&mut allocator, layout, heap::Payload::Bytes(&[0xC3]))
        .expect("shared heap allocation should succeed");
    let garbage = test
        .world()
        .runtime(runtime_id)
        .expect("runtime should exist")
        .shared
        .shared()
        .allocate(&mut allocator, layout, heap::Payload::Bytes(&[0xD4]))
        .expect("shared heap allocation should succeed");

    {
        let runtime = test
            .world_mut()
            .runtime_mut(runtime_id)
            .expect("runtime should exist");
        let worker_id = runtime.primary_worker_id();
        let worker = runtime
            .worker_mut(worker_id)
            .expect("runtime should keep its primary worker");

        worker.event_loop.enqueue_task(Task {
            id: TaskId::new(2),
            runnable: Continuation::Native(NativeContinuation::new(8)),
            resume_value: Value::SharedHeapReference(root),
            status: TaskStatus::Ready,
            priority: 0,
        });
    }

    // collect through the same world-facing root path that shared gc uses
    let roots = {
        let runtime = test
            .world_mut()
            .runtime_mut(runtime_id)
            .expect("runtime should exist");
        let mut roots = Vec::new();

        runtime
            .visit_roots(&mut RootVisitor::SharedHeap(&mut roots))
            .expect("root collection should succeed");

        roots
    };

    test.world()
        .runtime(runtime_id)
        .expect("runtime should exist")
        .shared
        .shared()
        .collect_full(roots.iter().copied())
        .expect("shared collection should succeed");

    let runtime = test
        .world()
        .runtime(runtime_id)
        .expect("runtime should exist");
    assert!(runtime.shared.shared().is_heap_live(root));
    assert!(!runtime.shared.shared().is_heap_live(garbage));
}

/// Ensures workers spawned during shared marking join the active root-scan pass.
#[test]
fn test_spawned_worker_joins_active_shared_root_scan_pass() {
    let options = RuntimeOptions::default();
    let mut test = TestWorld::with_options(&options);
    let runtime_id = test.spawn_runtime(&options, TestEngine::default());
    let reference_map = ReferenceMap::empty();
    let layout = byte_layout(1, &reference_map);
    let mut allocator = test
        .world()
        .runtime(runtime_id)
        .expect("runtime should exist")
        .shared
        .shared()
        .allocator();

    // install shared roots for the active mark phase
    for index in 0..128 {
        let shared = test
            .world()
            .runtime(runtime_id)
            .expect("runtime should exist")
            .shared
            .shared()
            .allocate(&mut allocator, layout, heap::Payload::Bytes(&[index as u8]))
            .expect("shared heap allocation should succeed");
        let runtime = test
            .world_mut()
            .runtime_mut(runtime_id)
            .expect("runtime should exist");
        let worker_id = runtime.primary_worker_id();
        let worker = runtime
            .worker_mut(worker_id)
            .expect("runtime should keep its primary worker");

        worker
            .watch_host_event(
                HostEventKind::Lifecycle,
                Continuation::Native(NativeContinuation::new(100 + index)),
                Value::SharedHeapReference(shared),
                0,
            )
            .expect("host-event watch should register");
    }

    let existing_worker_id = test.primary_worker_id(runtime_id);
    let runtime = test
        .world()
        .runtime(runtime_id)
        .expect("runtime should exist");
    runtime.shared.shared().request_gc();
    runtime
        .shared
        .shared()
        .start_gc()
        .expect("shared gc start should succeed");
    runtime.shared.queue_shared_root_scan(existing_worker_id);
    runtime.shared.join_shared_edge_scan(existing_worker_id);

    assert_eq!(
        runtime.shared.shared().gc_phase(),
        heap::SharedGcPhase::Mark
    );

    let worker_id = test
        .world_mut()
        .spawn_worker(runtime_id, TestEngine::default())
        .expect("worker should spawn during shared marking");

    // the new worker must join the active pass immediately
    let runtime = test
        .world()
        .runtime(runtime_id)
        .expect("runtime should exist");
    assert!(runtime.shared.mark_roots().is_root_scan_pending(worker_id));

    let runtime = test
        .world()
        .runtime(runtime_id)
        .expect("runtime should exist");
    let worker = runtime.worker(worker_id).expect("worker should exist");

    assert!(worker.shared_edge_scan_idle());
}

/// Rewinds one sparse suspend revision from the nearest materialized image.
#[test]
fn test_world_rewind_sparse_suspend_revision_replays_suffix() {
    let options = record_options();
    let mut test = TestWorld::with_options(&options);

    let runtime_a = test.spawn_vm_runtime(&options);
    let checkpoint_id = test
        .world_mut()
        .checkpoint("baseline")
        .expect("checkpoint should succeed");

    let runtime_b = test.spawn_vm_runtime(&options);
    let suspended_revision_id = test.world_mut().suspend().expect("suspend should succeed");

    let sparse_error = test
        .world()
        .revision_data(suspended_revision_id)
        .expect_err("suspended revision should not keep one exact image");
    assert!(
        sparse_error.to_string().contains("missing image"),
        "unexpected sparse revision error: {sparse_error}"
    );

    test.world_mut()
        .spawn_runtime(Vec::new(), &options, TestWorld::vm_engine())
        .expect("third runtime should spawn in world");

    test.world_mut()
        .rewind_revision(suspended_revision_id)
        .expect("rewind should replay from the baseline image");

    assert_eq!(test.world().runtime_ids(), vec![runtime_a, runtime_b]);

    let checkpoint = test
        .world()
        .checkpoint_info(checkpoint_id)
        .expect("checkpoint metadata should exist");
    assert_eq!(checkpoint.name, "baseline");
}

/// Forks one sparse suspend revision from the nearest materialized image.
#[test]
fn test_world_fork_sparse_suspend_revision_replays_suffix() {
    let options = record_options();
    let mut test = TestWorld::with_options(&options);

    let runtime_a = test.spawn_vm_runtime(&options);
    let _baseline = test
        .world_mut()
        .checkpoint("baseline")
        .expect("checkpoint should succeed");

    let runtime_b = test.spawn_vm_runtime(&options);
    let suspended_revision_id = test.world_mut().suspend().expect("suspend should succeed");

    let child = test
        .world_mut()
        .fork_revision(suspended_revision_id, "child")
        .expect("fork should replay from the baseline image");

    assert_eq!(test.world().runtime_ids(), vec![runtime_a, runtime_b]);
    assert_eq!(child.runtime_ids(), vec![runtime_a, runtime_b]);
}

/// Snapshots one sparse suspend revision by materializing its image on demand.
#[test]
fn test_world_snapshot_sparse_suspend_revision_materializes_on_demand() {
    let options = record_options();
    let mut test = TestWorld::with_options(&options);

    let runtime_a = test.spawn_vm_runtime(&options);
    let _baseline = test
        .world_mut()
        .checkpoint("baseline")
        .expect("checkpoint should succeed");

    let runtime_b = test.spawn_vm_runtime(&options);
    let suspended_revision_id = test.world_mut().suspend().expect("suspend should succeed");

    let snapshot = test
        .world()
        .snapshot_revision(suspended_revision_id)
        .expect("snapshot should materialize the sparse revision image");
    let image = snapshot.image().expect("snapshot should include one image");

    assert_eq!(snapshot.revision, suspended_revision_id);
    assert!(image.runtimes.contains_key(&runtime_a));
    assert!(image.runtimes.contains_key(&runtime_b));
}

/// Ensures exact snapshots prune later retained lineage while lineage snapshots keep it.
#[test]
fn test_world_snapshot_revision_prunes_later_lineage() {
    // build one world with one baseline checkpoint and one later revision
    let options = RuntimeOptions::default();
    let mut test = TestWorld::new();
    let runtime_a = test.spawn_vm_runtime(&options);
    let baseline_checkpoint_id = test
        .world_mut()
        .checkpoint("baseline")
        .expect("baseline checkpoint should succeed");
    let baseline_checkpoint = test
        .world()
        .checkpoint_info(baseline_checkpoint_id)
        .expect("baseline checkpoint metadata should exist");

    let _runtime_b = test.spawn_vm_runtime(&options);
    let later_checkpoint_id = test
        .world_mut()
        .checkpoint("later")
        .expect("later checkpoint should succeed");

    // capture one exact snapshot and one lineage snapshot for the baseline revision
    let exact_snapshot = test
        .world()
        .snapshot_revision(baseline_checkpoint.revision)
        .expect("exact snapshot should succeed");
    let lineage_snapshot = test
        .world()
        .snapshot_lineage_revision(baseline_checkpoint.revision)
        .expect("lineage snapshot should succeed");

    // the exact snapshot should only keep the selected revision closure
    assert_eq!(exact_snapshot.revision, baseline_checkpoint.revision);
    assert_eq!(exact_snapshot.lineage.branches.len(), 1);
    assert_eq!(exact_snapshot.lineage.revisions.len(), 1);
    assert_eq!(exact_snapshot.lineage.images.len(), 1);
    assert_eq!(exact_snapshot.lineage.trace_images.len(), 1);
    assert_eq!(exact_snapshot.lineage.checkpoints.len(), 1);
    assert!(
        exact_snapshot
            .lineage
            .checkpoints
            .contains_key(&baseline_checkpoint_id)
    );
    assert!(
        !exact_snapshot
            .lineage
            .checkpoints
            .contains_key(&later_checkpoint_id)
    );

    // the lineage snapshot should still retain the later world history
    assert_eq!(lineage_snapshot.revision, baseline_checkpoint.revision);
    assert!(lineage_snapshot.lineage.revisions.len() >= 2);
    assert!(
        lineage_snapshot
            .lineage
            .checkpoints
            .contains_key(&baseline_checkpoint_id)
    );
    assert!(
        lineage_snapshot
            .lineage
            .checkpoints
            .contains_key(&later_checkpoint_id)
    );

    // restoring the exact snapshot should rebuild only the baseline closure
    let restored_world =
        World::from_snapshot(&exact_snapshot, None).expect("exact snapshot should restore");
    assert_eq!(restored_world.runtime_ids(), vec![runtime_a]);
    assert_eq!(
        restored_world.checkpoint_ids(),
        vec![baseline_checkpoint_id]
    );
}

/// Ensures attached resources remain an explicit checkpoint barrier.
#[test]
fn test_world_checkpoint_rejects_attached_resources() {
    // build one runtime and attach one resource to its primary worker
    let options = RuntimeOptions::default();
    let mut test = TestWorld::new();
    let runtime_id = test.spawn_vm_runtime(&options);

    let world_ref = test.world_mut().world_ref();
    {
        let runtime = test
            .world_mut()
            .runtime_mut(runtime_id)
            .expect("runtime should exist");
        let worker_id = runtime.primary_worker_id();
        let worker = runtime
            .worker_mut(worker_id)
            .expect("runtime should keep its primary worker");
        let _ = worker
            .resources
            .insert(&world_ref, ResourceEntry::new(ResourceKind::Timer), None);
    }

    // checkpointing should fail loudly for resources without capture support
    let error = test
        .world_mut()
        .checkpoint("blocked")
        .expect_err("checkpoint should fail");
    let message = error.to_string();
    assert!(
        message.contains("does not support capture"),
        "unexpected checkpoint error: {message}"
    );
}

/// Ensures explicit observations stay separate from causal trace.
#[test]
fn test_world_observe_records_control_and_resource_events() {
    let options = RuntimeOptions::default();
    let mut test = TestWorld::new();
    let runtime_id = test.spawn_vm_runtime(&options);

    let _ = test.world_mut().observe(Observation::world_annotations(
        ObservationCategory::Diagnostic,
        "runtime.changed",
        [("operation", "set_policy")],
    ));

    let world_ref = test.world_mut().world_ref();
    {
        let runtime = test
            .world_mut()
            .runtime_mut(runtime_id)
            .expect("runtime should exist");
        let worker_id = runtime.primary_worker_id();
        let worker = runtime
            .worker_mut(worker_id)
            .expect("runtime should keep its primary worker");
        let resource_id =
            worker
                .resources
                .insert(&world_ref, ResourceEntry::new(ResourceKind::Timer), None);
        let _ = worker.resources.remove(&world_ref, resource_id, None);
    }

    let records = test.world().observations().records_after(None);
    assert!(
        records
            .iter()
            .any(|record| record.observation.name == "runtime.changed"),
        "expected one explicit diagnostic observation event"
    );
    assert!(
        records
            .iter()
            .any(|record| record.observation.name == "resource.attached"),
        "expected one resource attach observation event"
    );
    assert!(
        records
            .iter()
            .any(|record| record.observation.name == "resource.detached"),
        "expected one resource detach observation event"
    );
    assert!(
        records
            .iter()
            .any(|record| record.observation.category == ObservationCategory::Diagnostic),
        "expected one diagnostic observation kind"
    );
    assert!(
        records
            .iter()
            .filter(|record| record.observation.category == ObservationCategory::Resource)
            .count()
            >= 2,
        "expected resource observation kinds for resource lifecycle"
    );
}

/// Ensures observation subscriptions start live and apply event-kind filters.
#[test]
fn test_world_observe_subscriptions_filter_live_events() {
    let options = RuntimeOptions::default();
    let mut test = TestWorld::new();
    let runtime_id = test.spawn_vm_runtime(&options);

    let subscription = test.world().observations().open(ObservationOptions {
        runtime: false,
        topology: false,
        resource: true,
        scheduler: false,
        diagnostic: false,
        telemetry: false,
        domain: false,
    });

    // emit one filtered-out diagnostic observation
    let policy = test.world().policy();
    test.world_mut()
        .set_policy(policy)
        .expect("policy replacement should succeed");

    // emit resource lifecycle observations after the subscription opened
    let world_ref = test.world_mut().world_ref();
    {
        let runtime = test
            .world_mut()
            .runtime_mut(runtime_id)
            .expect("runtime should exist");
        let worker_id = runtime.primary_worker_id();
        let worker = runtime
            .worker_mut(worker_id)
            .expect("runtime should keep its primary worker");
        let resource_id =
            worker
                .resources
                .insert(&world_ref, ResourceEntry::new(ResourceKind::Timer), None);
        let _ = worker.resources.remove(&world_ref, resource_id, None);
    }

    let records = test
        .world()
        .observations()
        .next(subscription, 16)
        .expect("observe subscription should read");
    assert_eq!(records.len(), 2, "expected attach and detach observations");
    assert!(
        records
            .iter()
            .all(|record| record.observation.category == ObservationCategory::Resource),
        "expected only resource observations through the filter"
    );

    let records = test
        .world()
        .observations()
        .next(subscription, 16)
        .expect("observe subscription should advance");
    assert!(records.is_empty(), "subscription cursor should advance");

    test.world()
        .observations()
        .close(subscription)
        .expect("observe subscription should close");
}

/// Ensures scheduler observation subscriptions see virtual time advances.
#[test]
fn test_world_observe_subscriptions_report_scheduler_progress() {
    let mut options = RuntimeOptions::default();
    options.set_time_mode(TimeMode::Virtual);

    let mut world = World::from_options(&options).expect("world should construct");
    let subscription = world.observations().open(ObservationOptions {
        runtime: false,
        topology: false,
        resource: false,
        scheduler: true,
        diagnostic: false,
        telemetry: false,
        domain: false,
    });

    // schedule one simulated deadline so the world must advance time
    world
        .simulation_mut()
        .schedule_event(WorldInstant::new(5_000));

    let outcome = world.tick().expect("world tick should succeed");
    assert_eq!(outcome, crate::runtime::TickOutcome::AdvancedTime);

    let records = world
        .observations()
        .next(subscription, 16)
        .expect("observe subscription should read");
    assert_eq!(records.len(), 1, "expected one scheduler observation");
    assert!(
        records.first().is_some_and(|record| {
            record.observation.name == "scheduler.advanced_time"
                && record.observation.annotation("deadline_ns") == Some("5000")
        }),
        "expected one advanced-time scheduler observation"
    );
}

/// Ensures forked worlds restore from the checkpoint and diverge independently.
#[test]
fn test_world_fork_isolates_vm_runtime_state() {
    // build one vm-backed runtime and capture a baseline checkpoint
    let options = RuntimeOptions::default();
    let mut test = TestWorld::new();
    let runtime_id = test.spawn_vm_runtime(&options);

    test.allocate_vm_heap_allocation(runtime_id);
    let checkpoint_id = test
        .world_mut()
        .checkpoint("baseline")
        .expect("checkpoint should succeed");

    // fork one child world from that baseline
    let child = test
        .world_mut()
        .fork(checkpoint_id, "child")
        .expect("fork should succeed");
    let mut child_test = TestWorld::from_world(child);

    // parent and child should start from the same captured heap state
    assert_eq!(test.vm_heap_allocation_count(runtime_id), 1);
    assert_eq!(child_test.vm_heap_allocation_count(runtime_id), 1);

    // mutate parent and child independently after the fork
    test.allocate_vm_heap_allocation(runtime_id);
    test.allocate_vm_heap_allocation(runtime_id);
    child_test.allocate_vm_heap_allocation(runtime_id);

    // both worlds should diverge without affecting each other
    assert_eq!(test.vm_heap_allocation_count(runtime_id), 3);
    assert_eq!(child_test.vm_heap_allocation_count(runtime_id), 2);
}

/// Ensures forked worlds preserve heap leaves and invalidate external raw pointers.
#[test]
fn test_world_fork_shares_heap_leaves_before_mutation() {
    let options = RuntimeOptions::default();
    let mut test = TestWorld::new();
    let runtime_id = test.spawn_vm_runtime(&options);

    let managed = test.allocate_vm_heap_allocation(runtime_id);
    let raw = test.allocate_vm_raw_bytes(runtime_id, &[1, 2, 3]);
    let checkpoint_id = test
        .world_mut()
        .checkpoint("shared-heap")
        .expect("checkpoint should succeed");

    let child = test
        .world_mut()
        .fork(checkpoint_id, "child")
        .expect("fork should succeed");

    let parent_heap = test.runtime_heap_image(runtime_id);
    let mut child_test = TestWorld::from_world(child);
    let child_heap = child_test.runtime_heap_image(runtime_id);

    // managed heap storage still shares the captured leaves
    assert!(parent_heap.shares_heap_allocation_with(&child_heap, managed));

    // the original raw pointer stays valid only in the original world
    assert_eq!(test.read_vm_raw_bytes(runtime_id, raw), vec![1, 2, 3]);
    assert_eq!(
        child_test.read_vm_raw_bytes_result(runtime_id, raw),
        Err(heap::HeapError::InvalidRawPointer { pointer: raw })
    );
}

/// Ensures child worlds reject external raw pointers from the parent world.
#[test]
fn test_world_fork_invalidates_external_raw_pointers() {
    let options = RuntimeOptions::default();
    let mut test = TestWorld::new();
    let runtime_id = test.spawn_vm_runtime(&options);

    let first = test.allocate_vm_raw_bytes(runtime_id, &[1, 2, 3]);
    let _second = test.allocate_vm_raw_bytes(runtime_id, &[4, 5, 6]);
    let checkpoint_id = test
        .world_mut()
        .checkpoint("shared-raw")
        .expect("checkpoint should succeed");
    let child = test
        .world_mut()
        .fork(checkpoint_id, "child")
        .expect("fork should succeed");

    let mut child_test = TestWorld::from_world(child);
    // the child does not inherit the parent's external raw pointer identity
    assert_eq!(
        child_test.read_vm_raw_bytes_result(runtime_id, first),
        Err(heap::HeapError::InvalidRawPointer { pointer: first })
    );
    assert_eq!(
        child_test.mutate_vm_raw_byte_result(runtime_id, first, 1, 9),
        Err(heap::HeapError::InvalidRawPointer { pointer: first })
    );

    // the parent allocation remains valid
    assert_eq!(test.read_vm_raw_bytes(runtime_id, first), vec![1, 2, 3]);
}

/// Ensures rewind restores live heaps from the checkpoint image leaves.
#[test]
fn test_world_rewind_restores_checkpoint_heap_leaves() {
    let options = RuntimeOptions::default();
    let mut test = TestWorld::new();
    let runtime_id = test.spawn_vm_runtime(&options);

    let managed = test.allocate_vm_heap_allocation(runtime_id);
    let raw = test.allocate_vm_raw_bytes(runtime_id, &[0xCA, 0xFE, 0xBA, 0xBE]);
    let checkpoint_id = test
        .world_mut()
        .checkpoint("rewind-shared")
        .expect("checkpoint should succeed");
    let checkpoint = test
        .world()
        .checkpoint_info(checkpoint_id)
        .expect("checkpoint metadata should exist");
    let revision = test
        .world()
        .revision_state(checkpoint.revision)
        .expect("checkpoint revision should exist");
    let image = test
        .world()
        .image_info(revision.image_id)
        .expect("checkpoint image should exist");
    let worker_id = test.primary_worker_id(runtime_id);

    test.allocate_vm_heap_allocation(runtime_id);
    test.mutate_vm_raw_byte(runtime_id, raw, 0, 0xFF);

    test.world_mut()
        .rewind(checkpoint_id)
        .expect("rewind should restore the checkpoint");

    let restored_heap = test.runtime_heap_image(runtime_id);
    let stored_heap = heap::HeapImage::from_snapshot(
        &image
            .worker(worker_id)
            .expect("worker image should exist")
            .heap,
    )
    .expect("stored worker heap snapshot should restore");

    // managed heap storage still comes back from the checkpoint leaves
    assert!(restored_heap.shares_heap_allocation_with(&stored_heap, managed));

    // the pre-rewind raw pointer is stale after restore
    assert_eq!(
        test.read_vm_raw_bytes_result(runtime_id, raw),
        Err(heap::HeapError::InvalidRawPointer { pointer: raw })
    );
}

/// Ensures one committed branch moment can restore intermediate state from trace.
#[test]
fn test_world_restore_moment_replays_to_intermediate_sequence() {
    let options = record_options();
    let mut test = TestWorld::with_options(&options);

    test.record_world_entity_kind("a");
    let _checkpoint_a = test
        .world_mut()
        .checkpoint("moment-a")
        .expect("checkpoint should succeed");

    test.record_world_entity_kind("b");
    let moment = test.world().moment();

    test.record_world_entity_kind("c");
    let _checkpoint_b = test
        .world_mut()
        .checkpoint("moment-b")
        .expect("checkpoint should succeed");

    let entity_kinds = test.world().entity_kinds();
    assert!(entity_kinds.contains_key("app.record.shared.a"));
    assert!(entity_kinds.contains_key("app.record.shared.b"));
    assert!(entity_kinds.contains_key("app.record.shared.c"));

    // the live branch trace should replay all three topology commands directly
    let replay_trace = Trace::from_log(ExecutionMode::Replay, test.world().trace().log().clone());
    replay_trace
        .seek_sequence(TraceSequence::new(0))
        .expect("replay trace should seek to the root boundary");
    let _ = replay_trace
        .next_command()
        .expect("replay trace should include command a");
    let _ = replay_trace
        .next_command()
        .expect("replay trace should include command b");
    let _ = replay_trace
        .next_command()
        .expect("replay trace should include command c");

    test.world_mut()
        .restore_moment(moment)
        .expect("moment restore should replay to the target sequence");

    let entity_kinds = test.world().entity_kinds();
    assert!(entity_kinds.contains_key("app.record.shared.a"));
    assert!(entity_kinds.contains_key("app.record.shared.b"));
    assert!(!entity_kinds.contains_key("app.record.shared.c"));
}

/// Ensures world event queries project both trace and observation data at moments.
#[test]
fn test_world_events_between_projects_trace_and_observation() {
    let options = record_options();
    let mut test = TestWorld::with_options(&options);

    let start = test.world().moment();
    test.record_world_entity_kind("event");
    let _ = test.world_mut().observe(Observation::world_annotations(
        ObservationCategory::Domain,
        "entity.kind.recorded",
        [("kind", "event")],
    ));
    let end = test.world().moment();

    let events = test
        .world()
        .events()
        .between(start, end)
        .expect("event query should succeed");
    let events = events.as_slice();

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].moment, end);
    assert!(events[0].is_input());
    assert_eq!(events[0].name(), Some("topology.define_entity_kind"));
    assert_eq!(events[1].moment, end);
    assert!(events[1].is_observation());

    let Some(Command::DefineEntityKind { .. }) = events[0].input() else {
        panic!("first projected event should be one world topology command");
    };
    let Some(observation) = events[1].observation() else {
        panic!("second projected event should be one explicit observation");
    };
    assert_eq!(observation.name, "entity.kind.recorded");

    let trace_events = test
        .world()
        .events()
        .between(start, end)
        .expect("event query should succeed")
        .inputs()
        .name("topology.define_entity_kind");

    assert_eq!(trace_events.len(), 1);
}

/// Ensures event selectors match structured observation names, scopes, and labels.
#[test]
fn test_world_events_between_filter_structured_observations() {
    let options = record_options();
    let mut test = TestWorld::with_options(&options);
    let runtime_id = test.spawn_vm_runtime(&options);

    let before = test.world().moment();
    let _ = test.world_mut().observe(
        Observation::runtime_annotations(
            ObservationCategory::Domain,
            runtime_id,
            "ui.click",
            [("message", "save button clicked")],
        )
        .label("screen", "checkout")
        .label("element", "save-button"),
    );
    let moment = test.world().moment();
    assert_eq!(before, moment);

    let events = test
        .world()
        .events()
        .up_to(moment)
        .expect("event query should succeed");

    let events = events
        .observations()
        .name("ui.click")
        .category(ObservationCategory::Domain)
        .runtime(runtime_id)
        .label("screen", "checkout");

    assert_eq!(events.len(), 1);
    assert_eq!(
        events.first().and_then(|event| event.name()),
        Some("ui.click")
    );
}

/// Ensures world transition queries derive adjacent state transitions from trace.
#[test]
fn test_world_transitions_between_project_trace_steps() {
    let options = record_options();
    let mut test = TestWorld::with_options(&options);

    let start = test.world().moment();
    test.record_world_entity_kind("transition");
    let end = test.world().moment();

    let transitions = test
        .world()
        .transitions()
        .between(start, end)
        .expect("transition query should succeed");
    let transitions = transitions.as_slice();

    assert_eq!(transitions.len(), 1);
    assert_eq!(transitions[0].before, start);
    assert_eq!(transitions[0].after, end);
    assert!(transitions[0].is_input());

    let TraceRecord::Command(Command::DefineEntityKind { .. }) = test
        .world()
        .transition_record(&transitions[0])
        .expect("transition cause should resolve")
    else {
        panic!("transition cause should be one world topology command");
    };
}

/// Ensures lineage event queries surface committed observations across branches.
#[test]
fn test_lineage_events_on_project_committed_trace_and_observation() {
    let options = record_options();
    let mut test = TestWorld::with_options(&options);

    test.record_world_entity_kind("lineage");
    let _ = test.world_mut().observe(Observation::world_annotations(
        ObservationCategory::Domain,
        "entity.kind.recorded",
        [("kind", "lineage")],
    ));
    let _ = test.suspend();

    let branch = test.branch();
    let events = test
        .world()
        .lineage()
        .events()
        .branch(branch.id)
        .expect("lineage event query should succeed");
    let events = events.as_slice();

    assert_eq!(events.len(), 2);
    assert!(events[0].is_input());
    assert_eq!(events[0].name(), Some("topology.define_entity_kind"));
    assert!(events[1].is_observation());

    let Some(Command::DefineEntityKind { .. }) = events[0].input() else {
        panic!("first lineage event should be one committed world topology command");
    };
    let Some(observation) = events[1].observation() else {
        panic!("second lineage event should be one committed observation");
    };
    assert_eq!(observation.name, "entity.kind.recorded");
}

/// Ensures committed event selectors match structured observation metadata.
#[test]
fn test_lineage_events_on_filter_structured_observations() {
    let options = record_options();
    let mut test = TestWorld::with_options(&options);
    let runtime_id = test.spawn_vm_runtime(&options);

    let _ = test.world_mut().observe(
        Observation::runtime_annotations(
            ObservationCategory::Domain,
            runtime_id,
            "db.role.changed",
            [("message", "storage promoted to log")],
        )
        .label("cluster", "main"),
    );
    let _ = test.suspend();

    let events = test
        .world()
        .lineage()
        .events()
        .branch(test.branch().id)
        .expect("lineage event query should succeed")
        .observations()
        .name("db.role.changed")
        .category(ObservationCategory::Domain)
        .runtime(runtime_id)
        .label("cluster", "main");

    assert_eq!(events.len(), 1);
}

/// Ensures lineage queries only see committed child-branch history.
#[test]
fn test_lineage_events_on_child_branch_exclude_live_tail_until_commit() {
    let options = record_options();
    let mut parent = TestWorld::with_options(&options);
    let checkpoint_id = parent.checkpoint("fork-base");
    let mut child = parent.fork(checkpoint_id, "child");
    let child_branch = child.branch();

    child.record_world_entity_kind("child-live");
    let _ = child.world_mut().observe(Observation::world_annotations(
        ObservationCategory::Domain,
        "entity.kind.recorded",
        [("kind", "child-live")],
    ));

    let committed_events = child
        .world()
        .lineage()
        .events_on(child_branch.id)
        .expect("committed lineage query should succeed");
    assert!(
        committed_events
            .as_slice()
            .iter()
            .all(|event| event.moment.sequence.get() == 0),
        "child committed lineage should still end at the fork point"
    );

    let live_end = child.world().moment();
    let live_events = child
        .world()
        .events_between(
            child
                .world()
                .lineage()
                .branch_head_moment(child_branch.id)
                .expect("child branch head moment should exist"),
            live_end,
        )
        .expect("live branch query should include uncommitted tail");
    assert!(
        live_events.as_slice().iter().any(|event| event
            .observation()
            .is_some_and(|observation| observation.name == "entity.kind.recorded")),
        "live branch query should include the uncommitted observation"
    );

    let _ = child.suspend();

    let committed_events = child
        .world()
        .lineage()
        .events_on(child_branch.id)
        .expect("committed lineage query should succeed after suspend");
    assert!(
        committed_events.as_slice().iter().any(|event| event
            .observation()
            .is_some_and(|observation| observation.name == "entity.kind.recorded")),
        "committed lineage should include the child observation after commit"
    );
}

/// Ensures lineage transition queries derive committed branch-local steps.
#[test]
fn test_lineage_transitions_on_project_committed_steps() {
    let options = record_options();
    let mut test = TestWorld::with_options(&options);

    test.record_world_entity_kind("transition-lineage");
    let _ = test.suspend();

    let transitions = test
        .world()
        .lineage()
        .transitions()
        .branch(test.branch().id)
        .expect("lineage transition query should succeed");
    let transitions = transitions.as_slice();

    assert_eq!(transitions.len(), 1);
    assert!(transitions[0].is_input());
    let TraceRecord::Command(Command::DefineEntityKind { .. }) = test
        .world()
        .lineage()
        .transition_record(&transitions[0])
        .expect("committed transition cause should resolve")
    else {
        panic!("committed lineage transition should be caused by one world topology command");
    };
}

/// Ensures lineage descendant queries include forked child branches.
#[test]
fn test_lineage_descendants_of_includes_child_branch() {
    let options = record_options();
    let mut parent = TestWorld::with_options(&options);
    let checkpoint_id = parent.checkpoint("fork-base");
    let child = parent.fork(checkpoint_id, "child");

    let descendants = parent
        .world()
        .lineage()
        .descendants_of(parent.branch().id)
        .expect("descendant query should succeed");

    assert!(
        descendants
            .as_slice()
            .iter()
            .any(|branch| branch.id == parent.branch().id),
        "descendants should include the ancestor branch itself"
    );
    assert!(
        descendants
            .as_slice()
            .iter()
            .any(|branch| branch.id == child.branch().id),
        "descendants should include the child branch"
    );
}

/// Ensures lineage descendant event queries aggregate committed child history.
#[test]
fn test_lineage_events_on_descendants_of_include_child_history() {
    let options = record_options();
    let mut parent = TestWorld::with_options(&options);
    parent.record_world_entity_kind("parent");
    let _ = parent.suspend();

    let checkpoint_id = parent.checkpoint("fork-base");
    let mut child = parent.fork(checkpoint_id, "child");
    child.record_world_entity_kind("child");
    let _ = child.suspend();

    let events = parent
        .world()
        .lineage()
        .events_descendants_of(parent.branch().id)
        .expect("descendant event query should succeed");

    let input_count = events
        .inputs()
        .filter(|event| matches!(event.input(), Some(Command::DefineEntityKind { .. })))
        .len();

    assert!(
        input_count >= 2,
        "descendant event query should include committed parent and child topology commands"
    );
}

/// Ensures lineage divergence queries report the fork point between parent and child.
#[test]
fn test_lineage_divergence_moment_reports_fork_point() {
    let options = record_options();
    let mut parent = TestWorld::with_options(&options);
    parent.record_world_entity_kind("parent");
    let fork_revision_id = parent.suspend();
    let fork_moment = parent
        .world()
        .revision_moment(fork_revision_id)
        .expect("fork revision should resolve to one moment");

    let checkpoint_id = parent.checkpoint("fork-base");
    let child = parent.fork(checkpoint_id, "child");

    let divergence = parent
        .world()
        .lineage()
        .divergence(parent.branch().id, child.branch().id)
        .expect("divergence query should succeed");

    assert_eq!(divergence.base.sequence, fork_moment.sequence);
}

/// Ensures lineage views expose exact committed world state at one moment.
#[test]
fn test_lineage_view_materializes_committed_state_at_moment() {
    let options = record_options();
    let mut test = TestWorld::with_options(&options);

    // one mutation before the target moment
    test.record_world_entity_kind("before-target");
    let revision_id = test.suspend();
    let moment = test
        .world()
        .revision_moment(revision_id)
        .expect("revision should resolve to one committed moment");

    // one later mutation after the target moment
    test.record_world_entity_kind("after-target");
    let _ = test.suspend();

    let view = test
        .world()
        .lineage()
        .view(moment)
        .expect("lineage view should materialize one committed image");

    assert!(
        view.entity_kind("app.record.shared.before-target")
            .is_some()
    );
    assert!(view.entity_kind("app.record.shared.after-target").is_none());
}

/// Ensures lineage views expose direct policy, runtime, and count accessors.
#[test]
fn test_lineage_view_exposes_policy_runtime_and_count_accessors() {
    let options = record_options();
    let mut test = TestWorld::with_options(&options);
    let runtime_id = test.spawn_vm_runtime(&options);

    let revision_id = test.suspend();
    let moment = test
        .world()
        .revision_moment(revision_id)
        .expect("revision should resolve to one committed moment");

    let view = test
        .world()
        .lineage()
        .view(moment)
        .expect("lineage view should materialize one committed image");
    // policy and runtime access
    assert_eq!(view.policy(), &test.world().policy());
    assert_eq!(view.runtime_count(), 1);
    assert_eq!(view.runtimes().len(), 1);
    assert!(view.has_runtime(runtime_id));
    assert!(view.runtime(runtime_id).is_ok());

    // empty collections and negative lookups
    assert_eq!(view.resource_count(), 0);
    assert_eq!(view.resources().len(), 0);
    assert!(!view.has_resource(WorldResourceId::new(WorkerId(u64::MAX), ResourceId(999))));
    assert!(!view.has_entity("missing.entity"));
    assert!(!view.has_edge("missing.edge"));
}

/// Ensures forked worlds share the same immutable trace head before divergence.
#[test]
fn test_world_fork_shares_trace_head_before_mutation() {
    let mut options = RuntimeOptions::default();
    options.set_execution_mode(ExecutionMode::Record);
    let mut test = TestWorld::with_options(&options);

    test.record_world_entity_kind("baseline");
    let checkpoint_id = test
        .world_mut()
        .checkpoint("trace-shared")
        .expect("checkpoint should succeed");
    let child = test
        .world_mut()
        .fork(checkpoint_id, "child")
        .expect("fork should succeed");

    let parent_trace = test.world().trace().capture_image();
    let child_trace = child.trace().capture_image();

    assert!(parent_trace.shares_log_head_with(&child_trace));
    assert_eq!(
        parent_trace.next_sequence.get(),
        child_trace.next_sequence.get()
    );
}

/// Ensures child trace mutation extends the shared fork prefix instead of replacing it.
#[test]
fn test_world_fork_child_trace_extends_shared_prefix() {
    let mut options = RuntimeOptions::default();
    options.set_execution_mode(ExecutionMode::Record);
    let mut test = TestWorld::with_options(&options);

    test.record_world_entity_kind("baseline");
    let checkpoint_id = test
        .world_mut()
        .checkpoint("trace-prefix")
        .expect("checkpoint should succeed");
    let child = test
        .world_mut()
        .fork(checkpoint_id, "child")
        .expect("fork should succeed");
    let mut child_test = TestWorld::from_world(child);

    let parent_trace = test.world().trace().capture_image();
    child_test.record_world_entity_kind("child-extra");
    let child_trace = child_test.world().trace().capture_image();

    assert!(child_trace.extends_log_head_of(&parent_trace));
    assert!(!child_trace.shares_log_head_with(&parent_trace));
}

/// Ensures serialized world snapshots preserve lineage metadata and restore state.
#[test]
fn test_world_snapshot_roundtrip_restores_lineage_and_state() {
    // build one vm-backed runtime and capture one checkpoint
    let options = RuntimeOptions::default();
    let mut test = TestWorld::new();
    let runtime_id = test.spawn_vm_runtime(&options);

    test.allocate_vm_heap_allocation(runtime_id);
    let checkpoint_id = test
        .world_mut()
        .checkpoint("baseline")
        .expect("checkpoint should succeed");
    let checkpoint = test
        .world()
        .checkpoint_info(checkpoint_id)
        .expect("checkpoint metadata should exist");
    let revision = test
        .world()
        .revision_state(checkpoint.revision)
        .expect("checkpoint revision should exist");
    let image_id = revision.image_id;

    // encode one serialized snapshot from that checkpoint image
    let snapshot = test
        .world()
        .snapshot(image_id)
        .expect("snapshot should build");
    let bytes = snapshot.encode().expect("snapshot should encode");
    let snapshot =
        crate::runtime::world::WorldSnapshot::decode(&bytes).expect("snapshot should decode");

    // mutate the world after the snapshot
    test.allocate_vm_heap_allocation(runtime_id);
    test.world_mut()
        .spawn_runtime(Vec::new(), &options, TestWorld::vm_engine())
        .expect("second runtime should spawn in world");
    assert_eq!(test.world().runtime_ids().len(), 2);
    assert_eq!(test.vm_heap_allocation_count(runtime_id), 2);

    // restore from the serialized snapshot
    test.world_mut()
        .restore_snapshot(&snapshot, None)
        .expect("snapshot should restore");

    // the snapshot should restore both runtime state and lineage metadata
    assert_eq!(test.world().runtime_ids(), vec![runtime_id]);
    assert_eq!(test.vm_heap_allocation_count(runtime_id), 1);
    assert_eq!(test.world().checkpoint_ids(), vec![checkpoint_id]);
    assert_eq!(test.world().revision(), checkpoint.revision);

    // rebuilding one fresh world from the same snapshot should preserve the same lineage
    let mut restored_test = TestWorld::from_world(
        World::from_snapshot(&snapshot, None).expect("snapshot should rebuild world"),
    );
    assert_eq!(restored_test.world().branch_id(), test.world().branch_id());
    assert_eq!(restored_test.world().runtime_ids(), vec![runtime_id]);
    assert_eq!(restored_test.vm_heap_allocation_count(runtime_id), 1);
    assert_eq!(restored_test.world().checkpoint_ids(), vec![checkpoint_id]);
    assert_eq!(restored_test.world().revision(), checkpoint.revision);
}

/// Ensures live revision forks preserve non-quiescent scheduler ingress.
#[test]
fn test_world_fork_preserves_pending_scheduler_ingress() {
    // build one runtime with one queued poller event before the committed revision
    let options = RuntimeOptions::default();
    let mut test = TestWorld::new();
    let runtime_id = test.spawn_vm_runtime(&options);
    let runtime = test
        .world_mut()
        .runtime_mut(runtime_id)
        .expect("runtime should exist");
    let worker_id = runtime.primary_worker_id();
    let worker = runtime
        .worker_mut(worker_id)
        .expect("primary worker should exist");
    worker.event_loop.enqueue_events(vec![PollerEvent {
        resource_id: ResourceId(91),
        source: PollerEventSource::Io,
        mask: PollerEventMask::READABLE,
        flags: PollerEventFlags::NONE,
        token: PollerToken(812),
        payload: PollerEventPayload::Io { data: 7 },
    }]);

    let revision = test.world_mut().suspend().expect("suspend should succeed");

    // forking that exact head revision should preserve the queued ingress in both worlds
    let mut child = test
        .world_mut()
        .fork_revision(revision, "child")
        .expect("fork should succeed");

    let parent_runtime = test
        .world_mut()
        .runtime_mut(runtime_id)
        .expect("parent runtime should exist");
    let parent_agent = parent_runtime
        .worker_mut(worker_id)
        .expect("parent primary worker should exist");
    let parent_next = parent_agent
        .event_loop
        .next_runnable(0, 0)
        .expect("parent pending ingress should be inspectable");

    let child_runtime = child
        .runtime_mut(runtime_id)
        .expect("child runtime should exist");
    let child_agent = child_runtime
        .worker_mut(worker_id)
        .expect("child primary worker should exist");
    let child_next = child_agent
        .event_loop
        .next_runnable(0, 0)
        .expect("child pending ingress should be inspectable");

    assert!(matches!(parent_next, Some(Runnable::PollerEvent(_))));
    assert!(matches!(child_next, Some(Runnable::PollerEvent(_))));
}

/// Ensures hibernation snapshots preserve pending suspendable runtime state.
#[test]
fn test_world_hibernate_snapshot_roundtrip_preserves_pending_state() {
    // build one runtime with pending ingress but no suspended continuations
    let options = RuntimeOptions::default();
    let mut test = TestWorld::new();
    let runtime_id = test.spawn_vm_runtime(&options);
    let runtime = test
        .world_mut()
        .runtime_mut(runtime_id)
        .expect("runtime should exist");
    let worker_id = runtime.primary_worker_id();
    let worker = runtime
        .worker_mut(worker_id)
        .expect("primary worker should exist");
    worker.event_loop.enqueue_events(vec![PollerEvent {
        resource_id: ResourceId(71),
        source: PollerEventSource::Io,
        mask: PollerEventMask::READABLE,
        flags: PollerEventFlags::NONE,
        token: PollerToken(811),
        payload: PollerEventPayload::Io { data: 3 },
    }]);

    let snapshot = test
        .world_mut()
        .hibernate_snapshot()
        .expect("hibernate snapshot should succeed");

    // rebuilding one world from that snapshot should preserve the queued event
    let mut restored_world =
        World::from_snapshot(&snapshot, None).expect("snapshot should rebuild world");
    let runtime_id = restored_world.runtime_ids()[0];
    let runtime = restored_world
        .runtime_mut(runtime_id)
        .expect("runtime should exist");
    let worker_id = runtime.primary_worker_id();
    let worker = runtime
        .worker_mut(worker_id)
        .expect("primary worker should exist");
    let next = worker
        .event_loop
        .next_runnable(0, 0)
        .expect("queued state should be inspectable");

    assert!(matches!(next, Some(Runnable::PollerEvent(_))));
}

/// Ensures worlds track unique runtime identities for explicitly spawned runtimes.
#[test]
fn test_world_spawn_runtime_tracks_identity() {
    // create one shared world and two runtimes
    let options = RuntimeOptions::default();
    let mut test = TestWorld::new();
    let runtime_a = test.spawn_vm_runtime(&options);
    let runtime_b = test.spawn_vm_runtime(&options);

    // both runtimes should keep unique identities in one shared world
    assert_ne!(runtime_a, runtime_b);
    assert_eq!(test.world().runtime_ids().len(), 2);
}

/// Ensures shared worlds expose one shared control state across workers.
#[test]
fn test_world_shared_commands_affect_detached_workers() {
    // create one shared world with two workers
    let options = RuntimeOptions::default();
    let mut world =
        World::from_options(&RuntimeOptions::default()).expect("world test should build");
    let world_ref = world.world_ref();
    let shared = super::tests::runtime_shared_heap(&world, &options);
    let _ = Worker::new_in_world(
        Vec::new(),
        &options,
        &world_ref,
        &shared,
        &destack_engine::StaticSpace::empty(),
        TestEngine::default(),
    )
    .expect("worker should construct in world");
    let _ = Worker::new_in_world(
        Vec::new(),
        &options,
        &world_ref,
        &shared,
        &destack_engine::StaticSpace::empty(),
        TestEngine::default(),
    )
    .expect("worker should construct in world");

    // install one rule through one world mutation
    world
        .install_rule(Rule {
            id: RuleId("test.shared.world.command".to_string()),
            enabled: true,
            when: Some(RuntimeSelector {
                binding: Some("destack.test.shared.world".to_string()),
                ..RuntimeSelector::default()
            }),
            action: Effect::SetAccess {
                access: RuntimeAccess::Deny,
            },
            trigger: None,
        })
        .expect("world mutation should apply");

    // both workers should observe the same world policy view
    assert_eq!(world.policy().rules.len(), 1);
}

/// Ensures worlds expose runtime identity only for explicitly spawned runtimes.
#[test]
fn test_world_spawn_runtime_registers_identity() {
    // create one shared world and one runtime in that world
    let options = RuntimeOptions::default();
    let mut test = TestWorld::new();
    let runtime_id = test.spawn_vm_runtime(&options);

    // one runtime registration should appear in world topology
    assert_eq!(test.world().runtime_ids(), vec![runtime_id]);
    assert_eq!(test.world().runtime_ids().len(), 1);
}

/// Ensures live world policy updates affect binding checks for existing workers.
#[test]
fn test_worker_world_control_update_refreshes_policy() {
    // create one worker in one shared world
    let options = RuntimeOptions::default();
    let mut world =
        World::from_options(&RuntimeOptions::default()).expect("world test should build");
    let world_ref = world.world_ref();
    let shared = super::tests::runtime_shared_heap(&world, &options);
    let worker = Worker::new_in_world(
        Vec::new(),
        &options,
        &world_ref,
        &shared,
        &destack_engine::StaticSpace::empty(),
        TestEngine::default(),
    )
    .expect("worker should construct in world");
    let host = Session::from_runtime_options(&options, worker.runtime_id);
    let descriptor = BindingDescriptor::pure("destack.test.live.policy", "()");

    // baseline policy should allow the call
    let baseline_call_context = super::tests::binding_call_context(&worker, &host, &world_ref);
    let baseline_result = baseline_call_context.on_before_binding(descriptor);
    assert!(baseline_result.is_ok());

    // install one deny rule in the shared world
    world
        .set_policy(Policy {
            rules: vec![Rule {
                id: RuleId("test.runtime.live.policy".to_string()),
                enabled: true,
                when: Some(RuntimeSelector {
                    binding: Some("destack.test.live.policy".to_string()),
                    ..RuntimeSelector::default()
                }),
                action: Effect::SetAccess {
                    access: RuntimeAccess::Deny,
                },
                trigger: None,
            }],
        })
        .expect("policy update should succeed");

    // updated policy should deny the same call without worker refresh
    let refreshed_call_context = super::tests::binding_call_context(&worker, &host, &world_ref);
    let refreshed_result = refreshed_call_context.on_before_binding(descriptor);
    assert!(refreshed_result.is_err());
}

/// Ensures live world policy updates affect hook plans for existing workers.
#[test]
fn test_worker_world_control_update_refreshes_hooks() {
    // create one worker in one shared world
    let options = RuntimeOptions::default();
    let mut world =
        World::from_options(&RuntimeOptions::default()).expect("world test should build");
    let world_ref = world.world_ref();
    let shared = super::tests::runtime_shared_heap(&world, &options);
    let worker = Worker::new_in_world(
        Vec::new(),
        &options,
        &world_ref,
        &shared,
        &destack_engine::StaticSpace::empty(),
        TestEngine::default(),
    )
    .expect("worker should construct in world");
    let host = Session::from_runtime_options(&options, worker.runtime_id);
    let descriptor = BindingDescriptor::pure("destack.test.live.hooks", "()");

    // install one hook-bearing fault rule in the shared world
    world
        .set_policy(Policy {
            rules: vec![Rule {
                id: RuleId("test.runtime.live.hooks".to_string()),
                enabled: true,
                when: Some(RuntimeSelector::default()),
                action: Effect::Fault {
                    fault: Fault {
                        target: FaultTarget::Call {},
                        fault_type: FaultType::Error {
                            code: "EFAULT".to_string(),
                        },
                    },
                },
                trigger: Some(Trigger {
                    on: Hook::BindingBefore,
                    activation: None,
                    lifetime: None,
                    activation_ppm: None,
                    probability_ppm: None,
                    max_occurrences: None,
                    cooldown_ns: None,
                    burst: None,
                    interval_hits: None,
                    skip_hits: None,
                }),
            }],
        })
        .expect("policy update should succeed");

    // firing the matching hook should enqueue one unapplied policy decision
    let call_context = super::tests::binding_call_context(&worker, &host, &world_ref);
    let hook_result = call_context.on_before_binding(descriptor);
    assert!(hook_result.is_ok());
    assert_eq!(worker.hooks.unapplied_policy_decision_count(), 1);
}

/// Ensures worker selectors match only the targeted worker in one shared world.
#[test]
fn test_worker_world_control_worker_selector() {
    // create two workers attached to one shared world
    let mut options_a = RuntimeOptions::default();
    options_a.primary_worker.name = Some("worker-a".to_string());
    let mut options_b = RuntimeOptions::default();
    options_b.primary_worker.name = Some("worker-b".to_string());
    let mut world =
        World::from_options(&RuntimeOptions::default()).expect("world test should build");
    let world_ref = world.world_ref();
    let shared = super::tests::runtime_shared_heap(&world, &options_a);
    let mut worker_a = Worker::new_in_world(
        Vec::new(),
        &options_a,
        &world_ref,
        &shared,
        &destack_engine::StaticSpace::empty(),
        TestEngine::default(),
    )
    .expect("worker should construct in world");
    let mut worker_b = Worker::new_in_world(
        Vec::new(),
        &options_b,
        &world_ref,
        &shared,
        &destack_engine::StaticSpace::empty(),
        TestEngine::default(),
    )
    .expect("worker should construct in world");
    let host_a = Session::from_runtime_options(&options_a, worker_a.runtime_id);
    let host_b = Session::from_runtime_options(&options_b, worker_b.runtime_id);

    // install one scheduler hook rule scoped to worker_a
    world
        .set_policy(Policy {
            rules: vec![Rule {
                id: RuleId("test.runtime.selector.instance".to_string()),
                enabled: true,
                when: Some(RuntimeSelector {
                    worker: Some(RuntimeIdentitySelector {
                        name: Some("worker-a".to_string()),
                        labels: None,
                    }),
                    ..RuntimeSelector::default()
                }),
                action: Effect::Fault {
                    fault: Fault {
                        target: FaultTarget::Call {},
                        fault_type: FaultType::Error {
                            code: "EFAULT".to_string(),
                        },
                    },
                },
                trigger: Some(Trigger {
                    on: Hook::SchedulerDequeue,
                    activation: None,
                    lifetime: None,
                    activation_ppm: None,
                    probability_ppm: None,
                    max_occurrences: None,
                    cooldown_ns: None,
                    burst: None,
                    interval_hits: None,
                    skip_hits: None,
                }),
            }],
        })
        .expect("policy update should succeed");

    // apply control updates on both workers
    let _ = worker_a
        .tick(
            &world_ref,
            &shared,
            &destack_engine::StaticSpace::empty(),
            &host_a,
        )
        .expect("tick should refresh policy state");
    let _ = worker_b
        .tick(
            &world_ref,
            &shared,
            &destack_engine::StaticSpace::empty(),
            &host_b,
        )
        .expect("tick should refresh policy state");

    // fire the same hook on both workers
    worker_a.hooks.on_scheduler_dequeue(&world_ref);
    worker_b.hooks.on_scheduler_dequeue(&world_ref);

    // only the targeted worker should match the rule
    assert_eq!(worker_a.hooks.unapplied_policy_decision_count(), 1);
    assert_eq!(worker_b.hooks.unapplied_policy_decision_count(), 0);
}

/// Ensures one policy mutation can install one deny rule.
#[test]
fn test_world_apply_policy_command_updates_rules() {
    // create one worker in one shared world
    let options = RuntimeOptions::default();
    let mut world =
        World::from_options(&RuntimeOptions::default()).expect("world test should build");
    let world_ref = world.world_ref();
    let shared = super::tests::runtime_shared_heap(&world, &options);
    let worker = Worker::new_in_world(
        Vec::new(),
        &options,
        &world_ref,
        &shared,
        &destack_engine::StaticSpace::empty(),
        TestEngine::default(),
    )
    .expect("worker should construct in world");
    let host = Session::from_runtime_options(&options, worker.runtime_id);
    let descriptor = BindingDescriptor::pure("destack.test.program.policy", "()");

    // install one deny rule through one world mutation
    world
        .install_rule(Rule {
            id: RuleId("test.runtime.program.policy".to_string()),
            enabled: true,
            when: Some(RuntimeSelector {
                binding: Some("destack.test.program.policy".to_string()),
                ..RuntimeSelector::default()
            }),
            action: Effect::SetAccess {
                access: RuntimeAccess::Deny,
            },
            trigger: None,
        })
        .expect("policy mutation should apply");

    // the installed rule should deny matching calls
    let call_context = super::tests::binding_call_context(&worker, &host, &world_ref);
    let result = call_context.on_before_binding(descriptor);
    assert!(result.is_err());
}

/// Ensures world topology control supports kind and graph mutation.
#[test]
fn test_world_topology_control_mutates_graph() {
    // create one world
    let mut world =
        World::from_options(&RuntimeOptions::default()).expect("world test should build");

    // define one custom entity and edge kind
    world
        .define_entity_kind(WorldEntityKindDefinition {
            kind: "app.node".into(),
            labels: std::collections::BTreeMap::new(),
            supported_faults: Default::default(),
        })
        .expect("entity kind should define");
    world
        .define_edge_kind(WorldEdgeKindDefinition {
            kind: "app.link".into(),
            labels: std::collections::BTreeMap::new(),
            supported_faults: Default::default(),
        })
        .expect("edge kind should define");

    // insert two entities and one connecting edge
    world
        .upsert_entity(WorldEntity {
            id: "node-a".into(),
            kind: "app.node".into(),
            labels: std::collections::BTreeMap::new(),
        })
        .expect("first entity should upsert");
    world
        .upsert_entity(WorldEntity {
            id: "node-b".into(),
            kind: "app.node".into(),
            labels: std::collections::BTreeMap::new(),
        })
        .expect("second entity should upsert");
    world
        .upsert_edge(WorldEdge {
            id: "link-a-b".into(),
            kind: "app.link".into(),
            from: "node-a".into(),
            to: "node-b".into(),
            labels: std::collections::BTreeMap::new(),
        })
        .expect("edge should upsert");

    // removing one entity should also remove incident edges
    world
        .remove_entity("node-a".into())
        .expect("entity removal should succeed");
    assert!(!world.edges().contains_key("link-a-b"));
}

/// Ensures one failed topology command does not roll back prior successful commands.
#[test]
fn test_world_topology_command_failure_does_not_revert_prior_commands() {
    // create one world
    let mut world =
        World::from_options(&RuntimeOptions::default()).expect("world test should build");

    // apply one valid command first
    world
        .define_entity_kind(WorldEntityKindDefinition {
            kind: "app.atomic.node".into(),
            labels: std::collections::BTreeMap::new(),
            supported_faults: Default::default(),
        })
        .expect("entity kind should define");

    // apply one invalid command next
    let result = world.upsert_entity(WorldEntity {
        id: "bad-node".into(),
        kind: "app.missing.kind".into(),
        labels: std::collections::BTreeMap::new(),
    });

    // the failing mutation should not mutate topology, prior mutation stays committed
    let entity_kinds = world.entity_kinds();
    let entities = world.entities();
    assert!(result.is_err());
    assert!(entity_kinds.contains_key("app.atomic.node"));
    assert!(!entities.contains_key("bad-node"));
}

/// Ensures resource attach and detach operations synchronize into world topology and resource state.
#[test]
fn test_world_resource_lifecycle_updates_topology() {
    // create one worker and insert one resource
    let options = RuntimeOptions::default();
    let mut world =
        World::from_options(&RuntimeOptions::default()).expect("world test should build");
    let world_ref = world.world_ref();
    let shared = super::tests::runtime_shared_heap(&world, &options);
    let worker = Worker::new_in_world(
        Vec::new(),
        &options,
        &world_ref,
        &shared,
        &destack_engine::StaticSpace::empty(),
        TestEngine::default(),
    )
    .expect("worker should construct in world");
    let resource_id = worker.resources.insert(
        &world_ref,
        ResourceEntry::new(ResourceKind::Timer).with_label("test-timer"),
        None,
    );

    // verify world resource payload and topology metadata exist
    let resources = world.resources();
    let world_resource_id = crate::runtime::WorldResourceId::new(worker.id, resource_id);
    let world_resource = resources
        .get(&world_resource_id)
        .expect("resource should exist in world resource state");
    let resource_entity_id = world_resource_id.entity_id();
    let resource_edge_id = world_resource_id.ownership_edge_id();
    let entities = world.entities();
    let edges = world.edges();
    assert_eq!(world_resource.kind.as_str(), ResourceKind::Timer.kind_id());
    assert_eq!(world_resource.label.as_deref(), Some("test-timer"));
    assert!(entities.contains_key(&resource_entity_id));
    assert!(edges.contains_key(&resource_edge_id));

    // remove the resource and verify both payload and topology metadata disappear
    let removed = worker.resources.remove(&world_ref, resource_id, None);
    assert!(removed.is_some());
    let resources = world.resources();
    let entities = world.entities();
    let edges = world.edges();
    assert!(!resources.contains_key(&world_resource_id));
    assert!(!entities.contains_key(&resource_entity_id));
    assert!(!edges.contains_key(&resource_edge_id));
}

/// Ensures explicit world worker removal clears selector metadata and topology ownership.
#[test]
fn test_world_remove_worker_cleans_topology() {
    // create one world and one detached worker
    let mut world =
        World::from_options(&RuntimeOptions::default()).expect("world test should build");
    let options = RuntimeOptions::default();
    let world_ref = world.world_ref();
    let shared = super::tests::runtime_shared_heap(&world, &options);
    let worker = Worker::new_in_world(
        Vec::new(),
        &options,
        &world_ref,
        &shared,
        &destack_engine::StaticSpace::empty(),
        TestEngine::default(),
    )
    .expect("worker should construct in world");
    let worker_id = worker.id;
    let host = Session::from_runtime_options(&options, worker.runtime_id);
    let descriptor = BindingDescriptor::pure("destack.test.removed.worker", "()");

    // binding checks should work before removal
    let before_context = super::tests::binding_call_context(&worker, &host, &world_ref);
    let before_result = before_context.on_before_binding(descriptor);
    assert!(before_result.is_ok());

    // removing the worker should clear its selector metadata
    world
        .remove_worker(worker_id)
        .expect("worker removal should succeed");

    let entities = world.entities();
    assert!(!entities.contains_key(&worker_id.entity_id()));

    let after_context = super::tests::binding_call_context(&worker, &host, &world_ref);
    let after_result = after_context.on_before_binding(descriptor);
    assert!(after_result.is_err());
}

/// Ensures failed world mutations do not append replay events.
#[test]
fn test_world_apply_record_failure_does_not_append_replay_events() {
    // create one record-mode world
    let mut options = RuntimeOptions::default();
    options.set_execution_mode(ExecutionMode::Record);
    let mut world = World::from_options(&options).expect("world should construct");

    // apply one successful command first
    world
        .define_entity_kind(WorldEntityKindDefinition {
            kind: "app.record.atomic.node".into(),
            labels: Default::default(),
            supported_faults: Default::default(),
        })
        .expect("define entity kind should succeed");
    let sequence_after_success = world.trace().log().next_sequence().get();
    assert_eq!(sequence_after_success, 1);

    // apply one failing command after that
    let result = world.upsert_entity(WorldEntity {
        id: "missing-kind-node".into(),
        kind: "app.record.atomic.missing".into(),
        labels: Default::default(),
    });
    assert!(result.is_err());

    // replay log should not advance after failure
    let next_sequence = world.trace().log().next_sequence().get();
    assert_eq!(next_sequence, sequence_after_success);
}

/// Ensures spawned workers inherit world-scoped runtime options.
#[test]
fn test_runtime_spawn_worker_aligns_world_scoped_options() {
    // create one runtime with one shared world
    let options = RuntimeOptions::default();
    let mut test = TestWorld::with_options(&options);
    let runtime_id = test.spawn_vm_runtime(&options);

    // request one conflicting option set for spawn
    let mut spawn_options = RuntimeOptions::default();
    spawn_options.set_execution_mode(ExecutionMode::Replay);
    spawn_options.effect.access = RuntimeAccess::Deny;
    spawn_options.effect.backend = RuntimeWorld::Simulation;
    spawn_options.set_random_mode(RandomMode::Host);
    spawn_options.set_time_mode(TimeMode::Host);

    // spawned worker should keep runtime world-scoped settings
    let world_ref = test.world_mut().world_ref();
    let runtime = test
        .world_mut()
        .runtime_mut(runtime_id)
        .expect("runtime should exist");
    let spawned_worker_id = runtime
        .spawn_worker_with_options(&world_ref, &spawn_options, TestWorld::vm_engine())
        .expect("spawn should succeed");
    let spawned_worker = runtime
        .worker(spawned_worker_id)
        .expect("spawned worker should exist");
    assert_eq!(spawned_worker.options.scheduler, options.scheduler);
    assert_eq!(spawned_worker.options.effect, options.effect);
    assert_eq!(spawned_worker.options.simulation, options.simulation);
    assert_eq!(spawned_worker.options.trace, options.trace);
}

/// Ensures deterministic worlds reject secure randomness bindings by default.
#[test]
fn test_world_deterministic_mode_rejects_secure_randomness() {
    // construct one deterministic-random world
    let mut options = RuntimeOptions::default();
    options.set_random_mode(RandomMode::Deterministic);
    let world = World::from_options(&options).expect("world should construct");

    // secure host randomness should fail in deterministic mode
    let mut bytes = [0u8; 16];
    assert!(world.fill_secure_bytes(&mut bytes).is_err());
    assert!(world.try_fill_secure_bytes(&mut bytes).is_err());
}

/// Ensures capability profiles configure binding policy capability enforcement.
#[test]
fn test_worker_capability_profile_configures_binding_policy() {
    let mut options = RuntimeOptions::default();
    options.security.capability_profile = Some("fs.read,net.connect".to_string());

    let mut world = World::from_options(&options).expect("world should construct");
    let world_ref = world.world_ref();
    let shared = super::tests::runtime_shared_heap(&world, &options);
    let worker = Worker::new_in_world(
        Vec::new(),
        &options,
        &world_ref,
        &shared,
        &destack_engine::StaticSpace::empty(),
        TestEngine::default(),
    )
    .expect("worker should construct");
    let policy = worker.bindings.policy().read();

    assert!(policy.is_capability_requirements_enforced());
    assert!(policy.capabilities().contains_name("fs.read"));
    assert!(policy.capabilities().contains_name("net.connect"));
    assert_eq!(policy.capabilities().len(), 2);
}

/// Ensures simulation deadlines publish the earliest scheduled event.
#[test]
fn test_world_next_simulation_deadline_returns_earliest_deadline() {
    // create one world and schedule several simulated events
    let mut world =
        World::from_options(&RuntimeOptions::default()).expect("world test should build");
    {
        let simulation = world.simulation_mut();
        simulation.schedule_event(WorldInstant::new(9_000));
        simulation.schedule_event(WorldInstant::new(5_000));
        simulation.schedule_event(WorldInstant::new(7_000));
        simulation.schedule_event(WorldInstant::new(6_000));
    }

    // the world should expose the earliest published deadline
    assert_eq!(
        world.next_simulation_deadline(),
        Some(WorldInstant::new(5_000))
    );
}
