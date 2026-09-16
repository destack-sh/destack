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
    release v0
    return
}

function drop.local<Box, 'a>(v0: ref<Box, borrowed, 'a & local, exclusive>): void {
entry(v0: ref<Box, borrowed, 'a & local, exclusive>):
    v1: ref<ref<int32, unique, mutable, local>, borrowed, 'a & local, exclusive> = address (*v0).0
    v2: ref<int32, unique, mutable, local> = load (*v1)
    release v2
    return
}
"#,
    );
}

/// Release an allocation after moving its entire pointee into the return value.
#[test]
fn test_release_allocation_after_moving_pointee() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test(v0: ref<Box, unique, mutable, local>): Box {
entry(v0: ref<Box, unique, mutable, local>):
    v1: Box = load (*v0)
    return v1
}
"#,
    );

    program.assert_optimized(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test(v0: ref<Box, unique, mutable, local>): Box {
entry(v0: ref<Box, unique, mutable, local>):
    v1: Box = load (*v0)
    v2: ref<uninit<Box>, unique, mutable, local> = cast.bit v0 -> ref<uninit<Box>, unique, mutable, local>
    release v2
    return v1
}
"#,
    );
}

/// Destroy only the remaining field before releasing a partially moved allocation.
#[test]
fn test_drop_remaining_pointee_fields() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    first: ref<int32, unique, mutable, local>;
    second: ref<int32, unique, mutable, local>;
}

function test(v0: ref<Pair, unique, mutable, local>): ref<int32, unique, mutable, local> {
entry(v0: ref<Pair, unique, mutable, local>):
    v2: ref<int32, unique, mutable, local> = load (*v0).0
    return v2
}
"#,
    );

    program.assert_optimized(
        r#"
type Pair {
    first: ref<int32, unique, mutable, local>;
    second: ref<int32, unique, mutable, local>;
}

function test(v0: ref<Pair, unique, mutable, local>): ref<int32, unique, mutable, local> {
entry(v0: ref<Pair, unique, mutable, local>):
    v2: ref<int32, unique, mutable, local> = load (*v0).0
    v3: ref<int32, unique, mutable, local> = load (*v0).1
    release v3
    v4: ref<uninit<Pair>, unique, mutable, local> = cast.bit v0 -> ref<uninit<Pair>, unique, mutable, local>
    release v4
    return v2
}
"#,
    );
}
