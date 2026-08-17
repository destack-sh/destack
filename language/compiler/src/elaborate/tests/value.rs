use crate::tests::TestProgram;

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

    program.assert_elaborated(
        r#"
function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    free v0
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
    v1: slice<int32, unique, mutable> = new.slice.zeroed int32, v0
    return
}
"#,
    );

    program.assert_elaborated(
        r#"
function test(): void {
entry:
    v0: int64 = 4
    v1: slice<int32, unique, mutable> = new.slice.zeroed int32, v0
    free v1
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

function test(v0: dynamic<Writer, unique, mutable>): void {
entry(v0: dynamic<Writer, unique, mutable>):
    return
}
"#,
    );

    program.assert_elaborated(
        r#"
@copy
type Writer {
    write: fn() => void;
}

function test(v0: dynamic<Writer, unique, mutable>): void {
entry(v0: dynamic<Writer, unique, mutable>):
    drop v0
    free v0
    return
}
"#,
    );
}

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

    program.assert_elaborated(
        r#"
function test(v0: function<() => void, once, unique, mutable>): void {
entry(v0: function<() => void, once, unique, mutable>):
    drop v0
    free v0
    return
}
"#,
    );
}
