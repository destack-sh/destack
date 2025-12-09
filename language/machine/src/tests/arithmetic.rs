use crate::diagnostic::Error;
use crate::memory::Value;
use crate::tests::{run_mir, run_mir_expect};

/// Integer addition produces the sum of two i32 values.
#[test]
fn test_add_i32() {
    let mir = r#"
function @add(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "add",
        &[Value::int32(1), Value::int32(2)],
        Value::int32(3),
    );
}

/// Integer subtraction produces the difference of two i32 values.
#[test]
fn test_subtract_i32() {
    let mir = r#"
function @sub(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = isub v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "sub",
        &[Value::int32(10), Value::int32(3)],
        Value::int32(7),
    );
}

/// Integer multiplication produces the product of two i32 values.
#[test]
fn test_multiply_i32() {
    let mir = r#"
function @mul(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = imul v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "mul",
        &[Value::int32(6), Value::int32(7)],
        Value::int32(42),
    );
}

/// Signed integer division produces the quotient of two i32 values.
#[test]
fn test_divide_i32() {
    let mir = r#"
function @div(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = sdiv v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "div",
        &[Value::int32(20), Value::int32(4)],
        Value::int32(5),
    );
}

/// Integer negation produces the two's complement negation.
#[test]
fn test_negate_i32() {
    let mir = r#"
function @neg(v0: i32) -> i32 {
block0(v0: i32):
    v1 = ineg v0
    return v1
}
"#;
    run_mir_expect(mir, "neg", &[Value::int32(42)], Value::int32(-42));
}

/// Equality comparison returns true for equal values, false otherwise.
#[test]
fn test_compare_equal() {
    let mir = r#"
function @eq(v0: i32, v1: i32) -> bool {
block0(v0: i32, v1: i32):
    v2 = icmp_eq v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "eq",
        &[Value::int32(5), Value::int32(5)],
        Value::Bool(true),
    );
    run_mir_expect(
        mir,
        "eq",
        &[Value::int32(5), Value::int32(3)],
        Value::Bool(false),
    );
}

/// Signed less-than comparison returns true when left is smaller.
#[test]
fn test_compare_less_than() {
    let mir = r#"
function @lt(v0: i32, v1: i32) -> bool {
block0(v0: i32, v1: i32):
    v2 = icmp_slt v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "lt",
        &[Value::int32(3), Value::int32(5)],
        Value::Bool(true),
    );
    run_mir_expect(
        mir,
        "lt",
        &[Value::int32(5), Value::int32(3)],
        Value::Bool(false),
    );
}

/// Integer constants are loaded correctly.
#[test]
fn test_constant_i32() {
    let mir = r#"
function @constant() -> i32 {
block0:
    v0 = iconst 42i32
    return v0
}
"#;
    run_mir_expect(mir, "constant", &[], Value::int32(42));
}

/// Boolean true constant is loaded correctly.
#[test]
fn test_constant_bool_true() {
    let mir = r#"
function @const_true() -> bool {
block0:
    v0 = iconst true
    return v0
}
"#;
    run_mir_expect(mir, "const_true", &[], Value::Bool(true));
}

/// Boolean false constant is loaded correctly.
#[test]
fn test_constant_bool_false() {
    let mir = r#"
function @const_false() -> bool {
block0:
    v0 = iconst false
    return v0
}
"#;
    run_mir_expect(mir, "const_false", &[], Value::Bool(false));
}

/// Bitwise AND on booleans produces logical AND.
#[test]
fn test_boolean_and() {
    let mir = r#"
function @and(v0: bool, v1: bool) -> bool {
block0(v0: bool, v1: bool):
    v2 = band v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "and",
        &[Value::Bool(true), Value::Bool(true)],
        Value::Bool(true),
    );
    run_mir_expect(
        mir,
        "and",
        &[Value::Bool(true), Value::Bool(false)],
        Value::Bool(false),
    );
}

/// Bitwise OR on booleans produces logical OR.
#[test]
fn test_boolean_or() {
    let mir = r#"
function @or(v0: bool, v1: bool) -> bool {
block0(v0: bool, v1: bool):
    v2 = bor v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "or",
        &[Value::Bool(false), Value::Bool(true)],
        Value::Bool(true),
    );
    run_mir_expect(
        mir,
        "or",
        &[Value::Bool(false), Value::Bool(false)],
        Value::Bool(false),
    );
}

/// Bitwise NOT on booleans produces logical NOT.
#[test]
fn test_boolean_not() {
    let mir = r#"
function @not(v0: bool) -> bool {
block0(v0: bool):
    v1 = bnot v0
    return v1
}
"#;
    run_mir_expect(mir, "not", &[Value::Bool(true)], Value::Bool(false));
    run_mir_expect(mir, "not", &[Value::Bool(false)], Value::Bool(true));
}

/// Floating point addition produces the sum of two f64 values.
#[test]
fn test_float_add() {
    let mir = r#"
function @fadd(v0: f64, v1: f64) -> f64 {
block0(v0: f64, v1: f64):
    v2 = fadd v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "fadd",
        &[Value::float64(1.5), Value::float64(2.5)],
        Value::float64(4.0),
    );
}

