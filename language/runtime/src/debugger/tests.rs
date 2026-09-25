use std::num::NonZeroU64;

use tspp_program as program;
use tspp_repository::RuntimeOptions;

use crate::debugger::{
    Breakpoint, EventFilter, MemoryFilter, PointFilter, Probe, ProbeAction, ProbeFilter, Watchpoint,
};
use crate::tests::{TestProgram, TestWorld};
use crate::world::RunOutcome;
use crate::world::observation::{Observation, ObservationQuery, ObservationScope};

/// Runs one task to an explicit bytecode breakpoint and resumes it explicitly.
#[test]
fn test_stop_at_instruction_breakpoint() {
    // configure one explicit shared world and runtime
    let options = RuntimeOptions::default();
    let program = TestProgram::mir(
        r#"
export function stop(v0: int32): int32 {
entry(v0: int32):
    breakpoint
    return v0
}
"#,
    )
    .compile_native();
    let mut runtime = TestWorld::build(&options, program);
    let worker_id = runtime.default_worker_id();
    let runnable_id = runtime.enqueue_task(worker_id, "stop", 313);
    let point = runtime.point("stop", 0);
    let before = runtime.moment();

    // stepping one task should retain the stopped execution
    let stop = runtime.run_to_stop();
    assert_eq!(stop.reason, program::StopReason::Instruction { point });
    assert_eq!(stop.worker_id, worker_id);
    assert_eq!(stop.moment.branch_id, before.branch_id);
    assert_eq!(stop.moment.sequence.get(), before.sequence.get() + 1);

    // task runs surface retained stops without advancing them
    let retained_before = runtime.moment();
    assert!(matches!(
        runtime.run_task(),
        RunOutcome::Stopped {
            stop: retained_stop
        } if retained_stop == stop
    ));
    assert_eq!(runtime.moment(), retained_before);

    // inspect the canonical stopped frame through its program layout
    let image = runtime
        .world_mut()
        .image()
        .expect("current world should be inspectable");
    let frames = image
        .worker_frames(worker_id)
        .expect("stopped worker should be inspectable");
    let mut frames = frames.into_iter();
    let frame = frames
        .next()
        .expect("stopped worker should retain one frame");
    assert!(
        frames.next().is_none(),
        "stopped worker should retain one frame"
    );
    assert_eq!(frame.id.fiber_id, program::FiberId::new(0, 1));
    assert_eq!(frame.runnable_id, Some(runnable_id));
    let program = image
        .program(frame.id.runtime_id)
        .expect("stopped runtime should retain its program");
    let state = program
        .frame_state(frame.frame_state_id)
        .expect("stopped frame state should exist");
    // the stop names the breakpoint, while the frame names its resume operation
    assert_eq!(
        program
            .frame_point(frame.frame_state_id)
            .expect("stopped frame point should exist"),
        program::FramePoint::operation(runtime.point("stop", 1))
    );
    let layout = program
        .frame_layout(state.layout)
        .expect("stopped frame layout should exist");
    let slots = program.frame_slots(layout);
    assert_eq!(slots[0].offset, 0);
    let bytes = frame.bytes.as_slice();
    let value = i32::from_le_bytes(
        bytes
            .try_into()
            .expect("stopped frame should contain one int32"),
    );
    assert_eq!(value, 313);

    // resume the stop to complete the retained execution
    assert_eq!(runtime.resume(stop), RunOutcome::Progressed);
    assert_eq!(runtime.run_task(), RunOutcome::Idle);
}

