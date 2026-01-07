use destack_vm::Value;

use crate::TestProgram;

/// Lower a simple add function into MIR and execute it.
#[test]
fn test_lower_add_function() {
    // setup module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function add(a: int32, b: int32): int32 {
    return a + b;
}
"#,
    );

    // lower and typecheck
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // assert mir output
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

    // assert execution
    test.assert_mir_function_output(
        module_id,
        "native",
        "add",
        &[Value::int32(1), Value::int32(2)],
        Value::int32(3),
    );
}

/// Lower fibonacci with recursion and execute it.
#[test]
fn test_lower_fibonacci_function() {
    // setup module
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

    // lower and typecheck
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // assert mir output
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

    // assert execution
    test.assert_mir_function_output(
        module_id,
        "native",
        "fibonacci",
        &[Value::float64(10.0)],
        Value::float64(55.0),
    );
}

/// Lower local assignments and preserve expression results.
#[test]
fn test_lower_assign_return() {
    // setup module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function assignReturn(x: number): number {
    let y = x + 1.0;
    y = y + 2.0;
    y = y + x;
    return y;
}
"#,
    );

    // lower and typecheck
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // assert mir output
    test.assert_mir(
        module_id,
        "native",
        r#"
function @assignReturn(v0: f64) -> f64 {
block0:
    v1 = iconst 1f64
    v2 = fadd v0, v1
    v3 = iconst 2f64
    v4 = fadd v2, v3
    v5 = fadd v4, v0
    return v5
}
        "#,
    );

    // assert execution
    test.assert_mir_function_output(
        module_id,
        "native",
        "assignReturn",
        &[Value::float64(10.0)],
        Value::float64(23.0),
    );
}
