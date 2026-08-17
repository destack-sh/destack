use crate::tests::TestProgram;

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
    value: ref<int32, unique, mutable>;
}

external function dropBox(ref<Box, borrowed, exclusive, frame>): void

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: Box = aggregate (v0)
    drop v1
    return
}

function drop.frame<Box>(v0: ref<Box, borrowed, exclusive, frame>): void {
entry(v0: ref<Box, borrowed, exclusive, frame>):
    call dropBox(v0): (ref<Box, borrowed, exclusive, frame>) => void
    v1: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v0, 0
    v2: ref<int32, unique, mutable> = load v1
    free v2
    return
}
"#,
    );
}

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
    value: ref<int32, unique, mutable>;
}

function dropBox(v0: ref<Box, borrowed, exclusive, frame>): void {
entry(v0: ref<Box, borrowed, exclusive, frame>):
    return
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: Box = aggregate (v0)
    drop v1
    return
}

function drop.frame<Box>(v0: ref<Box, borrowed, exclusive, frame>): void {
entry(v0: ref<Box, borrowed, exclusive, frame>):
    call dropBox(v0): (ref<Box, borrowed, exclusive, frame>) => void
    v1: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v0, 0
    v2: ref<int32, unique, mutable> = load v1
    free v2
    return
}
"#,
    );
}
