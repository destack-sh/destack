use crate::tests::TestProgram;

/// A type with a drop hook gets a generated drop that calls the hook before freeing its fields.
#[test]
fn test_insert_drop_calls_destructor_for_hook() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

external function dropBox<'a>(ref<Box, borrowed, 'a, mutable, frame>): void

function test(v0: ref<int32, unique, mutable, local>): void {
entry(v0: ref<int32, unique, mutable, local>):
    v1: Box = aggregate (v0)
    return
}
"#,
    );
    program.mark_drop_hook("Box", "dropBox");

    program.assert_optimized(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

external function dropBox<'a>(ref<Box, borrowed, 'a, mutable, frame>): void

function test(v0: ref<int32, unique, mutable, local>): void {
entry(v0: ref<int32, unique, mutable, local>):
    v1: Box = aggregate (v0)
    drop v1
    return
}

function drop.frame<Box, 'a>(v0: ref<Box, borrowed, 'a, mutable, frame>): void {
entry(v0: ref<Box, borrowed, 'a, mutable, frame>):
    call dropBox(v0): <'a>(ref<Box, borrowed, 'a, mutable, frame>) => void
    v1: ref<ref<int32, unique, mutable, local>, borrowed, 'a, mutable, frame> = field.project v0, 0
    v2: ref<int32, unique, mutable, local> = load v1
    release v2
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
    value: ref<int32, unique, mutable, local>;
}

function dropBox<'a>(v0: ref<Box, borrowed, 'a, mutable, frame>): void {
entry(v0: ref<Box, borrowed, 'a, mutable, frame>):
    return
}

function test(v0: ref<int32, unique, mutable, local>): void {
entry(v0: ref<int32, unique, mutable, local>):
    v1: Box = aggregate (v0)
    return
}
"#,
    );
    program.mark_drop_hook("Box", "dropBox");

    program.assert_optimized(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function dropBox<'a>(v0: ref<Box, borrowed, 'a, mutable, frame>): void {
entry(v0: ref<Box, borrowed, 'a, mutable, frame>):
    return
}

function test(v0: ref<int32, unique, mutable, local>): void {
entry(v0: ref<int32, unique, mutable, local>):
    v1: Box = aggregate (v0)
    drop v1
    return
}

function drop.frame<Box, 'a>(v0: ref<Box, borrowed, 'a, mutable, frame>): void {
entry(v0: ref<Box, borrowed, 'a, mutable, frame>):
    call dropBox(v0): <'a>(ref<Box, borrowed, 'a, mutable, frame>) => void
    v1: ref<ref<int32, unique, mutable, local>, borrowed, 'a, mutable, frame> = field.project v0, 0
    v2: ref<int32, unique, mutable, local> = load v1
    release v2
    return
}
"#,
    );
}
