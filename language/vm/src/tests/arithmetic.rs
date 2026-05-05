use crate::Value;
use crate::diagnostic::Error;
use crate::tests::{assert_runtime_error, run_mir, run_mir_expect};

/// Integer addition produces the sum of two i32 values.
#[test]
fn test_add_i32() {
    let mir = r#"
function add(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}"#;
    run_mir_expect(
        mir,
        "add",
        &[Value::int32(1), Value::int32(2)],
        Value::int32(3),
    );
}

/// Integer addition wraps and preserves signed i32 canonical bits.
#[test]
fn test_add_i32_wraps() {
    let mir = r#"
function add(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}"#;
    run_mir_expect(
        mir,
        "add",
        &[Value::int32(i32::MAX), Value::int32(1)],
        Value::int32(i32::MIN),
    );
}

/// Integer subtraction produces the difference of two i32 values.
#[test]
fn test_subtract_i32() {
    let mir = r#"
function sub(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.sub v0, v1
    return v2
}"#;
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
function mul(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.mul v0, v1
    return v2
}"#;
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
function div(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.div.s v0, v1
    return v2
}"#;
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
function neg(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.negate v0
    return v1
}"#;
    run_mir_expect(mir, "neg", &[Value::int32(42)], Value::int32(-42));
}

/// Equality comparison returns true for equal values, false otherwise.
#[test]
fn test_compare_equal() {
    let mir = r#"
function eq(v0: int32, v1: int32): boolean {
b0(v0: int32, v1: int32):
    v2: boolean = int.eq v0, v1
    return v2
}"#;
    run_mir_expect(
        mir,
        "eq",
        &[Value::int32(5), Value::int32(5)],
        Value::bool(true),
    );
    run_mir_expect(
        mir,
        "eq",
        &[Value::int32(5), Value::int32(3)],
        Value::bool(false),
    );
}

/// Signed less-than comparison returns true when left is smaller.
#[test]
fn test_compare_less_than() {
    let mir = r#"
function lt(v0: int32, v1: int32): boolean {
b0(v0: int32, v1: int32):
    v2: boolean = int.lt.s v0, v1
    return v2
}"#;
    run_mir_expect(
        mir,
        "lt",
        &[Value::int32(3), Value::int32(5)],
        Value::bool(true),
    );
    run_mir_expect(
        mir,
        "lt",
        &[Value::int32(5), Value::int32(3)],
        Value::bool(false),
    );
}

/// Integer constants are loaded correctly.
#[test]
fn test_constant_i32() {
    let mir = r#"
function constant(): int32 {
b0:
    v0: int32 = 42int32
    return v0
}"#;
    run_mir_expect(mir, "constant", &[], Value::int32(42));
}

/// Wide unsigned constants preserve bits above one VM word.
#[test]
fn test_constant_uint128() {
    let mir = r#"
function constantWide(): boolean {
b0:
    v0: uint128 = 18446744073709551616uint128
    v1: uint128 = 1uint128
    v2: uint128 = 64uint128
    v3: uint128 = int.shl v1, v2
    v4: boolean = int.eq v0, v3
    return v4
}"#;
    run_mir_expect(mir, "constantWide", &[], Value::bool(true));
}

/// Wide unsigned return values materialize through the engine boundary.
#[test]
fn test_return_uint128() {
    let mir = r#"
function returnWide(): uint128 {
b0:
    v0: uint128 = 18446744073709551616uint128
    return v0
}"#;
    let output = run_mir(mir, "returnWide", &[]).expect("execution failed");

    assert_eq!(
        output,
        Value::UInt {
            value: 18_446_744_073_709_551_616,
            width: 128,
        },
    );
}

/// Wide unsigned arithmetic runs through frame-backed integer bytes.
#[test]
fn test_add_uint128() {
    let mir = r#"
function addWide(): boolean {
b0:
    v0: uint128 = 18446744073709551615uint128
    v1: uint128 = 1uint128
    v2: uint128 = int.add v0, v1
    v3: uint128 = 1uint128
    v4: uint128 = 64uint128
    v5: uint128 = int.shl v3, v4
    v6: boolean = int.eq v2, v5
    return v6
}"#;
    run_mir_expect(mir, "addWide", &[], Value::bool(true));
}

