use destack_engine::Value;
use destack_mir as mir;

use crate::TestProgram;

/// Lower dynamic table metadata for structs.
#[test]
fn test_lower_struct_dynamic_table_metadata() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Drawable {
    color: int32;
    draw(): int32;
}

struct Circle implements Drawable {
    color: int32;
    radius: int32;

    draw(): int32 {
        return this.color;
    }
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // assert the lowered mir
    test.assert_mir(
        module_id,
        "native",
        r#"
type Drawable.function = () => int32;

type Circle {
    color: int32;
    radius: int32;
}

type Drawable.object {
    draw: Drawable.function;
    color: int32;
}

external function Drawable.draw(Drawable.object): int32

function Circle.draw(this0: Circle): int32 {
entry0(this0: Circle):
    value1: int32 = field.get this0, 0
    return value1
}
"#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // collect dynamic dispatch tables
        let dynamic_table = test.expect_single_dynamic_table(tree);

        // assert the field offset slot
        let offset = test.expect_dynamic_field_offset(dynamic_table, tree, strings, "color");
        assert_eq!(offset, 0);

        // assert the dynamic method slot
        let target_name =
            test.expect_dynamic_method_target_name(dynamic_table, tree, strings, "draw");
        assert_eq!(target_name, "Circle.draw");
    });
}

/// Lower erased dynamic layouts into dynamic values.
#[test]
fn test_lower_interface_any_layout() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Drawable {
    color: int32;
    draw(): int32;
}

struct Circle implements Drawable {
    color: int32;
    radius: int32;

    draw(): int32 { return this.color; }
}

function keep(value: Drawable): Drawable {
    return value;
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
type Drawable.function = () => int32;

type Circle {
    color: int32;
    radius: int32;
}

type Drawable.object {
    draw: Drawable.function;
    color: int32;
}
type Drawable {
    value: ref<void, managed, readonly>;
    table: ref<void, raw, readonly, space(static)>;
}

external function Drawable.draw(Drawable.object): int32

function keep(value0: Drawable): Drawable {
entry0(value0: Drawable):
    return value0
}

function Circle.draw(this0: Circle): int32 {
entry0(this0: Circle):
    value1: int32 = field.get this0, 0
    return value1
}
"#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        let constraint_type = test.type_by_metadata_name(tree, strings, "test/test:Drawable");

        let value_type =
            test.expect_struct_field_type_by_name(tree, strings, constraint_type, "value");
        let table_type = test.expect_struct_field_type_by_name(tree, strings, constraint_type, "table");

        let value_type = tree.get(value_type);
        let mir::Type::Reference { kind, pointee, .. } = value_type else {
            panic!("expected managed reference for dynamic value field");
        };
        assert!(matches!(kind, mir::ReferenceKind::Managed));
        assert!(matches!(
            tree.get(
                pointee
                    .ty()
                    .expect("dynamic value pointee should be concrete")
            ),
            mir::Type::Void
        ));

        let table_type = tree.get(table_type);
        assert!(matches!(
            table_type,
            mir::Type::Reference {
                kind: mir::ReferenceKind::Raw,
                space: mir::Space::Static,
                ..
            }
        ));
    });
}

/// Lower structural interface parameters for anonymous object types.
#[test]
#[ignore]
fn test_lower_structural_interface_parameter() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Drawable {
    draw(): int32;
}

struct Circle implements Drawable {
    value: int32;

    draw(): int32 { return this.value; }
}

function render(drawable: Drawable): int32 {
    return drawable.draw();
}

function run(): int32 {
    let circle = Circle { value: 11 };
    return render(circle);
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(module_id, "native", "run", &[], Value::int32(11));
}

/// Lower structural interface parameters with field-only shapes.
#[test]
#[ignore] // TODO #Incomplete: structural interfaces (see other structural interface tests here!)
fn test_lower_structural_interface_fields() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Rectangle {
    x: int32;
    y: int32;
    width: int32;
    height: int32;
}

