use crate::tests::TestProgram;

/// An owned parameter no body uses releases at the function entry.
#[test]
fn test_insert_drop_for_unused_owned_parameter() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}
"#,
    );

    program.assert_optimized(
        r#"
function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    release v0
    return
}
"#,
    );
}

/// A unique slice allocation releases after its last use.
#[test]
fn test_insert_drop_for_unique_slice_allocation() {
    let mut program = TestProgram::mir(
        r#"
function test(): void {
entry:
    v0: int64 = 4
    v1: slice<int32, unique, mutable> = new.slice.zeroed int32, v0, local
    return
}
"#,
    );

    program.assert_optimized(
        r#"
function test(): void {
entry:
    v0: int64 = 4
    v1: slice<int32, unique, mutable> = new.slice.zeroed int32, v0, local
    release v1
    return
}
"#,
    );
}

/// A unique dynamic parameter releases through the runtime.
#[test]
fn test_insert_runtime_drop_for_unique_dynamic() {
    let mut program = TestProgram::mir(
        r#"
type Writer {
    write: fn() => void;
}

function test(v0: dynamic<Writer, unique, mutable>): void {
entry(v0: dynamic<Writer, unique, mutable>):
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Writer {
    write: fn() => void;
}

function test(v0: dynamic<Writer, unique, mutable>): void {
entry(v0: dynamic<Writer, unique, mutable>):
    release v0
    return
}
"#,
    );
}

/// A unique callable parameter releases through the runtime.
#[test]
fn test_insert_runtime_drop_for_unique_function() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: function<() => void, once, unique, mutable>): void {
entry(v0: function<() => void, once, unique, mutable>):
    return
}
"#,
    );

    program.assert_optimized(
        r#"
function test(v0: function<() => void, once, unique, mutable>): void {
entry(v0: function<() => void, once, unique, mutable>):
    release v0
    return
}
"#,
    );
}

/// A template body stays as lowered, its instances planning their own destruction.
#[test]
fn test_leave_a_template_body_to_its_instances() {
    let mut program = TestProgram::mir(
        r#"
function hold<T>(v0: T): void {
entry(v0: T):
    return
}
"#,
    );
    program.assert_optimized(
        r#"
function hold<T>(v0: T): void {
entry(v0: T):
    return
}
"#,
    );
}
