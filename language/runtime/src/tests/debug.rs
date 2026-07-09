use destack_program as program;
use destack_repository::RuntimeOptions;

use crate::runtime::WorkerId;
use crate::runtime::scheduler::{Runnable, RunnableId};
use crate::tests::harness::{TestMachine, TestWorldRuntime};
use crate::world::observation::{Observation, ObservationScope};
use crate::world::{
    Breakpoint, BreakpointTarget, InstructionProbe, Probe, ProbeAction, ProbeTarget, RunOutcome,
    Watchpoint,
};

const WATCHPOINT_MIR: &str = r#"
function test.entry(): void {
b0:
    return
}

function test.complete(v0: int32): int32 {
b0(v0: int32):
    yield v0 => b1(v0)
b1(v1: int32, v2: int32):
    v3: ref<int32, managed, mutable> = new.zeroed int32
    store v3, v1
    v4: int32 = load v3
    return v4
}
"#;

/// Return the write site in the watchpoint test program.
fn memory_write_site(runtime: &mut TestWorldRuntime, worker_id: WorkerId) -> program::MemorySite {
    runtime.with_worker_mut(worker_id, |worker| {
        worker
            .program
            .sites()
            .memory_sites(worker.program.sections())
            .iter()
            .copied()
            .find(|site| site.access == program::MemoryAccess::Write)
            .expect("test program should contain one write site")
    })
}

/// Runs one task to an explicit MIR breakpoint and resumes it explicitly.
#[test]
fn test_stop_at_instruction_breakpoint() {
    // configure one explicit shared world and runtime
    let options = RuntimeOptions::default();
    let mut runtime = TestWorldRuntime::build(&options, TestMachine::default());
    let worker_id = runtime.default_worker_id();
    let continuation = runtime.breakpoint_continuation(worker_id, 313);
    runtime.with_worker_mut(worker_id, |worker| {
        worker.event_loop.enqueue_task(Runnable {
            id: RunnableId::new(44),
            continuation,
            resume_value: program::Value::Void,
        });
    });
    let before = runtime.moment();

    // stepping one task should retain the stopped continuation
    let outcome = runtime.run_task();
    let RunOutcome::Stopped { stop } = outcome else {
        panic!("expected stopped run outcome");
    };
    assert_eq!(
        stop.reason,
        program::StopReason::Instruction {
            point: program::ProgramPoint::new(program::FunctionId(3), 1),
        }
    );
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

    // continuing the stop should complete the retained continuation
    assert_eq!(runtime.run_continue(), RunOutcome::Progressed);
    assert_eq!(runtime.run_continue(), RunOutcome::Idle);
}

