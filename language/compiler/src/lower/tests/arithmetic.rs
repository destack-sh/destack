use destack_vm::Value;

use crate::TestProgram;

/// Lower and execute a simple add function.
#[test]
fn test_add() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function add(a: int32, b: int32): int32 {
    return a + b;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function @add(v0: i32, v1: i32) -> i32 {
block0:
    v2 = iadd v0, v1
    return v2
}
        "#,
    );

    test.assert_mir_function_output(
        module_id,
        "native",
        "add",
        &[Value::int32(1), Value::int32(2)],
        Value::int32(3),
    );
}

/// Verify all integer arithmetic operators: add, sub, mul, div, rem.
#[test]
fn test_integer_arithmetic() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function arithmetic(a: int32, b: int32): int32 {
    let sum = a + b;
    let diff = a - b;
    let prod = a * b;
    let quot = a / b;
    let rem = a % b;
    return sum + diff + prod + quot + rem;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // 10+3=13, 10-3=7, 10*3=30, 10/3=3, 10%3=1 => 54
    test.assert_mir_function_output(
        module_id,
        "native",
        "arithmetic",
        &[Value::int32(10), Value::int32(3)],
        Value::int32(54),
    );
}

/// Verify unary negation for integers and floats.
#[test]
fn test_unary_negate() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function negate(a: int32): int32 {
    return -a;
}
function floatNegate(a: number): number {
    return -a;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "negate",
        &[Value::int32(5)],
        Value::int32(-5),
    );

    test.assert_mir_function_output(
        module_id,
        "native",
        "floatNegate",
        &[Value::float64(3.14)],
        Value::float64(-3.14),
    );
}
