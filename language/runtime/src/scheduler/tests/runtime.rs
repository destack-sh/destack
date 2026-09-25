use std::sync::Arc;

use tspp_program as program;
use tspp_repository::{Environment, RuntimeOptions, WorldOptions};
use tspp_vm as vm;

use crate::binding::BindingTable;
use crate::host::time::TimerClock;
use crate::host::{HostEventKind, LifecycleState, ResourceId};
use crate::machine::Engine;
use crate::tests::{TestProgram, TestWorker, TestWorld};
use crate::world::observation::{Observation, ObservationQuery};
use crate::world::{Run, RunOutcome, World};

/// Executes one queued task in one run.
#[test]
fn test_run_executes_one_task() {
    // create runtime state with one queued task
    let program = TestProgram::mir(
        r#"
export function run(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#,
    );
    let mut runtime = TestWorker::bytecode(
        &RuntimeOptions::default(),
        program.build(),
        BindingTable::new(),
    );
    runtime.enqueue_task("run", 1);

    // execute one task
    let progressed = runtime.run();
    assert!(progressed, "run should report progress");
    assert!(
        !runtime.has_pending_work(),
        "completed task should leave no work"
    );
}

/// Shares one runtime condition set with every spawned worker.
#[test]
fn test_spawn_worker_shares_runtime_conditions() {
    let options = RuntimeOptions::default();
    let mut runtime = TestWorld::build(&options, TestProgram::mir(""));
    let conditions = runtime.conditions();
    let worker_id = runtime.spawn_worker();

    runtime.with_worker_mut(worker_id, |worker| {
        assert!(Arc::ptr_eq(&worker.conditions, &conditions));
    });
}

/// Executes microtasks only through the explicit microtask path.
#[test]
fn test_run_selects_tasks_and_microtasks_independently() {
    let options = RuntimeOptions::default();
    let program = TestProgram::mir(
        r#"
export function run(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#,
    );
    let mut runtime = TestWorld::build(&options, program);
    let worker_id = runtime.default_worker_id();
    let microtask_id = runtime.enqueue_microtask(worker_id, "run", 1);

    // task selection leaves the pending microtask untouched
    assert_eq!(runtime.run_task(), RunOutcome::Idle);

    // explicit microtask execution advances one moment and identifies that execution
    let before = runtime.moment();
    let outcome = runtime
        .world_mut()
        .run(Run::Microtask)
        .expect("microtask should run");
    assert_eq!(outcome, RunOutcome::Progressed);
    assert_eq!(runtime.moment().sequence.get(), before.sequence.get() + 1);
    let observation = runtime
        .world()
        .observations()
        .query(&ObservationQuery::default(), usize::MAX)
        .into_iter()
        .last()
        .expect("microtask should emit an observation");
    assert_eq!(
        observation.observation,
        Observation::MicrotaskRan {
            runtime_id: runtime.runtime_id(),
            worker_id,
            microtask_id,
        }
    );
}

/// Rotates task selection across runnable runtimes.
#[test]
fn test_run_rotates_runtimes() {
    let options = RuntimeOptions::default();
    let source = r#"
export function run(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#;
    let mut world = TestWorld::build(&options, TestProgram::mir(source));
    let first_runtime_id = world.runtime_id();
    let first_worker_id = world.default_worker_id();
    world.enqueue_task(first_worker_id, "run", 1);
    world.enqueue_task(first_worker_id, "run", 2);

    let second_runtime_id = world.spawn_runtime(&options, TestProgram::mir(source));
    world.select_runtime(second_runtime_id);
    let second_worker_id = world.default_worker_id();
    world.enqueue_task(second_worker_id, "run", 3);

    // run one task from each runtime before returning to the first runtime
    assert_eq!(world.run_task(), RunOutcome::Progressed);
    assert_eq!(world.run_task(), RunOutcome::Progressed);
    assert_eq!(world.run_task(), RunOutcome::Progressed);

    let runtime_ids = world
        .world()
        .observations()
        .query(&ObservationQuery::default(), usize::MAX)
        .into_iter()
        .filter_map(|entry| match entry.observation {
            Observation::TaskRan { runtime_id, .. } => Some(runtime_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        runtime_ids,
        vec![first_runtime_id, second_runtime_id, first_runtime_id]
    );
}

/// Records profile counters through worker task execution.
#[test]
fn test_run_records_worker_profile() {
    let program = TestProgram::mir(
        r#"
export function run(v0: int32): int32 {
entry(v0: int32):
    profile.increment counter(0)
    return v0
}
"#,
    );
    let mut runtime = TestWorker::bytecode(
        &RuntimeOptions::default(),
        program.build(),
        BindingTable::new(),
    );
    runtime.start_profile(program::ProfileOptions::STANDARD);
    runtime.enqueue_task("run", 1);

    // execute one profiled task resume
    let progressed = runtime.run();
    assert!(progressed, "run should report progress");

    let profile = runtime.profile().expect("worker profile should be active");
    assert_eq!(profile.counters.len(), 1);
    assert_eq!(profile.counters[0].count, 1);
}

/// Drain queued tasks until idle.
#[test]
fn test_drain_runs_queued_tasks() {
    // create runtime state with one queued task
    let program = TestProgram::mir(
        r#"
export function run(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#,
    );
    let mut runtime = TestWorker::bytecode(
        &RuntimeOptions::default(),
        program.build(),
        BindingTable::new(),
    );
    runtime.enqueue_task("run", 9);

    // run until the queue is drained
    runtime.drain();

    assert!(
        !runtime.has_pending_work(),
        "event loop should be idle after draining tasks"
    );
}

/// Dispatches registered timer waiters through the runtime run path.
#[test]
fn test_run_dispatches_timer_waiter_task() {
    // create runtime state with one timer waiter registration
    let program = TestProgram::mir(
        r#"
export function run(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#,
    );
    let mut runtime = TestWorker::bytecode(
        &RuntimeOptions::default(),
        program.build(),
        BindingTable::new(),
    );
    runtime.add_timer_waiter("run", 77, 31);
    runtime.schedule_timer(77, 0, None);

    // execute one timer callback
    let progressed = runtime.run();
    assert!(progressed, "run should report progress");
    assert!(
        !runtime.has_pending_work(),
        "one-shot timer should leave no work"
    );

    // one-shot waiter should be removed after the first dispatch
    assert!(
        !runtime.remove_timer_waiter(77),
        "one-shot timer waiter should be removed"
    );
}

/// Dispatches registered resource wakes through the runtime run path.
#[test]
fn test_run_dispatches_event_waiter_task() {
    // create runtime state with one resource waiter registration
    let program = TestProgram::mir(
        r#"
export function run(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#,
    );
    let mut runtime = TestWorker::bytecode(
        &RuntimeOptions::default(),
        program.build(),
        BindingTable::new(),
    );
    runtime.add_resource_waiter("run", 5, 41);
    runtime.enqueue_io_event(5, 91, 9);

    // execute one resource callback
    let progressed = runtime.run();
    assert!(progressed, "run should report progress");
    assert!(
        runtime.has_pending_work(),
        "resource waiter should remain registered"
    );
}

/// Dispatches registered host wakes through the runtime run path.
#[test]
fn test_run_dispatches_host_event_waiter_task() {
    // create runtime state with one lifecycle host waiter registration
    let program = TestProgram::mir(
        r#"
export function run(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#,
    );
    let mut runtime = TestWorker::bytecode(
        &RuntimeOptions::default(),
        program.build(),
        BindingTable::new(),
    );
    runtime.add_host_waiter("run", HostEventKind::Lifecycle, 42);
    runtime.enqueue_lifecycle_host_event(LifecycleState::Running);

    // execute one host callback
    let progressed = runtime.run();
    assert!(progressed, "run should report progress");
    assert!(
        runtime.has_pending_work(),
        "host waiter should remain registered"
    );
}

/// Advances virtual time to the next deadline before dispatching the timer task.
#[test]
fn test_runtime_run_advances_virtual_time_before_dispatch() {
    // configure one virtual runtime with one future timer
    let options = RuntimeOptions::default();
    let program = TestProgram::mir(
        r#"
export function run(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#,
    );
    let mut runtime = TestWorld::build(&options, program);
    let default_worker_id = runtime.default_worker_id();
    let mono_before = runtime.mono_nanos();
    let fire_at_nanos = runtime.wall_nanos() + 5_000;
    let callback = runtime.callback(default_worker_id, "run", 111);
    runtime.with_worker_mut(default_worker_id, |worker| {
        worker.add_timer_waiter(ResourceId::new(worker.worker_id(), 950), callback);
    });
    runtime.schedule_timer(
        default_worker_id,
        TimerClock::Wall,
        950,
        fire_at_nanos,
        None,
    );

    // the first task run should only advance time
    let outcome = runtime.run_task();
    assert_eq!(outcome, RunOutcome::AdvancedTime);
    assert_eq!(
        runtime.wall_nanos(),
        fire_at_nanos,
        "virtual time should jump exactly to the next deadline"
    );
    assert_eq!(
        runtime.mono_nanos(),
        mono_before + 5_000,
        "virtual monotonic time should advance by the same delta"
    );

    // the next task run should dispatch the newly ready timer task
    let outcome = runtime.run_task();
    assert_eq!(outcome, RunOutcome::Progressed);
}

/// Executes one world task run through the attached runtime.
#[test]
fn test_world_run_task_drives_runtime() {
    let options = RuntimeOptions::default();
    let program = TestProgram::mir(
        r#"
export function run(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#,
    );
    let mut runtime = TestWorld::build(&options, program);
    let worker_id = runtime.default_worker_id();
    runtime.enqueue_task(worker_id, "run", 211);
    let before = runtime.moment();

    // world task run should delegate through the runtime and execute the task
    assert_eq!(runtime.run_task(), RunOutcome::Progressed);
    let after = runtime.moment();
    assert_eq!(after.branch_id, before.branch_id);
    assert_eq!(after.sequence.get(), before.sequence.get() + 1);
}

/// Records structural runtime changes as world observations.
#[test]
fn test_world_spawn_runtime_records_observation() {
    // configure one explicit shared world
    let options = RuntimeOptions::default();
    let world_options = WorldOptions::default();
    let mut world = World::new(&world_options, Environment::default()).expect("world should build");
    let program = Arc::new(TestProgram::mir("").build());
    let before = world.moment();

    // spawn one runtime and read the emitted observation
    let engine = Engine::new(program, vm::MachineLimits::test());
    let runtime_id = world
        .spawn_runtime(
            Environment::default(),
            &options,
            TestWorker::conditions(),
            Arc::new(BindingTable::new()),
            engine,
        )
        .expect("runtime should spawn");
    let observations = world
        .observations()
        .query(&ObservationQuery::default(), usize::MAX);

    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].moment.branch_id, before.branch_id);
    assert_eq!(
        observations[0].moment.sequence.get(),
        before.sequence.get() + 1
    );
    assert_eq!(
        observations[0].observation,
        Observation::RuntimeSpawned {
            runtime_id,
            worker_count: 1,
        }
    );
}

/// Dispatches equal-deadline timers in stable worker-id order.
#[test]
fn test_runtime_run_orders_equal_deadline_timers_by_worker_id() {
    // configure one virtual runtime with two workers and one equal deadline
    let options = RuntimeOptions::default();
    let program = TestProgram::mir(
        r#"
export function run(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#,
    );
    let mut runtime = TestWorld::build(&options, program);
    let default_worker_id = runtime.default_worker_id();
    let secondary_worker_id = runtime.spawn_worker();
    let fire_at_nanos = runtime.wall_nanos() + 10_000;
    let default_callback = runtime.callback(default_worker_id, "run", 201);
    let secondary_callback = runtime.callback(secondary_worker_id, "run", 202);

    // register one waiter timer on each worker
    runtime.with_worker_mut(default_worker_id, |worker| {
        worker.add_timer_waiter(ResourceId::new(worker.worker_id(), 960), default_callback);
    });
    runtime.schedule_timer(
        default_worker_id,
        TimerClock::Wall,
        960,
        fire_at_nanos,
        None,
    );
    runtime.with_worker_mut(secondary_worker_id, |worker| {
        worker.add_timer_waiter(ResourceId::new(worker.worker_id(), 961), secondary_callback);
    });
    runtime.schedule_timer(
        secondary_worker_id,
        TimerClock::Wall,
        961,
        fire_at_nanos,
        None,
    );

    // the first task run advances time and later task runs dispatch in worker order
    assert_eq!(runtime.run_task(), RunOutcome::AdvancedTime);
    assert_eq!(runtime.run_task(), RunOutcome::Progressed);
    assert_eq!(runtime.run_task(), RunOutcome::Progressed);
    assert_eq!(runtime.run_task(), RunOutcome::Idle);
}
