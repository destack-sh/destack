use crate::tests::TestProgram;

/// Emit scalar constants with their exact bytecode representations.
#[test]
fn test_emit_constants() {
    let program = TestProgram::mir(
        r#"
export function constants(): int128 {
entry:
    v0: ptr<int32, mutable> = null
    v2: boolean = true
    v3: char = 'A'
    v4: int8 = -8
    v5: uint16 = 65535
    v6: int128 = -170141183460469231731687303715884105728
    v7: uint128 = 340282366920938463463374607431768211455
    v8: float32 = 1.5
    v9: float64 = -2.25
    return v6
}
"#,
    );

    program.assert_bytecode(
        r#"
function constants {
    constant.null r0
    constant.boolean r0, true
    constant.uint32 r0, 65
    constant.int8 r0, -8
    constant.uint16 r0, 65535
    constant.int128 r1:r2, -170141183460469231731687303715884105728
    constant.uint128 r3:r4, 340282366920938463463374607431768211455
    constant.float32 r0, 1.5
    constant.float64 r0, -2.25
    return r1:r2
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx) -> i128 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64):
    v1 = iconst.i64 0
    v2 = iconst.i8 1
    v3 = iconst.i32 65
    v4 = iconst.i8 -8
    v5 = iconst.i16 -1
    v6 = iconst.i64 0
    v7 = iconst.i64 -9223372036854775808
    v8 = iconcat v6, v7  ; v6 = 0, v7 = -9223372036854775808
    v9 = iconst.i64 -1
    v10 = iconst.i64 -1
    v11 = iconcat v9, v10  ; v9 = -1, v10 = -1
    v12 = f32const 0x1.800000p0
    v13 = f64const -0x1.2000000000000p1
    return v8
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx) -> i128 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = call fn0(v0)
    store notrap aligned v3, v2
    return
}
"#,
    );
}

/// Emit signed, unsigned, bitwise, unary, comparison, and selection operations.
#[test]
fn test_emit_integer_operations() {
    let program = TestProgram::mir(
        r#"
export function integer(v0: int64, v1: int64, v2: uint64, v3: uint64): int64 {
entry(v0: int64, v1: int64, v2: uint64, v3: uint64):
    v4: int64 = add v0, v1
    v5: int64 = sub v4, v1
    v6: int64 = mul v5, v1
    v7: int64 = div v6, v1
    v8: int64 = rem v7, v1
    v9: uint64 = div v2, v3
    v10: uint64 = rem v9, v3
    v11: int64 = and v8, v1
    v12: int64 = or v11, v0
    v13: int64 = xor v12, v1
    v14: int64 = shl v13, v1
    v15: int64 = shr v14, v1
    v16: uint64 = ushr v10, v3
    v17: int64 = negate v15
    v18: int64 = not v17
    v19: boolean = lt v18, v0
    v20: boolean = ge v16, v2
    v21: int64 = select v19, v18, v0
    return v21
}
"#,
    );

    program.assert_bytecode(
        r#"
function integer {
    add.int64 r4, r0, r1
    sub.int64 r5, r4, r1
    mul.int64 r4, r5, r1
    div.int64 r5, r4, r1
    rem.int64 r4, r5, r1
    div.uint64 r5, r2, r3
    rem.uint64 r6, r5, r3
    and.int64 r5, r4, r1
    or.int64 r4, r5, r0
    xor.int64 r5, r4, r1
    shl.int64 r4, r5, r1
    shr.int64 r5, r4, r1
    shr.uint64 r1, r6, r3
    negate.int64 r3, r5
    not.int64 r4, r3
    lt.int64 r3, r4, r0
    ge.uint64 r5, r1, r2
    select r1, r3, r4, r0
    return r1
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64, i64, i64, i64) -> i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
    v5 = iadd v1, v2
    v6 = isub v5, v2
    v7 = imul v6, v2
    v8 = sdiv v7, v2
    v9 = srem v8, v2
    v10 = udiv v3, v4
    v11 = urem v10, v4
    v12 = band v9, v2
    v13 = bor v12, v1
    v14 = bxor v13, v2
    v15 = ishl v14, v2
    v16 = sshr v15, v2
    v17 = ushr v11, v4
    v18 = ineg v16
    v19 = bnot v18
    v20 = icmp slt v19, v1
    v21 = icmp uge v17, v3
    v22 = select v20, v19, v1
    return v22
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i64, i64, i64) -> i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i64 notrap aligned v1+8
    v5 = load.i64 notrap aligned v1+16
    v6 = load.i64 notrap aligned v1+24
    v7 = call fn0(v0, v3, v4, v5, v6)
    store notrap aligned v7, v2
    return
}
"#,
    );
}