struct Rect implements Rectangle {
    x: int32;
    y: int32;
    width: int32;
    height: int32;
}

function area(rect: Rectangle): int32 {
    return rect.width * rect.height;
}

function run(): int32 {
    let rect = Rect { x: 0, y: 0, width: 4, height: 5 };
    return area(rect);
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(module_id, "native", "run", &[], Value::int32(20));
}

/// Lower structural interface parameters with mixed fields and methods.
#[test]
#[ignore]
fn test_lower_structural_interface_mixed() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Rectangle {
    width: int32;
    height: int32;
    area(): int32;
}

struct Rect implements Rectangle {
    width: int32;
    height: int32;

    area(): int32 {
        return this.width * this.height;
    }
}

function render(rect: Rectangle): int32 {
    return rect.area();
}

function run(): int32 {
    let rect = Rect { width: 6, height: 7 };
    return render(rect);
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(module_id, "native", "run", &[], Value::int32(42));
}

/// Lower dynamic table slots in declaration order for mixed members.
#[test]
fn test_lower_orders_dynamic_table_slots() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Shape {
    width: int32;
    area(): int32;
}

interface Paint {
    color: int32;
    paint(): int32;
}

struct Widget implements Shape, Paint {
    width: int32;
    color: int32;

    area(): int32 {
        return this.width;
    }

    paint(): int32 {
        return this.color;
    }
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // assert the lowered mir
    test.assert_mir(
        module_id,
        "native",
        r#"
type Shape.function = () => int32;

type Shape.object {
    area: Shape.function;
    width: int32;
}

type Paint.object {
    paint: Shape.function;
    color: int32;
}

type Widget {
    width: int32;
    color: int32;
}

external function Shape.area(Shape.object): int32

external function Paint.paint(Paint.object): int32

function Widget.area(this0: Widget): int32 {
entry0(this0: Widget):
    value1: int32 = field.get this0, 0
    return value1
}

function Widget.paint(this0: Widget): int32 {
entry0(this0: Widget):
    value1: int32 = field.get this0, 1
    return value1
}
"#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        let shape_table = test.dynamic_dispatch_table(
            tree,
            strings,
            "test/test:Widget",
            "test/test:Shape.object",
        );
        let paint_table = test.dynamic_dispatch_table(
            tree,
            strings,
            "test/test:Widget",
            "test/test:Paint.object",
        );

        let shape = tree
            .metadata
            .dispatch
            .dynamic_shape(shape_table.constraint)
            .expect("missing Shape dynamic shape");
        match (&shape_table.entries[0], &shape.slots[0]) {
            (
                mir::DynamicEntry::Field { offset },
                mir::DynamicSlot::Field { name, .. },
            ) => {
                assert_eq!(strings.get(*name), "width");
                assert_eq!(*offset, 0);
            }
            _ => panic!("expected field offset slot for width"),
        }
        match (&shape_table.entries[1], &shape.slots[1]) {
            (
                mir::DynamicEntry::Method { .. },
                mir::DynamicSlot::Method { name, .. },
            ) => {
                assert_eq!(strings.get(*name), "area");
            }
            _ => panic!("expected dynamic method slot for area"),
        }

        let paint = tree
            .metadata
            .dispatch
            .dynamic_shape(paint_table.constraint)
            .expect("missing Paint dynamic shape");
        match (&paint_table.entries[0], &paint.slots[0]) {
            (
                mir::DynamicEntry::Field { offset },
                mir::DynamicSlot::Field { name, .. },
            ) => {
                assert_eq!(strings.get(*name), "color");
                assert_eq!(*offset, 4);
            }
            _ => panic!("expected field offset slot for color"),
        }
        match (&paint_table.entries[1], &paint.slots[1]) {
            (
                mir::DynamicEntry::Method { .. },
                mir::DynamicSlot::Method { name, .. },
            ) => {
                assert_eq!(strings.get(*name), "paint");
            }
            _ => panic!("expected dynamic method slot for paint"),
        }
    });
}

