use destack_vm::Value;

use crate::TestProgram;

/// Lower class construction with `new`.
#[test]
fn test_class_construction_with_new() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Point {
    x: number = 0;
    y: number = 0;
}

function sumFieldsClass(a: number, b: number): number {
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
        "sumFieldsClass",
        &[Value::float64(6.0), Value::float64(7.0)],
        Value::float64(13.0),
    );
}

/// Lower class construction to managed allocation in MIR.
#[test]
fn test_class_new_mir() {
    // set up the test program
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Box {
    value: int32 = 0;
}

function sumBox(value: int32): int32 {
    let b: Box = new Box(value);
    return b.value + 1;
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
function @sumBox(v0: i32) -> i32 {
block0(v0: i32):
    v1 = struct { value: i32 } (v0)
    v2 = managed.alloc { value: i32 }
    store v2, v1
    v3 = load v2
    v4 = field.get v3, 0
    v5 = iconst 1i32
    v6 = trunc v5 -> i32
    v7 = iadd v4, v6
    return v7
}
        "#,
    );

    // assert the runtime output
    test.assert_mir_function_output(
        module_id,
        "native",
        "sumBox",
        &[Value::int32(9)],
        Value::int32(10),
    );
}

/// Lower explicit class constructors.
#[test]
fn test_class_explicit_constructor() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Point {
    x: number;
    y: number;

    constructor(x: number, y: number) {
        this.x = x;
        this.y = y;
        return;
    }
}

function sumFieldsClassExplicit(a: number, b: number): number {
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
        "sumFieldsClassExplicit",
        &[Value::float64(8.0), Value::float64(9.0)],
        Value::float64(17.0),
    );
}

/// Lower class method that returns a field via `this`.
#[test]
fn test_class_method_returns_field() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Box {
    value: int32 = 0;

    get(): int32 {
        return this.value;
    }
}

function readValueClass(value: int32): int32 {
    let b: Box = new Box(value);
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
        "readValueClass",
        &[Value::int32(9)],
        Value::int32(9),
    );
}

/// Lower class method with parameters.
#[test]
fn test_class_method_with_parameters() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Adder {
    base: int32 = 0;

    add(n: int32): int32 {
        return this.base + n;
    }
}

function computeClass(base: int32, delta: int32): int32 {
    let a: Adder = new Adder(base);
    return a.add(delta);
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "computeClass",
        &[Value::int32(10), Value::int32(5)],
        Value::int32(15),
    );
}

/// Lower class vtable metadata with override reuse.
#[test]
fn test_class_vtable_metadata() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Animal {
    name: int32 = 0;
    speak(): int32 { return 1; }
}

class Dog extends Animal {
    breed: int32 = 0;
    override speak(): int32 { return 2; }
}

function useDog(d: Dog): int32 {
    return d.speak();
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // collect the class vtables
        let vtables = test.class_dispatch_tables(tree);
        assert_eq!(vtables.len(), 2);

        // resolve the vtables by unique field names
        let animal_table = test
            .find_class_vtable_by_field_name(tree, strings, &vtables, "name")
            .expect("missing animal vtable");
        let dog_table = test
            .find_class_vtable_by_field_name(tree, strings, &vtables, "breed")
            .expect("missing dog vtable");

        // assert the fixed vtable prefix
        test.assert_vtable_prefix(animal_table);

        // count the virtual method slots
        let animal_methods = test.count_vtable_methods(animal_table);
        let dog_methods = test.count_vtable_methods(dog_table);

        // assert the override reuse
        assert_eq!(animal_methods, 1);
        assert_eq!(dog_methods, 1);
    });
}
