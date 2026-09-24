use crate::tests::TestProgram;

/// An owner releases right after the last operation reading it.
#[test]
fn test_insert_drop_after_last_owned_use() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, unique, mutable>): int32 {
entry(v0: ref<int32, unique, mutable>):
    v1: int32 = load (*v0)
    return v1
}
"#,
    );

    program.assert_optimized(
        r#"
function test(v0: ref<int32, unique, mutable>): int32 {
entry(v0: ref<int32, unique, mutable>):
    v1: int32 = load (*v0)
    release v0
    return v1
}
"#,
    );
}

/// An owner releases before a later call that reaches none of its storage.
#[test]
fn test_insert_drop_before_later_unrelated_use() {
    let mut program = TestProgram::mir(
        r#"
function later(): void {
entry:
    return
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: int32 = load (*v0)
    call later(): () => void
    return
}
"#,
    );

    program.assert_optimized(
        r#"
function later(): void {
entry:
    return
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: int32 = load (*v0)
    release v0
    call later(): () => void
    return
}
"#,
    );
}

/// An owner stays live until the last use of a borrow into it.
#[test]
fn test_insert_drop_after_last_interior_borrow_use() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(v0: ref<Box, unique, mutable>): int32 {
entry(v0: ref<Box, unique, mutable>):
    v1: ref<int32, borrowed, 'frame, readonly> = address (*v0).0
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

function test(v0: ref<Box, unique, mutable>): int32 {
entry(v0: ref<Box, unique, mutable>):
    v1: ref<int32, borrowed, 'frame, readonly> = address (*v0).0
    v2: int32 = load (*v1)
    release v0
    return v2
}
"#,
    );
}

/// An owner borrowed across a parking call releases after the borrow's last use.
#[test]
fn test_keep_owner_alive_across_park() {
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

    program.assert_optimized(
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
    release v0
    return v2
}
"#,
    );
}

/// An owner stays live while an aggregate carries a borrow into it across a block.
#[test]
fn test_keep_owner_alive_through_aggregate_borrow() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

type Holder<'a> {
    value: ref<int32, borrowed, 'a, readonly>;
}

function test(v0: ref<Box, unique, mutable>): int32 {
entry(v0: ref<Box, unique, mutable>):
    v1: ref<int32, borrowed, 'frame, readonly> = address (*v0).0
    v2: Holder<'frame> = aggregate (v1)
    jump b1(v2)

b1(v3: Holder<'frame>):
    v4: ref<int32, borrowed, 'frame, readonly> = field.get v3, 0
    v5: int32 = load (*v4)
    return v5
}
"#,
    );

    program.assert_optimized(
        r#"
type Box {
    value: int32;
}

type Holder<'a> {
    value: ref<int32, borrowed, 'a, readonly>;
}

function test(v0: ref<Box, unique, mutable>): int32 {
entry(v0: ref<Box, unique, mutable>):
    v1: ref<int32, borrowed, 'frame, readonly> = address (*v0).0
    v2: Holder<'frame> = aggregate (v1)
    jump b1(v2)

b1(v3: Holder<'frame>):
    v4: ref<int32, borrowed, 'frame, readonly> = field.get v3, 0
    v5: int32 = load (*v4)
    release v0
    return v5
}
"#,
    );
}