/// Wide unsigned arithmetic scales beyond the literal carrier width.
#[test]
fn test_add_uint256() {
    let mir = r#"
function addVeryWide(): boolean {
b0:
    v0: uint256 = 340282366920938463463374607431768211455uint256
    v1: uint256 = 1uint256
    v2: uint256 = int.add v0, v1
    v3: boolean = int.gt.u v2, v0
    return v3
}"#;
    run_mir_expect(mir, "addVeryWide", &[], Value::bool(true));
}

/// Wide unsigned select copies the selected frame-backed scalar value.
#[test]
fn test_select_uint128() {
    let mir = r#"
function selectWide(): boolean {
b0:
    v0: boolean = true
    v1: uint128 = 18446744073709551616uint128
    v2: uint128 = 7uint128
    v3: uint128 = select v0, v1, v2
    v4: boolean = int.eq v3, v1
    return v4
}"#;
    run_mir_expect(mir, "selectWide", &[], Value::bool(true));
}

/// Boolean true constant is loaded correctly.
#[test]
fn test_constant_bool_true() {
    let mir = r#"
function constTrue(): boolean {
b0:
    v0: boolean = true
    return v0
}"#;
    run_mir_expect(mir, "constTrue", &[], Value::bool(true));
}

/// Boolean false constant is loaded correctly.
#[test]
fn test_constant_bool_false() {
    let mir = r#"
function constFalse(): boolean {
b0:
    v0: boolean = false
    return v0
}"#;
    run_mir_expect(mir, "constFalse", &[], Value::bool(false));
}

/// Bitwise AND on booleans produces logical AND.
#[test]
fn test_boolean_and() {
    let mir = r#"
function and(v0: boolean, v1: boolean): boolean {
b0(v0: boolean, v1: boolean):
    v2: boolean = int.and v0, v1
    return v2
}"#;
    run_mir_expect(
        mir,
        "and",
        &[Value::bool(true), Value::bool(true)],
        Value::bool(true),
    );
    run_mir_expect(
        mir,
        "and",
        &[Value::bool(true), Value::bool(false)],
        Value::bool(false),
    );
}

/// Bitwise OR on booleans produces logical OR.
#[test]
fn test_boolean_or() {
    let mir = r#"
function or(v0: boolean, v1: boolean): boolean {
b0(v0: boolean, v1: boolean):
    v2: boolean = int.or v0, v1
    return v2
}"#;
    run_mir_expect(
        mir,
        "or",
        &[Value::bool(false), Value::bool(true)],
        Value::bool(true),
    );
    run_mir_expect(
        mir,
        "or",
        &[Value::bool(false), Value::bool(false)],
        Value::bool(false),
    );
}

/// Bitwise NOT on booleans produces logical NOT.
#[test]
fn test_boolean_not() {
    let mir = r#"
function not(v0: boolean): boolean {
b0(v0: boolean):
    v1: boolean = int.not v0
    return v1
}"#;
    run_mir_expect(mir, "not", &[Value::bool(true)], Value::bool(false));
    run_mir_expect(mir, "not", &[Value::bool(false)], Value::bool(true));
}

/// Floating point addition produces the sum of two f64 values.
#[test]
fn test_float_add() {
    let mir = r#"
function fadd(v0: float64, v1: float64): float64 {
b0(v0: float64, v1: float64):
    v2: float64 = float.add v0, v1
    return v2
}"#;
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
function udiv(v0: uint32, v1: uint32): uint32 {
b0(v0: uint32, v1: uint32):
    v2: uint32 = int.div.u v0, v1
    return v2
}"#;
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
function srem(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.rem.s v0, v1
    return v2
}"#;
    run_mir_expect(
        mir,
        "srem",
        &[Value::int32(17), Value::int32(5)],
        Value::int32(2),
    );
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
function urem(v0: uint32, v1: uint32): uint32 {
b0(v0: uint32, v1: uint32):
    v2: uint32 = int.rem.u v0, v1
    return v2
}"#;
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
function divZero(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    v2: int32 = int.div.s v0, v1
    return v2
}"#;
    let result = run_mir(mir, "divZero", &[Value::int32(10)]);

    assert_runtime_error(result, Error::DivisionByZero);
}

