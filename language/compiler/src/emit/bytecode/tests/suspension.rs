use destack_program::object::{ContinuationSite, Point, Suspension, SuspensionSite};

use crate::tests::TestProgram;

/// Emit Await, Yield, Continuation, Waiter, and Task operations.
#[test]
fn test_emit_bytecode_suspension() {
    let program = TestProgram::mir(
        r#"
type Task {
    uint64;
}

external function park(int32, waiter<int32>): void

async function wait(v0: int32): int32 {
entry(v0: int32):
    await park(v0) => resumed | cancelled | unwind

resumed(v1: int32):
    return v1

cancelled:
    return

unwind:
    unwind.resume
}

function* generate(v0: int32): int32 {
entry(v0: int32):
    yield v0 => resumed | completed | unwind

resumed(v1: int32):
    return v1

completed(v2: int32):
    return v2

unwind:
    unwind.resume
}

function owner(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: continuation<int32, int32, int32> = continuation.new generate(v0)
    continuation.resume v2(v1) => yielded | returned | unwind

yielded(v3: int32, v4: continuation<int32, int32, int32>):
    continuation.destroy v4
    return v3

returned(v5: int32):
    return v5

unwind:
    unwind.resume
}

function complete(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: continuation<int32, int32, int32> = continuation.new generate(v0)
    continuation.complete v2(v1) => yielded | returned | unwind

yielded(v3: int32, v4: continuation<int32, int32, int32>):
    continuation.destroy v4
    return v3

returned(v5: int32):
    return v5

unwind:
    unwind.resume
}

function settle(v0: waiter<int32>, v1: int32): void {
entry(v0: waiter<int32>, v1: int32):
    v2: boolean = waiter.queue v0, v1
    return
}

function cancel(v0: waiter<int32>): void {
entry(v0: waiter<int32>):
    v1: boolean = waiter.cancel v0
    return
}

function resolveTask(v0: int32, v1: waiter<int32>): void {
entry(v0: int32, v1: waiter<int32>):
    v2: Task = task.resolve v0
    task.cancel v2
    task.park v2, v1
    return
}

function startTask(v0: continuation<void, never, int32>): void {
entry(v0: continuation<void, never, int32>):
    v1: Task = task.start v0
    task.detach v1
    return
}
"#,
    );

    let object = program.assert_bytecode(
        r#"
external function park

function wait {
    await r0, park, r0 => b0 | b1 | b2

b0:
    return r0

b1:
    return

b2:
    unwind.resume
}

function generate {
    yield r0, r0, r0 => b0 | b1 | b2

b0:
    return r0

b1:
    return r0

b2:
    unwind.resume
}

function owner {
    continuation.new r2, generate, r0
    continuation.resume r0, r1, r0, r2, r1 => b0 | b1 | b2

b0:
    continuation.destroy r1
    return r0

b1:
    return r0

b2:
    unwind.resume
}

function complete {
    continuation.new r2, generate, r0
    continuation.complete r0, r1, r0, r2, r1 => b0 | b1 | b2

b0:
    continuation.destroy r1
    return r0

b1:
    return r0

b2:
    unwind.resume
}

function settle {
    waiter.queue r2, r0, r1, t2
    return
}

function cancel {
    waiter.cancel r1, r0
    return
}

function resolveTask {
    task.resolve r2, r0, t2
    task.cancel r2
    task.park r2, r1
    return
}

function startTask {
    task.start r1, r0
    task.detach r1
    return
}
"#,
    );

    // retain exact object metadata for suspension and continuation control
    let wait = program.function_by_name("wait");
    let generate = program.function_by_name("generate");
    let owner = program.function_by_name("owner");
    let complete = program.function_by_name("complete");
    let int32 = program.lowered.tree.get(wait).parameters[0].ty;
    assert_eq!(
        object.continuations(),
        &[
            ContinuationSite {
                point: Point::new(owner, 1),
                yielded: Point::new(owner, 2),
                returned: Point::new(owner, 4),
                unwind: Some(Point::new(owner, 5)),
            },
            ContinuationSite {
                point: Point::new(complete, 1),
                yielded: Point::new(complete, 2),
                returned: Point::new(complete, 4),
                unwind: Some(Point::new(complete, 5)),
            },
        ]
    );
    assert_eq!(
        object.suspensions(),
        &[
            SuspensionSite {
                point: Point::new(wait, 0),
                resume: Point::new(wait, 1),
                cancel: Some(Point::new(wait, 2)),
                complete: None,
                unwind: Some(Point::new(wait, 3)),
                operation: Suspension::Await,
                value_type: int32,
                resume_type: int32,
                complete_type: None,
            },
            SuspensionSite {
                point: Point::new(generate, 0),
                resume: Point::new(generate, 1),
                cancel: None,
                complete: Some(Point::new(generate, 2)),
                unwind: Some(Point::new(generate, 3)),
                operation: Suspension::Yield,
                value_type: int32,
                resume_type: int32,
                complete_type: Some(int32),
            },
        ]
    );
}
