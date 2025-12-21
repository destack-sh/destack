use destack_machine::Value;

use crate::TestProgram;

#[test]
fn test_lower_add_function() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function add(a: int32, b: int32): int32 {
    return a + b;
}
"#,
    );
    test.lower_module(module_id, "native");
    test.compile_dump_check();
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

#[test]
fn test_lower_fib_function() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function fib(n: number): number {
    if (n < 2) {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}
"#,
    );
    test.lower_module(module_id, "native");
    test.compile_dump_check();
    test.assert_mir(
        module_id,
        "native",
        r#"
function @fib(v0: f64) -> f64 {
block0:
    v1 = fconst 2f64
    v2 = fcmp_lt v0, v1
    branch v2, block1, block2
block1:
    return v0
block2:
    jump block3
block3:
    v3 = fconst 1f64
    v4 = fsub v0, v3
    v5 = call @fib(v4)
    v6 = fconst 2f64
    v7 = fsub v0, v6
    v8 = call @fib(v7)
    v9 = fadd v5, v8
    return v9
}
"#,
    );
    test.assert_mir_function_output(
        module_id,
        "native",
        "fib",
        &[Value::float64(10.0)],
        Value::float64(55.0),
    );
}