/// Not-equal comparison returns true for different values.
#[test]
fn test_compare_not_equal() {
    let mir = r#"
function ne(v0: int32, v1: int32): boolean {
b0(v0: int32, v1: int32):
    v2: boolean = int.ne v0, v1
    return v2
}"#;
    run_mir_expect(
        mir,
        "ne",
        &[Value::int32(5), Value::int32(3)],
        Value::bool(true),
    );
    run_mir_expect(
        mir,
        "ne",
        &[Value::int32(5), Value::int32(5)],
        Value::bool(false),
    );
}

/// Signed greater-than comparison.
#[test]
fn test_compare_signed_greater() {
    let mir = r#"
function sgt(v0: int32, v1: int32): boolean {
b0(v0: int32, v1: int32):
    v2: boolean = int.gt.s v0, v1
    return v2
}"#;
    run_mir_expect(
        mir,
        "sgt",
        &[Value::int32(5), Value::int32(3)],
        Value::bool(true),
    );
    run_mir_expect(
        mir,
        "sgt",
        &[Value::int32(3), Value::int32(5)],
        Value::bool(false),
    );
    run_mir_expect(
        mir,
        "sgt",
        &[Value::int32(-1), Value::int32(-5)],
        Value::bool(true),
    );
}

/// Signed greater-or-equal comparison.
#[test]
fn test_compare_signed_greater_equal() {
    let mir = r#"
function sge(v0: int32, v1: int32): boolean {
b0(v0: int32, v1: int32):
    v2: boolean = int.ge.s v0, v1
    return v2
}"#;
    run_mir_expect(
        mir,
        "sge",
        &[Value::int32(5), Value::int32(5)],
        Value::bool(true),
    );
    run_mir_expect(
        mir,
        "sge",
        &[Value::int32(5), Value::int32(3)],
        Value::bool(true),
    );
    run_mir_expect(
        mir,
        "sge",
        &[Value::int32(3), Value::int32(5)],
        Value::bool(false),
    );
}

/// Signed less-or-equal comparison.
#[test]
fn test_compare_signed_less_equal() {
    let mir = r#"
function sle(v0: int32, v1: int32): boolean {
b0(v0: int32, v1: int32):
    v2: boolean = int.le.s v0, v1
    return v2
}"#;
    run_mir_expect(
        mir,
        "sle",
        &[Value::int32(3), Value::int32(5)],
        Value::bool(true),
    );
    run_mir_expect(
        mir,
        "sle",
        &[Value::int32(5), Value::int32(5)],
        Value::bool(true),
    );
    run_mir_expect(
        mir,
        "sle",
        &[Value::int32(5), Value::int32(3)],
        Value::bool(false),
    );
}

/// Unsigned less-than comparison.
#[test]
fn test_compare_unsigned_less() {
    let mir = r#"
function ult(v0: uint32, v1: uint32): boolean {
b0(v0: uint32, v1: uint32):
    v2: boolean = int.lt.u v0, v1
    return v2
}"#;
    run_mir_expect(
        mir,
        "ult",
        &[Value::uint32(3), Value::uint32(5)],
        Value::bool(true),
    );
    run_mir_expect(
        mir,
        "ult",
        &[Value::uint32(5), Value::uint32(3)],
        Value::bool(false),
    );
}

/// Unsigned greater-than comparison.
#[test]
fn test_compare_unsigned_greater() {
    let mir = r#"
function ugt(v0: uint32, v1: uint32): boolean {
b0(v0: uint32, v1: uint32):
    v2: boolean = int.gt.u v0, v1
    return v2
}"#;
    run_mir_expect(
        mir,
        "ugt",
        &[Value::uint32(5), Value::uint32(3)],
        Value::bool(true),
    );
    run_mir_expect(
        mir,
        "ugt",
        &[Value::uint32(3), Value::uint32(5)],
        Value::bool(false),
    );
}

