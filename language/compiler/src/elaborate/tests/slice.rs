use crate::tests::TestProgram;

/// A unique slice of owning elements gains a destructor for its element type.
#[test]
fn test_generate_unique_slice_destructor() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: slice<Box, unique, mutable>): void {
entry(v0: slice<Box, unique, mutable>):
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: slice<Box, unique, mutable>): void {
entry(v0: slice<Box, unique, mutable>):
    release v0
    return
}

function drop.frame<slice<Box, unique, mutable>, 'a>(v0: ref<slice<Box, unique, mutable>, borrowed, 'a, exclusive>): void {
entry(v0: ref<slice<Box, unique, mutable>, borrowed, 'a, exclusive>):
    v1: slice<Box, unique, mutable> = load (*v0)
    release v1
    return
}

function drop.local<Box, 'a>(v0: ref<Box, borrowed, 'a, exclusive>): void {
entry(v0: ref<Box, borrowed, 'a, exclusive>):
    v1: ref<ref<int32, unique, mutable>, borrowed, 'a, exclusive> = address (*v0).0
    v2: ref<int32, unique, mutable> = load (*v1)
    release v2
    return
}

function drop.shared<Box, 'a>(v0: ref<Box, borrowed, 'a, exclusive>): void {
entry(v0: ref<Box, borrowed, 'a, exclusive>):
    v1: ref<ref<int32, unique, mutable>, borrowed, 'a, exclusive> = address (*v0).0
    v2: ref<int32, unique, mutable> = load (*v1)
    release v2
    return
}
"#,
    );
}

/// A unique slice of unique slices gains a destructor at each nesting level.
#[test]
fn test_generate_nested_unique_slice_destructor() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: slice<slice<Box, unique, mutable>, unique, mutable>): void {
entry(v0: slice<slice<Box, unique, mutable>, unique, mutable>):
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: slice<slice<Box, unique, mutable>, unique, mutable>): void {
entry(v0: slice<slice<Box, unique, mutable>, unique, mutable>):
    release v0
    return
}

function drop.frame<slice<slice<Box, unique, mutable>, unique, mutable>, 'a>(v0: ref<slice<slice<Box, unique, mutable>, unique, mutable>, borrowed, 'a, exclusive>): void {
entry(v0: ref<slice<slice<Box, unique, mutable>, unique, mutable>, borrowed, 'a, exclusive>):
    v1: slice<slice<Box, unique, mutable>, unique, mutable> = load (*v0)
    release v1
    return
}

function drop.local<slice<Box, unique, mutable>, 'a>(v0: ref<slice<Box, unique, mutable>, borrowed, 'a, exclusive>): void {
entry(v0: ref<slice<Box, unique, mutable>, borrowed, 'a, exclusive>):
    v1: slice<Box, unique, mutable> = load (*v0)
    release v1
    return
}

function drop.local<Box, 'a>(v0: ref<Box, borrowed, 'a, exclusive>): void {
entry(v0: ref<Box, borrowed, 'a, exclusive>):
    v1: ref<ref<int32, unique, mutable>, borrowed, 'a, exclusive> = address (*v0).0
    v2: ref<int32, unique, mutable> = load (*v1)
    release v2
    return
}

function drop.shared<Box, 'a>(v0: ref<Box, borrowed, 'a, exclusive>): void {
entry(v0: ref<Box, borrowed, 'a, exclusive>):
    v1: ref<ref<int32, unique, mutable>, borrowed, 'a, exclusive> = address (*v0).0
    v2: ref<int32, unique, mutable> = load (*v1)
    release v2
    return
}

function drop.shared<slice<Box, unique, mutable>, 'a>(v0: ref<slice<Box, unique, mutable>, borrowed, 'a, exclusive>): void {
entry(v0: ref<slice<Box, unique, mutable>, borrowed, 'a, exclusive>):
    v1: slice<Box, unique, mutable> = load (*v0)
    release v1
    return
}
"#,
    );
}

/// A unique slice of managed dynamic elements releases without an element destructor.
#[test]
fn test_free_unique_slice_with_dynamic_elements() {
    let mut program = TestProgram::mir(
        r#"
type Writer {
    write: fn() => uint32;
}

function test(v0: slice<dynamic<Writer, managed, mutable, local>, unique, mutable>): void {
entry(v0: slice<dynamic<Writer, managed, mutable, local>, unique, mutable>):
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Writer {
    write: fn() => uint32;
}

function test(v0: slice<dynamic<Writer, managed, mutable, local>, unique, mutable>): void {
entry(v0: slice<dynamic<Writer, managed, mutable, local>, unique, mutable>):
    release v0
    return
}
"#,
    );
}
