use crate::tests::TestProgram;

/// A loop header free of managed borrows polls the runtime.
#[test]
fn test_poll_at_a_loop_header() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: boolean): void {
entry(v0: boolean):
    jump next

next:
    branch v0 => again | done

again:
    jump next

done:
    return
}
"#,
    );

    program.assert_elaborated(
        r#"
function test(v0: boolean): void {
entry(v0: boolean):
    jump b1

b1:
    poll
    branch v0 => b2 | b3

b2:
    jump b1

b3:
    return
}
"#,
    );
}

/// A loop header inside a local managed borrow does not poll, since a collection could move the referent.
#[test]
fn test_skip_the_poll_at_a_loop_header_inside_a_local_managed_borrow() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Box {
    value: int32;
}

function test(v0: ref<Box, managed, mutable>, v1: boolean): int32 {
entry(v0: ref<Box, managed, mutable>, v1: boolean):
    v2: ref<int32, borrowed, readonly> = field.address v0, 0
    jump next

next:
    branch v1 => again | done

again:
    jump next

done:
    v3: int32 = load v2
    return v3
}
"#,
    );

    program.assert_elaborated(
        r#"
@copy
type Box {
    value: int32;
}

function test(v0: ref<Box, managed, mutable, local>, v1: boolean): int32 {
entry(v0: ref<Box, managed, mutable, local>, v1: boolean):
    v2: ref<int32, borrowed, readonly, local> = field.address v0, 0
    jump b1

b1:
    branch v1 => b2 | b3

b2:
    jump b1

b3:
    v3: int32 = load v2
    return v3
}
"#,
    );
}

/// A tail call is the back edge of a recursive loop, so it polls the runtime before replacing the frame.
#[test]
fn test_poll_before_a_tail_call() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: int32): int32 {
entry(v0: int32):
    tail.call test(v0): (int32) => int32
}
"#,
    );

    program.assert_elaborated(
        r#"
function test(v0: int32): int32 {
entry(v0: int32):
    poll
    tail.call test(v0): (int32) => int32
}
"#,
    );
}

/// A borrow of local managed storage live across a parking call is pinned over the call, since the local heap moves only at safepoints.
#[test]
fn test_pin_a_local_managed_borrow_over_a_parking_call() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): void

function test(v0: ref<Box, managed, mutable>): int32 {
entry(v0: ref<Box, managed, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    call park(): () => void
    v2: int32 = load v1
    return v2
}
"#,
    );

    program.assert_elaborated(
        r#"
@copy
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): void

function test(v0: ref<Box, managed, mutable, local>): int32 {
entry(v0: ref<Box, managed, mutable, local>):
    v1: ref<int32, borrowed, readonly, local> = field.address v0, 0
    v3: ref<int32, borrowed, readonly, local> = pin v1
    call park(): () => void
    unpin v3
    v2: int32 = load v1
    return v2
}
"#,
    );
}

/// A borrow of local managed storage live across a parking invoke is pinned before it and unpinned on each edge out.
#[test]
fn test_pin_a_local_managed_borrow_over_a_parking_invoke() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): int32

function test(v0: ref<Box, managed, mutable>): int32 {
entry(v0: ref<Box, managed, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    invoke park(): () => int32 => resume | cleanup

resume(v2: int32):
    v3: int32 = load v1
    return v3

cleanup:
    unwind.resume
}
"#,
    );

    program.assert_elaborated(
        r#"
@copy
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): int32

function test(v0: ref<Box, managed, mutable, local>): int32 {
entry(v0: ref<Box, managed, mutable, local>):
    v1: ref<int32, borrowed, readonly, local> = field.address v0, 0
    v4: ref<int32, borrowed, readonly, local> = pin v1
    invoke park(): () => int32 => b1 | b2

b1(v2: int32):
    unpin v4
    v3: int32 = load v1
    return v3

b2:
    unpin v4
    unwind.resume
}
"#,
    );
}

/// A borrow of local managed storage that dies before the parking call is left unpinned.
#[test]
fn test_leave_a_dead_local_managed_borrow_unpinned_at_a_park() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): void

function test(v0: ref<Box, managed, mutable>): int32 {
entry(v0: ref<Box, managed, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: int32 = load v1
    call park(): () => void
    return v2
}
"#,
    );

    program.assert_elaborated(
        r#"
@copy
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): void

function test(v0: ref<Box, managed, mutable, local>): int32 {
entry(v0: ref<Box, managed, mutable, local>):
    v1: ref<int32, borrowed, readonly, local> = field.address v0, 0
    v2: int32 = load v1
    call park(): () => void
    return v2
}
"#,
    );
}
