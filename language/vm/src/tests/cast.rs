use crate::Value;
use crate::diagnostic::Error;
use crate::tests::{run_mir_expect, run_mir_expect_error};

/// Truncate i64 to i32 correctly masks the value.
#[test]
fn test_truncate_i64_to_i32() {
    let mir = r#"
function trunc(v0: int64): int32 {
b0(v0: int64):
    v1: int32 = cast.truncate v0 -> int32
    return v1
}"#;
    run_mir_expect(
        mir,
        "trunc",
        &[Value::int64(0x1_0000_0042)],
        Value::int32(0x42),
    );
}

/// Truncate preserves sign for values that fit.
#[test]
fn test_truncate_preserves_sign() {
    let mir = r#"
function trunc(v0: int64): int32 {
b0(v0: int64):
    v1: int32 = cast.truncate v0 -> int32
    return v1
}"#;
    run_mir_expect(mir, "trunc", &[Value::int64(-1)], Value::int32(-1));
    run_mir_expect(mir, "trunc", &[Value::int64(-42)], Value::int32(-42));
}

/// Zero-extend u8 to u32.
#[test]
fn test_zero_extend() {
    let mir = r#"
function uext(v0: uint8): uint32 {
b0(v0: uint8):
    v1: uint32 = cast.extend.u v0 -> uint32
    return v1
}"#;
    run_mir_expect(mir, "uext", &[Value::uint(200, 8)], Value::uint32(200));
}

/// Sign-extend i8 to i32.
#[test]
fn test_sign_extend() {
    let mir = r#"
function sext(v0: int8): int32 {
b0(v0: int8):
    v1: int32 = cast.extend.s v0 -> int32
    return v1
}"#;
    run_mir_expect(mir, "sext", &[Value::int(100, 8)], Value::int32(100));
    run_mir_expect(mir, "sext", &[Value::int(-1, 8)], Value::int32(-1));
    run_mir_expect(mir, "sext", &[Value::int(-100, 8)], Value::int32(-100));
}

/// Float64 to signed integer.
#[test]
fn test_float_to_signed_int() {
    let mir = r#"
function f2i(v0: float64): int32 {
b0(v0: float64):
    v1: int32 = cast.floatToInt.s v0 -> int32
    return v1
}"#;
    run_mir_expect(mir, "f2i", &[Value::float64(42.9)], Value::int32(42));
    run_mir_expect(mir, "f2i", &[Value::float64(-42.9)], Value::int32(-42));
}

/// Float64 to signed integer traps on NaN.
#[test]
fn test_float_to_signed_int_nan_traps() {
    let mir = r#"
function f2iNan(): int32 {
b0:
    v0: float64 = 0float64
    v1: float64 = float.div v0, v0
    v2: int32 = cast.floatToInt.s v1 -> int32
    return v2
}"#;
    run_mir_expect_error(mir, "f2iNan", &[], Error::BadConversionToInteger);
}

/// Float64 to signed integer traps on out of range values.
#[test]
fn test_float_to_signed_int_overflow_traps() {
    let mir = r#"
function f2iOverflow(v0: float64): int32 {
b0(v0: float64):
    v1: int32 = cast.floatToInt.s v0 -> int32
    return v1
}"#;
    run_mir_expect_error(
        mir,
        "f2iOverflow",
        &[Value::float64(1e40)],
        Error::BadConversionToInteger,
    );
}

/// Float64 to unsigned integer.
#[test]
fn test_float_to_unsigned_int() {
    let mir = r#"
function f2u(v0: float64): uint32 {
b0(v0: float64):
    v1: uint32 = cast.floatToInt.u v0 -> uint32
    return v1
}"#;
    run_mir_expect(mir, "f2u", &[Value::float64(42.9)], Value::uint32(42));
}

/// Float64 to unsigned integer traps on negative inputs.
#[test]
fn test_float_to_unsigned_int_negative_traps() {
    let mir = r#"
function f2uNegative(v0: float64): uint32 {
b0(v0: float64):
    v1: uint32 = cast.floatToInt.u v0 -> uint32
    return v1
}"#;
    run_mir_expect_error(
        mir,
        "f2uNegative",
        &[Value::float64(-1.0)],
        Error::BadConversionToInteger,
    );
}

/// Float64 to unsigned integer traps on out of range values.
#[test]
fn test_float_to_unsigned_int_overflow_traps() {
    let mir = r#"
function f2uOverflow(v0: float64): uint32 {
b0(v0: float64):
    v1: uint32 = cast.floatToInt.u v0 -> uint32
    return v1
}"#;
    run_mir_expect_error(
        mir,
        "f2uOverflow",
        &[Value::float64(1e40)],
        Error::BadConversionToInteger,
    );
}

