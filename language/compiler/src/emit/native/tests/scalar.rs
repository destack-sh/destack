use destack_native::BlockId;
use destack_native::abi::Trap;

use crate::tests::TestProgram;

/// Emit scalar constants with their exact native bit representations.
#[test]
fn test_emit_native_constants() {
    let program = TestProgram::mir(
        r#"
export function constants(): int128 {
entry:
    v0: ref<int32, managed, mutable, nullable> = null
    v1: ref<int32, managed, mutable, undefined> = undefined
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

    program.assert_native(
        r#"
function u0:0(i64) -> i128 native {
block0(v0: i64):
    v1 = iconst.i64 0
    v2 = iconst.i64 1
    v3 = iconst.i8 1
    v4 = iconst.i32 65
    v5 = iconst.i8 -8
    v6 = iconst.i16 -1
    v7 = iconst.i64 0
    v8 = iconst.i64 -9223372036854775808
    v9 = iconcat v7, v8  ; v7 = 0, v8 = -9223372036854775808
    v10 = iconst.i64 -1
    v11 = iconst.i64 -1
    v12 = iconcat v10, v11  ; v10 = -1, v11 = -1
    v13 = f32const 0x1.800000p0
    v14 = f64const -0x1.2000000000000p1
    return v9
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64) -> i128 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = call fn0(v0)
    store notrap aligned v3, v2
    return
}
"#,
    );
}

/// Emit wrapping integer arithmetic and trapping division operations.
#[test]
fn test_emit_native_integer_arithmetic() {
    let program = TestProgram::mir(
        r#"
export function calculate(v0: int64, v1: int64, v2: uint64, v3: uint64): uint64 {
entry(v0: int64, v1: int64, v2: uint64, v3: uint64):
    v4: int64 = int.add v0, v1
    v5: int64 = int.sub v4, v1
    v6: int64 = int.mul v5, v1
    v7: int64 = int.div.s v6, v1
    v8: int64 = int.rem.s v7, v1
    v9: uint64 = int.div.u v2, v3
    v10: uint64 = int.rem.u v9, v3
    return v10
}
"#,
    );

    let object = program.assert_native(
        r#"
function u0:0(i64, i64, i64, i64, i64) -> i64 native {
block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
    v5 = iadd v1, v2
    v6 = isub v5, v2
    v7 = imul v6, v2
    v8 = sdiv v7, v2
    v9 = srem v8, v2
    v10 = udiv v3, v4
    v11 = urem v10, v4
    return v11
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64, i64, i64, i64) -> i64 native
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

    // preserve every arithmetic fault as one object-local language trap
    let traps = object
        .map()
        .traps(object.sections())
        .iter()
        .map(|trap| (trap.block, trap.trap))
        .collect::<Vec<_>>();
    assert_eq!(
        traps,
        [
            (BlockId(0), Trap::DivisionByZero),
            (BlockId(0), Trap::IntegerOverflow),
            (BlockId(0), Trap::DivisionByZero),
            (BlockId(0), Trap::DivisionByZero),
            (BlockId(0), Trap::DivisionByZero),
        ]
    );
}

/// Emit bitwise operations and width-masked shifts.
#[test]
fn test_emit_native_bitwise_operations() {
    let program = TestProgram::mir(
        r#"
export function bits(v0: int64, v1: int64, v2: uint64, v3: uint64): uint64 {
entry(v0: int64, v1: int64, v2: uint64, v3: uint64):
    v4: int64 = int.and v0, v1
    v5: int64 = int.or v4, v1
    v6: int64 = int.xor v5, v0
    v7: int64 = int.shl v6, v1
    v8: int64 = int.shr.s v7, v1
    v9: uint64 = int.shr.u v2, v3
    return v9
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64, i64, i64, i64) -> i64 native {
block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
    v5 = band v1, v2
    v6 = bor v5, v2
    v7 = bxor v6, v1
    v8 = ishl v7, v2
    v9 = sshr v8, v2
    v10 = ushr v3, v4
    return v10
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64, i64, i64, i64) -> i64 native
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

/// Emit integer and floating-point unary operations.
#[test]
fn test_emit_native_unary_operations() {
    let program = TestProgram::mir(
        r#"
export function negate(v0: int64, v1: float64): int64 {
entry(v0: int64, v1: float64):
    v2: int64 = int.negate v0
    v3: int64 = int.not v2
    v4: float64 = float.negate v1
    return v3
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64, f64) -> i64 native {
block0(v0: i64, v1: i64, v2: f64):
    v3 = ineg v1
    v4 = bnot v3
    v5 = fneg v2
    return v4
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64, f64) -> i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.f64 notrap aligned v1+8
    v5 = call fn0(v0, v3, v4)
    store notrap aligned v5, v2
    return
}
"#,
    );
}

/// Emit signed and unsigned integer comparisons.
#[test]
fn test_emit_native_integer_comparisons() {
    let program = TestProgram::mir(
        r#"
export function compare(v0: int32, v1: int32, v2: uint32, v3: uint32): boolean {
entry(v0: int32, v1: int32, v2: uint32, v3: uint32):
    v4: boolean = int.eq v0, v1
    v5: boolean = int.ne v0, v1
    v6: boolean = int.lt.s v0, v1
    v7: boolean = int.le.s v0, v1
    v8: boolean = int.gt.s v0, v1
    v9: boolean = int.ge.s v0, v1
    v10: boolean = int.lt.u v2, v3
    v11: boolean = int.le.u v2, v3
    v12: boolean = int.gt.u v2, v3
    v13: boolean = int.ge.u v2, v3
    return v13
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i32, i32, i32, i32) -> i8 native {
block0(v0: i64, v1: i32, v2: i32, v3: i32, v4: i32):
    v5 = icmp eq v1, v2
    v6 = icmp ne v1, v2
    v7 = icmp slt v1, v2
    v8 = icmp sle v1, v2
    v9 = icmp sgt v1, v2
    v10 = icmp sge v1, v2
    v11 = icmp ult v3, v4
    v12 = icmp ule v3, v4
    v13 = icmp ugt v3, v4
    v14 = icmp uge v3, v4
    return v14
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i32, i32, i32, i32) -> i8 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    v4 = load.i32 notrap aligned v1+8
    v5 = load.i32 notrap aligned v1+16
    v6 = load.i32 notrap aligned v1+24
    v7 = call fn0(v0, v3, v4, v5, v6)
    store notrap aligned v7, v2
    return
}
"#,
    );
}

/// Emit IEEE floating-point arithmetic and ordered comparisons.
#[test]
fn test_emit_native_float_operations() {
    let program = TestProgram::mir(
        r#"
export function calculate(v0: float64, v1: float64): boolean {
entry(v0: float64, v1: float64):
    v2: float64 = float.add v0, v1
    v3: float64 = float.sub v2, v1
    v4: float64 = float.mul v3, v1
    v5: float64 = float.div v4, v1
    v6: boolean = float.eq v5, v0
    v7: boolean = float.ne v5, v0
    v8: boolean = float.lt v5, v0
    v9: boolean = float.le v5, v0
    v10: boolean = float.gt v5, v0
    v11: boolean = float.ge v5, v0
    return v11
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, f64, f64) -> i8 native {
block0(v0: i64, v1: f64, v2: f64):
    v3 = fadd v1, v2
    v4 = fsub v3, v2
    v5 = fmul v4, v2
    v6 = fdiv v5, v2
    v7 = fcmp eq v6, v1
    v8 = fcmp ne v6, v1
    v9 = fcmp lt v6, v1
    v10 = fcmp le v6, v1
    v11 = fcmp gt v6, v1
    v12 = fcmp ge v6, v1
    return v12
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, f64, f64) -> i8 native
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

/// Select one scalar without introducing control flow.
#[test]
fn test_emit_native_select() {
    let program = TestProgram::mir(
        r#"
export function choose(v0: boolean, v1: int32, v2: int32): int32 {
entry(v0: boolean, v1: int32, v2: int32):
    v3: int32 = select v0, v1, v2
    return v3
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i8, i32, i32) -> i32 native {
block0(v0: i64, v1: i8, v2: i32, v3: i32):
    v4 = select v1, v2, v3
    return v4
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i8, i32, i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i8 notrap aligned v1
    v4 = load.i32 notrap aligned v1+8
    v5 = load.i32 notrap aligned v1+16
    v6 = call fn0(v0, v3, v4, v5)
    store notrap aligned v6, v2
    return
}
"#,
    );
}

/// Emit integer width, saturation, extension, and bit casts.
#[test]
fn test_emit_native_integer_casts() {
    let program = TestProgram::mir(
        r#"
export function cast(v0: int64, v1: uint64): int128 {
entry(v0: int64, v1: uint64):
    v2: int8 = cast.truncate v0 -> int8
    v3: int8 = cast.saturate v0 -> int8
    v4: uint128 = cast.extend.u v1 -> uint128
    v5: int128 = cast.extend.s v0 -> int128
    v6: int128 = cast.bit v4 -> int128
    return v6
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64, i64) -> i128 native {
block0(v0: i64, v1: i64, v2: i64):
    v3 = ireduce.i8 v1
    v4 = iconst.i64 -128
    v5 = smax v1, v4  ; v4 = -128
    v6 = iconst.i64 127
    v7 = smin v5, v6  ; v6 = 127
    v8 = ireduce.i8 v7
    v9 = uextend.i128 v2
    v10 = sextend.i128 v1
    v11 = bitcast.i128 v9
    return v11
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64, i64) -> i128 native
    fn0 = colocated u0:0 sig0

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

/// Emit trapping, saturating, and width-changing floating-point casts.
#[test]
fn test_emit_native_float_casts() {
    let program = TestProgram::mir(
        r#"
export function cast(v0: int64, v1: uint64, v2: float64): float64 {
entry(v0: int64, v1: uint64, v2: float64):
    v3: int32 = cast.floatToInt.s v2 -> int32
    v4: uint32 = cast.floatToInt.u v2 -> uint32
    v5: int32 = cast.floatToIntSaturating.s v2 -> int32
    v6: uint32 = cast.floatToIntSaturating.u v2 -> uint32
    v7: float64 = cast.intToFloat.s v0 -> float64
    v8: float64 = cast.intToFloat.u v1 -> float64
    v9: float32 = cast.floatTruncate v2 -> float32
    v10: float64 = cast.floatExtend v9 -> float64
    v11: float64 = cast.floatConvert v10 -> float64
    return v11
}
"#,
    );

    let object = program.assert_native(
        r#"
function u0:0(i64, i64, i64, f64) -> f64 native {
block0(v0: i64, v1: i64, v2: i64, v3: f64):
    v4 = fcvt_to_sint.i32 v3
    v5 = fcvt_to_uint.i32 v3
    v6 = fcvt_to_sint_sat.i32 v3
    v7 = fcvt_to_uint_sat.i32 v3
    v8 = fcvt_from_sint.f64 v1
    v9 = fcvt_from_uint.f64 v2
    v10 = fdemote.f32 v3
    v11 = fpromote.f64 v10
    return v11
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64, i64, f64) -> f64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = load.i64 notrap aligned v1+8
    v5 = load.f64 notrap aligned v1+16
    v6 = call fn0(v0, v3, v4, v5)
    store notrap aligned v6, v2
    return
}
"#,
    );

    // preserve every conversion fault as one object-local overflow trap
    let traps = object
        .map()
        .traps(object.sections())
        .iter()
        .map(|trap| (trap.block, trap.trap))
        .collect::<Vec<_>>();
    assert_eq!(traps, [(BlockId(0), Trap::IntegerOverflow); 6]);
}

/// Preserve pointer bits across explicit pointer and integer casts.
#[test]
fn test_emit_native_pointer_casts() {
    let program = TestProgram::mir(
        r#"
export function cast(
    v0: ref<int32, borrowed, mutable, nullable>,
): ref<int32, borrowed, mutable, nullable> {
entry(v0: ref<int32, borrowed, mutable, nullable>):
    v1: uint64 = cast.pointerToInt v0 -> uint64
    v2: ref<int32, borrowed, mutable, nullable> = cast.intToPointer v1 -> ref<int32, borrowed, mutable, nullable>
    return v2
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64, i64) -> i64 native {
block0(v0: i64, v1: i64):
    return v1
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64) -> i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = call fn0(v0, v3)
    store notrap aligned v4, v2
    return
}
"#,
    );
}
