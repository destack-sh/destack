use crate::tests::TestProgram;

/// A managed parameter belongs to the collector, so elaboration adds no drop.
#[test]
fn test_skip_drop_for_managed_parameter() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, managed, mutable, local>): void {
entry(v0: ref<int32, managed, mutable, local>):
    return
}
"#,
    );

    program.assert_optimized(
        r#"
function test(v0: ref<int32, managed, mutable, local>): void {
entry(v0: ref<int32, managed, mutable, local>):
    return
}
"#,
    );
}

/// A managed allocation belongs to the collector, so elaboration adds no drop.
#[test]
fn test_skip_drop_for_managed_allocation() {
    let mut program = TestProgram::mir(
        r#"
function test(): void {
entry:
    v0: ref<int32, managed, mutable, local> = new.zeroed int32, local
    return
}
"#,
    );

    program.assert_optimized(
        r#"
function test(): void {
entry:
    v0: ref<int32, managed, mutable, local> = new.zeroed int32, local
    return
}
"#,
    );
}

/// A managed slice allocation belongs to the collector, so elaboration adds no drop.
#[test]
fn test_skip_drop_for_managed_slice_allocation() {
    let mut program = TestProgram::mir(
        r#"
function test(): void {
entry:
    v0: int64 = 4
    v1: slice<int32, managed, mutable, local> = new.slice.zeroed int32, v0, local
    return
}
"#,
    );

    program.assert_optimized(
        r#"
function test(): void {
entry:
    v0: int64 = 4
    v1: slice<int32, managed, mutable, local> = new.slice.zeroed int32, v0, local
    return
}
"#,
    );
}

/// A borrow into a managed field owns nothing, so elaboration adds no drop.
#[test]
fn test_skip_drop_for_borrow_into_managed_field() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test(v0: ref<User, managed, mutable, local>): int32 {
entry(v0: ref<User, managed, mutable, local>):
    v1: ref<int32, borrowed, 'managed, readonly> = address (*v0).0
    v2: int32 = load (*v1)
    return v2
}
"#,
    );

    program.assert_optimized(
        r#"
type User {
    id: int32;
}

function test(v0: ref<User, managed, mutable, local>): int32 {
entry(v0: ref<User, managed, mutable, local>):
    v1: ref<int32, borrowed, 'managed, readonly> = address (*v0).0
    v2: int32 = load (*v1)
    return v2
}
"#,
    );
}

/// A borrow into a managed slice owns nothing, so elaboration adds no drop.
#[test]
fn test_skip_drop_for_borrow_into_managed_slice() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: slice<int32, managed, mutable, local>): int32 {
entry(v0: slice<int32, managed, mutable, local>):
    v1: int64 = 0
    v2: ref<int32, borrowed, 'managed, readonly> = address (*v0)[v1]
    v3: int32 = load (*v2)
    return v3
}
"#,
    );

    program.assert_optimized(
        r#"
function test(v0: slice<int32, managed, mutable, local>): int32 {
entry(v0: slice<int32, managed, mutable, local>):
    v1: int64 = 0
    v2: ref<int32, borrowed, 'managed, readonly> = address (*v0)[v1]
    v3: int32 = load (*v2)
    return v3
}
"#,
    );
}

/// A struct whose fields all copy needs no generated destructor.
#[test]
fn test_skip_generated_destructor_for_all_copy_fields() {
    let mut program = TestProgram::mir(
        r#"
type Point {
    x: int32;
    y: int32;
}

function test(v0: Point): void {
entry(v0: Point):
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Point {
    x: int32;
    y: int32;
}

function test(v0: Point): void {
entry(v0: Point):
    return
}
"#,
    );
}

/// A zero-length array holds no element, so it needs no generated destructor.
#[test]
fn test_skip_generated_function_for_zero_length_array() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: [Box; 0]): void {
entry(v0: [Box; 0]):
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: [Box; 0]): void {
entry(v0: [Box; 0]):
    return
}
"#,
    );
}

/// A returned owner transfers to the caller, so elaboration adds no drop.
#[test]
fn test_skip_drop_after_owned_return() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, unique, mutable>): ref<int32, unique, mutable> {
entry(v0: ref<int32, unique, mutable>):
    return v0
}
"#,
    );

    program.assert_optimized(
        r#"
function test(v0: ref<int32, unique, mutable>): ref<int32, unique, mutable> {
entry(v0: ref<int32, unique, mutable>):
    return v0
}
"#,
    );
}

/// An owner passed to a call transfers to the callee, so elaboration adds no drop.
#[test]
fn test_skip_drop_after_owned_call() {
    let mut program = TestProgram::mir(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    call consume(v0): (ref<int32, unique, mutable>) => void
    return
}
"#,
    );

    program.assert_optimized(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    release v0
    return
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    call consume(v0): (ref<int32, unique, mutable>) => void
    return
}
"#,
    );
}