/// Unsigned division produces the quotient of two u32 values.
#[test]
fn test_unsigned_divide() {
    let mir = r#"
function @udiv(v0: u32, v1: u32) -> u32 {
block0(v0: u32, v1: u32):
    v2 = udiv v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "udiv",
        &[Value::uint32(20), Value::uint32(4)],
        Value::uint32(5),
    );
}

/// Signed remainder produces the remainder of two i32 values.
#[test]
fn test_signed_remainder() {
    let mir = r#"
function @srem(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = srem v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "srem",
        &[Value::int32(17), Value::int32(5)],
        Value::int32(2),
    );
    // negative remainder
    run_mir_expect(
        mir,
        "srem",
        &[Value::int32(-17), Value::int32(5)],
        Value::int32(-2),
    );
}

/// Unsigned remainder produces the remainder of two u32 values.
#[test]
fn test_unsigned_remainder() {
    let mir = r#"
function @urem(v0: u32, v1: u32) -> u32 {
block0(v0: u32, v1: u32):
    v2 = urem v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "urem",
        &[Value::uint32(17), Value::uint32(5)],
        Value::uint32(2),
    );
}

/// Division by zero produces an error.
#[test]
fn test_division_by_zero() {
    let mir = r#"
function @div_zero(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = sdiv v0, v1
    return v2
}
"#;
    let result = run_mir(mir, "div_zero", &[Value::int32(10)]);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err().error, Error::DivisionByZero));
}

/// Not-equal comparison returns true for different values.
#[test]
fn test_compare_not_equal() {
    let mir = r#"
function @ne(v0: i32, v1: i32) -> bool {
block0(v0: i32, v1: i32):
    v2 = icmp_ne v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "ne",
        &[Value::int32(5), Value::int32(3)],
        Value::Bool(true),
    );
    run_mir_expect(
        mir,
        "ne",
        &[Value::int32(5), Value::int32(5)],
        Value::Bool(false),
    );
}

/// Signed greater-than comparison.
#[test]
fn test_compare_signed_greater() {
    let mir = r#"
function @sgt(v0: i32, v1: i32) -> bool {
block0(v0: i32, v1: i32):
    v2 = icmp_sgt v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "sgt",
        &[Value::int32(5), Value::int32(3)],
        Value::Bool(true),
    );
    run_mir_expect(
        mir,
        "sgt",
        &[Value::int32(3), Value::int32(5)],
        Value::Bool(false),
    );
    // negative numbers
    run_mir_expect(
        mir,
        "sgt",
        &[Value::int32(-1), Value::int32(-5)],
        Value::Bool(true),
    );
}

/// Signed greater-or-equal comparison.
#[test]
fn test_compare_signed_greater_equal() {
    let mir = r#"
function @sge(v0: i32, v1: i32) -> bool {
block0(v0: i32, v1: i32):
    v2 = icmp_sge v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "sge",
        &[Value::int32(5), Value::int32(5)],
        Value::Bool(true),
    );
    run_mir_expect(
        mir,
        "sge",
        &[Value::int32(5), Value::int32(3)],
        Value::Bool(true),
    );
    run_mir_expect(
        mir,
        "sge",
        &[Value::int32(3), Value::int32(5)],
        Value::Bool(false),
    );
}

/// Signed less-or-equal comparison.
#[test]
fn test_compare_signed_less_equal() {
    let mir = r#"
function @sle(v0: i32, v1: i32) -> bool {
block0(v0: i32, v1: i32):
    v2 = icmp_sle v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "sle",
        &[Value::int32(3), Value::int32(5)],
        Value::Bool(true),
    );
    run_mir_expect(
        mir,
        "sle",
        &[Value::int32(5), Value::int32(5)],
        Value::Bool(true),
    );
    run_mir_expect(
        mir,
        "sle",
        &[Value::int32(5), Value::int32(3)],
        Value::Bool(false),
    );
}

/// Unsigned less-than comparison.
#[test]
fn test_compare_unsigned_less() {
    let mir = r#"
function @ult(v0: u32, v1: u32) -> bool {
block0(v0: u32, v1: u32):
    v2 = icmp_ult v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "ult",
        &[Value::uint32(3), Value::uint32(5)],
        Value::Bool(true),
    );
    run_mir_expect(
        mir,
        "ult",
        &[Value::uint32(5), Value::uint32(3)],
        Value::Bool(false),
    );
}

/// Unsigned greater-than comparison.
#[test]
fn test_compare_unsigned_greater() {
    let mir = r#"
function @ugt(v0: u32, v1: u32) -> bool {
block0(v0: u32, v1: u32):
    v2 = icmp_ugt v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "ugt",
        &[Value::uint32(5), Value::uint32(3)],
        Value::Bool(true),
    );
    run_mir_expect(
        mir,
        "ugt",
        &[Value::uint32(3), Value::uint32(5)],
        Value::Bool(false),
    );
}

/// XOR on integers produces bitwise exclusive-or.
#[test]
fn test_bitwise_xor() {
    let mir = r#"
function @xor(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = bxor v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "xor",
        &[Value::int32(0b1100), Value::int32(0b1010)],
        Value::int32(0b0110),
    );
}

