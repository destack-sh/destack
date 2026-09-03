use crate::tests::TestProgram;

/// A type with a drop hook gets a generated drop that calls the hook before freeing its fields.
#[test]
fn test_insert_drop_calls_destructor_for_hook() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

external function dropBox(ref<Box, borrowed, exclusive, frame>): void

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: Box = aggregate (v0)
    return
}
"#,
    );
    program.mark_drop_hook("Box", "dropBox");

    program.assert_elaborated(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

external function dropBox(ref<Box, borrowed, exclusive, frame>): void

function test(v0: ref<int32, unique, mutable, local>): void {
entry(v0: ref<int32, unique, mutable, local>):
    v1: Box = aggregate (v0)
    drop v1
    return
}

function drop.frame<Box>(v0: ref<Box, borrowed, exclusive, frame>): void {
entry(v0: ref<Box, borrowed, exclusive, frame>):
    call dropBox(v0): (ref<Box, borrowed, exclusive, frame>) => void
    v1: ref<ref<int32, unique, mutable, local>, borrowed, exclusive, frame> = field.project v0, 0
    v2: ref<int32, unique, mutable, local> = load v1
    free v2
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

function dropBox(v0: ref<Box, borrowed, exclusive, frame>): void {
entry(v0: ref<Box, borrowed, exclusive, frame>):
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

    program.assert_elaborated(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function dropBox(v0: ref<Box, borrowed, exclusive, frame>): void {
entry(v0: ref<Box, borrowed, exclusive, frame>):
    return
}

function test(v0: ref<int32, unique, mutable, local>): void {
entry(v0: ref<int32, unique, mutable, local>):
    v1: Box = aggregate (v0)
    drop v1
    return
}

function drop.frame<Box>(v0: ref<Box, borrowed, exclusive, frame>): void {
entry(v0: ref<Box, borrowed, exclusive, frame>):
    call dropBox(v0): (ref<Box, borrowed, exclusive, frame>) => void
    v1: ref<ref<int32, unique, mutable, local>, borrowed, exclusive, frame> = field.project v0, 0
    v2: ref<int32, unique, mutable, local> = load v1
    free v2
    return
}
"#,
    );
}
