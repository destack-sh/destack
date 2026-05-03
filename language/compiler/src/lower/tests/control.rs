use destack_engine::Value;

use crate::TestProgram;

/// Lower and execute fibonacci with recursion and if/else control flow.
#[test]
fn test_lower_computes_fibonacci() {
    let test = TestProgram::memory_sequential_with_prelude();
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
function fibonacci(value0: float64): float64 {
entry0(value0: float64):
    value1: int32 = 2int32
    value2: float64 = cast.intToFloat.s value1 -> float64
    value3: boolean = float.lt value0, value2
    branch value3, block1, block2

block1:
    return value0

block2:
    jump block3

block3:
    value6: int32 = 1int32
    value7: float64 = cast.intToFloat.s value6 -> float64
    value8: float64 = float.sub value0, value7
    value9: float64 = call fibonacci(value8): (float64) -> float64
    value10: int32 = 2int32
    value11: float64 = cast.intToFloat.s value10 -> float64
    value12: float64 = float.sub value0, value11
    value13: float64 = call fibonacci(value12): (float64) -> float64
    value14: float64 = float.add value9, value13
    return value14
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
fn test_lower_while_loop() {
    let test = TestProgram::memory_sequential_with_prelude();
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
fn test_lower_for_loop() {
    let test = TestProgram::memory_sequential_with_prelude();
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

/// Lower uninitialized let bindings that are assigned in control flow.
#[test]
fn test_lower_uninitialized_let_assignment_flow() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function choose(flag: boolean, a: number, b: number): number {
    let value: number;
    if (flag) {
        value = a;
    } else {
        value = b;
    }
    return value;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "choose",
        &[Value::bool(true), Value::float64(3.0), Value::float64(5.0)],
        Value::float64(3.0),
    );
    test.assert_mir_function_output(
        module_id,
        "native",
        "choose",
        &[Value::bool(false), Value::float64(3.0), Value::float64(5.0)],
        Value::float64(5.0),
    );
}

/// Lower and execute unlabeled break in a while loop.
#[test]
fn test_lower_unlabeled_break() {
    let test = TestProgram::memory_sequential_with_prelude();
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
fn test_lower_unlabeled_continue() {
    let test = TestProgram::memory_sequential_with_prelude();
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
fn test_lower_ternary_expression() {
    let test = TestProgram::memory_sequential_with_prelude();
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
fn test_lower_ternary_inline_condition() {
    let test = TestProgram::memory_sequential_with_prelude();
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
fn test_lower_nested_ternary() {
    let test = TestProgram::memory_sequential_with_prelude();
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

/// Lower nested loops with inner break.
#[test]
fn test_lower_nested_loops() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function nestedSum(rows: number, cols: number): number {
    let sum: number = 0.0;
    let i: number = 0.0;
    while (i < rows) {
        let j: number = 0.0;
        while (j < cols) {
            sum = sum + 1.0;
            j = j + 1.0;
        }
        i = i + 1.0;
    }
    return sum;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // 3 rows * 4 cols = 12 iterations
    test.assert_mir_function_output(
        module_id,
        "native",
        "nestedSum",
        &[Value::float64(3.0), Value::float64(4.0)],
        Value::float64(12.0),
    );
}

/// Lower multiple functions that call each other.
#[test]
fn test_lower_mutual_function_calls() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function double(x: number): number {
    return x * 2.0;
}

function triple(x: number): number {
    return x * 3.0;
}

function combine(x: number): number {
    return double(x) + triple(x);
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // double(5) + triple(5) = 10 + 15 = 25
    test.assert_mir_function_output(
        module_id,
        "native",
        "combine",
        &[Value::float64(5.0)],
        Value::float64(25.0),
    );
}

/// Lower nested if-else chains.
#[test]
fn test_lower_nested_if_else() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function classify(x: number, y: number): number {
    let result: number = 0.0;
    if (x > 0.0) {
        if (y > 0.0) {
            result = 1.0;
        } else {
            result = 4.0;
        }
    } else {
        if (y > 0.0) {
            result = 2.0;
        } else {
            result = 3.0;
        }
    }
    return result;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "classify",
        &[Value::float64(1.0), Value::float64(1.0)],
        Value::float64(1.0),
    );
    test.assert_mir_function_output(
        module_id,
        "native",
        "classify",
        &[Value::float64(-1.0), Value::float64(1.0)],
        Value::float64(2.0),
    );
    test.assert_mir_function_output(
        module_id,
        "native",
        "classify",
        &[Value::float64(-1.0), Value::float64(-1.0)],
        Value::float64(3.0),
    );
    test.assert_mir_function_output(
        module_id,
        "native",
        "classify",
        &[Value::float64(1.0), Value::float64(-1.0)],
        Value::float64(4.0),
    );
}

/// Lower early return from loop.
#[test]
fn test_lower_early_return_from_loop() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function findFirst(target: number): number {
    let i: number = 0.0;
    while (i < 100.0) {
        if (i == target) {
            return i;
        }
        i = i + 1.0;
    }
    return -1.0;
}
"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "findFirst",
        &[Value::float64(42.0)],
        Value::float64(42.0),
    );
}

/// Lower and execute a switch statement.
#[test]
fn test_lower_switch_statement() {
    let test = TestProgram::memory_sequential_with_prelude();
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
