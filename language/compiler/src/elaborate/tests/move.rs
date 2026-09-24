use crate::tests::TestProgram;

/// Moving one field out of an aggregate frees the sibling field that stays behind.
#[test]
fn test_drop_sibling_after_partial_move() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: Pair): void {
entry(v0: Pair):
    v1: ref<int32, unique, mutable> = field.get v0, 0
    call consume(v1): (ref<int32, unique, mutable>) => void
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    release v0
    return
}

function test(v0: Pair): void {
entry(v0: Pair):
    v1: ref<int32, unique, mutable> = field.get v0, 0
    v2: ref<int32, unique, mutable> = field.get v0, 1
    release v2
    call consume(v1): (ref<int32, unique, mutable>) => void
    return
}
"#,
    );
}

/// Setting a field frees the value it overwrites before the aggregate is rebuilt.
#[test]
fn test_drop_replaced_field_before_reconstruction() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: Pair, v1: ref<int32, unique, mutable>): void {
entry(v0: Pair, v1: ref<int32, unique, mutable>):
    v2: Pair = field.set v0, 0, v1
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: Pair, v1: ref<int32, unique, mutable>): void {
entry(v0: Pair, v1: ref<int32, unique, mutable>):
    v3: ref<int32, unique, mutable> = field.get v0, 0
    release v3
    v2: Pair = field.set v0, 0, v1
    drop v2
    return
}

function drop.frame<Pair, 'a>(v0: ref<Pair, borrowed, 'a, exclusive>): void {
entry(v0: ref<Pair, borrowed, 'a, exclusive>):
    v1: ref<ref<int32, unique, mutable>, borrowed, 'a, exclusive> = address (*v0).1
    v2: ref<int32, unique, mutable> = load (*v1)
    release v2
    v3: ref<ref<int32, unique, mutable>, borrowed, 'a, exclusive> = address (*v0).0
    v4: ref<int32, unique, mutable> = load (*v3)
    release v4
    return
}
"#,
    );
}

/// A local initialized on one path is freed on each path before the branches join.
#[test]
fn test_drop_path_dependent_local_before_join() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, unique, mutable>, v1: boolean): void {
    local l0: ref<int32, unique, mutable>

entry(v0: ref<int32, unique, mutable>, v1: boolean):
    branch v1 => initialize | skip

initialize:
    store l0, v0
    jump done

skip:
    jump done

done:
    return
}
"#,
    );

    program.assert_optimized(
        r#"
function test(v0: ref<int32, unique, mutable>, v1: boolean): void {
    local l0: ref<int32, unique, mutable>

entry(v0: ref<int32, unique, mutable>, v1: boolean):
    branch v1 => b1 | b2

b1:
    store l0, v0
    v2: ref<int32, unique, mutable> = load l0
    release v2
    jump b3

b2:
    release v0
    jump b3

b3:
    return
}
"#,
    );
}

/// Each branch edge frees the value the other edge hands to the join.
#[test]
fn test_drop_edge_dependent_values_before_join() {
    let mut program = TestProgram::mir(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: boolean): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: boolean):
    branch v2 => done(v0) | done(v1)

done(v3: ref<int32, unique, mutable>):
    call consume(v3): (ref<int32, unique, mutable>) => void
    return
}
"#,
    );

    program.assert_optimized(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    release v0
    return
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: boolean): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: boolean):
    branch v2 => b2(v0) | b1(v1)

b1(v5: ref<int32, unique, mutable>):
    release v0
    jump b3(v5)

b2(v4: ref<int32, unique, mutable>):
    release v1
    jump b3(v4)

b3(v3: ref<int32, unique, mutable>):
    call consume(v3): (ref<int32, unique, mutable>) => void
    return
}
"#,
    );
}

/// An owner outlives a borrow of it that escapes through a stored alias.
#[test]
fn test_retain_owner_after_borrow_escapes_through_alias() {
    let mut program = TestProgram::mir(
        r#"
type Owner {
    value: int32;
}

type View<'a> {
    value: ref<int32, borrowed, 'a, readonly>;
}

function test<'a>(v0: ref<Owner, unique, mutable>, v1: ref<View<'frame>, borrowed, 'a, mutable>): void {
entry(v0: ref<Owner, unique, mutable>, v1: ref<View<'frame>, borrowed, 'a, mutable>):
    v2: ref<int32, borrowed, 'frame, readonly> = address (*v0).0
    v3: ref<ref<int32, borrowed, 'frame, readonly>, borrowed, 'a, mutable> = address (*v1).0
    store (*v3), v2
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Owner {
    value: int32;
}

type View<'a> {
    value: ref<int32, borrowed, 'a, readonly>;
}

function test<'a>(v0: ref<Owner, unique, mutable>, v1: ref<View<'frame>, borrowed, 'a, mutable>): void {
entry(v0: ref<Owner, unique, mutable>, v1: ref<View<'frame>, borrowed, 'a, mutable>):
    v2: ref<int32, borrowed, 'frame, readonly> = address (*v0).0
    v3: ref<ref<int32, borrowed, 'frame, readonly>, borrowed, 'a, mutable> = address (*v1).0
    store (*v3), v2
    release v0
    return
}
"#,
    );
}

/// A taken unique pointee drops with the frame while its freed allocation drops nothing.
#[test]
fn test_drop_a_taken_unique_pointee_after_its_storage_release() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: ref<Box, unique, mutable>): void {
entry(v0: ref<Box, unique, mutable>):
    v1: Box = load (*v0)
    v2: ref<uninit<Box>, unique, mutable> = cast.bit v0 -> ref<uninit<Box>, unique, mutable>
    release v2
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
    v1: Box = load (*v0)
    drop v1
    v2: ref<uninit<Box>, unique, mutable> = cast.bit v0 -> ref<uninit<Box>, unique, mutable>
    release v2
    return
}

function drop.frame<Box, 'a>(v0: ref<Box, borrowed, 'a, exclusive>): void {
entry(v0: ref<Box, borrowed, 'a, exclusive>):
    v1: ref<ref<int32, unique, mutable>, borrowed, 'a, exclusive> = address (*v0).0
    v2: ref<int32, unique, mutable> = load (*v1)
    release v2
    return
}
"#,
    );
}
