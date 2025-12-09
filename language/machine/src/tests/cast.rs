use crate::memory::Value;
use crate::tests::run_mir_expect;

/// Truncate i64 to i32 correctly masks the value.
#[test]
fn test_truncate_i64_to_i32() {
    let mir = r#"
function @trunc(v0: i64) -> i32 {
block0(v0: i64):
    v1 = trunc v0 -> i32
    return v1
}
"#;
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
function @trunc(v0: i64) -> i32 {
block0(v0: i64):
    v1 = trunc v0 -> i32
    return v1
}
"#;
    run_mir_expect(mir, "trunc", &[Value::int64(-1)], Value::int32(-1));
    run_mir_expect(mir, "trunc", &[Value::int64(-42)], Value::int32(-42));
}

/// Zero-extend u8 to u32.
#[test]
fn test_zero_extend() {
    let mir = r#"
function @uext(v0: u8) -> u32 {
block0(v0: u8):
    v1 = uextend v0 -> u32
    return v1
}
"#;
    run_mir_expect(
        mir,
        "uext",
        &[Value::UInt {
            value: 200,
            width: 8,
        }],
        Value::uint32(200),
    );
}

/// Sign-extend i8 to i32.
#[test]
fn test_sign_extend() {
    let mir = r#"
function @sext(v0: i8) -> i32 {
block0(v0: i8):
    v1 = sextend v0 -> i32
    return v1
}
"#;
    // positive value
    run_mir_expect(
        mir,
        "sext",
        &[Value::Int {
            value: 100,
            width: 8,
        }],
        Value::int32(100),
    );
    // negative value: -1 as i8 should become -1 as i32
    run_mir_expect(
        mir,
        "sext",
        &[Value::Int {
            value: -1,
            width: 8,
        }],
        Value::int32(-1),
    );
    // -100 as i8
    run_mir_expect(
        mir,
        "sext",
        &[Value::Int {
            value: -100,
            width: 8,
        }],
        Value::int32(-100),
    );
}

/// Float64 to signed integer.
#[test]
fn test_float_to_signed_int() {
    let mir = r#"
function @f2i(v0: f64) -> i32 {
block0(v0: f64):
    v1 = fcvt_to_sint v0 -> i32
    return v1
}
"#;
    run_mir_expect(mir, "f2i", &[Value::float64(42.9)], Value::int32(42));
    run_mir_expect(mir, "f2i", &[Value::float64(-42.9)], Value::int32(-42));
}

/// Float64 to unsigned integer.
#[test]
fn test_float_to_unsigned_int() {
    let mir = r#"
function @f2u(v0: f64) -> u32 {
block0(v0: f64):
    v1 = fcvt_to_uint v0 -> u32
    return v1
}
"#;
    run_mir_expect(mir, "f2u", &[Value::float64(42.9)], Value::uint32(42));
}

/// Signed integer to float64.
#[test]
fn test_signed_int_to_float() {
    let mir = r#"
function @i2f(v0: i32) -> f64 {
block0(v0: i32):
    v1 = scvt_to_float v0 -> f64
    return v1
}
"#;
    run_mir_expect(mir, "i2f", &[Value::int32(42)], Value::float64(42.0));
    run_mir_expect(mir, "i2f", &[Value::int32(-42)], Value::float64(-42.0));
}

/// Unsigned integer to float64.
#[test]
fn test_unsigned_int_to_float() {
    let mir = r#"
function @u2f(v0: u32) -> f64 {
block0(v0: u32):
    v1 = ucvt_to_float v0 -> f64
    return v1
}
"#;
    run_mir_expect(mir, "u2f", &[Value::uint32(42)], Value::float64(42.0));
}

/// Float32 to float64 extension.
#[test]
fn test_float_extend() {
    let mir = r#"
function @fext(v0: f32) -> f64 {
block0(v0: f32):
    v1 = fwiden v0 -> f64
    return v1
}
"#;
    run_mir_expect(mir, "fext", &[Value::float32(3.5)], Value::float64(3.5));
}

/// Float64 to float32 truncation.
#[test]
fn test_float_truncate() {
    let mir = r#"
function @ftrunc(v0: f64) -> f32 {
block0(v0: f64):
    v1 = fnarrow v0 -> f32
    return v1
}
"#;
    run_mir_expect(mir, "ftrunc", &[Value::float64(3.5)], Value::float32(3.5));
}

/// Integer to float32.
#[test]
fn test_int_to_float32() {
    let mir = r#"
function @i2f32(v0: i32) -> f32 {
block0(v0: i32):
    v1 = scvt_to_float v0 -> f32
    return v1
}
"#;
    run_mir_expect(mir, "i2f32", &[Value::int32(42)], Value::float32(42.0));
}
