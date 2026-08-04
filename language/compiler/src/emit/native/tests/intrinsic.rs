use crate::tests::TestProgram;

/// Emit integer machine intrinsics without runtime calls.
#[test]
fn test_emit_native_integer_intrinsics() {
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

    program.assert_native(
        r#"
function u0:0(i64, i32, i32) -> i32 native {
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
    sig0 = (i64, i32, i32) -> i32 native
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

/// Emit direct floating intrinsics and explicit platform math imports.
#[test]
fn test_emit_native_float_intrinsics() {
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
    v12: float64 = intrinsic.math.float.sin(v11)
    v13: float64 = intrinsic.math.float.cos(v12)
    v14: float64 = intrinsic.math.float.atan2(v13, v1)
    v15: float64 = intrinsic.math.float.exp(v14)
    v16: float64 = intrinsic.math.float.log(v15)
    v17: float64 = intrinsic.math.float.pow(v16, v1)
    return v17
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, f64, f64) -> f64 native {
    sig0 = (f64) -> f64 native
    sig1 = (f64) -> f64 native
    sig2 = (f64, f64) -> f64 native
    sig3 = (f64) -> f64 native
    sig4 = (f64) -> f64 native
    sig5 = (f64, f64) -> f64 native
    fn0 = colocated u0:2 sig0
    fn1 = colocated u0:3 sig1
    fn2 = colocated u0:4 sig2
    fn3 = colocated u0:5 sig3
    fn4 = colocated u0:6 sig4
    fn5 = colocated u0:7 sig5

block0(v0: i64, v1: f64, v2: f64):
    v3 = sqrt v1
    v4 = fabs v3
    v5 = fma v4, v2, v1
    v6 = fcopysign v5, v2
    v7 = fmin v6, v1
    v8 = fmax v7, v2
    v9 = floor v8
    v10 = ceil v9
    v11 = trunc v10
    v12 = nearest v11
    v13 = call fn0(v12)
    v14 = call fn1(v13)
    v15 = call fn2(v14, v2)
    v16 = call fn3(v15)
    v17 = call fn4(v16)
    v18 = call fn5(v17, v2)
    return v18
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, f64, f64) -> f64 native
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
