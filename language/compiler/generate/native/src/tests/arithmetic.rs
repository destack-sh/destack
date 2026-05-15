//! Arithmetic operation tests.
//!
//! Tests for lowering MIR arithmetic and logical instructions to Cranelift IR.

use super::compile_mir_to_normalized_clif;

/// Integer arithmetic operations chain correctly.
/// Each operation uses the result of the previous one, testing SSA value flow.
#[test]
fn test_integer_arithmetic_chain() {
    let mir = r#"
function arithmetic(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: int32 = int.sub v2, v0
    v4: int32 = int.mul v3, v1
    return v4
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int32, int32): int32 native {
b0(v0: int32, v1: int32):
    v2 = int.add v0, v1
    v3 = int.sub v2, v0
    v4 = int.mul v3, v1
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
function divide(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.div.s v0, v1
    v3: int32 = int.rem.s v0, v1
    v4: int32 = int.add v2, v3
    return v4
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int32, int32): int32 native {
b0(v0: int32, v1: int32):
    v2 = int.div.s v0, v1
    v3 = int.rem.s v0, v1
    v4 = int.add v2, v3
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
function bitwise(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.and v0, v1
    v3: int32 = int.or v2, v0
    v4: int32 = int.xor v3, v1
    return v4
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int32, int32): int32 native {
b0(v0: int32, v1: int32):
    v2 = int.and v0, v1
    v3 = int.or v2, v0
    v4 = int.xor v3, v1
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
function compare(v0: int32, v1: int32): boolean {
b0(v0: int32, v1: int32):
    v2: boolean = int.lt.s v0, v1
    return v2
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int32, int32): int8 native {
b0(v0: int32, v1: int32):
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
function negate(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.negate v0
    return v1
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(int32): int32 native {
b0(v0: int32):
    v1 = int.negate v0
    return v1
}
"#
    .trim();
    assert_eq!(clif, expected);
}

/// Integer constants include type suffixes in CLIF output.
/// The const instruction shows the type (const.int32 42).
#[test]
fn test_integer_constants() {
    let mir = r#"
function constants(): int32 {
b0:
    v0: int32 = 42int32
    v1: int32 = 100int32
    v2: int32 = int.add v0, v1
    return v2
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    let expected = r#"
function u0:0(): int32 native {
b0:
    v0 = const.int32 42
    v1 = const.int32 100
    v2 = int.add v0, v1
    return v2
}
"#
    .trim();
    assert_eq!(clif, expected);
}

/// Character constants lower to i32 (unicode codepoint).
#[test]
fn test_char_constant() {
    let mir = r#"
function char_const(): int32 {
b0:
    v0: uint32 = 'A'
    return v0
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // 'A' = 65 in unicode
    let expected = r#"
function u0:0(): int32 native {
b0:
    v0 = const.int32 65
    return v0
}
"#
    .trim();
    assert_eq!(clif, expected);
}

/// Unicode character constants beyond ASCII.
#[test]
fn test_char_constant_unicode() {
    let mir = r#"
function emoji(): int32 {
b0:
    v0: uint32 = '😀'
    return v0
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // '😀' = U+1F600 = 128512
    let expected = r#"
function u0:0(): int32 native {
b0:
    v0 = const.int32 0x0001_f600
    return v0
}
"#
    .trim();
    assert_eq!(clif, expected);
}