/// Runs one task to a runtime breakpoint and resumes it explicitly.
#[test]
fn test_stop_at_runtime_breakpoint() {
    // configure one explicit shared world and runtime
    let options = RuntimeOptions::default();
    let mut runtime = TestWorldRuntime::build(&options, TestMachine::default());
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let point = program::ProgramPoint::new(program::FunctionId(2), 1);
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

    // enqueue one continuation that reaches the breakpoint after resume
    let continuation = runtime.completing_continuation(worker_id, 313);
    runtime.with_worker_mut(worker_id, |worker| {
        worker.event_loop.enqueue_task(Runnable {
            id: RunnableId::new(45),
            continuation,
            resume_value: program::Value::Void,
        });
    });

    // stepping one task should stop at the runtime breakpoint
    let outcome = runtime.run_task();
    let RunOutcome::Stopped { stop } = outcome else {
        panic!("expected stopped run outcome");
    };
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

/// Stops one worker at a watched memory write.
#[test]
fn test_stop_at_memory_watchpoint() {
    // configure one runtime watchpoint at the worker's write site
    let options = RuntimeOptions::default();
    let mut runtime = TestWorldRuntime::build(&options, TestMachine::with_mir(WATCHPOINT_MIR));
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let write_site = memory_write_site(&mut runtime, worker_id);
    let watchpoint_id = runtime
        .world_mut()
        .add_watchpoint(
            Some(runtime_id),
            Some(worker_id),
            program::MemoryAccess::Write,
            program::MemoryTarget::Point(write_site.point),
        )
        .expect("watchpoint should add");

    // enqueue one continuation that reaches the watched write
    let continuation = runtime.completing_continuation(worker_id, 313);
    runtime.with_worker_mut(worker_id, |worker| {
        worker.event_loop.enqueue_task(Runnable {
            id: RunnableId::new(52),
            continuation,
            resume_value: program::Value::Void,
        });
    });

    // stepping one task should stop at the watchpoint and emit an observation
    let outcome = runtime.run_task();
    let RunOutcome::Stopped { stop } = outcome else {
        panic!("expected stopped run outcome");
    };
    assert_eq!(
        stop.reason,
        program::StopReason::Watchpoint {
            watchpoint_id,
            point: write_site.point,
        }
    );
    let after_stop = runtime.world().observations().records_after(None);
    assert!(after_stop.iter().any(|entry| {
        entry.observation
            == Observation::StopReached {
                runtime_id,
                worker_id,
                reason: program::StopReason::Watchpoint {
                    watchpoint_id,
                    point: write_site.point,
                },
            }
    }));

    // continuing the stop should complete the retained continuation
    assert_eq!(runtime.run_continue(), RunOutcome::Progressed);
    assert_eq!(runtime.run_continue(), RunOutcome::Idle);
}

/// Stops one worker at a watched local heap byte range.
#[test]
fn test_stop_at_memory_range_watchpoint() {
    // configure one runtime watchpoint over local heap storage
    let options = RuntimeOptions::default();
    let mut runtime = TestWorldRuntime::build(&options, TestMachine::with_mir(WATCHPOINT_MIR));
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let write_site = memory_write_site(&mut runtime, worker_id);
    let watchpoint_id = runtime
        .world_mut()
        .add_watchpoint(
            Some(runtime_id),
            Some(worker_id),
            program::MemoryAccess::Write,
            program::MemoryTarget::Range(program::MemoryRange::local_heap(0, 1024 * 1024)),
        )
        .expect("watchpoint should add");

    // enqueue one continuation that reaches the watched write
    let continuation = runtime.completing_continuation(worker_id, 313);
    runtime.with_worker_mut(worker_id, |worker| {
        worker.event_loop.enqueue_task(Runnable {
            id: RunnableId::new(55),
            continuation,
            resume_value: program::Value::Void,
        });
    });

    // stepping one task should stop at the watched range
    let outcome = runtime.run_task();
    let RunOutcome::Stopped { stop } = outcome else {
        panic!("expected stopped run outcome");
    };
    assert_eq!(
        stop.reason,
        program::StopReason::Watchpoint {
            watchpoint_id,
            point: write_site.point,
        }
    );
}

/// Ignores a disabled memory watchpoint while running the selected write.
#[test]
fn test_disabled_memory_watchpoint_does_not_stop() {
    // configure one watchpoint and disable it before execution
    let options = RuntimeOptions::default();
    let mut runtime = TestWorldRuntime::build(&options, TestMachine::with_mir(WATCHPOINT_MIR));
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let write_site = memory_write_site(&mut runtime, worker_id);
    let watchpoint_id = runtime
        .world_mut()
        .add_watchpoint(
            Some(runtime_id),
            Some(worker_id),
            program::MemoryAccess::Write,
            program::MemoryTarget::Point(write_site.point),
        )
        .expect("watchpoint should add");
    runtime
        .world_mut()
        .disable_watchpoint(watchpoint_id)
        .expect("watchpoint should disable");

    // enqueue work that would hit the watchpoint if it were enabled
    let continuation = runtime.completing_continuation(worker_id, 313);
    runtime.with_worker_mut(worker_id, |worker| {
        worker.event_loop.enqueue_task(Runnable {
            id: RunnableId::new(53),
            continuation,
            resume_value: program::Value::Void,
        });
    });

    assert_eq!(runtime.run_task(), RunOutcome::Progressed);
    assert_eq!(runtime.run_task(), RunOutcome::Idle);
}

/// Ignores a memory watchpoint scoped to a different worker.
#[test]
fn test_worker_scoped_memory_watchpoint_does_not_stop_other_worker() {
    // configure one watchpoint for a second worker
    let options = RuntimeOptions::default();
    let mut runtime = TestWorldRuntime::build(&options, TestMachine::with_mir(WATCHPOINT_MIR));
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let other_worker_id = runtime.spawn_worker();
    let write_site = memory_write_site(&mut runtime, worker_id);
    runtime
        .world_mut()
        .add_watchpoint(
            Some(runtime_id),
            Some(other_worker_id),
            program::MemoryAccess::Write,
            program::MemoryTarget::Point(write_site.point),
        )
        .expect("watchpoint should add");

    // enqueue matching work on the default worker instead
    let continuation = runtime.completing_continuation(worker_id, 313);
    runtime.with_worker_mut(worker_id, |worker| {
        worker.event_loop.enqueue_task(Runnable {
            id: RunnableId::new(54),
            continuation,
            resume_value: program::Value::Void,
        });
    });

    assert_eq!(runtime.run_task(), RunOutcome::Progressed);
    assert_eq!(runtime.run_task(), RunOutcome::Idle);
}

/// Ignores a disabled runtime breakpoint while running the selected point.
#[test]
fn test_disabled_runtime_breakpoint_does_not_stop() {
    // configure one runtime breakpoint and disable it before execution
    let options = RuntimeOptions::default();
    let mut runtime = TestWorldRuntime::build(&options, TestMachine::default());
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let point = program::ProgramPoint::new(program::FunctionId(2), 1);
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
    let continuation = runtime.completing_continuation(worker_id, 313);
    runtime.with_worker_mut(worker_id, |worker| {
        worker.event_loop.enqueue_task(Runnable {
            id: RunnableId::new(46),
            continuation,
            resume_value: program::Value::Void,
        });
    });

    assert_eq!(runtime.run_task(), RunOutcome::Progressed);
    assert_eq!(runtime.run_task(), RunOutcome::Idle);
}

/// Ignores a runtime breakpoint scoped to a different worker.
#[test]
fn test_worker_scoped_runtime_breakpoint_does_not_stop_other_worker() {
    // configure one breakpoint for a second worker
    let options = RuntimeOptions::default();
    let mut runtime = TestWorldRuntime::build(&options, TestMachine::default());
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let other_worker_id = runtime.spawn_worker();
    runtime
        .world_mut()
        .add_breakpoint(BreakpointTarget {
            runtime_id: Some(runtime_id),
            worker_id: Some(other_worker_id),
            point: program::ProgramPoint::new(program::FunctionId(2), 1),
        })
        .expect("breakpoint should add");

    // enqueue matching work on the default worker instead
    let continuation = runtime.completing_continuation(worker_id, 313);
    runtime.with_worker_mut(worker_id, |worker| {
        worker.event_loop.enqueue_task(Runnable {
            id: RunnableId::new(47),
            continuation,
            resume_value: program::Value::Void,
        });
    });

    assert_eq!(runtime.run_task(), RunOutcome::Progressed);
    assert_eq!(runtime.run_task(), RunOutcome::Idle);
}

/// Applies an updated runtime breakpoint before the next worker run.
#[test]
fn test_updated_runtime_breakpoint_changes_hit_point() {
    // add one breakpoint at a point the task will not execute
    let options = RuntimeOptions::default();
    let mut runtime = TestWorldRuntime::build(&options, TestMachine::default());
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let point = program::ProgramPoint::new(program::FunctionId(2), 1);
    let breakpoint_id = runtime
        .world_mut()
        .add_breakpoint(BreakpointTarget {
            runtime_id: Some(runtime_id),
            worker_id: Some(worker_id),
            point: program::ProgramPoint::new(program::FunctionId(3), 1),
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

    // enqueue one continuation that reaches the updated breakpoint
    let continuation = runtime.completing_continuation(worker_id, 313);
    runtime.with_worker_mut(worker_id, |worker| {
        worker.event_loop.enqueue_task(Runnable {
            id: RunnableId::new(48),
            continuation,
            resume_value: program::Value::Void,
        });
    });

    let outcome = runtime.run_task();
    let RunOutcome::Stopped { stop } = outcome else {
        panic!("expected stopped run outcome");
    };
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
    let mut runtime = TestWorldRuntime::build(&options, TestMachine::default());
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let breakpoint_id = runtime
        .world_mut()
        .add_breakpoint(BreakpointTarget {
            runtime_id: Some(runtime_id),
            worker_id: Some(worker_id),
            point: program::ProgramPoint::new(program::FunctionId(2), 1),
        })
        .expect("breakpoint should add");
    runtime
        .world_mut()
        .remove_breakpoint(breakpoint_id)
        .expect("breakpoint should remove");

    // enqueue work that would hit the breakpoint if it were still active
    let continuation = runtime.completing_continuation(worker_id, 313);
    runtime.with_worker_mut(worker_id, |worker| {
        worker.event_loop.enqueue_task(Runnable {
            id: RunnableId::new(49),
            continuation,
            resume_value: program::Value::Void,
        });
    });

    assert_eq!(runtime.run_task(), RunOutcome::Progressed);
    assert_eq!(runtime.run_task(), RunOutcome::Idle);
}

/// Stops at the first runtime breakpoint when multiple breakpoints select one point.
#[test]
fn test_duplicate_runtime_breakpoints_stop_at_first_breakpoint() {
    // add two breakpoints at the same executable point
    let options = RuntimeOptions::default();
    let mut runtime = TestWorldRuntime::build(&options, TestMachine::default());
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let point = program::ProgramPoint::new(program::FunctionId(2), 1);
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

    // enqueue one continuation that reaches the shared breakpoint point
    let continuation = runtime.completing_continuation(worker_id, 313);
    runtime.with_worker_mut(worker_id, |worker| {
        worker.event_loop.enqueue_task(Runnable {
            id: RunnableId::new(50),
            continuation,
            resume_value: program::Value::Void,
        });
    });

    let outcome = runtime.run_task();
    let RunOutcome::Stopped { stop } = outcome else {
        panic!("expected stopped run outcome");
    };
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
    let mut runtime = TestWorldRuntime::build(&options, TestMachine::default());
    let runtime_id = runtime.runtime_id();
    let worker_id = runtime.default_worker_id();
    let point = program::ProgramPoint::new(program::FunctionId(2), 1);

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