/// Lower dynamic call metadata for dynamic dispatch.
#[test]
fn test_lower_dynamic_call_metadata() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Drawable {
    color: int32;
    draw(): int32;
}

struct Circle implements Drawable {
    color: int32;
    radius: int32;

    draw(): int32 { return this.color; }
}

function useDrawable(d: Drawable): int32 {
    return d.draw();
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // assert the lowered mir
    test.assert_mir(
        module_id,
        "native",
        r#"
type Drawable.function = () => int32;
type Circle {
    color: int32;
    radius: int32;
}
type Drawable.object {
    draw: Drawable.function;
    color: int32;
}
type Drawable {
    value: ref<void, managed, readonly>;
    table: ref<void, raw, readonly, space(static)>;
}

external function Drawable.draw(Drawable.object): int32

function useDrawable(value0: Drawable): int32 {
entry0(value0: Drawable):
    value1: ref<void, managed, readonly> = field.get value0, 0
    value2: int32 = call.dynamic value0, Drawable.object, 2(value1): (Drawable.object) -> int32
    return value2
}

function Circle.draw(this0: Circle): int32 {
entry0(this0: Circle):
    value1: int32 = field.get this0, 0
    return value1
}
        "#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // locate the dynamic call metadata
        let info = test.dynamic_call_info_by_name(tree, strings, "useDrawable");
        let constraint_type = test.type_by_metadata_name(tree, strings, "test/test:Drawable.object");

        // assert the dispatch payload
        assert_eq!(info.slot, mir::DispatchSlot::new(2));
        assert_eq!(info.constraint, constraint_type);
    });
}

/// Lower dynamic upcasts into dynamic values in MIR.
#[test]
fn test_lower_interface_upcast() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Renderable {
    draw(): int32;
}

struct Sprite implements Renderable {
    value: int32;

    draw(): int32 { return this.value; }
}

function castRenderable(value: int32): Renderable {
    let c: Sprite = Sprite { value: value };
    return c;
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
type Renderable.function = () => int32;

type Renderable.object {
    draw: Renderable.function;
}

type Renderable {
    value: ref<void, managed, readonly>;
    table: ref<void, raw, readonly, space(static)>;
}

type Sprite {
    value: int32;
}

external function Renderable.draw(Renderable.object): int32

function castRenderable(value0: int32): Renderable {
entry0(value0: int32):
    value1: Sprite = struct Sprite (value0)
    value2: ref<Sprite, managed, readonly> = new.zeroed Sprite
    store value2, value1
    value3: ref<void, managed, readonly> = cast.bit value2 -> ref<void, managed, readonly>
    value4: ref<[usize; 2], raw, readonly, space(static)> = global.address Sprite#as#Renderable#dynamic_table
    value5: ref<void, raw, readonly, space(static)> = cast.bit value4 -> ref<void, raw, readonly, space(static)>
    value6: Renderable = struct Renderable (value3, value5)
    return value6
}

function Sprite.draw(this0: Sprite): int32 {
entry0(this0: Sprite):
    value1: int32 = field.get this0, 0
    return value1
}
"#,
    );
}

/// Execute a dynamic call through a dynamic value.
#[test]
fn test_lower_executes_interface_call() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Greeter {
    greet(): int32;
}

class GreeterImpl implements Greeter {
    value: int32;

    constructor(value: int32) {
        this.value = value;
        return;
    }

    greet(): int32 {
        return this.value + 1;
    }
}

function callInterface(g: Greeter): int32 {
    return g.greet();
}

function runInterface(): int32 {
    let g: Greeter = new GreeterImpl(41);
    return callInterface(g);
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(module_id, "native", "runInterface", &[], Value::int32(42));
}
