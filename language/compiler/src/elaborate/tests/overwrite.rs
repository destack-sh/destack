use crate::tests::TestProgram;

/// A store over a frame-rooted owned value drops the old value before the write.
#[test]
fn test_drop_an_overwritten_frame_value_before_the_store() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test(v0: Box, v1: Box): void {
    local l0: Box

entry(v0: Box, v1: Box):
    store l0, v0
    v2: ref<Box, borrowed, 'frame, mutable, frame> = address l0
    store (*v2), v1
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test(v0: Box, v1: Box): void {
    local l0: Box

entry(v0: Box, v1: Box):
    store l0, v0
    v2: ref<Box, borrowed, 'frame & frame, mutable> = address l0
    v3: Box = load l0
    drop v3
    store (*v2), v1
    v4: Box = load l0
    drop v4
    return
}

function drop.frame<Box, 'a>(v0: ref<Box, borrowed, 'a & frame, exclusive>): void {
entry(v0: ref<Box, borrowed, 'a & frame, exclusive>):
    v1: ref<ref<int32, unique, mutable, local>, borrowed, 'a & frame, exclusive> = address (*v0).0
    v2: ref<int32, unique, mutable, local> = load (*v1)
    release v2
    return
}
"#,
    );
}

/// A store through a reference drops the value it overwrites before the store.
#[test]
fn test_drop_an_overwritten_value_behind_a_reference_before_the_store() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test<'a>(v0: ref<Box, borrowed, 'a, mutable, local>, v1: Box): void {
entry(v0: ref<Box, borrowed, 'a, mutable, local>, v1: Box):
    store (*v0), v1
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test<'a>(v0: ref<Box, borrowed, 'a & local, mutable>, v1: Box): void {
entry(v0: ref<Box, borrowed, 'a & local, mutable>, v1: Box):
    v2: Box = load (*v0)
    drop v2
    v3: ref<Box, raw, mutable, local> = address (*v0)
    store (*v3), v1
    v4: usize = 0
    v5: usize = 8
    barrier.write v3, v4, v5
    return
}

function drop.frame<Box, 'a>(v0: ref<Box, borrowed, 'a & frame, exclusive>): void {
entry(v0: ref<Box, borrowed, 'a & frame, exclusive>):
    v1: ref<ref<int32, unique, mutable, local>, borrowed, 'a & frame, exclusive> = address (*v0).0
    v2: ref<int32, unique, mutable, local> = load (*v1)
    release v2
    return
}
"#,
    );
}