/// Emit floating-point arithmetic, unary, comparison, and selection operations.
#[test]
fn test_emit_float_expressions() {
    let program = TestProgram::mir(
        r#"
export function float(v0: float64, v1: float64): float64 {
entry(v0: float64, v1: float64):
    v2: float64 = add v0, v1
    v3: float64 = sub v2, v1
    v4: float64 = mul v3, v1
    v5: float64 = div v4, v1
    v6: float64 = rem v5, v1
    v7: float64 = negate v6
    v8: boolean = lt v7, v0
    v9: float64 = select v8, v7, v0
    return v9
}
"#,
    );

    program.assert_bytecode(
        r#"
function float {
    add.float64 r2, r0, r1
    sub.float64 r3, r2, r1
    mul.float64 r2, r3, r1
    div.float64 r3, r2, r1
    rem.float64 r2, r3, r1
    negate.float64 r1, r2
    lt.float64 r2, r1, r0
    select r3, r2, r1, r0
    return r3
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, f64, f64) -> f64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    sig0 = (f64, f64) -> f64 native
    fn0 = colocated u0:2 sig0
    stack_limit = gv1

block0(v0: i64, v1: f64, v2: f64):
    v3 = fadd v1, v2
    v4 = fsub v3, v2
    v5 = fmul v4, v2
    v6 = fdiv v5, v2
    v7 = call fn0(v6, v2)
    v8 = fneg v7
    v9 = fcmp lt v8, v1
    v10 = select v9, v8, v1
    return v10
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, f64, f64) -> f64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.f64 notrap aligned v1
    v4 = load.f64 notrap aligned v1+8
    v5 = call fn0(v0, v3, v4)
    store notrap aligned v5, v2
    return
}
"#,
    );
}

/// Preserve signed extension, unsigned extension, and truncation across register widths.
#[test]
fn test_emit_integer_casts() {
    let program = TestProgram::mir(
        r#"
export function casts(v0: int8, v1: uint8): int16 {
entry(v0: int8, v1: uint8):
    v2: int128 = cast.intToInt v0 -> int128
    v3: uint128 = cast.intToInt v2 -> uint128
    v4: int16 = cast.intToInt v3 -> int16
    v5: uint128 = cast.intToInt v1 -> uint128
    v6: uint64 = cast.intToInt v5 -> uint64
    return v4
}
"#,
    );

    program.assert_bytecode(
        r#"
function casts {
    extend.int8.int64 r2, r0
    constant.uint64 r6, 63
    shr.int64 r3, r2, r6
    move r4:r5, r2:r3
    truncate.uint64.int16 r0, r4
    extend.uint8.uint64 r2, r1
    constant.uint64 r3, 0
    move r1, r2
    return r0
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i8, i8) -> i16 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i8, v2: i8):
    v3 = sextend.i128 v1
    v4 = ireduce.i16 v3
    v5 = uextend.i128 v2
    v6 = ireduce.i64 v5
    return v4
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i8, i8) -> i16 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i8 notrap aligned v1
    v4 = load.i8 notrap aligned v1+8
    v5 = call fn0(v0, v3, v4)
    store notrap aligned v5, v2
    return
}
"#,
    );
}
