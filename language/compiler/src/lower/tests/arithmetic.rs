use destack_engine::Value;

use crate::TestProgram;

/// Lower and execute a simple add function.
#[test]
fn test_lower_adds_integers() {
    let test = TestProgram::memory_sequential_with_prelude();
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
function add(value0: int32, value1: int32): int32 {
entry0(value0: int32, value1: int32):
    value2: int32 = int.add value0, value1
    return value2
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
fn test_lower_computes_integer_arithmetic() {
    let test = TestProgram::memory_sequential_with_prelude();
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
fn test_lower_negates_values() {
    let test = TestProgram::memory_sequential_with_prelude();
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
        &[Value::float64(std::f64::consts::PI)],
        Value::float64(-std::f64::consts::PI),
    );
}

/// Verify int64 arithmetic operations.
#[test]
fn test_lower_computes_int64_arithmetic() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function addInt64(a: int64, b: int64): int64 {
    return a + b;
}

function mulInt64(a: int64, b: int64): int64 {
    return a * b;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // large values that exceed int32 range
    test.assert_mir_function_output(
        module_id,
        "native",
        "addInt64",
        &[Value::int64(3_000_000_000), Value::int64(2_000_000_000)],
        Value::int64(5_000_000_000),
    );

    test.assert_mir_function_output(
        module_id,
        "native",
        "mulInt64",
        &[Value::int64(100_000), Value::int64(100_000)],
        Value::int64(10_000_000_000),
    );
}

/// Verify float32 arithmetic operations.
#[test]
fn test_lower_computes_float32_arithmetic() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function addFloat32(a: float32, b: float32): float32 {
    return a + b;
}
    
function mulFloat32(a: float32, b: float32): float32 {
    return a * b;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "addFloat32",
        &[Value::float32(1.5), Value::float32(2.5)],
        Value::float32(4.0),
    );

    test.assert_mir_function_output(
        module_id,
        "native",
        "mulFloat32",
        &[Value::float32(3.0), Value::float32(4.0)],
        Value::float32(12.0),
    );
}

/// Verify mixed integer widths with explicit casts.
#[test]
fn test_lower_widens_integer_values() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function widen(a: int32): int64 {
    return a as int64;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "widen",
        &[Value::int32(42)],
        Value::int64(42),
    );
}
