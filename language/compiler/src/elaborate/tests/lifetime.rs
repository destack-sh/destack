use crate::tests::TestProgram;

#[test]
fn test_insert_drop_after_last_owned_use() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, unique, mutable>): int32 {
entry(v0: ref<int32, unique, mutable>):
    v1: int32 = load v0
    return v1
}
"#,
    );

    program.assert_elaborated(
        r#"
function test(v0: ref<int32, unique, mutable>): int32 {
entry(v0: ref<int32, unique, mutable>):
    v1: int32 = load v0
    free v0
    return v1
}
"#,
    );
}

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
    v1: int32 = load v0
    call later(): () => void
    return
}
"#,
    );

    program.assert_elaborated(
        r#"
function later(): void {
entry:
    return
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: int32 = load v0
    free v0
    call later(): () => void
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_after_last_interior_borrow_use() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Box {
    value: int32;
}

function test(v0: ref<Box, unique, mutable>): int32 {
entry(v0: ref<Box, unique, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
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

function test(v0: ref<Box, unique, mutable>): int32 {
entry(v0: ref<Box, unique, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: int32 = load v1
    free v0
    return v2
}
"#,
    );
}

#[test]
fn test_keep_owner_alive_across_park() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): void

function test(v0: ref<Box, unique, mutable>): int32 {
entry(v0: ref<Box, unique, mutable>):
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

function test(v0: ref<Box, unique, mutable>): int32 {
entry(v0: ref<Box, unique, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    call park(): () => void
    v2: int32 = load v1
    free v0
    return v2
}
"#,
    );
}

#[test]
fn test_keep_owner_alive_through_aggregate_borrow() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Box {
    value: int32;
}

@copy
type Holder {
    value: ref<int32, borrowed, readonly>;
}

function test(v0: ref<Box, unique, mutable>): int32 {
entry(v0: ref<Box, unique, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: Holder = aggregate (v1)
    jump b1(v2)

b1(v3: Holder):
    v4: ref<int32, borrowed, readonly> = field.get v3, 0
    v5: int32 = load v4
    return v5
}
"#,
    );

    program.assert_elaborated(
        r#"
@copy
type Box {
    value: int32;
}

@copy
type Holder {
    value: ref<int32, borrowed, readonly>;
}

function test(v0: ref<Box, unique, mutable>): int32 {
entry(v0: ref<Box, unique, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: Holder = aggregate (v1)
    jump b1(v2)

b1(v3: Holder):
    v4: ref<int32, borrowed, readonly> = field.get v3, 0
    v5: int32 = load v4
    free v0
    return v5
}
"#,
    );
}
