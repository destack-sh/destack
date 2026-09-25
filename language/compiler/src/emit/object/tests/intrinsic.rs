use crate::tests::TestProgram;

/// Emit integer machine intrinsics without runtime calls.
#[test]
fn test_emit_integer_intrinsics() {
    let program = TestProgram::mir(
        r#"
export function bits(v0: uint32, v1: uint32): uint32 {
entry(v0: uint32, v1: uint32):
    v2: uint32 = intrinsic.math.bits.leadingZeroCount(v0)
    v3: uint32 = intrinsic.math.bits.trailingZeroCount(v2)
    v4: uint32 = intrinsic.math.bits.populationCount(v3)
    v5: uint32 = intrinsic.math.bits.byteSwap(v4)
    v6: uint32 = intrinsic.math.bits.bitReverse(v5)
    v7: uint32 = intrinsic.math.bits.rotateLeft(v6, v1)
    v8: uint32 = intrinsic.math.bits.rotateRight(v7, v1)
    v9: uint32 = intrinsic.math.arithmetic.unchecked.add(v8, v1)
    v10: uint32 = intrinsic.math.arithmetic.unchecked.subtract(v9, v1)
    v11: uint32 = intrinsic.math.arithmetic.unchecked.multiply(v10, v1)
    v12: uint32 = intrinsic.math.arithmetic.saturating.add(v11, v1)
    v13: uint32 = intrinsic.math.arithmetic.saturating.subtract(v12, v1)
    return v13
}
"#,
    );

    program.assert_bytecode(
        r#"
function bits {
    countLeadingZeros.uint32 r2, r0
    countTrailingZeros.uint32 r0, r2
    countOnes.uint32 r2, r0
    byteSwap.uint32 r0, r2
    reverseBits.uint32 r2, r0
    rotateLeft.uint32 r0, r2, r1
    rotateRight.uint32 r2, r0, r1
    add.uint32 r0, r2, r1
    sub.uint32 r2, r0, r1
    mul.uint32 r0, r2, r1
    add.saturating.uint32 r2, r0, r1
    sub.saturating.uint32 r0, r2, r1
    return r0
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i32, i32) -> i32 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i32, v2: i32):
    v3 = clz v1
    v4 = ctz v3
    v5 = popcnt v4
    v6 = bswap v5
    v7 = bitrev v6
    v8 = rotl v7, v2
    v9 = rotr v8, v2
    v10 = iadd v9, v2
    v11 = isub v10, v2
    v12 = imul v11, v2
    v13, v14 = uadd_overflow v12, v2
    v15 = iconst.i32 -1
    v16 = select v14, v15, v13  ; v15 = -1
    v17, v18 = usub_overflow v16, v2
    v19 = iconst.i32 0
    v20 = select v18, v19, v17  ; v19 = 0
    return v20
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i32, i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    v4 = load.i32 notrap aligned v1+8
    v5 = call fn0(v0, v3, v4)
    store notrap aligned v5, v2
    return
}
"#,
    );
}

