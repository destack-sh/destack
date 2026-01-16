use destack_vm::Value;

use crate::{TaskPhase, TestProgram};

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

/// Lower struct construction with `new`.
#[test]
fn test_struct_construction_with_new() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Point { x: number, y: number }

function sumFieldsNew(a: number, b: number): number {
    let p: Point = new Point(a, b);
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
        "sumFieldsNew",
        &[Value::float64(5.0), Value::float64(6.0)],
        Value::float64(11.0),
    );
}

/// Lower struct construction to value initialization in MIR.
#[test]
fn test_struct_new_mir() {
    // set up the test program
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Point { x: int32, y: int32 }

function sumPoint(a: int32, b: int32): int32 {
    let p: Point = new Point(a, b);
    return p.x + p.y;
}
"#,
    );

    // lower the module
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // assert the lowered mir
    test.assert_mir(
        module_id,
        "native",
        r#"
function @sumPoint(v0: i32, v1: i32) -> i32 {
block0:
    v2 = struct { x: i32, y: i32 } (v0, v1)
    v3 = field.get v2, 0
    v4 = field.get v2, 1
    v5 = iadd v3, v4
    return v5
}
        "#,
    );

    // assert the runtime output
    test.assert_mir_function_output(
        module_id,
        "native",
        "sumPoint",
        &[Value::int32(3), Value::int32(4)],
        Value::int32(7),
    );
}

/// Lower explicit struct constructors.
#[test]
fn test_struct_explicit_constructor() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Point {
    x: number;
    y: number;

    constructor(x: number, y: number) {
        this.x = x;
        this.y = y;
        return;
    }
}

function sumFieldsExplicit(a: number, b: number): number {
    let p: Point = new Point(a, b);
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
        "sumFieldsExplicit",
        &[Value::float64(2.0), Value::float64(9.0)],
        Value::float64(11.0),
    );
}

/// Constructor return values should be rejected.
#[test]
fn test_struct_constructor_return_value_error() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Point {
    x: number;
    y: number;

    constructor(x: number, y: number) {
        this.x = x;
        this.y = y;
        return this;
    }
}

function makePoint(a: number, b: number): Point {
    return new Point(a, b);
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile();
    test.check_no_diagnostics_up_to_excluding_phase(TaskPhase::Lower);
    test.check_has_diagnostic("EM200");
}

/// Constructors must initialize all fields before returning.
#[test]
fn test_struct_constructor_missing_field_error() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Point {
    x: number;
    y: number;

    constructor(x: number) {
        this.x = x;
    }
}

function makePoint(a: number): Point {
    return new Point(a);
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile();
    test.check_no_diagnostics_up_to_excluding_phase(TaskPhase::Lower);
    test.check_has_diagnostic("EM200");
}

/// Constructors cannot read fields before initialization.
#[test]
fn test_struct_constructor_read_before_init_error() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Point {
    x: number;
    y: number;

    constructor(x: number, y: number) {
        let previous = this.y;
        this.x = x;
        this.y = y + previous;
    }
}

function makePoint(a: number, b: number): Point {
    return new Point(a, b);
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile();
    test.check_no_diagnostics_up_to_excluding_phase(TaskPhase::Lower);
    test.check_has_diagnostic("EM200");
}

/// Lower struct method that returns a field via `this`.
#[test]
fn test_struct_method_returns_field() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Box {
    value: int32;

    get(): int32 {
        return this.value;
    }
}

function readValue(value: int32): int32 {
    let b: Box = Box { value: value };
    return b.get();
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "readValue",
        &[Value::int32(7)],
        Value::int32(7),
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
