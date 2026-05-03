use destack_engine::Value;

use crate::TestProgram;

/// Verify logical AND truth table.
#[test]
fn test_lower_logical_and() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function logicalAnd(a: boolean, b: boolean): boolean {
    return a && b;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // true && true = true
    test.assert_mir_function_output(
        module_id,
        "native",
        "logicalAnd",
        &[Value::bool(true), Value::bool(true)],
        Value::bool(true),
    );

    // true && false = false
    test.assert_mir_function_output(
        module_id,
        "native",
        "logicalAnd",
        &[Value::bool(true), Value::bool(false)],
        Value::bool(false),
    );

    // false && true = false (short-circuits, doesn't evaluate second operand)
    test.assert_mir_function_output(
        module_id,
        "native",
        "logicalAnd",
        &[Value::bool(false), Value::bool(true)],
        Value::bool(false),
    );

    // false && false = false
    test.assert_mir_function_output(
        module_id,
        "native",
        "logicalAnd",
        &[Value::bool(false), Value::bool(false)],
        Value::bool(false),
    );
}

/// Verify logical OR truth table.
#[test]
fn test_lower_logical_or() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function logicalOr(a: boolean, b: boolean): boolean {
    return a || b;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // true || true = true (short-circuits)
    test.assert_mir_function_output(
        module_id,
        "native",
        "logicalOr",
        &[Value::bool(true), Value::bool(true)],
        Value::bool(true),
    );

    // true || false = true (short-circuits, doesn't evaluate second operand)
    test.assert_mir_function_output(
        module_id,
        "native",
        "logicalOr",
        &[Value::bool(true), Value::bool(false)],
        Value::bool(true),
    );

    // false || true = true
    test.assert_mir_function_output(
        module_id,
        "native",
        "logicalOr",
        &[Value::bool(false), Value::bool(true)],
        Value::bool(true),
    );

    // false || false = false
    test.assert_mir_function_output(
        module_id,
        "native",
        "logicalOr",
        &[Value::bool(false), Value::bool(false)],
        Value::bool(false),
    );
}

/// Verify AND generates short-circuit control flow that skips RHS when LHS is false.
#[test]
fn test_lower_logical_and_short_circuit() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function logicalAnd(a: boolean, b: boolean): boolean {
    return a && b;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // block1 is the short-circuit path (false), block2 evaluates b
    test.assert_mir(
        module_id,
        "native",
        r#"
function logicalAnd(value0: boolean, value1: boolean): boolean {
entry0(value0: boolean, value1: boolean):
    branch value0, block2, block1

block1:
    value2: boolean = false
    jump block3(value2)

block2:
    jump block3(value1)

block3(value4: boolean):
    return value4
}
"#,
    );
}

/// Verify OR generates short-circuit control flow that skips RHS when LHS is true.
#[test]
fn test_lower_logical_or_short_circuit() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function logicalOr(a: boolean, b: boolean): boolean {
    return a || b;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // block1 is the short-circuit path (true), block2 evaluates b
    test.assert_mir(
        module_id,
        "native",
        r#"
function logicalOr(value0: boolean, value1: boolean): boolean {
entry0(value0: boolean, value1: boolean):
    branch value0, block1, block2

block1:
    value2: boolean = true
    jump block3(value2)

block2:
    jump block3(value1)

block3(value4: boolean):
    return value4
}
"#,
    );
}

/// Verify logical NOT operator.
#[test]
fn test_lower_logical_not() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function logicalNot(a: boolean): boolean {
    return !a;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // !true = false
    test.assert_mir_function_output(
        module_id,
        "native",
        "logicalNot",
        &[Value::bool(true)],
        Value::bool(false),
    );

    // !false = true
    test.assert_mir_function_output(
        module_id,
        "native",
        "logicalNot",
        &[Value::bool(false)],
        Value::bool(true),
    );
}
