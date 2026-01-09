use destack_vm::Value;

use crate::TestProgram;

/// Verify module-level const declarations are lowered correctly.
#[test]
fn test_module_const() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
const PI = 3.14159;
const TWO = 2;

function getCircumference(radius: number): number {
    return TWO * PI * radius;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "getCircumference",
        &[Value::float64(1.0)],
        Value::float64(6.28318),
    );
}

/// Verify let bindings and reassignments produce correct SSA form.
#[test]
fn test_let_and_assign() {
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

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

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

    test.assert_mir_function_output(
        module_id,
        "native",
        "assignReturn",
        &[Value::float64(10.0)],
        Value::float64(23.0),
    );
}
