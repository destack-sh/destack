use destack_core::CaptureMode;
use destack_heap as heap;
use destack_mir::TraceMap;
use destack_program as program;
use destack_repository::{ExecutionMode, RuntimeOptions};
use destack_vm as vm;

use crate::binding::BindingTable;
use crate::diagnostic::RuntimeError;
use crate::host::{HostEvent, HostEventKind, LifecycleEvent, LifecycleSourceKind, LifecycleState};
use crate::machine::Engine;
use crate::tests::{TestProgram, TestWorker, TestWorld};
use crate::world::RunOutcome;

/// Execute one unreachable allocation's destructor before reclamation.
#[test]
fn test_collect_runs_allocation_destructor() {
    let program = TestProgram::mir(
        r#"
function Item.destruct(v0: ref<int32, borrowed, exclusive>): void {
entry(v0: ref<int32, borrowed, exclusive>):
    unreachable
}

export function entry(): void {
entry:
    return
}
"#,
    )
    .destructor("Item.destruct");
    let machine = program;
    let mut runtime = TestWorker::build(
        &RuntimeOptions::default(),
        machine.build(),
        BindingTable::new(),
        Engine::vm(vm::MachineLimits::test()),
    );
    let drop = heap::DropId::from_index(0);
    let element = heap::AllocationShape::new(4, 4, None, TraceMap::empty())
        .with_drop(drop)
        .expect("Drop plan should build");
    let element = heap::HeapOptions::local().allocation_plan(&element);
    let shape = element
        .repeat(&TraceMap::empty(), 2)
        .expect("repeated allocation should build");
    runtime.allocate(shape);
    runtime.request_full_gc();

    // advance collection until the configured callback reaches its abort terminator
    let error = loop {
        match runtime.step_gc() {
            Ok(Some(_)) => {}
            Ok(None) => panic!("collection became idle before allocation Drop"),
            Err(error) => break error,
        }
    };

    assert!(matches!(error.as_ref(), RuntimeError::Vm(_)), "{error}");
}

/// Reclaim one managed allocation after its destructor completes.
#[test]
fn test_collect_reclaims_allocation_after_drop() {
    let program = TestProgram::mir(
        r#"
function Item.destruct(v0: ref<int32, borrowed, exclusive>): void {
entry(v0: ref<int32, borrowed, exclusive>):
    return
}

export function entry(): void {
entry:
    return
}
"#,
    )
    .destructor("Item.destruct");
    let machine = program;
    let mut runtime = TestWorker::build(
        &RuntimeOptions::default(),
        machine.build(),
        BindingTable::new(),
        Engine::vm(vm::MachineLimits::test()),
    );
    let drop = heap::DropId::from_index(0);
    let shape = heap::AllocationShape::new(4, 1, None, TraceMap::empty())
        .with_drop(drop)
        .expect("Drop plan should build");
    let reference = runtime.allocate(shape);
    runtime.request_full_gc();

    // run the managed Drop and finish physical reclamation
    loop {
        let progress = runtime
            .step_gc()
            .expect("collection should execute managed Drop");
        let Some(progress) = progress else {
            panic!("collection became idle before reclamation");
        };
        if progress.completed_stats().is_some() {
            break;
        }
    }

    assert!(!runtime.is_live(reference));
}

/// Execute one shared allocation's destructor before reclamation.
#[test]
fn test_collect_reclaims_shared_allocation_after_drop() {
    let options = RuntimeOptions {
        mode: ExecutionMode::Strict,
        ..RuntimeOptions::default()
    };
    let program = TestProgram::mir(
        r#"
function Item.destruct(v0: ref<int32, borrowed, exclusive, space(shared)>): void {
entry(v0: ref<int32, borrowed, exclusive, space(shared)>):
    return
}

export function entry(): void {
entry:
    return
}
"#,
    )
    .destructor("Item.destruct");
    let machine = program;
    let mut runtime = TestWorld::build(&options, machine);
    let drop = heap::DropId::from_index(0);
    let shape = heap::AllocationShape::new(4, 1, None, TraceMap::empty())
        .with_drop(drop)
        .expect("Drop plan should build");
    let reference = runtime.allocate_shared(shape);
    runtime.request_shared_gc();

    // drive world scheduling through Drop and physical reclamation
    let mut last_outcome = RunOutcome::Idle;
    for _ in 0..1024 {
        last_outcome = runtime.run_task();
        if !runtime.is_shared_live(reference) {
            break;
        }
        if last_outcome == RunOutcome::Idle {
            panic!("world became idle before shared allocation reclamation");
        }
    }

    assert!(
        !runtime.is_shared_live(reference),
        "shared allocation remained live in {:?} after {last_outcome:?}",
        runtime.shared_gc_phase(),
    );
}

