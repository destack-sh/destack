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

    program.assert_optimized(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: ref<Box, unique, mutable>): void {
entry(v0: ref<Box, unique, mutable>):
    release v0
    return
}

function drop.local<Box, 'a>(v0: ref<Box, borrowed, 'a, exclusive>): void {
entry(v0: ref<Box, borrowed, 'a, exclusive>):
    v1: ref<ref<int32, unique, mutable>, borrowed, 'a, exclusive> = address (*v0).0
    v2: ref<int32, unique, mutable> = load (*v1)
    release v2
    return
}

function drop.shared<Box, 'a>(v0: ref<Box, borrowed, 'a, exclusive>): void {
entry(v0: ref<Box, borrowed, 'a, exclusive>):
    v1: ref<ref<int32, unique, mutable>, borrowed, 'a, exclusive> = address (*v0).0
    v2: ref<int32, unique, mutable> = load (*v1)
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
    value: ref<int32, unique, mutable>;
}

function test(v0: ref<Box, unique, mutable>): Box {
entry(v0: ref<Box, unique, mutable>):
    v1: Box = load (*v0)
    return v1
}
"#,
    );

    program.assert_optimized(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: ref<Box, unique, mutable>): Box {
entry(v0: ref<Box, unique, mutable>):
    v1: Box = load (*v0)
    v2: ref<uninit<Box>, unique, mutable> = cast.bit v0 -> ref<uninit<Box>, unique, mutable>
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
    first: ref<int32, unique, mutable>;
    second: ref<int32, unique, mutable>;
}

function test(v0: ref<Pair, unique, mutable>): ref<int32, unique, mutable> {
entry(v0: ref<Pair, unique, mutable>):
    v2: ref<int32, unique, mutable> = load (*v0).0
    return v2
}
"#,
    );

    program.assert_optimized(
        r#"
type Pair {
    first: ref<int32, unique, mutable>;
    second: ref<int32, unique, mutable>;
}

function test(v0: ref<Pair, unique, mutable>): ref<int32, unique, mutable> {
entry(v0: ref<Pair, unique, mutable>):
    v2: ref<int32, unique, mutable> = load (*v0).0
    v3: ref<int32, unique, mutable> = load (*v0).1
    release v3
    v4: ref<uninit<Pair>, unique, mutable> = cast.bit v0 -> ref<uninit<Pair>, unique, mutable>
    release v4
    return v2
}
"#,
    );
}
