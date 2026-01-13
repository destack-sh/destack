use destack_vm::Value;

use crate::TestProgram;

/// Lower and execute fibonacci with recursion and if/else control flow.
#[test]
fn test_fibonacci() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function fibonacci(n: number): number {
    if (n < 2) {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
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
function @fibonacci(v0: f64) -> f64 {
block0:
    v1 = iconst 2i32
    v2 = scvt_to_float v1 -> f64
    v3 = fcmp_lt v0, v2
    branch v3, block1, block2
block1:
    return v0
block2:
    jump block3
block3:
    v6 = iconst 1f64
    v7 = fsub v0, v6
    v8 = call @fibonacci(v7)
    v9 = iconst 2f64
    v10 = fsub v0, v9
    v11 = call @fibonacci(v10)
    v12 = fadd v8, v11
    return v12
}
"#,
    );

    test.assert_mir_function_output(
        module_id,
        "native",
        "fibonacci",
        &[Value::float64(10.0)],
        Value::float64(55.0),
    );
}

/// Lower and execute a simple while loop.
#[test]
fn test_while_loop() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function sumTo(n: number): number {
    let sum: number = 0.0;
    let i: number = 0.0;
    while (i <= n) {
        sum = sum + i;
        i = i + 1.0;
    }
    return sum;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // sum of 0..10 = 55
    test.assert_mir_function_output(
        module_id,
        "native",
        "sumTo",
        &[Value::float64(10.0)],
        Value::float64(55.0),
    );
}

/// Lower and execute a for loop.
#[test]
fn test_for_loop() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function factorial(n: number): number {
    let result: number = 1.0;
    for (let i: number = 1.0; i <= n; i = i + 1.0) {
        result = result * i;
    }
    return result;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // 5! = 120
    test.assert_mir_function_output(
        module_id,
        "native",
        "factorial",
        &[Value::float64(5.0)],
        Value::float64(120.0),
    );
}

/// Lower and execute unlabeled break in a while loop.
#[test]
fn test_unlabeled_break() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function findFirst(n: number): number {
    let i: number = 0.0;
    while (i < n) {
        if (i >= 5.0) {
            break;
        }
        i = i + 1.0;
    }
    return i;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // should stop at 5
    test.assert_mir_function_output(
        module_id,
        "native",
        "findFirst",
        &[Value::float64(10.0)],
        Value::float64(5.0),
    );
}

/// Lower and execute unlabeled continue in a for loop.
#[test]
fn test_unlabeled_continue() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function sumSkipMiddle(n: number): number {
    let sum: number = 0.0;
    for (let i: number = 1.0; i <= n; i = i + 1.0) {
        // skip values 4, 5, 6
        if (i >= 4.0) {
            if (i <= 6.0) {
                continue;
            }
        }
        sum = sum + i;
    }
    return sum;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // sum of 1+2+3+7+8+9+10 = 40 (skipping 4+5+6)
    test.assert_mir_function_output(
        module_id,
        "native",
        "sumSkipMiddle",
        &[Value::float64(10.0)],
        Value::float64(40.0),
    );
}

/// Lower and execute ternary expression with boolean variable.
#[test]
fn test_ternary_expression() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function abs(n: number): number {
    let isPositive: boolean = n >= 0.0;
    return isPositive ? n : 0.0 - n;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // abs(-5) = 5
    test.assert_mir_function_output(
        module_id,
        "native",
        "abs",
        &[Value::float64(-5.0)],
        Value::float64(5.0),
    );

    // abs(3) = 3
    test.assert_mir_function_output(
        module_id,
        "native",
        "abs",
        &[Value::float64(3.0)],
        Value::float64(3.0),
    );
}

/// Lower and execute ternary with inline condition.
#[test]
fn test_ternary_inline_condition() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function max(a: number, b: number): number {
    return (a > b) ? a : b;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "max",
        &[Value::float64(3.0), Value::float64(5.0)],
        Value::float64(5.0),
    );

    test.assert_mir_function_output(
        module_id,
        "native",
        "max",
        &[Value::float64(7.0), Value::float64(2.0)],
        Value::float64(7.0),
    );
}

/// Lower and execute nested ternary expressions.
#[test]
fn test_nested_ternary() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function sign(n: number): number {
    return (n > 0.0) ? 1.0 : ((n < 0.0) ? -1.0 : 0.0);
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "sign",
        &[Value::float64(5.0)],
        Value::float64(1.0),
    );

    test.assert_mir_function_output(
        module_id,
        "native",
        "sign",
        &[Value::float64(-3.0)],
        Value::float64(-1.0),
    );

    test.assert_mir_function_output(
        module_id,
        "native",
        "sign",
        &[Value::float64(0.0)],
        Value::float64(0.0),
    );
}

/// Lower and execute a switch statement.
#[test]
fn test_switch_statement() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function dayType(day: number): number {
    let result: number = 0.0;
    switch (day) {
        case 0.0:
        case 6.0:
            result = 1.0;  // weekend
            break;
        case 1.0:
        case 2.0:
        case 3.0:
        case 4.0:
        case 5.0:
            result = 2.0;  // weekday
            break;
        default:
            result = 0.0;  // invalid
            break;
    }
    return result;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // Sunday (0) = weekend
    test.assert_mir_function_output(
        module_id,
        "native",
        "dayType",
        &[Value::float64(0.0)],
        Value::float64(1.0),
    );

    // Wednesday (3) = weekday
    test.assert_mir_function_output(
        module_id,
        "native",
        "dayType",
        &[Value::float64(3.0)],
        Value::float64(2.0),
    );

    // Saturday (6) = weekend
    test.assert_mir_function_output(
        module_id,
        "native",
        "dayType",
        &[Value::float64(6.0)],
        Value::float64(1.0),
    );

    // Invalid (10) = invalid
    test.assert_mir_function_output(
        module_id,
        "native",
        "dayType",
        &[Value::float64(10.0)],
        Value::float64(0.0),
    );
}
