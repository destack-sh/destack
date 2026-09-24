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

    program.assert_optimized(
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

/// A loop header inside a managed borrow polls and holds the borrow's handle past the poll.
///
/// The collector may run at the poll.
#[test]
fn test_poll_and_hold_the_handle_at_a_loop_header_inside_a_managed_borrow() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(v0: ref<Box, managed, mutable, local>, v1: boolean): int32 {
entry(v0: ref<Box, managed, mutable, local>, v1: boolean):
    v2: ref<int32, borrowed, 'managed, readonly> = address (*v0).0
    jump next

next:
    branch v1 => again | done

again:
    jump next

done:
    v3: int32 = load (*v2)
    return v3
}
"#,
    );

    program.assert_optimized(
        r#"
type Box {
    value: int32;
}

function test(v0: ref<Box, managed, mutable, local>, v1: boolean): int32 {
entry(v0: ref<Box, managed, mutable, local>, v1: boolean):
    v2: ref<int32, borrowed, 'managed, readonly> = address (*v0).0
    jump b1

b1:
    poll
    branch v1 => b2 | b3

b2:
    jump b1

b3:
    v3: int32 = load (*v2)
    return v3
}
"#,
    );
}

/// A tail call is the back edge of a recursive loop.
///
/// It polls the runtime before replacing the frame.
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

    program.assert_optimized(
        r#"
function test(v0: int32): int32 {
entry(v0: int32):
    poll
    tail.call test(v0): (int32) => int32
}
"#,
    );
}

/// The handle a managed borrow live across a parking call was taken through is held past the call.
///
/// The frame state at the park then roots its storage.
#[test]
fn test_hold_the_handle_of_a_managed_borrow_over_a_parking_call() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): void

function test(v0: ref<Box, managed, mutable, local>): int32 {
entry(v0: ref<Box, managed, mutable, local>):
    v1: ref<int32, borrowed, 'managed, readonly> = address (*v0).0
    call park(): () => void
    v2: int32 = load (*v1)
    return v2
}
"#,
    );

    program.assert_optimized(
        r#"
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): void

function test(v0: ref<Box, managed, mutable, local>): int32 {
entry(v0: ref<Box, managed, mutable, local>):
    v1: ref<int32, borrowed, 'managed, readonly> = address (*v0).0
    call park(): () => void
    v2: int32 = load (*v1)
    return v2
}
"#,
    );
}

/// The handle of a managed borrow live across a parking invoke is held on each edge out of it.
#[test]
fn test_hold_the_handle_of_a_managed_borrow_on_each_edge_out_of_a_parking_invoke() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): int32

function test(v0: ref<Box, managed, mutable, local>): int32 {
entry(v0: ref<Box, managed, mutable, local>):
    v1: ref<int32, borrowed, 'managed, readonly> = address (*v0).0
    invoke park(): () => int32 => resume | cleanup

resume(v2: int32):
    v3: int32 = load (*v1)
    return v3

cleanup:
    unwind.resume
}
"#,
    );

    program.assert_optimized(
        r#"
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): int32

function test(v0: ref<Box, managed, mutable, local>): int32 {
entry(v0: ref<Box, managed, mutable, local>):
    v1: ref<int32, borrowed, 'managed, readonly> = address (*v0).0
    invoke park(): () => int32 => b1 | b2

b1(v2: int32):
    v3: int32 = load (*v1)
    return v3

b2:
    unwind.resume
}
"#,
    );
}

/// A managed borrow that dies before the parking call holds nothing.
#[test]
fn test_hold_nothing_for_a_managed_borrow_dead_before_a_park() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): void

function test(v0: ref<Box, managed, mutable, local>): int32 {
entry(v0: ref<Box, managed, mutable, local>):
    v1: ref<int32, borrowed, 'managed, readonly> = address (*v0).0
    v2: int32 = load (*v1)
    call park(): () => void
    return v2
}
"#,
    );

    program.assert_optimized(
        r#"
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): void

function test(v0: ref<Box, managed, mutable, local>): int32 {
entry(v0: ref<Box, managed, mutable, local>):
    v1: ref<int32, borrowed, 'managed, readonly> = address (*v0).0
    v2: int32 = load (*v1)
    call park(): () => void
    return v2
}
"#,
    );
}

/// A borrowed parameter live across a parking call holds nothing.
///
/// Its owner lives in the caller's frame.
#[test]
fn test_hold_nothing_for_a_borrowed_parameter_across_a_park() {
    let mut program = TestProgram::mir(
        r#"
@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): void

function test<'a>(v0: ref<int32, borrowed, 'a, readonly>): int32 {
entry(v0: ref<int32, borrowed, 'a, readonly>):
    call park(): () => void
    v1: int32 = load (*v0)
    return v1
}
"#,
    );

    program.assert_optimized(
        r#"
@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): void

function test<'a>(v0: ref<int32, borrowed, 'a, readonly>): int32 {
entry(v0: ref<int32, borrowed, 'a, readonly>):
    call park(): () => void
    v1: int32 = load (*v0)
    return v1
}
"#,
    );
}