/// Queues one shared-root rescan when a host event mutates worker state during marking.
#[test]
fn test_host_event_queues_shared_root_publication() {
    let options = RuntimeOptions::default();
    let program = TestProgram::mir(
        r#"
export function retain(v0: ref<int32, managed, mutable, space(shared)>): void {
entry(v0: ref<int32, managed, mutable, space(shared)>):
    return
}
"#,
    );
    let mut runtime = TestWorld::build(&options, program);
    let worker_id = runtime.default_worker_id();
    let shape = heap::AllocationShape::new(1, 1, None, TraceMap::empty());
    let shared_root = runtime.allocate_shared(shape);
    let shared_value = runtime.argument(
        "retain",
        [program::Word::from_bits(shared_root.bits() as u64)],
    );
    runtime.add_host_waiter(worker_id, "retain", HostEventKind::Lifecycle, shared_value);
    runtime.start_shared_gc();

    // publish the initial worker roots
    let (published_worker, advance) = runtime
        .run_safepoint()
        .expect("worker should publish shared roots");
    assert_eq!(published_worker, worker_id);
    assert!(matches!(
        advance,
        heap::GcAdvance::Stepped(heap::GcStep {
            collector: heap::GcCollector::Shared,
            phase: heap::GcPhase::PublishRoots,
            ..
        })
    ));
    assert!(!runtime.has_pending_shared_roots(worker_id));

    // mutate direct roots through host delivery before the worker runs again
    let handled = runtime.deliver_host_event(HostEvent::Lifecycle(LifecycleEvent {
        source_kind: LifecycleSourceKind::Application,
        state: LifecycleState::Running,
    }));
    assert!(handled);
    assert!(runtime.has_pending_shared_roots(worker_id));
}

/// Publishes worker-local shared allocation caches before capturing a runtime image.
#[test]
fn test_capture_publishes_shared_allocations() {
    let options = RuntimeOptions::default();
    let mut runtime = TestWorld::build(&options, TestProgram::mir(""));
    let worker_id = runtime.default_worker_id();
    let shape = heap::AllocationShape::new(16, 1, None, TraceMap::empty());
    runtime.allocate_cached_shared(worker_id, shape);

    assert_eq!(runtime.shared_allocation_count(), 0);

    runtime.capture(CaptureMode::Suspend);

    assert_eq!(runtime.shared_allocation_count(), 1);
}

/// Publishes one worker's direct shared roots from its runtime safepoint.
#[test]
fn test_safepoint_publishes_shared_roots() {
    let options = RuntimeOptions::default();
    let program = TestProgram::mir(
        r#"
export function retain(v0: ref<int32, managed, mutable, space(shared)>): void {
entry(v0: ref<int32, managed, mutable, space(shared)>):
    return
}
"#,
    );
    let mut runtime = TestWorld::build(&options, program);
    let worker_id = runtime.default_worker_id();
    let shape = heap::AllocationShape::new(1, 1, None, TraceMap::empty());
    let shared_root = runtime.allocate_shared(shape);
    let shared_value = runtime.argument(
        "retain",
        [program::Word::from_bits(shared_root.bits() as u64)],
    );
    runtime.add_host_waiter(worker_id, "retain", HostEventKind::Lifecycle, shared_value);
    runtime.start_shared_gc();
    runtime.queue_shared_roots(worker_id);

    // publish the queued worker roots through one scheduler safepoint
    let (published_worker, advance) = runtime
        .run_safepoint()
        .expect("worker should publish shared roots");
    assert_eq!(published_worker, worker_id);
    assert!(matches!(
        advance,
        heap::GcAdvance::Stepped(heap::GcStep {
            collector: heap::GcCollector::Shared,
            phase: heap::GcPhase::PublishRoots,
            ..
        })
    ));
    assert!(!runtime.has_pending_shared_roots(worker_id));
    assert_eq!(runtime.shared_roots().as_ref(), &[shared_root]);
}
