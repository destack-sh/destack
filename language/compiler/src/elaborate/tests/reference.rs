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
    value: ref<int32, unique, mutable, local>;
}

function test(v0: ref<Box, unique, mutable, local>): void {
entry(v0: ref<Box, unique, mutable, local>):
    v1: ref<Box, borrowed, exclusive, local> = cast.bit v0 -> ref<Box, borrowed, exclusive, local>
    call drop.local<Box>(v1): (ref<Box, borrowed, exclusive, local>) => void
    free v0
    return
}

function drop.local<Box>(v0: ref<Box, borrowed, exclusive, local>): void {
entry(v0: ref<Box, borrowed, exclusive, local>):
    v1: ref<ref<int32, unique, mutable, local>, borrowed, exclusive, local> = field.project v0, 0
    v2: ref<int32, unique, mutable, local> = load v1
    free v2
    return
}
"#,
    );
}
