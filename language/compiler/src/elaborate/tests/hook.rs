use crate::tests::TestProgram;

/// A type with a drop hook gets a generated drop that calls the hook before freeing its fields.
#[test]
fn test_insert_drop_calls_destructor_for_hook() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

external function dropBox<'a>(ref<Box, borrowed, 'a, mutable>): void

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: Box = aggregate (v0)
    return
}
"#,
    );
    program.mark_drop_hook("Box", "dropBox");

    program.assert_optimized(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

external function dropBox<'a>(ref<Box, borrowed, 'a, mutable>): void

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: Box = aggregate (v0)
    drop v1
    return
}

function drop.frame<Box, 'a>(v0: ref<Box, borrowed, 'a, exclusive>): void {
entry(v0: ref<Box, borrowed, 'a, exclusive>):
    v1: ref<Box, borrowed, 'a, mutable> = address (*v0)
    call dropBox(v1): <'a_1>(ref<Box, borrowed, 'a_1, mutable>) => void
    v2: ref<ref<int32, unique, mutable>, borrowed, 'a, exclusive> = address (*v0).0
    v3: ref<int32, unique, mutable> = load (*v2)
    release v3
    return
}
"#,
    );
}

/// A drop hook's own body keeps its borrowed receiver as written.
#[test]
fn test_insert_drop_skips_receiver_inside_hook() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function dropBox<'a>(v0: ref<Box, borrowed, 'a, mutable>): void {
entry(v0: ref<Box, borrowed, 'a, mutable>):
    return
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: Box = aggregate (v0)
    return
}
"#,
    );
    program.mark_drop_hook("Box", "dropBox");

    program.assert_optimized(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function dropBox<'a>(v0: ref<Box, borrowed, 'a, mutable>): void {
entry(v0: ref<Box, borrowed, 'a, mutable>):
    return
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: Box = aggregate (v0)
    drop v1
    return
}

function drop.frame<Box, 'a>(v0: ref<Box, borrowed, 'a, exclusive>): void {
entry(v0: ref<Box, borrowed, 'a, exclusive>):
    v1: ref<Box, borrowed, 'a, mutable> = address (*v0)
    call dropBox(v1): <'a_1>(ref<Box, borrowed, 'a_1, mutable>) => void
    v2: ref<ref<int32, unique, mutable>, borrowed, 'a, exclusive> = address (*v0).0
    v3: ref<int32, unique, mutable> = load (*v2)
    release v3
    return
}
"#,
    );
}
