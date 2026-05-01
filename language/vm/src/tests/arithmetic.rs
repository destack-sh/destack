use destack_engine::Value;

use crate::Word;
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
        &[Word::int32(1), Word::int32(2)],
        Word::int32(3),
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
        &[Word::int32(10), Word::int32(3)],
        Word::int32(7),
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
        &[Word::int32(6), Word::int32(7)],
        Word::int32(42),
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
        &[Word::int32(20), Word::int32(4)],
        Word::int32(5),
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
    run_mir_expect(mir, "neg", &[Word::int32(42)], Word::int32(-42));
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
        &[Word::int32(5), Word::int32(5)],
        Word::bool(true),
    );
    run_mir_expect(
        mir,
        "eq",
        &[Word::int32(5), Word::int32(3)],
        Word::bool(false),
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
        &[Word::int32(3), Word::int32(5)],
        Word::bool(true),
    );
    run_mir_expect(
        mir,
        "lt",
        &[Word::int32(5), Word::int32(3)],
        Word::bool(false),
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
    run_mir_expect(mir, "constant", &[], Word::int32(42));
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
    run_mir_expect(mir, "constantWide", &[], Word::bool(true));
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
    let output = run_mir(mir, "returnWide", &[])
        .expect("execution failed")
        .value;

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
    run_mir_expect(mir, "addWide", &[], Word::bool(true));
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
    run_mir_expect(mir, "addVeryWide", &[], Word::bool(true));
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
    run_mir_expect(mir, "selectWide", &[], Word::bool(true));
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
    run_mir_expect(mir, "constTrue", &[], Word::bool(true));
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
    run_mir_expect(mir, "constFalse", &[], Word::bool(false));
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
        &[Word::bool(true), Word::bool(true)],
        Word::bool(true),
    );
    run_mir_expect(
        mir,
        "and",
        &[Word::bool(true), Word::bool(false)],
        Word::bool(false),
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
        &[Word::bool(false), Word::bool(true)],
        Word::bool(true),
    );
    run_mir_expect(
        mir,
        "or",
        &[Word::bool(false), Word::bool(false)],
        Word::bool(false),
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
    run_mir_expect(mir, "not", &[Word::bool(true)], Word::bool(false));
    run_mir_expect(mir, "not", &[Word::bool(false)], Word::bool(true));
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
        &[Word::float64(1.5), Word::float64(2.5)],
        Word::float64(4.0),
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
        &[Word::uint32(20), Word::uint32(4)],
        Word::uint32(5),
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
        &[Word::int32(17), Word::int32(5)],
        Word::int32(2),
    );
    // negative remainder
    run_mir_expect(
        mir,
        "srem",
        &[Word::int32(-17), Word::int32(5)],
        Word::int32(-2),
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
        &[Word::uint32(17), Word::uint32(5)],
        Word::uint32(2),
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
    let result = run_mir(mir, "divZero", &[Word::int32(10)]);

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
        &[Word::int32(5), Word::int32(3)],
        Word::bool(true),
    );
    run_mir_expect(
        mir,
        "ne",
        &[Word::int32(5), Word::int32(5)],
        Word::bool(false),
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
        &[Word::int32(5), Word::int32(3)],
        Word::bool(true),
    );
    run_mir_expect(
        mir,
        "sgt",
        &[Word::int32(3), Word::int32(5)],
        Word::bool(false),
    );
    // negative numbers
    run_mir_expect(
        mir,
        "sgt",
        &[Word::int32(-1), Word::int32(-5)],
        Word::bool(true),
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
        &[Word::int32(5), Word::int32(5)],
        Word::bool(true),
    );
    run_mir_expect(
        mir,
        "sge",
        &[Word::int32(5), Word::int32(3)],
        Word::bool(true),
    );
    run_mir_expect(
        mir,
        "sge",
        &[Word::int32(3), Word::int32(5)],
        Word::bool(false),
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
        &[Word::int32(3), Word::int32(5)],
        Word::bool(true),
    );
    run_mir_expect(
        mir,
        "sle",
        &[Word::int32(5), Word::int32(5)],
        Word::bool(true),
    );
    run_mir_expect(
        mir,
        "sle",
        &[Word::int32(5), Word::int32(3)],
        Word::bool(false),
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
        &[Word::uint32(3), Word::uint32(5)],
        Word::bool(true),
    );
    run_mir_expect(
        mir,
        "ult",
        &[Word::uint32(5), Word::uint32(3)],
        Word::bool(false),
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
        &[Word::uint32(5), Word::uint32(3)],
        Word::bool(true),
    );
    run_mir_expect(
        mir,
        "ugt",
        &[Word::uint32(3), Word::uint32(5)],
        Word::bool(false),
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
        &[Word::int32(0b1100), Word::int32(0b1010)],
        Word::int32(0b0110),
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
        &[Word::int32(1), Word::int32(4)],
        Word::int32(16),
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
        &[Word::int32(16), Word::int32(2)],
        Word::int32(4),
    );
    // negative number: sign bit preserved
    run_mir_expect(
        mir,
        "sshr",
        &[Word::int32(-16), Word::int32(2)],
        Word::int32(-4),
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
        &[Word::uint32(16), Word::uint32(2)],
        Word::uint32(4),
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
        &[Word::float64(5.5), Word::float64(2.5)],
        Word::float64(3.0),
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
        &[Word::float64(3.0), Word::float64(4.0)],
        Word::float64(12.0),
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
        &[Word::float64(10.0), Word::float64(4.0)],
        Word::float64(2.5),
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
    run_mir_expect(mir, "fneg", &[Word::float64(3.5)], Word::float64(-3.5));
    run_mir_expect(mir, "fneg", &[Word::float64(-3.5)], Word::float64(3.5));
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
        &[Word::float64(3.5), Word::float64(3.5)],
        Word::bool(true),
    );
    run_mir_expect(
        mir,
        "fcmpEq",
        &[Word::float64(3.5), Word::float64(4.5)],
        Word::bool(false),
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
        &[Word::float64(3.0), Word::float64(4.0)],
        Word::bool(true),
    );
    run_mir_expect(
        mir,
        "fcmpLt",
        &[Word::float64(4.0), Word::float64(3.0)],
        Word::bool(false),
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
        &[Word::float32(1.5), Word::float32(2.5)],
        Word::float32(4.0),
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
        &[Word::float32(1.0), Word::float32(2.0)],
        Word::bool(true),
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
    run_mir_expect(mir, "bnot", &[Word::int32(0)], Word::int32(-1));
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
        &[Word::uint32(0b1100), Word::uint32(0b1010)],
        Word::uint32(0b1000),
    );
}
