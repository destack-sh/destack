use crate::tests::TestProgram;

#[test]
fn test_insert_drop_for_unused_owned_parameter() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, unique, mutable, local>): void {
entry(v0: ref<int32, unique, mutable, local>):
    return
}
"#,
    );

    program.assert_optimized(
        r#"
function test(v0: ref<int32, unique, mutable, local>): void {
entry(v0: ref<int32, unique, mutable, local>):
    release v0
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_for_unique_slice_allocation() {
    let mut program = TestProgram::mir(
        r#"
function test(): void {
entry:
    v0: int64 = 4
    v1: slice<int32, unique, mutable, local> = new.slice.zeroed int32, v0
    return
}
"#,
    );

    program.assert_optimized(
        r#"
function test(): void {
entry:
    v0: int64 = 4
    v1: slice<int32, unique, mutable, local> = new.slice.zeroed int32, v0
    release v1
    return
}
"#,
    );
}

#[test]
fn test_insert_runtime_drop_for_unique_dynamic() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Writer {
    write: fn() => void;
}

function test(v0: dynamic<Writer, unique, mutable, local>): void {
entry(v0: dynamic<Writer, unique, mutable, local>):
    return
}
"#,
    );

    program.assert_optimized(
        r#"
@copy
type Writer {
    write: fn() => void;
}

function test(v0: dynamic<Writer, unique, mutable, local>): void {
entry(v0: dynamic<Writer, unique, mutable, local>):
    release v0
    return
}
"#,
    );
}

#[test]
fn test_insert_runtime_drop_for_unique_function() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: function<() => void, once, unique, mutable, local>): void {
entry(v0: function<() => void, once, unique, mutable, local>):
    return
}
"#,
    );

    program.assert_optimized(
        r#"
function test(v0: function<() => void, once, unique, mutable, local>): void {
entry(v0: function<() => void, once, unique, mutable, local>):
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
