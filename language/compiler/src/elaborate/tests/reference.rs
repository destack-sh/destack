use crate::tests::TestProgram;

#[test]
fn test_insert_drop_drops_unique_pointee_before_free() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: ref<Box, unique, mutable>): void {
entry(v0: ref<Box, unique, mutable>):
    return
}
"#,
    );

    program.assert_elaborated(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: ref<Box, unique, mutable>): void {
entry(v0: ref<Box, unique, mutable>):
    v1: ref<Box, borrowed, exclusive> = cast.bit v0 -> ref<Box, borrowed, exclusive>
    call drop.local<Box>(v1): (ref<Box, borrowed, exclusive>) => void
    free v0
    return
}

function drop.local<Box>(v0: ref<Box, borrowed, exclusive>): void {
entry(v0: ref<Box, borrowed, exclusive>):
    v1: ref<ref<int32, unique, mutable>, borrowed, exclusive> = field.address v0, 0
    v2: ref<int32, unique, mutable> = load v1
    free v2
    return
}
"#,
    );
}