/// XOR on integers produces bitwise exclusive-or.
#[test]
fn test_bitwise_xor() {
    let mir = r#"
function xor(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.xor v0, v1
    return v2
}"#;
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
function shl(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.shiftLeft v0, v1
    return v2
}"#;
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
function sshr(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.shiftRight.s v0, v1
    return v2
}"#;
    run_mir_expect(
        mir,
        "sshr",
        &[Value::int32(16), Value::int32(2)],
        Value::int32(4),
    );
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
function ushr(v0: uint32, v1: uint32): uint32 {
b0(v0: uint32, v1: uint32):
    v2: uint32 = int.shiftRight.u v0, v1
    return v2
}"#;
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
function fsub(v0: float64, v1: float64): float64 {
b0(v0: float64, v1: float64):
    v2: float64 = float.sub v0, v1
    return v2
}"#;
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
function fmul(v0: float64, v1: float64): float64 {
b0(v0: float64, v1: float64):
    v2: float64 = float.mul v0, v1
    return v2
}"#;
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
function fdiv(v0: float64, v1: float64): float64 {
b0(v0: float64, v1: float64):
    v2: float64 = float.div v0, v1
    return v2
}"#;
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
function fneg(v0: float64): float64 {
b0(v0: float64):
    v1: float64 = float.negate v0
    return v1
}"#;
    run_mir_expect(mir, "fneg", &[Value::float64(3.5)], Value::float64(-3.5));
    run_mir_expect(mir, "fneg", &[Value::float64(-3.5)], Value::float64(3.5));
}

/// Float equal comparison.
#[test]
fn test_float_compare_equal() {
    let mir = r#"
function fcmpEq(v0: float64, v1: float64): boolean {
b0(v0: float64, v1: float64):
    v2: boolean = float.eq v0, v1
    return v2
}"#;
    run_mir_expect(
        mir,
        "fcmpEq",
        &[Value::float64(3.5), Value::float64(3.5)],
        Value::bool(true),
    );
    run_mir_expect(
        mir,
        "fcmpEq",
        &[Value::float64(3.5), Value::float64(4.5)],
        Value::bool(false),
    );
}

/// Float less-than comparison.
#[test]
fn test_float_compare_less() {
    let mir = r#"
function fcmpLt(v0: float64, v1: float64): boolean {
b0(v0: float64, v1: float64):
    v2: boolean = float.lt v0, v1
    return v2
}"#;
    run_mir_expect(
        mir,
        "fcmpLt",
        &[Value::float64(3.0), Value::float64(4.0)],
        Value::bool(true),
    );
    run_mir_expect(
        mir,
        "fcmpLt",
        &[Value::float64(4.0), Value::float64(3.0)],
        Value::bool(false),
    );
}

/// Float32 operations.
#[test]
fn test_float32_operations() {
    let mir = r#"
function f32Add(v0: float32, v1: float32): float32 {
b0(v0: float32, v1: float32):
    v2: float32 = float.add v0, v1
    return v2
}"#;
    run_mir_expect(
        mir,
        "f32Add",
        &[Value::float32(1.5), Value::float32(2.5)],
        Value::float32(4.0),
    );
}

/// Float32 comparison.
#[test]
fn test_float32_compare() {
    let mir = r#"
function f32Lt(v0: float32, v1: float32): boolean {
b0(v0: float32, v1: float32):
    v2: boolean = float.lt v0, v1
    return v2
}"#;
    run_mir_expect(
        mir,
        "f32Lt",
        &[Value::float32(1.0), Value::float32(2.0)],
        Value::bool(true),
    );
}

/// Bitwise NOT on integers.
#[test]
fn test_bitwise_not_int() {
    let mir = r#"
function bnot(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.not v0
    return v1
}"#;
    run_mir_expect(mir, "bnot", &[Value::int32(0)], Value::int32(-1));
}

/// Bitwise operations on unsigned integers.
#[test]
fn test_unsigned_bitwise() {
    let mir = r#"
function uand(v0: uint32, v1: uint32): uint32 {
b0(v0: uint32, v1: uint32):
    v2: uint32 = int.and v0, v1
    return v2
}"#;
    run_mir_expect(
        mir,
        "uand",
        &[Value::uint32(0b1100), Value::uint32(0b1010)],
        Value::uint32(0b1000),
    );
}