/// Runs one task to a runtime breakpoint and resumes it explicitly.
#[test]
fn test_stop_at_runtime_breakpoint() {
    // configure one explicit shared world and runtime
    let options = RuntimeOptions::default();
    let program = TestProgram::mir(
        r#"
export function run(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#,
    )
    .compile_native();
    let mut runtime = TestWorld::build(&options, program);
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let point = runtime.point("run", 0);
    let breakpoint_id = runtime
        .world_mut()
        .add_breakpoint(PointFilter {
            runtime_id: Some(runtime_id),
            worker_id: Some(worker_id),
            point,
        })
        .expect("breakpoint should add");
    let after_breakpoint = runtime
        .world()
        .observations()
        .query(&ObservationQuery::default(), usize::MAX)
        .into_iter()
        .last()
        .expect("breakpoint mutation should emit an observation");
    assert_eq!(
        after_breakpoint.observation,
        Observation::BreakpointAdded { breakpoint_id }
    );
    assert!(
        after_breakpoint
            .observation
            .is_on(&ObservationScope::world())
    );

    // enqueue one invocation that reaches the breakpoint
    runtime.enqueue_task(worker_id, "run", 313);

    // stepping one task should stop at the runtime breakpoint
    let stop = runtime.run_to_stop();
    assert_eq!(
        stop.reason,
        program::StopReason::Breakpoint {
            breakpoint_id,
            point,
        }
    );
    assert_eq!(stop.worker_id, worker_id);
    let after_stop = runtime.world().observations().query(
        &ObservationQuery::after(after_breakpoint.sequence),
        usize::MAX,
    );
    assert_eq!(after_stop.len(), 1);
    assert_eq!(
        after_stop[0].observation,
        Observation::StopReached {
            runtime_id,
            worker_id,
            fiber_id: stop.fiber_id,
            reason: program::StopReason::Breakpoint {
                breakpoint_id,
                point,
            },
        }
    );
    assert_eq!(after_stop[0].moment, stop.moment);

    // resume after the breakpoint without stopping at it again
    assert_eq!(runtime.resume(stop), RunOutcome::Progressed);
    assert_eq!(runtime.run_task(), RunOutcome::Idle);
}

/// Ignores a disabled runtime breakpoint while running the selected point.
#[test]
fn test_disabled_runtime_breakpoint_does_not_stop() {
    // configure one runtime breakpoint and disable it before execution
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
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let point = runtime.point("run", 0);
    let selector = PointFilter {
        runtime_id: Some(runtime_id),
        worker_id: Some(worker_id),
        point,
    };
    let breakpoint_id = runtime
        .world_mut()
        .add_breakpoint(selector)
        .expect("breakpoint should add");
    let mut breakpoint = Breakpoint::new(breakpoint_id, selector);
    breakpoint.is_enabled = false;
    runtime
        .world_mut()
        .update_breakpoint(breakpoint)
        .expect("breakpoint should disable");

    // enqueue work that would hit the breakpoint if it were enabled
    runtime.enqueue_task(worker_id, "run", 313);

    assert_eq!(runtime.run_task(), RunOutcome::Progressed);
    assert_eq!(runtime.run_task(), RunOutcome::Idle);
}

/// Ignores a runtime breakpoint scoped to a different worker.
#[test]
fn test_worker_scoped_runtime_breakpoint_does_not_stop_other_worker() {
    // configure one breakpoint for a second worker
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
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let other_worker_id = runtime.spawn_worker();
    let point = runtime.point("run", 0);
    runtime
        .world_mut()
        .add_breakpoint(PointFilter {
            runtime_id: Some(runtime_id),
            worker_id: Some(other_worker_id),
            point,
        })
        .expect("breakpoint should add");

    // enqueue matching work on the default worker instead
    runtime.enqueue_task(worker_id, "run", 313);

    assert_eq!(runtime.run_task(), RunOutcome::Progressed);
    assert_eq!(runtime.run_task(), RunOutcome::Idle);
}

/// Applies an updated runtime breakpoint before the next worker run.
#[test]
fn test_updated_runtime_breakpoint_changes_hit_point() {
    // add one breakpoint at a point the task will not execute
    let options = RuntimeOptions::default();
    let program = TestProgram::mir(
        r#"
export function run(v0: int32): int32 {
entry(v0: int32):
    return v0
}

export function other(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#,
    );
    let mut runtime = TestWorld::build(&options, program);
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let point = runtime.point("run", 0);
    let other = runtime.point("other", 0);
    let breakpoint_id = runtime
        .world_mut()
        .add_breakpoint(PointFilter {
            runtime_id: Some(runtime_id),
            worker_id: Some(worker_id),
            point: other,
        })
        .expect("breakpoint should add");

    // update the breakpoint to the point reached by the next task
    runtime
        .world_mut()
        .update_breakpoint(Breakpoint::new(
            breakpoint_id,
            PointFilter {
                runtime_id: Some(runtime_id),
                worker_id: Some(worker_id),
                point,
            },
        ))
        .expect("breakpoint should update");

    // enqueue one invocation that reaches the updated breakpoint
    runtime.enqueue_task(worker_id, "run", 313);

    let stop = runtime.run_to_stop();
    assert_eq!(
        stop.reason,
        program::StopReason::Breakpoint {
            breakpoint_id,
            point,
        }
    );
}

/// Ignores a removed runtime breakpoint while running the formerly selected point.
#[test]
fn test_removed_runtime_breakpoint_does_not_stop() {
    // add and remove one breakpoint before execution
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
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let point = runtime.point("run", 0);
    let breakpoint_id = runtime
        .world_mut()
        .add_breakpoint(PointFilter {
            runtime_id: Some(runtime_id),
            worker_id: Some(worker_id),
            point,
        })
        .expect("breakpoint should add");
    runtime
        .world_mut()
        .remove_breakpoint(breakpoint_id)
        .expect("breakpoint should remove");

    // enqueue work that would hit the breakpoint if it were still active
    runtime.enqueue_task(worker_id, "run", 313);

    assert_eq!(runtime.run_task(), RunOutcome::Progressed);
    assert_eq!(runtime.run_task(), RunOutcome::Idle);
}

/// Stops at the first runtime breakpoint when multiple breakpoints select one point.
#[test]
fn test_duplicate_runtime_breakpoints_stop_at_first_breakpoint() {
    // add two breakpoints at the same executable point
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
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let point = runtime.point("run", 0);
    let first_breakpoint_id = runtime
        .world_mut()
        .add_breakpoint(PointFilter {
            runtime_id: Some(runtime_id),
            worker_id: Some(worker_id),
            point,
        })
        .expect("breakpoint should add");
    let _second_breakpoint_id = runtime
        .world_mut()
        .add_breakpoint(PointFilter {
            runtime_id: Some(runtime_id),
            worker_id: Some(worker_id),
            point,
        })
        .expect("breakpoint should add");

    // enqueue one invocation that reaches the shared breakpoint point
    runtime.enqueue_task(worker_id, "run", 313);

    let stop = runtime.run_to_stop();
    assert_eq!(
        stop.reason,
        program::StopReason::Breakpoint {
            breakpoint_id: first_breakpoint_id,
            point,
        }
    );
    assert_eq!(runtime.resume(stop), RunOutcome::Progressed);
    assert_eq!(runtime.run_task(), RunOutcome::Idle);
}

/// Applies debugger mutations symmetrically.
#[test]
fn test_update_debugger_entries() {
    // configure one runtime world
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
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let point = runtime.point("run", 0);

    // add the three debugger nouns
    let breakpoint_id = runtime
        .world_mut()
        .add_breakpoint(PointFilter {
            runtime_id: Some(runtime_id),
            worker_id: Some(worker_id),
            point,
        })
        .expect("breakpoint should add");
    let watchpoint_id = runtime
        .world_mut()
        .add_watchpoint(MemoryFilter {
            runtime_id: Some(runtime_id),
            worker_id: Some(worker_id),
            access: program::MemoryAccess::Write,
            target: program::MemoryTarget::Any,
        })
        .expect("watchpoint should add");
    let probe_id = runtime
        .world_mut()
        .add_probe(
            ProbeFilter {
                runtime_id: Some(runtime_id),
                worker_id: Some(worker_id),
                fiber_id: None,
                event: EventFilter::Point { point: Some(point) },
            },
            ProbeAction::Count,
        )
        .expect("probe should add");

    // update and disable each debugger rule through one mutation
    let mut breakpoint = Breakpoint::new(
        breakpoint_id,
        PointFilter {
            runtime_id: None,
            worker_id: Some(worker_id),
            point,
        },
    );
    breakpoint.is_enabled = false;
    runtime
        .world_mut()
        .update_breakpoint(breakpoint)
        .expect("breakpoint should update");

    let mut watchpoint = Watchpoint::new(
        watchpoint_id,
        MemoryFilter {
            runtime_id: None,
            worker_id: Some(worker_id),
            access: program::MemoryAccess::Read,
            target: program::MemoryTarget::Any,
        },
    );
    watchpoint.is_enabled = false;
    runtime
        .world_mut()
        .update_watchpoint(watchpoint)
        .expect("watchpoint should update");

    let mut probe = Probe::new(
        probe_id,
        ProbeFilter {
            runtime_id: None,
            worker_id: Some(worker_id),
            fiber_id: None,
            event: EventFilter::Point { point: Some(point) },
        },
        ProbeAction::Observe {
            interval: NonZeroU64::MIN,
        },
    );
    probe.is_enabled = false;
    runtime
        .world_mut()
        .update_probe(probe)
        .expect("probe should update");

    // remove all three and verify durable debugger state is empty
    runtime
        .world_mut()
        .remove_breakpoint(breakpoint_id)
        .expect("breakpoint should remove");
    runtime
        .world_mut()
        .remove_watchpoint(watchpoint_id)
        .expect("watchpoint should remove");
    runtime
        .world_mut()
        .remove_probe(probe_id)
        .expect("probe should remove");

    assert!(runtime.world().debugger().is_empty());
}

/// Counts every selected event and emits observations at the configured interval.
#[test]
fn test_run_point_probes() {
    let options = RuntimeOptions::default();
    let program = TestProgram::mir(
        r#"
export function run(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#,
    )
    .compile_native();
    let mut runtime = TestWorld::build(&options, program);
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let point = runtime.point("run", 0);
    let filter = ProbeFilter {
        runtime_id: Some(runtime_id),
        worker_id: Some(worker_id),
        fiber_id: None,
        event: EventFilter::Point { point: Some(point) },
    };
    let count_probe = runtime
        .world_mut()
        .add_probe(filter, ProbeAction::Count)
        .expect("count probe should add");
    let observe_probe = runtime
        .world_mut()
        .add_probe(
            filter,
            ProbeAction::Observe {
                interval: NonZeroU64::new(2).expect("probe interval should be nonzero"),
            },
        )
        .expect("observation probe should add");
    let after_probe = runtime
        .world()
        .observations()
        .query(&ObservationQuery::default(), usize::MAX)
        .into_iter()
        .last()
        .expect("probe mutation should emit an observation");

    // execute the selected point twice through the native fallback path
    let first_task = runtime.enqueue_task(worker_id, "run", 1);
    let second_task = runtime.enqueue_task(worker_id, "run", 2);
    assert_eq!(runtime.run_task(), RunOutcome::Progressed);
    assert_eq!(runtime.run_task(), RunOutcome::Progressed);

    // retain exact counts for both actions in captured World state
    let image = runtime
        .world_mut()
        .image()
        .expect("probed World should be capturable");
    let probe_counts = image
        .debugger()
        .probes()
        .iter()
        .map(|probe| (probe.id, probe.hit_count))
        .collect::<Vec<_>>();
    assert_eq!(probe_counts, vec![(count_probe, 2), (observe_probe, 2)]);

    // emit only the second selected point at the configured interval
    let observations = runtime
        .world()
        .observations()
        .query(&ObservationQuery::after(after_probe.sequence), usize::MAX)
        .into_iter()
        .map(|entry| entry.observation)
        .collect::<Vec<_>>();
    assert_eq!(
        observations,
        vec![
            Observation::TaskRan {
                runtime_id,
                worker_id,
                task_id: first_task,
            },
            Observation::ProbeHit {
                probe_id: observe_probe,
                count: 2,
                runtime_id,
                worker_id,
                fiber_id: Some(program::FiberId::new(0, 2)),
                event: program::Event::Point { point },
            },
            Observation::TaskRan {
                runtime_id,
                worker_id,
                task_id: second_task,
            },
        ]
    );
}