/// Emit integer arithmetic intrinsics without runtime calls.
#[test]
fn test_emit_integer_arithmetic_intrinsics() {
    let program = TestProgram::mir(
        r#"
export function arithmetic(v0: int64, v1: int64, v2: int64): boolean {
entry(v0: int64, v1: int64, v2: int64):
    v3: int64 = intrinsic.math.arithmetic.midpoint(v0, v1)
    v4: int64 = intrinsic.math.arithmetic.clamp(v3, v1, v2)
    v5: int64 = intrinsic.math.arithmetic.divideCeil(v4, v1)
    v6: int64 = intrinsic.math.arithmetic.remainderEuclidean(v5, v2)
    v7: int64 = intrinsic.math.bits.isolateLowestOne(v6)
    v8: boolean = intrinsic.math.arithmetic.isMultipleOf(v7, v2)
    return v8
}

export function difference(v0: int64, v1: int64): uint64 {
entry(v0: int64, v1: int64):
    v2: uint64 = intrinsic.math.arithmetic.absDiff(v0, v1)
    return v2
}
"#,
    );

    program.assert_bytecode(
        r#"
function arithmetic {
    midpoint.int64 r3, r0, r1
    clamp.int64 r0, r3, r1, r2
    divideCeil.int64 r3, r0, r1
    remainderEuclidean.int64 r0, r3, r2
    isolateLowestOne.int64 r1, r0
    isMultipleOf.int64 r0, r1, r2
    return r0
}

function difference {
    absDiff.int64 r2, r0, r1
    return r2
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i64, i64, i64) -> i8 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i64, v3: i64):
    v4 = bxor v1, v2
    v5 = band v1, v2
    v6 = iconst.i64 1
    v7 = sshr v4, v6  ; v6 = 1
    v8 = iadd v7, v5
    v9 = iconst.i64 0
    v10 = iconst.i64 1
    v11 = icmp slt v8, v9  ; v9 = 0
    v12 = select v11, v10, v9  ; v10 = 1, v9 = 0
    v13 = band v12, v4
    v14 = iadd v8, v13
    v15 = icmp sgt v2, v3
    trapnz v15, user3
    v16 = icmp slt v14, v2
    v17 = icmp sgt v14, v3
    v18 = select v17, v3, v14
    v19 = select v16, v2, v18
    v20 = iconst.i64 0
    v21 = iconst.i64 1
    v22 = sdiv v19, v2
    v23 = srem v19, v2
    v24 = icmp ne v23, v20  ; v20 = 0
    v25 = icmp slt v23, v20  ; v20 = 0
    v26 = icmp slt v2, v20  ; v20 = 0
    v27 = icmp eq v25, v26
    v28 = band v24, v27
    v29 = select v28, v21, v20  ; v21 = 1, v20 = 0
    v30 = iadd v22, v29
    v31 = iconst.i64 0
    v32 = srem v30, v3
    v33 = icmp slt v3, v31  ; v31 = 0
    v34 = ineg v3
    v35 = select v33, v34, v3
    v36 = iadd v32, v35
    v37 = icmp slt v32, v31  ; v31 = 0
    v38 = select v37, v36, v32
    v39 = ineg v38
    v40 = band v38, v39
    v41 = iconst.i64 0
    v42 = iconst.i64 1
    v43 = icmp eq v3, v41  ; v41 = 0
    v44 = icmp eq v40, v41  ; v41 = 0
    v45 = iconst.i64 -9223372036854775808
    v46 = iconst.i64 -1
    v47 = icmp eq v40, v45  ; v45 = -9223372036854775808
    v48 = icmp eq v3, v46  ; v46 = -1
    v49 = band v47, v48
    v50 = bor v43, v49
    v51 = select v50, v42, v3  ; v42 = 1
    v52 = srem v40, v51
    v53 = icmp eq v52, v41  ; v41 = 0
    v54 = select v43, v44, v53
    return v54
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i64, i64) -> i8 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i64 notrap aligned v1+8
    v5 = load.i64 notrap aligned v1+16
    v6 = call fn0(v0, v3, v4, v5)
    store notrap aligned v6, v2
    return
}

function u0:2(i64 vmctx, i64, i64) -> i64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i64, v2: i64):
    v3 = icmp sge v1, v2
    v4 = isub v1, v2
    v5 = isub v2, v1
    v6 = select v3, v4, v5
    return v6
}

function u1:1(i64, i64, i64) native {
    sig0 = (i64 vmctx, i64, i64) -> i64 native
    fn0 = colocated u0:2 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i64 notrap aligned v1+8
    v5 = call fn0(v0, v3, v4)
    store notrap aligned v5, v2
    return
}
"#,
    );
}

