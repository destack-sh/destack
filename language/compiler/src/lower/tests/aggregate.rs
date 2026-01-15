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

/// Lower nested struct construction and field access.
#[test]
fn test_nested_struct_access() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Inner { value: number }
struct Outer { inner: Inner, scale: number }

function getScaledValue(v: number, s: number): number {
    let inner: Inner = Inner { value: v };
    let outer: Outer = Outer { inner: inner, scale: s };
    return outer.inner.value * outer.scale;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "getScaledValue",
        &[Value::float64(5.0), Value::float64(3.0)],
        Value::float64(15.0),
    );
}

/// Lower struct with multiple nested levels.
#[test]
fn test_deeply_nested_struct() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct A { x: number }
struct B { a: A }
struct C { b: B }

function deepAccess(val: number): number {
    let a: A = A { x: val };
    let b: B = B { a: a };
    let c: C = C { b: b };
    return c.b.a.x;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "deepAccess",
        &[Value::float64(42.0)],
        Value::float64(42.0),
    );
}

/// Lower struct with a simple method call.
#[test]
fn test_struct_method_call() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Point {
    x: int32
    y: int32

    sum(): int32 {
        this.x + this.y
    }
}

function getSum(a: int32, b: int32): int32 {
    let p: Point = Point { x: a, y: b };
    p.sum()
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "getSum",
        &[Value::int32(3), Value::int32(4)],
        Value::int32(7),
    );
}

/// Lower struct method that returns a new instance of the same type.
#[test]
fn test_struct_method_returning_self() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Counter {
    value: int32

    increment(): Counter {
        Counter { value: this.value + 1 }
    }
}

function bump(n: int32): int32 {
    let c: Counter = Counter { value: n };
    let c2: Counter = c.increment();
    c2.value
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "bump",
        &[Value::int32(5)],
        Value::int32(6),
    );
}

/// Lower struct method with parameters.
#[test]
fn test_struct_method_with_parameters() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Adder {
    base: int32

    add(n: int32): int32 {
        this.base + n
    }
}

function compute(base: int32, delta: int32): int32 {
    let a: Adder = Adder { base: base };
    a.add(delta)
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "compute",
        &[Value::int32(10), Value::int32(5)],
        Value::int32(15),
    );
}

/// Lower chained method calls.
#[test]
fn test_struct_chained_method_calls() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Counter {
    value: int32

    increment(): Counter {
        Counter { value: this.value + 1 }
    }

    getValue(): int32 {
        this.value
    }
}

function bumpTwice(n: int32): int32 {
    let c: Counter = Counter { value: n };
    c.increment().increment().getValue()
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "bumpTwice",
        &[Value::int32(0)],
        Value::int32(2),
    );
}
