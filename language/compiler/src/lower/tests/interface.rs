use destack_mir as mir;
use destack_vm::Value;

use crate::TestProgram;

/// Lower interface itab metadata for structs.
#[test]
fn test_lower_struct_itab_metadata() {
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
type Circle {
    color: int32;
    radius: int32;
}
extern function Drawable.draw({ draw: closure() -> int32, color: int32 }): int32
function Circle.draw(v0: Circle): int32 {
b0(v0: Circle):
    v1: int32 = field.get v0, 0
    return v1
}"#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // collect interface dispatch tables
        let itab = test.expect_single_interface_table(tree);

        // assert the itab slot layout
        assert!(matches!(
            itab.entries[0],
            destack_mir::ItabEntry::TypeDescriptor
        ));

        // assert the field offset slot
        let offset = test.expect_interface_field_offset(itab, strings, "color");
        assert_eq!(offset, 0);

        // assert the interface method slot
        let target_name =
            test.expect_interface_method_target_name(itab, tree, strings, "Drawable.draw");
        assert_eq!(target_name, "Circle.draw");
    });
}

/// Lower interface reference layouts into fat pointer structs.
#[test]
fn test_lower_interface_reference_layout() {
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
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
type Circle {
    color: int32;
    radius: int32;
}
extern function Drawable.draw({ draw: closure() -> int32, color: int32 }): int32
function Circle.draw(v0: Circle): int32 {
b0(v0: Circle):
    v1: int32 = field.get v0, 0
    return v1
}"#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        let interface_type = test.type_by_metadata_name(tree, strings, "test/test:Drawable");

        let object_type =
            test.expect_struct_field_type_by_name(tree, strings, interface_type, "@object");
        let itab_type =
            test.expect_struct_field_type_by_name(tree, strings, interface_type, "@itab");

        let object_type = tree.get(object_type);
        let mir::Type::Reference { kind, pointee, .. } = object_type else {
            panic!("expected managed reference for interface object field");
        };
        assert!(matches!(kind, mir::ReferenceKind::Managed));
        assert!(matches!(tree.get(*pointee), mir::Type::Void));

        let itab_type = tree.get(itab_type);
        assert!(matches!(itab_type, mir::Type::Usize));
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

/// Lower interface itab slots in declaration order for mixed members.
#[test]
fn test_lower_orders_interface_itab_slots() {
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
type Widget {
    width: int32;
    color: int32;
}
extern function Shape.area({ area: closure() -> int32, width: int32 }): int32
extern function Paint.paint({ paint: closure() -> int32, color: int32 }): int32
function Widget.area(v0: Widget): int32 {
b0(v0: Widget):
    v1: int32 = field.get v0, 0
    return v1
}
function Widget.paint(v0: Widget): int32 {
b0(v0: Widget):
    v1: int32 = field.get v0, 1
    return v1
}"#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        let shape_table = test.interface_dispatch_table(
            tree,
            strings,
            "test/test:Widget",
            "test/test:Shape#object",
        );
        let paint_table = test.interface_dispatch_table(
            tree,
            strings,
            "test/test:Widget",
            "test/test:Paint#object",
        );

        assert!(matches!(
            shape_table.entries[0],
            mir::ItabEntry::TypeDescriptor
        ));
        match &shape_table.entries[1] {
            mir::ItabEntry::FieldOffset {
                field: _,
                field_name,
                offset,
            } => {
                assert_eq!(strings.get(*field_name), "width");
                assert_eq!(*offset, 0);
            }
            _ => panic!("expected field offset slot for width"),
        }
        match &shape_table.entries[2] {
            mir::ItabEntry::Method {
                declared_method, ..
            } => {
                let method_name = strings.get(tree.get(*declared_method).name);
                assert_eq!(method_name, "Shape.area");
            }
            _ => panic!("expected interface method slot for area"),
        }

        assert!(matches!(
            paint_table.entries[0],
            mir::ItabEntry::TypeDescriptor
        ));
        match &paint_table.entries[1] {
            mir::ItabEntry::FieldOffset {
                field: _,
                field_name,
                offset,
            } => {
                assert_eq!(strings.get(*field_name), "color");
                assert_eq!(*offset, 4);
            }
            _ => panic!("expected field offset slot for color"),
        }
        match &paint_table.entries[2] {
            mir::ItabEntry::Method {
                declared_method, ..
            } => {
                let method_name = strings.get(tree.get(*declared_method).name);
                assert_eq!(method_name, "Paint.paint");
            }
            _ => panic!("expected interface method slot for paint"),
        }
    });
}

/// Lower interface call metadata for interface dispatch.
#[test]
fn test_lower_interface_call_metadata() {
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
type Drawable#method:draw#function closure() -> int32
type Drawable { object: ref<void, managed, readonly>, itab: usize }
type Circle { color: int32, radius: int32 }
type Drawable#object { draw: Drawable#method:draw#function, color: int32 }

extern function Drawable.draw(Drawable#object): int32

function useDrawable(v0: Drawable): int32 {
b0(v0: Drawable):
    v1: ref<void, managed, readonly> = field.get v0, 0
    v2: int32 = call.interface v0, Drawable#object, 2(v1): (Drawable#object) -> int32
    return v2
}

function Circle.draw(v0: Circle): int32 {
b0(v0: Circle):
    v1: int32 = field.get v0, 0
    return v1
}
        "#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // locate the interface call metadata
        let info = test.interface_call_info_by_name(tree, strings, "useDrawable");
        let interface_type = test.type_by_metadata_name(tree, strings, "test/test:Drawable#object");

        // assert the dispatch payload
        assert_eq!(info.slot_id, mir::InterfaceSlotId::new(2));
        assert_eq!(info.declaring_type, interface_type);
    });
}

/// Lower interface upcasts into fat pointer values in MIR.
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
type Renderable {
    object: ref<void, managed, readonly>;
    itab: usize;
}
type Sprite {
    value: int32;
}
extern function Renderable.draw({ draw: closure() -> int32 }): int32
function castRenderable(v0: int32): Renderable {
b0(v0: int32):
    v1: Sprite = struct Sprite (v0)
    v2: ref<Sprite, managed, readonly> = managed.alloc Sprite
    store v2, v1
    v3: ref<void, managed, readonly> = cast.bit v2 -> ref<void, managed, readonly>
    v4: uint64 = 0uint64
    v5: usize = cast.bit v4 -> usize
    v6: Renderable = struct Renderable (v3, v5)
    return v6
}
function Sprite.draw(v0: Sprite): int32 {
b0(v0: Sprite):
    v1: int32 = field.get v0, 0
    return v1
}"#,
    );
}

/// Execute an interface call through an interface-typed reference.
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