/// Float64 to unsigned integer traps on NaN.
#[test]
fn test_float_to_unsigned_int_nan_traps() {
    let mir = r#"
function f2uNan(): uint32 {
b0:
    v0: float64 = 0float64
    v1: float64 = float.div v0, v0
    v2: uint32 = cast.floatToInt.u v1 -> uint32
    return v2
}"#;
    run_mir_expect_error(mir, "f2uNan", &[], Error::BadConversionToInteger);
}

/// Float64 to signed integer saturating conversion.
#[test]
fn test_float_to_signed_int_saturating() {
    let mir = r#"
function f2iSat(v0: float64): int32 {
b0(v0: float64):
    v1: int32 = cast.floatToIntSaturating.s v0 -> int32
    return v1
}"#;
    run_mir_expect(mir, "f2iSat", &[Value::float64(42.9)], Value::int32(42));
    run_mir_expect(mir, "f2iSat", &[Value::float64(-42.9)], Value::int32(-42));
    run_mir_expect(
        mir,
        "f2iSat",
        &[Value::float64(1e40)],
        Value::int32(i32::MAX),
    );
    run_mir_expect(
        mir,
        "f2iSat",
        &[Value::float64(-1e40)],
        Value::int32(i32::MIN),
    );
}

/// Float64 to signed integer saturates NaN to zero.
#[test]
fn test_float_to_signed_int_saturating_nan() {
    let mir = r#"
function f2iSatNan(): int32 {
b0:
    v0: float64 = 0float64
    v1: float64 = float.div v0, v0
    v2: int32 = cast.floatToIntSaturating.s v1 -> int32
    return v2
}"#;
    run_mir_expect(mir, "f2iSatNan", &[], Value::int32(0));
}

/// Float64 to unsigned integer saturating conversion.
#[test]
fn test_float_to_unsigned_int_saturating() {
    let mir = r#"
function f2uSat(v0: float64): uint32 {
b0(v0: float64):
    v1: uint32 = cast.floatToIntSaturating.u v0 -> uint32
    return v1
}"#;
    run_mir_expect(mir, "f2uSat", &[Value::float64(42.9)], Value::uint32(42));
    run_mir_expect(mir, "f2uSat", &[Value::float64(-1.0)], Value::uint32(0));
    run_mir_expect(
        mir,
        "f2uSat",
        &[Value::float64(1e40)],
        Value::uint32(u32::MAX),
    );
}

/// Float64 to unsigned integer saturates NaN to zero.
#[test]
fn test_float_to_unsigned_int_saturating_nan() {
    let mir = r#"
function f2uSatNan(): uint32 {
b0:
    v0: float64 = 0float64
    v1: float64 = float.div v0, v0
    v2: uint32 = cast.floatToIntSaturating.u v1 -> uint32
    return v2
}"#;
    run_mir_expect(mir, "f2uSatNan", &[], Value::uint32(0));
}

/// Signed integer to float64.
#[test]
fn test_signed_int_to_float() {
    let mir = r#"
function i2f(v0: int32): float64 {
b0(v0: int32):
    v1: float64 = cast.intToFloat.s v0 -> float64
    return v1
}"#;
    run_mir_expect(mir, "i2f", &[Value::int32(42)], Value::float64(42.0));
    run_mir_expect(mir, "i2f", &[Value::int32(-42)], Value::float64(-42.0));
}

/// Unsigned integer to float64.
#[test]
fn test_unsigned_int_to_float() {
    let mir = r#"
function u2f(v0: uint32): float64 {
b0(v0: uint32):
    v1: float64 = cast.intToFloat.u v0 -> float64
    return v1
}"#;
    run_mir_expect(mir, "u2f", &[Value::uint32(42)], Value::float64(42.0));
}

/// Float32 to float64 extension.
#[test]
fn test_float_extend() {
    let mir = r#"
function fext(v0: float32): float64 {
b0(v0: float32):
    v1: float64 = cast.floatExtend v0 -> float64
    return v1
}"#;
    run_mir_expect(mir, "fext", &[Value::float32(3.5)], Value::float64(3.5));
}

/// Float64 to float32 truncation.
#[test]
fn test_float_truncate() {
    let mir = r#"
function ftrunc(v0: float64): float32 {
b0(v0: float64):
    v1: float32 = cast.floatTruncate v0 -> float32
    return v1
}"#;
    run_mir_expect(mir, "ftrunc", &[Value::float64(3.5)], Value::float32(3.5));
}

/// Integer to float32.
#[test]
fn test_int_to_float32() {
    let mir = r#"
function i2f32(v0: int32): float32 {
b0(v0: int32):
    v1: float32 = cast.intToFloat.s v0 -> float32
    return v1
}"#;
    run_mir_expect(mir, "i2f32", &[Value::int32(42)], Value::float32(42.0));
}
