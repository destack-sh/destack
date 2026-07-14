use destack_heap as heap;
use destack_mir::TraceMap;
use destack_repository::{ExecutionMode, RuntimeOptions};

use crate::diagnostic::{MachineError, RuntimeError};
use crate::tests::harness::{TestMachine, TestRuntime, TestWorldRuntime};
use crate::world::RunOutcome;

const DROP_MIR: &str = r#"
type Item {
    value: int32;
}

function test.entry(): void {
entry:
    return
}

function Item.drop(v0: ref<Item, borrowed, exclusive>): void {
entry(v0: ref<Item, borrowed, exclusive>):
    trap.abort
}
"#;

const RETURN_DROP_MIR: &str = r#"
type Item {
    value: int32;
}

function test.entry(): void {
entry:
    return
}

function Item.drop(v0: ref<Item, borrowed, exclusive>): void {
entry(v0: ref<Item, borrowed, exclusive>):
    return
}
"#;

const SUSPEND_DROP_MIR: &str = r#"
type Item {
    value: int32;
}

function test.entry(): void {
entry:
    return
}

function Item.drop(v0: ref<Item, borrowed, exclusive>): void {
entry(v0: ref<Item, borrowed, exclusive>):
    v1: int32 = 0
    yield v1 => resumed
resumed(v2: int32):
    return
}
"#;

const STOP_DROP_MIR: &str = r#"
type Item {
    value: int32;
}

function test.entry(): void {
entry:
    return
}

function Item.drop(v0: ref<Item, borrowed, exclusive>): void {
entry(v0: ref<Item, borrowed, exclusive>):
    breakpoint
    return
}
"#;

/// Execute one unreachable allocation's destructor before reclamation.
#[test]
fn test_collect_runs_allocation_destructor() {
    let machine = TestMachine::with_mir(DROP_MIR).with_drop("Item", "Item.drop");
    let mut runtime = TestRuntime::build(&RuntimeOptions::default(), machine);
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
    let machine = TestMachine::with_mir(RETURN_DROP_MIR).with_drop("Item", "Item.drop");
    let mut runtime = TestRuntime::build(&RuntimeOptions::default(), machine);
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

/// Reject one destructor that suspends during collection.
#[test]
fn test_collect_rejects_suspending_destructor() {
    let machine = TestMachine::with_mir(SUSPEND_DROP_MIR).with_drop("Item", "Item.drop");
    let mut runtime = TestRuntime::build(&RuntimeOptions::default(), machine);
    let drop = heap::DropId::from_index(0);
    let shape = heap::AllocationShape::new(4, 1, None, TraceMap::empty())
        .with_drop(drop)
        .expect("Drop plan should build");
    runtime.allocate(shape);
    runtime.request_full_gc();

    // advance collection until the destructor attempts to suspend
    let error = loop {
        match runtime.step_gc() {
            Ok(Some(_)) => {}
            Ok(None) => panic!("collection became idle before allocation Drop"),
            Err(error) => break error,
        }
    };

    assert!(matches!(
        error.as_ref(),
        RuntimeError::Machine {
            reason: MachineError::DropSuspended,
            ..
        }
    ));
}

/// Reject one destructor that reaches a runtime stop point during collection.
#[test]
fn test_collect_rejects_stopping_destructor() {
    let machine = TestMachine::with_mir(STOP_DROP_MIR).with_drop("Item", "Item.drop");
    let mut runtime = TestRuntime::build(&RuntimeOptions::default(), machine);
    let drop = heap::DropId::from_index(0);
    let shape = heap::AllocationShape::new(4, 1, None, TraceMap::empty())
        .with_drop(drop)
        .expect("Drop plan should build");
    runtime.allocate(shape);
    runtime.request_full_gc();

    // advance collection until the destructor reaches its stop point
    let error = loop {
        match runtime.step_gc() {
            Ok(Some(_)) => {}
            Ok(None) => panic!("collection became idle before allocation Drop"),
            Err(error) => break error,
        }
    };

    assert!(matches!(
        error.as_ref(),
        RuntimeError::Machine {
            reason: MachineError::DropStopped,
            ..
        }
    ));
}

/// Execute one shared allocation's destructor before reclamation.
#[test]
fn test_collect_reclaims_shared_allocation_after_drop() {
    let options = RuntimeOptions {
        mode: ExecutionMode::Strict,
        ..RuntimeOptions::default()
    };
    let machine = TestMachine::with_mir(RETURN_DROP_MIR).with_drop("Item", "Item.drop");
    let mut runtime = TestWorldRuntime::build(&options, machine);
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
