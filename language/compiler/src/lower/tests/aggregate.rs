use destack_vm::Value;

use crate::TestProgram;

/// Lower struct construction and field access.
#[test]
fn test_struct_construction_and_access() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Point { x: number, y: number }

function sumFields(a: number, b: number): number {
    let p: Point = Point { x: a, y: b };
    return p.x + p.y;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "sumFields",
        &[Value::float64(3.0), Value::float64(4.0)],
        Value::float64(7.0),
    );
}

/// Lower struct with multiple field accesses.
#[test]
fn test_struct_multiple_field_access() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Data { a: number, b: number, c: number }

function accessAll(x: number, y: number, z: number): number {
    let d: Data = Data { a: x, b: y, c: z };
    return d.a + d.b + d.c;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "accessAll",
        &[
            Value::float64(10.0),
            Value::float64(20.0),
            Value::float64(30.0),
        ],
        Value::float64(60.0),
    );
}

/// Lower struct with fields initialized in different order than declaration.
#[test]
fn test_struct_field_order_independence() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Vec2 { x: number, y: number }

function makeReversed(a: number, b: number): number {
    let v: Vec2 = Vec2 { y: b, x: a };
    return v.x * 10.0 + v.y;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // x=1, y=2 -> 1*10 + 2 = 12
    test.assert_mir_function_output(
        module_id,
        "native",
        "makeReversed",
        &[Value::float64(1.0), Value::float64(2.0)],
        Value::float64(12.0),
    );
}
