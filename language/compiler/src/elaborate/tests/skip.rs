use crate::tests::TestProgram;

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

#[test]
fn test_skip_drop_for_managed_allocation() {
    let mut program = TestProgram::mir(
        r#"
function test(): void {
entry:
    v0: ref<int32, managed, mutable, local> = new.zeroed int32
    return
}
"#,
    );

    program.assert_optimized(
        r#"
function test(): void {
entry:
    v0: ref<int32, managed, mutable, local> = new.zeroed int32
    return
}
"#,
    );
}

#[test]
fn test_skip_drop_for_managed_slice_allocation() {
    let mut program = TestProgram::mir(
        r#"
function test(): void {
entry:
    v0: int64 = 4
    v1: slice<int32, managed, mutable, local> = new.slice.zeroed int32, v0
    return
}
"#,
    );

    program.assert_optimized(
        r#"
function test(): void {
entry:
    v0: int64 = 4
    v1: slice<int32, managed, mutable, local> = new.slice.zeroed int32, v0
    return
}
"#,
    );
}

#[test]
fn test_skip_drop_for_borrow_into_managed_field() {
    let mut program = TestProgram::mir(
        r#"
@copy
type User {
    id: int32;
}

function test(v0: ref<User, managed, mutable, local>): int32 {
entry(v0: ref<User, managed, mutable, local>):
    v1: ref<int32, borrowed, 'managed, readonly, local> = address (*v0).0
    v2: int32 = load (*v1)
    return v2
}
"#,
    );

    program.assert_optimized(
        r#"
@copy
type User {
    id: int32;
}

function test(v0: ref<User, managed, mutable, local>): int32 {
entry(v0: ref<User, managed, mutable, local>):
    v1: ref<int32, borrowed, 'managed & local, readonly> = address (*v0).0
    v2: int32 = load (*v1)
    return v2
}
"#,
    );
}

#[test]
fn test_skip_drop_for_borrow_into_managed_slice() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: slice<int32, managed, mutable, local>): int32 {
entry(v0: slice<int32, managed, mutable, local>):
    v1: int64 = 0
    v2: ref<int32, borrowed, 'managed, readonly, local> = address (*v0)[v1]
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
    v2: ref<int32, borrowed, 'managed & local, readonly> = address (*v0)[v1]
    v3: int32 = load (*v2)
    return v3
}
"#,
    );
}

#[test]
fn test_skip_generated_function_for_move_only_copy_fields() {
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

#[test]
fn test_skip_generated_function_for_zero_length_array() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
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
    value: ref<int32, unique, mutable, local>;
}

function test(v0: [Box; 0]): void {
entry(v0: [Box; 0]):
    return
}
"#,
    );
}

#[test]
fn test_skip_drop_after_owned_return() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, unique, mutable, local>): ref<int32, unique, mutable, local> {
entry(v0: ref<int32, unique, mutable, local>):
    return v0
}
"#,
    );

    program.assert_optimized(
        r#"
function test(v0: ref<int32, unique, mutable, local>): ref<int32, unique, mutable, local> {
entry(v0: ref<int32, unique, mutable, local>):
    return v0
}
"#,
    );
}

#[test]
fn test_skip_drop_after_owned_call() {
    let mut program = TestProgram::mir(
        r#"
function consume(v0: ref<int32, unique, mutable, local>): void {
entry(v0: ref<int32, unique, mutable, local>):
    return
}

function test(v0: ref<int32, unique, mutable, local>): void {
entry(v0: ref<int32, unique, mutable, local>):
    call consume(v0): (ref<int32, unique, mutable, local>) => void
    return
}
"#,
    );

    program.assert_optimized(
        r#"
function consume(v0: ref<int32, unique, mutable, local>): void {
entry(v0: ref<int32, unique, mutable, local>):
    release v0
    return
}

function test(v0: ref<int32, unique, mutable, local>): void {
entry(v0: ref<int32, unique, mutable, local>):
    call consume(v0): (ref<int32, unique, mutable, local>) => void
    return
}
"#,
    );
}
