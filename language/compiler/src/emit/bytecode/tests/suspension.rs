use crate::tests::TestProgram;

/// Emit await and yield suspension into explicit continuation edges.
#[test]
fn test_emit_suspension() {
    let program = TestProgram::mir(
        r#"
external function park(int32, waiter<int32>): void

async function wait(v0: int32): int32 {
entry(v0: int32):
    await park(v0) => resumed | cancelled | failed

resumed(v1: int32):
    return v1

cancelled:
    return

failed:
    unwind.resume
}

function* generate(v0: int32): int32 {
entry(v0: int32):
    yield v0 => resumed | failed

resumed(v1: int32):
    return v1

failed:
    unwind.resume
}

function owner(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: continuation<int32, int32, int32> = continuation.new generate(v0)
    resume v2(v1) => yielded | returned | failed

yielded(v3: int32, v4: continuation<int32, int32, int32>):
    return v3

returned(v5: int32):
    return v5

failed:
    unwind.resume
}

function settle(v0: waiter<int32>, v1: int32): void {
entry(v0: waiter<int32>, v1: int32):
    waiter.queue v0, v1
    return
}

function cancel(v0: waiter<int32>): void {
entry(v0: waiter<int32>):
    waiter.cancel v0
    return
}
"#,
    );

    program.assert_bytecode(
        r#"
function park(r0: t1, r1: t2): t0

async function wait(r0: t1): t1 {
    await r0, park, r0 => b0 | b1 | b2

b0:
    return r0

b1:
    return

b2:
    unwind.resume
}

function* generate(r0: t1): t1 {
    yield r0, r0 => b0 | b1

b0:
    return r0

b1:
    unwind.resume
}

function owner(r0: t1, r1: t1): t1 {
    continuation.new r2, generate, r0
    resume r0, r1, r0, r2, r1 => b0 | b1 | b2

b0:
    return r0

b1:
    return r0

b2:
    unwind.resume
}

function settle(r0: t2, r1: t1): t0 {
    waiter.queue r0, r1, t1
    return
}

function cancel(r0: t2): t0 {
    waiter.cancel r0
    return
}
"#,
    );
}
