use crate::tests::TestProgram;

#[test]
fn test_insert_drop_drops_unique_pointee_before_free() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test(v0: ref<Box, unique, mutable, local>): void {
entry(v0: ref<Box, unique, mutable, local>):
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test(v0: ref<Box, unique, mutable, local>): void {
entry(v0: ref<Box, unique, mutable, local>):
    v1: ref<Box, borrowed, 'frame, mutable, local> = cast.bit v0 -> ref<Box, borrowed, 'frame, mutable, local>
    call drop.local<Box>(v1): <'a>(ref<Box, borrowed, 'a, mutable, local>) => void
    release v0
    return
}

function drop.local<Box, 'a>(v0: ref<Box, borrowed, 'a, mutable, local>): void {
entry(v0: ref<Box, borrowed, 'a, mutable, local>):
    v1: ref<ref<int32, unique, mutable, local>, borrowed, 'a, mutable, local> = field.project v0, 0
    v2: ref<int32, unique, mutable, local> = load v1
    release v2
    return
}
"#,
    );
}
