use crate::memory::Value;
use crate::tests::expect_evaluate_mir;

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
    expect_evaluate_mir(
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
    expect_evaluate_mir(
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
    expect_evaluate_mir(
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
    expect_evaluate_mir(
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
    expect_evaluate_mir(mir, "neg", &[Value::int32(42)], Value::int32(-42));
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
    expect_evaluate_mir(
        mir,
        "eq",
        &[Value::int32(5), Value::int32(5)],
        Value::Bool(true),
    );
    expect_evaluate_mir(
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
    expect_evaluate_mir(
        mir,
        "lt",
        &[Value::int32(3), Value::int32(5)],
        Value::Bool(true),
    );
    expect_evaluate_mir(
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
    expect_evaluate_mir(mir, "constant", &[], Value::int32(42));
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
    expect_evaluate_mir(mir, "const_true", &[], Value::Bool(true));
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
    expect_evaluate_mir(mir, "const_false", &[], Value::Bool(false));
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
    expect_evaluate_mir(
        mir,
        "and",
        &[Value::Bool(true), Value::Bool(true)],
        Value::Bool(true),
    );
    expect_evaluate_mir(
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
    expect_evaluate_mir(
        mir,
        "or",
        &[Value::Bool(false), Value::Bool(true)],
        Value::Bool(true),
    );
    expect_evaluate_mir(
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
    expect_evaluate_mir(mir, "not", &[Value::Bool(true)], Value::Bool(false));
    expect_evaluate_mir(mir, "not", &[Value::Bool(false)], Value::Bool(true));
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
    expect_evaluate_mir(
        mir,
        "fadd",
        &[Value::float64(1.5), Value::float64(2.5)],
        Value::float64(4.0),
    );
}
