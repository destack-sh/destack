use destack_engine::Value;

use crate::TestProgram;

/// Verify all integer comparison operators.
#[test]
fn test_lower_compares_integers() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function lessThan(a: int32, b: int32): boolean {
    return a < b;
}

function lessEqual(a: int32, b: int32): boolean {
    return a <= b;
}

function greaterThan(a: int32, b: int32): boolean {
    return a > b;
}

function greaterEqual(a: int32, b: int32): boolean {
    return a >= b;
}

function equal(a: int32, b: int32): boolean {
    return a == b;
}

function notEqual(a: int32, b: int32): boolean {
    return a != b;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "lessThan",
        &[Value::int32(3), Value::int32(5)],
        Value::bool(true),
    );

    test.assert_mir_function_output(
        module_id,
        "native",
        "lessEqual",
        &[Value::int32(5), Value::int32(5)],
        Value::bool(true),
    );

    test.assert_mir_function_output(
        module_id,
        "native",
        "greaterThan",
        &[Value::int32(7), Value::int32(5)],
        Value::bool(true),
    );

    test.assert_mir_function_output(
        module_id,
        "native",
        "greaterEqual",
        &[Value::int32(5), Value::int32(5)],
        Value::bool(true),
    );

    test.assert_mir_function_output(
        module_id,
        "native",
        "equal",
        &[Value::int32(5), Value::int32(5)],
        Value::bool(true),
    );

    test.assert_mir_function_output(
        module_id,
        "native",
        "notEqual",
        &[Value::int32(5), Value::int32(5)],
        Value::bool(false),
    );
}
