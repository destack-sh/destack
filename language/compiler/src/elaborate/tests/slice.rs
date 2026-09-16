use crate::tests::TestProgram;

#[test]
fn test_generate_unique_slice_destructor() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test(v0: slice<Box, unique, mutable, local>): void {
entry(v0: slice<Box, unique, mutable, local>):
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test(v0: slice<Box, unique, mutable, local>): void {
entry(v0: slice<Box, unique, mutable, local>):
    release v0
    return
}

function drop.frame<slice<Box, unique, mutable, local>, 'a>(v0: ref<slice<Box, unique, mutable, local>, borrowed, 'a & frame, exclusive>): void {
entry(v0: ref<slice<Box, unique, mutable, local>, borrowed, 'a & frame, exclusive>):
    v1: slice<Box, unique, mutable, local> = load (*v0)
    release v1
    return
}

function drop.local<Box, 'a>(v0: ref<Box, borrowed, 'a & local, exclusive>): void {
entry(v0: ref<Box, borrowed, 'a & local, exclusive>):
    v1: ref<ref<int32, unique, mutable, local>, borrowed, 'a & local, exclusive> = address (*v0).0
    v2: ref<int32, unique, mutable, local> = load (*v1)
    release v2
    return
}
"#,
    );
}

#[test]
fn test_generate_nested_unique_slice_destructor() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test(v0: slice<slice<Box, unique, mutable, local>, unique, mutable, local>): void {
entry(v0: slice<slice<Box, unique, mutable, local>, unique, mutable, local>):
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test(v0: slice<slice<Box, unique, mutable, local>, unique, mutable, local>): void {
entry(v0: slice<slice<Box, unique, mutable, local>, unique, mutable, local>):
    release v0
    return
}

function drop.frame<slice<slice<Box, unique, mutable, local>, unique, mutable, local>, 'a>(v0: ref<slice<slice<Box, unique, mutable, local>, unique, mutable, local>, borrowed, 'a & frame, exclusive>): void {
entry(v0: ref<slice<slice<Box, unique, mutable, local>, unique, mutable, local>, borrowed, 'a & frame, exclusive>):
    v1: slice<slice<Box, unique, mutable, local>, unique, mutable, local> = load (*v0)
    release v1
    return
}

function drop.local<slice<Box, unique, mutable, local>, 'a>(v0: ref<slice<Box, unique, mutable, local>, borrowed, 'a & local, exclusive>): void {
entry(v0: ref<slice<Box, unique, mutable, local>, borrowed, 'a & local, exclusive>):
    v1: slice<Box, unique, mutable, local> = load (*v0)
    release v1
    return
}

function drop.local<Box, 'a>(v0: ref<Box, borrowed, 'a & local, exclusive>): void {
entry(v0: ref<Box, borrowed, 'a & local, exclusive>):
    v1: ref<ref<int32, unique, mutable, local>, borrowed, 'a & local, exclusive> = address (*v0).0
    v2: ref<int32, unique, mutable, local> = load (*v1)
    release v2
    return
}
"#,
    );
}

#[test]
fn test_free_unique_slice_with_dynamic_elements() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Writer {
    write: fn() => uint32;
}

function test(v0: slice<dynamic<Writer, managed, mutable, local>, unique, mutable, local>): void {
entry(v0: slice<dynamic<Writer, managed, mutable, local>, unique, mutable, local>):
    return
}
"#,
    );

    program.assert_optimized(
        r#"
@copy
type Writer {
    write: fn() => uint32;
}

function test(v0: slice<dynamic<Writer, managed, mutable, local>, unique, mutable, local>): void {
entry(v0: slice<dynamic<Writer, managed, mutable, local>, unique, mutable, local>):
    release v0
    return
}
"#,
    );
}
