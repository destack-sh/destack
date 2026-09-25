use crate::tests::TestProgram;

/// A borrow of local managed storage held across a parking call verifies and is pinned by elaboration.
#[test]
fn test_allow_managed_borrow_across_park() {
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

    program.assert_verified();
}

/// A borrow of local managed storage passed to a parking call verifies.
#[test]
fn test_allow_managed_borrow_passed_to_park() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park<'a>(ref<int32, borrowed, 'a, readonly>): void

function test(v0: ref<Box, managed, mutable, local>): void {
entry(v0: ref<Box, managed, mutable, local>):
    v1: ref<int32, borrowed, 'managed, readonly> = address (*v0).0
    call park(v1): <'a>(ref<int32, borrowed, 'a, readonly>) => void
    return
}
"#,
    );

    program.assert_verified();
}

/// A borrow held across a call that parks transitively verifies.
#[test]
fn test_allow_managed_borrow_across_transitive_park() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): void

function helper(): void {
entry():
    call park(): () => void
    return
}

function test(v0: ref<Box, managed, mutable, local>): int32 {
entry(v0: ref<Box, managed, mutable, local>):
    v1: ref<int32, borrowed, 'managed, readonly> = address (*v0).0
    call helper(): () => void
    v2: int32 = load (*v1)
    return v2
}
"#,
    );

    program.assert_verified();
}

/// A borrow of local managed storage held across a parking invoke verifies and is pinned by elaboration.
#[test]
fn test_allow_managed_borrow_across_parking_invoke() {
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

    program.assert_verified();
}

/// A borrow of unique storage survives a parking call.
#[test]
fn test_allow_unique_borrow_across_park() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): void

function test(v0: ref<Box, unique, mutable>): int32 {
entry(v0: ref<Box, unique, mutable>):
    v1: ref<int32, borrowed, 'frame, readonly> = address (*v0).0
    call park(): () => void
    v2: int32 = load (*v1)
    return v2
}
"#,
    );

    program.assert_verified();
}

/// Keep a borrow of constant storage live across a parking call.
#[test]
fn test_allow_constant_borrow_across_park() {
    let mut program = TestProgram::mir(
        r#"
@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): void

constant MAGIC: int32 = 42

function test(): int32 {
entry:
    v0: ref<int32, borrowed, 'static, readonly> = address @MAGIC
    call park(): () => void
    v1: int32 = load (*v0)
    return v1
}
"#,
    );

    program.assert_verified();
}

/// Keep a readonly borrow of shared heap storage live across a parking call.
#[test]
fn test_allow_shared_readonly_borrow_across_park() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): void

function test(v0: ref<Box, managed, readonly, shared>): int32 {
entry(v0: ref<Box, managed, readonly, shared>):
    v1: ref<int32, borrowed, 'managed, readonly> = address (*v0).0
    call park(): () => void
    v2: int32 = load (*v1)
    return v2
}
"#,
    );

    program.assert_verified();
}
