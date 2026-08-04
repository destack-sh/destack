use crate::tests::TestProgram;

/// Construct and project one two-scalar aggregate entirely in SSA.
#[test]
fn test_emit_native_scalar_pair_aggregate() {
    let program = TestProgram::mir(
        r#"
type Pair {
    first: int32;
    second: int64;
}

export function second(v0: int32, v1: int64): int64 {
entry(v0: int32, v1: int64):
    v2: Pair = aggregate (v0, v1)
    v3: int64 = field.get v2, 1
    return v3
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i32, i64) -> i64 native {
block0(v0: i64, v1: i32, v2: i64):
    return v2
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i32, i64) -> i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    v4 = load.i64 notrap aligned v1+8
    v5 = call fn0(v0, v3, v4)
    store notrap aligned v5, v2
    return
}
"#,
    );
}

/// Construct and project one indirect aggregate in canonical stack storage.
#[test]
fn test_emit_native_indirect_aggregate() {
    let program = TestProgram::mir(
        r#"
type Triple {
    first: int64;
    second: int64;
    third: int64;
}

export function third(v0: int64, v1: int64, v2: int64): int64 {
entry(v0: int64, v1: int64, v2: int64):
    v3: Triple = aggregate (v0, v1, v2)
    v4: int64 = field.get v3, 2
    return v4
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64, i64, i64) -> i64 native {
    ss0 = explicit_slot 24, align = 8

block0(v0: i64, v1: i64, v2: i64, v3: i64):
    v4 = stack_addr.i64 ss0
    v5 = iconst.i64 0
    v6 = iadd v4, v5  ; v5 = 0
    store notrap aligned v1, v6
    v7 = iconst.i64 8
    v8 = iadd v4, v7  ; v7 = 8
    store notrap aligned v2, v8
    v9 = iconst.i64 16
    v10 = iadd v4, v9  ; v9 = 16
    store notrap aligned v3, v10
    v11 = iconst.i64 16
    v12 = iadd v4, v11  ; v11 = 16
    v13 = load.i64 notrap aligned v12
    return v13
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64, i64, i64) -> i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i64 notrap aligned v1+8
    v5 = load.i64 notrap aligned v1+16
    v6 = call fn0(v0, v3, v4, v5)
    store notrap aligned v6, v2
    return
}
"#,
    );
}

/// Construct and project one fixed array in canonical stack storage.
#[test]
fn test_emit_native_fixed_array_aggregate() {
    let program = TestProgram::mir(
        r#"
export function second(v0: int32, v1: int32, v2: int32): int32 {
entry(v0: int32, v1: int32, v2: int32):
    v3: [int32; 3] = aggregate (v0, v1, v2)
    v4: int32 = element.get v3, 1
    return v4
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i32, i32, i32) -> i32 native {
    ss0 = explicit_slot 12, align = 4

block0(v0: i64, v1: i32, v2: i32, v3: i32):
    v4 = stack_addr.i64 ss0
    v5 = iconst.i64 0
    v6 = iadd v4, v5  ; v5 = 0
    store notrap aligned v1, v6
    v7 = iconst.i64 4
    v8 = iadd v4, v7  ; v7 = 4
    store notrap aligned v2, v8
    v9 = iconst.i64 8
    v10 = iadd v4, v9  ; v9 = 8
    store notrap aligned v3, v10
    v11 = iconst.i64 4
    v12 = iadd v4, v11  ; v11 = 4
    v13 = load.i32 notrap aligned v12
    return v13
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i32, i32, i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    v4 = load.i32 notrap aligned v1+8
    v5 = load.i32 notrap aligned v1+16
    v6 = call fn0(v0, v3, v4, v5)
    store notrap aligned v6, v2
    return
}
"#,
    );
}

/// Replace one field in a two-scalar aggregate without materializing memory.
#[test]
fn test_emit_native_scalar_pair_update() {
    let program = TestProgram::mir(
        r#"
type Pair {
    first: int32;
    second: int64;
}

export function replace(v0: int32, v1: int64, v2: int64): int64 {
entry(v0: int32, v1: int64, v2: int64):
    v3: Pair = aggregate (v0, v1)
    v4: Pair = field.set v3, 1, v2
    v5: int64 = field.get v4, 1
    return v5
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i32, i64, i64) -> i64 native {
block0(v0: i64, v1: i32, v2: i64, v3: i64):
    return v3
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i32, i64, i64) -> i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    v4 = load.i64 notrap aligned v1+8
    v5 = load.i64 notrap aligned v1+16
    v6 = call fn0(v0, v3, v4, v5)
    store notrap aligned v6, v2
    return
}
"#,
    );
}

/// Copy and update one indirect aggregate in canonical stack storage.
#[test]
fn test_emit_native_indirect_update() {
    let program = TestProgram::mir(
        r#"
type Triple {
    first: int64;
    second: int64;
    third: int64;
}

export function replace(v0: Triple, v1: int64): int64 {
entry(v0: Triple, v1: int64):
    v2: Triple = field.set v0, 1, v1
    v3: int64 = field.get v2, 1
    return v3
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64, i64) -> i64 native {
    ss0 = explicit_slot 24, align = 8

block0(v0: i64, v1: i64, v2: i64):
    v3 = stack_addr.i64 ss0
    v4 = load.i64 notrap aligned v1
    store notrap aligned v4, v3
    v5 = load.i64 notrap aligned v1+8
    store notrap aligned v5, v3+8
    v6 = load.i64 notrap aligned v1+16
    store notrap aligned v6, v3+16
    v7 = iconst.i64 8
    v8 = iadd v3, v7  ; v7 = 8
    store notrap aligned v2, v8
    v9 = iconst.i64 8
    v10 = iadd v3, v9  ; v9 = 8
    v11 = load.i64 notrap aligned v10
    return v11
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64, i64) -> i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = iconst.i64 0
    v4 = iadd v1, v3  ; v3 = 0
    v5 = load.i64 notrap aligned v1+24
    v6 = call fn0(v0, v4, v5)
    store notrap aligned v6, v2
    return
}
"#,
    );
}

/// Replace and project one fixed-array element.
#[test]
fn test_emit_native_fixed_array_update() {
    let program = TestProgram::mir(
        r#"
export function replace(v0: [int32; 3], v1: int32): int32 {
entry(v0: [int32; 3], v1: int32):
    v2: [int32; 3] = element.set v0, 1, v1
    v3: int32 = element.get v2, 1
    return v3
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64, i32) -> i32 native {
    ss0 = explicit_slot 12, align = 4

block0(v0: i64, v1: i64, v2: i32):
    v3 = stack_addr.i64 ss0
    v4 = load.i64 notrap aligned v1
    store notrap aligned v4, v3
    v5 = load.i32 notrap aligned v1+8
    store notrap aligned v5, v3+8
    v6 = iconst.i64 4
    v7 = iadd v3, v6  ; v6 = 4
    store notrap aligned v2, v7
    v8 = iconst.i64 4
    v9 = iadd v3, v8  ; v8 = 4
    v10 = load.i32 notrap aligned v9
    return v10
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64, i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = iconst.i64 0
    v4 = iadd v1, v3  ; v3 = 0
    v5 = load.i32 notrap aligned v1+16
    v6 = call fn0(v0, v4, v5)
    store notrap aligned v6, v2
    return
}
"#,
    );
}
