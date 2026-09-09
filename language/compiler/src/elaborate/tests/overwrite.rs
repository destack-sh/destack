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
    local.set l0, v0
    v2: ref<Box, borrowed, 'frame, mutable, frame> = local.project l0
    store v2, v1
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
    local.set l0, v0
    v2: ref<Box, borrowed, 'frame, mutable, frame> = local.project l0
    v3: Box = local.get l0
    drop v3
    store v2, v1
    v4: Box = local.get l0
    drop v4
    return
}

function drop.frame<Box, 'a>(v0: ref<Box, borrowed, 'a, mutable, frame>): void {
entry(v0: ref<Box, borrowed, 'a, mutable, frame>):
    v1: ref<ref<int32, unique, mutable, local>, borrowed, 'a, mutable, frame> = field.project v0, 0
    v2: ref<int32, unique, mutable, local> = load v1
    release v2
    return
}
"#,
    );
}

/// A store through a reference hands the old value to the collector in a fresh managed cell.
#[test]
fn test_defer_an_overwritten_value_behind_a_reference_to_the_collector() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test<'a>(v0: ref<Box, borrowed, 'a, mutable, local>, v1: Box): void {
entry(v0: ref<Box, borrowed, 'a, mutable, local>, v1: Box):
    store v0, v1
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test<'a>(v0: ref<Box, borrowed, 'a, mutable, local>, v1: Box): void {
entry(v0: ref<Box, borrowed, 'a, mutable, local>, v1: Box):
    v2: Box = load v0
    v3: ref<Box, managed, mutable, local> = new.zeroed Box
    store v3, v2
    v4: usize = 0
    v5: usize = 8
    barrier.write v3, v4, v5
    store v0, v1
    v6: usize = 0
    v7: usize = 8
    barrier.write v0, v6, v7
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