/// Emit direct floating intrinsics and explicit platform math imports.
#[test]
fn test_emit_float_intrinsics() {
    let program = TestProgram::mir(
        r#"
export function math(v0: float64, v1: float64): float64 {
entry(v0: float64, v1: float64):
    v2: float64 = intrinsic.math.float.sqrt(v0)
    v3: float64 = intrinsic.math.float.abs(v2)
    v4: float64 = intrinsic.math.float.fma(v3, v1, v0)
    v5: float64 = intrinsic.math.float.copySign(v4, v1)
    v6: float64 = intrinsic.math.float.min(v5, v0)
    v7: float64 = intrinsic.math.float.max(v6, v1)
    v8: float64 = intrinsic.math.float.floor(v7)
    v9: float64 = intrinsic.math.float.ceil(v8)
    v10: float64 = intrinsic.math.float.trunc(v9)
    v11: float64 = intrinsic.math.float.round(v10)
    v12: float64 = intrinsic.math.float.roundTiesEven(v11)
    v13: float64 = intrinsic.math.float.roundTiesAway(v12)
    v14: float64 = intrinsic.math.float.sin(v13)
    v15: float64 = intrinsic.math.float.cos(v14)
    v16: float64 = intrinsic.math.float.atan2(v15, v1)
    v17: float64 = intrinsic.math.float.exp(v16)
    v18: float64 = intrinsic.math.float.log(v17)
    v19: float64 = intrinsic.math.float.pow(v18, v1)
    v20: float64 = intrinsic.math.float.cbrt(v19)
    v21: float64 = intrinsic.math.float.expm1(v20)
    v22: float64 = intrinsic.math.float.log1p(v21)
    return v22
}
"#,
    );

    program.assert_bytecode(
        r#"
function math {
    sqrt.float64 r2, r0
    abs.float64 r3, r2
    fma.float64 r2, r3, r1, r0
    copySign.float64 r3, r2, r1
    min.float64 r2, r3, r0
    max.float64 r0, r2, r1
    floor.float64 r2, r0
    ceil.float64 r0, r2
    truncate.float64 r2, r0
    round.float64 r0, r2
    roundTiesEven.float64 r2, r0
    roundTiesAway.float64 r0, r2
    sin.float64 r2, r0
    cos.float64 r0, r2
    atan2.float64 r2, r0, r1
    exp.float64 r0, r2
    log.float64 r2, r0
    pow.float64 r0, r2, r1
    cbrt.float64 r1, r0
    expm1.float64 r0, r1
    log1p.float64 r1, r0
    return r1
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
    sig0 = (f64) -> f64 native
    sig1 = (f64) -> f64 native
    sig2 = (f64, f64) -> f64 native
    sig3 = (f64) -> f64 native
    sig4 = (f64) -> f64 native
    sig5 = (f64, f64) -> f64 native
    sig6 = (f64) -> f64 native
    sig7 = (f64) -> f64 native
    sig8 = (f64) -> f64 native
    fn0 = colocated u0:2 sig0
    fn1 = colocated u0:3 sig1
    fn2 = colocated u0:4 sig2
    fn3 = colocated u0:5 sig3
    fn4 = colocated u0:6 sig4
    fn5 = colocated u0:7 sig5
    fn6 = colocated u0:8 sig6
    fn7 = colocated u0:9 sig7
    fn8 = colocated u0:10 sig8
    stack_limit = gv1

block0(v0: i64, v1: f64, v2: f64):
    v3 = sqrt v1
    v4 = fabs v3
    v5 = fma v4, v2, v1
    v6 = fcopysign v5, v2
    v7 = f64const 0.0
    v8 = f64const 0x1.0000000000000p0
    v9 = fcopysign v8, v6  ; v8 = 0x1.0000000000000p0
    v10 = fcmp lt v9, v7  ; v7 = 0.0
    v11 = fcmp lt v6, v1
    v12 = fcmp eq v6, v1
    v13 = select v11, v6, v1
    v14 = select v10, v6, v1
    v15 = select v12, v14, v13
    v16 = fcmp uno v1, v1
    v17 = select v16, v1, v15
    v18 = fcmp uno v6, v6
    v19 = select v18, v6, v17
    v20 = f64const 0.0
    v21 = f64const 0x1.0000000000000p0
    v22 = fcopysign v21, v19  ; v21 = 0x1.0000000000000p0
    v23 = fcmp gt v22, v20  ; v20 = 0.0
    v24 = fcmp gt v19, v2
    v25 = fcmp eq v19, v2
    v26 = select v24, v19, v2
    v27 = select v23, v19, v2
    v28 = select v25, v27, v26
    v29 = fcmp uno v2, v2
    v30 = select v29, v2, v28
    v31 = fcmp uno v19, v19
    v32 = select v31, v19, v30
    v33 = floor v32
    v34 = ceil v33
    v35 = trunc v34
    v36 = f64const 0.0
    v37 = f64const 0x1.0000000000000p-1
    v38 = f64const 0x1.0000000000000p0
    v39 = floor v35
    v40 = fsub v35, v39
    v41 = fcmp lt v40, v37  ; v37 = 0x1.0000000000000p-1
    v42 = select v41, v36, v38  ; v36 = 0.0, v38 = 0x1.0000000000000p0
    v43 = fadd v39, v42
    v44 = fcopysign v43, v35
    v45 = nearest v44
    v46 = f64const 0.0
    v47 = f64const 0x1.0000000000000p-1
    v48 = f64const 0x1.0000000000000p0
    v49 = trunc v45
    v50 = fsub v45, v49
    v51 = fabs v50
    v52 = fcmp lt v51, v47  ; v47 = 0x1.0000000000000p-1
    v53 = fcopysign v48, v45  ; v48 = 0x1.0000000000000p0
    v54 = fadd v49, v53
    v55 = select v52, v49, v54
    v56 = fcopysign v55, v45
    v57 = call fn0(v56)
    v58 = call fn1(v57)
    v59 = call fn2(v58, v2)
    v60 = call fn3(v59)
    v61 = call fn4(v60)
    v62 = call fn5(v61, v2)
    v63 = call fn6(v62)
    v64 = call fn7(v63)
    v65 = call fn8(v64)
    return v65
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

/// Emit floating-point midpoint, clamp, and predicates without runtime calls.
#[test]
fn test_emit_float_midpoint_clamp_and_predicates() {
    let program = TestProgram::mir(
        r#"
export function bounds(v0: float64, v1: float64, v2: float64): float64 {
entry(v0: float64, v1: float64, v2: float64):
    v3: float64 = intrinsic.math.arithmetic.midpoint(v0, v1)
    v4: float64 = intrinsic.math.arithmetic.clamp(v3, v1, v2)
    v5: boolean = intrinsic.math.float.isFinite(v4)
    v6: boolean = intrinsic.math.float.isInfinite(v0)
    v7: float64 = select v5, v4, v0
    v8: float64 = select v6, v7, v1
    return v8
}
"#,
    );

    program.assert_bytecode(
        r#"
function bounds {
    midpoint.float64 r3, r0, r1
    clamp.float64 r4, r3, r1, r2
    isFinite.float64 r2, r4
    isInfinite.float64 r3, r0
    select r5, r2, r4, r0
    select r0, r3, r5, r1
    return r0
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, f64, f64, f64) -> f64 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: f64, v2: f64, v3: f64):
    v4 = f64const 0x1.0000000000000p-1
    v5 = f64const 0x1.fffffffffffffp1022
    v6 = fabs v1
    v7 = fabs v2
    v8 = fcmp le v6, v5  ; v5 = 0x1.fffffffffffffp1022
    v9 = fcmp le v7, v5  ; v5 = 0x1.fffffffffffffp1022
    v10 = band v8, v9
    v11 = fadd v1, v2
    v12 = fmul v11, v4  ; v4 = 0x1.0000000000000p-1
    v13 = fmul v1, v4  ; v4 = 0x1.0000000000000p-1
    v14 = fmul v2, v4  ; v4 = 0x1.0000000000000p-1
    v15 = fadd v13, v14
    v16 = select v10, v12, v15
    v17 = fcmp gt v2, v3
    v18 = fcmp uno v2, v3
    v19 = bor v17, v18
    trapnz v19, user3
    v20 = fcmp lt v16, v2
    v21 = fcmp gt v16, v3
    v22 = select v21, v3, v16
    v23 = select v20, v2, v22
    v24 = f64const +Inf
    v25 = fabs v23
    v26 = fcmp lt v25, v24  ; v24 = +Inf
    v27 = f64const +Inf
    v28 = fabs v1
    v29 = fcmp eq v28, v27  ; v27 = +Inf
    v30 = select v26, v23, v1
    v31 = select v29, v30, v2
    return v31
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, f64, f64, f64) -> f64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.f64 notrap aligned v1
    v4 = load.f64 notrap aligned v1+8
    v5 = load.f64 notrap aligned v1+16
    v6 = call fn0(v0, v3, v4, v5)
    store notrap aligned v6, v2
    return
}
"#,
    );
}
