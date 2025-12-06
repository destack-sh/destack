//! Arithmetic operation tests.
//!
//! Tests for lowering MIR arithmetic and logical instructions to Cranelift IR.

use super::compile_mir_to_normalized_clif;

/// Integer arithmetic operations chain correctly.
/// Each operation uses the result of the previous one, testing SSA value flow.
#[test]
fn test_integer_arithmetic_chain() {
    let mir = r#"
function @arithmetic(v0: i32, v1: i32) -> i32 {
block0:
    v2 = iadd v0, v1
    v3 = isub v2, v0
    v4 = imul v3, v1
    return v4
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(i32, i32) -> i32 native {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    v3 = isub v2, v0
    v4 = imul v3, v1
    return v4
}
"#
    .trim();
    assert_eq!(clif, expected);
}

/// Signed division and remainder lower to sdiv and srem.
/// These are distinct from unsigned variants (udiv, urem).
#[test]
fn test_signed_division() {
    let mir = r#"
function @divide(v0: i32, v1: i32) -> i32 {
block0:
    v2 = sdiv v0, v1
    v3 = srem v0, v1
    v4 = iadd v2, v3
    return v4
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(i32, i32) -> i32 native {
block0(v0: i32, v1: i32):
    v2 = sdiv v0, v1
    v3 = srem v0, v1
    v4 = iadd v2, v3
    return v4
}
"#
    .trim();
    assert_eq!(clif, expected);
}

/// Bitwise operations: and, or, xor.
/// These operate on integer bits without signedness concerns.
#[test]
fn test_bitwise_operations() {
    let mir = r#"
function @bitwise(v0: i32, v1: i32) -> i32 {
block0:
    v2 = band v0, v1
    v3 = bor v2, v0
    v4 = bxor v3, v1
    return v4
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(i32, i32) -> i32 native {
block0(v0: i32, v1: i32):
    v2 = band v0, v1
    v3 = bor v2, v0
    v4 = bxor v3, v1
    return v4
}
"#
    .trim();
    assert_eq!(clif, expected);
}

/// Signed comparison produces an i8 boolean result.
/// The icmp instruction specifies the comparison type (slt = signed less than).
#[test]
fn test_signed_comparison() {
    let mir = r#"
function @compare(v0: i32, v1: i32) -> bool {
block0:
    v2 = icmp_slt v0, v1
    return v2
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(i32, i32) -> i8 native {
block0(v0: i32, v1: i32):
    v2 = icmp slt v0, v1
    return v2
}
"#
    .trim();
    assert_eq!(clif, expected);
}

/// Unary negation produces the two's complement negative.
/// The ineg instruction works on signed integers.
#[test]
fn test_unary_negation() {
    let mir = r#"
function @negate(v0: i32) -> i32 {
block0:
    v1 = ineg v0
    return v1
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(i32) -> i32 native {
block0(v0: i32):
    v1 = ineg v0
    return v1
}
"#
    .trim();
    assert_eq!(clif, expected);
}

/// Integer constants include type suffixes in CLIF output.
/// The iconst instruction shows the type (iconst.i32 42).
#[test]
fn test_integer_constants() {
    let mir = r#"
function @constants() -> i32 {
block0:
    v0 = iconst 42i32
    v1 = iconst 100i32
    v2 = iadd v0, v1
    return v2
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0() -> i32 native {
block0:
    v0 = iconst.i32 42
    v1 = iconst.i32 100
    v2 = iadd v0, v1
    return v2
}
"#
    .trim();
    assert_eq!(clif, expected);
}
