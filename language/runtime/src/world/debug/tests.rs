use destack_program as program;
use destack_repository::RuntimeOptions;

use crate::tests::{TestProgram, TestWorld};
use crate::world::observation::{Observation, ObservationScope};
use crate::world::{
    Breakpoint, BreakpointTarget, FrameSource, InstructionProbe, Probe, ProbeAction, ProbeTarget,
    RunOutcome, View, Watchpoint,
};

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
    );
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
    let view = runtime
        .world_mut()
        .view(View::Now)
        .expect("current world should be inspectable");
    let frames = view
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
    assert_eq!(frame.source, FrameSource::Stopped { runnable_id });
    let program = view
        .program(frame.runtime_id)
        .expect("stopped runtime should retain its program");
    let state = program
        .frame_state(frame.frame_state())
        .expect("stopped frame state should exist");
    // the stop names the breakpoint, while the frame names its resume operation
    assert_eq!(
        program
            .frame_point(frame.frame_state())
            .expect("stopped frame point should exist"),
        program::FramePoint::operation(runtime.point("stop", 1))
    );
    let layout = program
        .frame_layout(state.layout)
        .expect("stopped frame layout should exist");
    let slots = program.frame_slots(layout);
    assert_eq!(slots[0].offset, 0);
    let bytes = frame.bytes();
    let value = i32::from_le_bytes(
        bytes
            .try_into()
            .expect("stopped frame should contain one int32"),
    );
    assert_eq!(value, 313);

    // continuing the stop should complete the retained execution
    assert_eq!(runtime.run_continue(), RunOutcome::Progressed);
    assert_eq!(runtime.run_continue(), RunOutcome::Idle);
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
    );
    let mut runtime = TestWorld::build(&options, program);
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let point = runtime.point("run", 0);
    let breakpoint_id = runtime
        .world_mut()
        .add_breakpoint(BreakpointTarget {
            runtime_id: Some(runtime_id),
            worker_id: Some(worker_id),
            point,
        })
        .expect("breakpoint should add");
    let after_breakpoint = runtime
        .world()
        .observations()
        .records_after(None)
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
    let after_stop = runtime
        .world()
        .observations()
        .records_after(Some(after_breakpoint.sequence));
    assert_eq!(after_stop.len(), 1);
    assert_eq!(
        after_stop[0].observation,
        Observation::StopReached {
            runtime_id,
            worker_id,
            reason: program::StopReason::Breakpoint {
                breakpoint_id,
                point,
            },
        }
    );
    assert_eq!(after_stop[0].moment, stop.moment);

    // continuing the stop skips the same breakpoint once and completes
    assert_eq!(runtime.run_continue(), RunOutcome::Progressed);
    assert_eq!(runtime.run_continue(), RunOutcome::Idle);
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
    let breakpoint_id = runtime
        .world_mut()
        .add_breakpoint(BreakpointTarget {
            runtime_id: Some(runtime_id),
            worker_id: Some(worker_id),
            point,
        })
        .expect("breakpoint should add");
    runtime
        .world_mut()
        .disable_breakpoint(breakpoint_id)
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
        .add_breakpoint(BreakpointTarget {
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
        .add_breakpoint(BreakpointTarget {
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
            BreakpointTarget {
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
        .add_breakpoint(BreakpointTarget {
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
        .add_breakpoint(BreakpointTarget {
            runtime_id: Some(runtime_id),
            worker_id: Some(worker_id),
            point,
        })
        .expect("breakpoint should add");
    let _second_breakpoint_id = runtime
        .world_mut()
        .add_breakpoint(BreakpointTarget {
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
    assert_eq!(runtime.run_continue(), RunOutcome::Progressed);
    assert_eq!(runtime.run_continue(), RunOutcome::Idle);
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
        .add_breakpoint(BreakpointTarget {
            runtime_id: Some(runtime_id),
            worker_id: Some(worker_id),
            point,
        })
        .expect("breakpoint should add");
    let watchpoint_id = runtime
        .world_mut()
        .add_watchpoint(
            Some(runtime_id),
            Some(worker_id),
            program::MemoryAccess::Write,
            program::MemoryTarget::Any,
        )
        .expect("watchpoint should add");
    let probe_id = runtime
        .world_mut()
        .add_probe(
            ProbeTarget::Instruction(InstructionProbe {
                runtime_id: Some(runtime_id),
                worker_id: Some(worker_id),
                point,
            }),
            ProbeAction::Count,
        )
        .expect("probe should add");

    // update and disable each noun through the same mutation path
    runtime
        .world_mut()
        .update_breakpoint(Breakpoint::new(
            breakpoint_id,
            BreakpointTarget {
                runtime_id: None,
                worker_id: Some(worker_id),
                point,
            },
        ))
        .expect("breakpoint should update");
    runtime
        .world_mut()
        .update_watchpoint(Watchpoint::new(
            watchpoint_id,
            None,
            Some(worker_id),
            program::MemoryAccess::Read,
            program::MemoryTarget::Any,
        ))
        .expect("watchpoint should update");
    runtime
        .world_mut()
        .update_probe(Probe::new(
            probe_id,
            ProbeTarget::Instruction(InstructionProbe {
                runtime_id: None,
                worker_id: Some(worker_id),
                point,
            }),
            ProbeAction::Observe,
        ))
        .expect("probe should update");
    runtime
        .world_mut()
        .disable_breakpoint(breakpoint_id)
        .expect("breakpoint should disable");
    runtime
        .world_mut()
        .disable_watchpoint(watchpoint_id)
        .expect("watchpoint should disable");
    runtime
        .world_mut()
        .disable_probe(probe_id)
        .expect("probe should disable");

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
