use destack_engine::Value;

use crate::TestProgram;

/// Verify bitwise AND, OR, and XOR operators.
#[test]
fn test_lower_bitwise_and_or_xor() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function bitwiseAnd(a: int32, b: int32): int32 {
    return a & b;
}
function bitwiseOr(a: int32, b: int32): int32 {
    return a | b;
}
function bitwiseXor(a: int32, b: int32): int32 {
    return a ^ b;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // 0b1010 & 0b1100 = 0b1000 = 8
    test.assert_mir_function_output(
        module_id,
        "native",
        "bitwiseAnd",
        &[Value::int32(0b1010), Value::int32(0b1100)],
        Value::int32(0b1000),
    );

    // 0b1010 | 0b1100 = 0b1110 = 14
    test.assert_mir_function_output(
        module_id,
        "native",
        "bitwiseOr",
        &[Value::int32(0b1010), Value::int32(0b1100)],
        Value::int32(0b1110),
    );

    // 0b1010 ^ 0b1100 = 0b0110 = 6
    test.assert_mir_function_output(
        module_id,
        "native",
        "bitwiseXor",
        &[Value::int32(0b1010), Value::int32(0b1100)],
        Value::int32(0b0110),
    );
}

/// Verify left and right shift operators.
#[test]
fn test_lower_shifts() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function shiftLeft(a: int32, b: int32): int32 {
    return a << b;
}
function shiftRight(a: int32, b: int32): int32 {
    return a >> b;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // 1 << 4 = 16
    test.assert_mir_function_output(
        module_id,
        "native",
        "shiftLeft",
        &[Value::int32(1), Value::int32(4)],
        Value::int32(16),
    );

    // 32 >> 2 = 8
    test.assert_mir_function_output(
        module_id,
        "native",
        "shiftRight",
        &[Value::int32(32), Value::int32(2)],
        Value::int32(8),
    );

    // -8 >> 1 = -4 (arithmetic shift preserves sign)
    test.assert_mir_function_output(
        module_id,
        "native",
        "shiftRight",
        &[Value::int32(-8), Value::int32(1)],
        Value::int32(-4),
    );
}

/// Verify bitwise NOT operator.
#[test]
fn test_lower_bitwise_not() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function bitwiseNot(a: int32): int32 {
    return ~a;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // ~0 = -1 (all bits flipped)
    test.assert_mir_function_output(
        module_id,
        "native",
        "bitwiseNot",
        &[Value::int32(0)],
        Value::int32(-1),
    );
}