/// Shift left on signed integers.
#[test]
fn test_shift_left() {
    let mir = r#"
function @shl(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = ishl v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "shl",
        &[Value::int32(1), Value::int32(4)],
        Value::int32(16),
    );
}

/// Arithmetic shift right preserves sign.
#[test]
fn test_arithmetic_shift_right() {
    let mir = r#"
function @sshr(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = sshr v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "sshr",
        &[Value::int32(16), Value::int32(2)],
        Value::int32(4),
    );
    // negative number: sign bit preserved
    run_mir_expect(
        mir,
        "sshr",
        &[Value::int32(-16), Value::int32(2)],
        Value::int32(-4),
    );
}

/// Logical shift right fills with zeros.
#[test]
fn test_logical_shift_right() {
    let mir = r#"
function @ushr(v0: u32, v1: u32) -> u32 {
block0(v0: u32, v1: u32):
    v2 = ushr v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "ushr",
        &[Value::uint32(16), Value::uint32(2)],
        Value::uint32(4),
    );
}

/// Float subtraction.
#[test]
fn test_float_subtract() {
    let mir = r#"
function @fsub(v0: f64, v1: f64) -> f64 {
block0(v0: f64, v1: f64):
    v2 = fsub v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "fsub",
        &[Value::float64(5.5), Value::float64(2.5)],
        Value::float64(3.0),
    );
}

/// Float multiplication.
#[test]
fn test_float_multiply() {
    let mir = r#"
function @fmul(v0: f64, v1: f64) -> f64 {
block0(v0: f64, v1: f64):
    v2 = fmul v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "fmul",
        &[Value::float64(3.0), Value::float64(4.0)],
        Value::float64(12.0),
    );
}

/// Float division.
#[test]
fn test_float_divide() {
    let mir = r#"
function @fdiv(v0: f64, v1: f64) -> f64 {
block0(v0: f64, v1: f64):
    v2 = fdiv v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "fdiv",
        &[Value::float64(10.0), Value::float64(4.0)],
        Value::float64(2.5),
    );
}

/// Float negation.
#[test]
fn test_float_negate() {
    let mir = r#"
function @fneg(v0: f64) -> f64 {
block0(v0: f64):
    v1 = fneg v0
    return v1
}
"#;
    run_mir_expect(mir, "fneg", &[Value::float64(3.5)], Value::float64(-3.5));
    run_mir_expect(mir, "fneg", &[Value::float64(-3.5)], Value::float64(3.5));
}

/// Float equal comparison.
#[test]
fn test_float_compare_equal() {
    let mir = r#"
function @fcmp_eq(v0: f64, v1: f64) -> bool {
block0(v0: f64, v1: f64):
    v2 = fcmp_eq v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "fcmp_eq",
        &[Value::float64(3.5), Value::float64(3.5)],
        Value::Bool(true),
    );
    run_mir_expect(
        mir,
        "fcmp_eq",
        &[Value::float64(3.5), Value::float64(4.5)],
        Value::Bool(false),
    );
}

/// Float less-than comparison.
#[test]
fn test_float_compare_less() {
    let mir = r#"
function @fcmp_lt(v0: f64, v1: f64) -> bool {
block0(v0: f64, v1: f64):
    v2 = fcmp_lt v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "fcmp_lt",
        &[Value::float64(3.0), Value::float64(4.0)],
        Value::Bool(true),
    );
    run_mir_expect(
        mir,
        "fcmp_lt",
        &[Value::float64(4.0), Value::float64(3.0)],
        Value::Bool(false),
    );
}

/// Float32 operations.
#[test]
fn test_float32_operations() {
    let mir = r#"
function @f32_add(v0: f32, v1: f32) -> f32 {
block0(v0: f32, v1: f32):
    v2 = fadd v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "f32_add",
        &[Value::float32(1.5), Value::float32(2.5)],
        Value::float32(4.0),
    );
}

/// Float32 comparison.
#[test]
fn test_float32_compare() {
    let mir = r#"
function @f32_lt(v0: f32, v1: f32) -> bool {
block0(v0: f32, v1: f32):
    v2 = fcmp_lt v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "f32_lt",
        &[Value::float32(1.0), Value::float32(2.0)],
        Value::Bool(true),
    );
}

/// Bitwise NOT on integers.
#[test]
fn test_bitwise_not_int() {
    let mir = r#"
function @bnot(v0: i32) -> i32 {
block0(v0: i32):
    v1 = bnot v0
    return v1
}
"#;
    run_mir_expect(mir, "bnot", &[Value::int32(0)], Value::int32(-1));
}

/// Bitwise operations on unsigned integers.
#[test]
fn test_unsigned_bitwise() {
    let mir = r#"
function @uand(v0: u32, v1: u32) -> u32 {
block0(v0: u32, v1: u32):
    v2 = band v0, v1
    return v2
}
"#;
    run_mir_expect(
        mir,
        "uand",
        &[Value::uint32(0b1100), Value::uint32(0b1010)],
        Value::uint32(0b1000),
    );
}
