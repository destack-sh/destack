use destack_mir as mir;
use destack_vm::Value;

use crate::TestProgram;

/// Lower interface itab metadata for structs.
#[test]
fn test_lower_struct_itab_metadata() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // collect interface dispatch tables
        let itab = test.expect_single_interface_table(tree);

        // assert the itab slot layout
        assert!(matches!(itab.slots[0], destack_mir::DispatchSlot::TypeTag));

        // assert the field offset slot
        let offset = test.expect_interface_field_offset(itab, strings, "color");
        assert_eq!(offset, 0);

        // assert the interface method slot
        let target_name = test.expect_interface_method_target_name(itab, tree, strings, "draw");
        assert_eq!(target_name, "draw");
    });
}

/// Lower interface reference layouts into fat pointer structs.
#[test]
fn test_lower_interface_reference_layout() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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

    // lower the module
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // find the interface reference struct type
        let interface_type = test.type_by_metadata_name(tree, strings, "test/test:Drawable");

        // resolve the object and itab field types
        let object_type =
            test.expect_struct_field_type_by_name(tree, strings, interface_type, "@object");
        let itab_type =
            test.expect_struct_field_type_by_name(tree, strings, interface_type, "@itab");

        // assert the object pointer field type
        let object_type = tree.get(object_type);
        let mir::Type::Reference { kind, pointee, .. } = object_type else {
            panic!("expected managed reference for interface object field");
        };
        assert!(matches!(kind, mir::ReferenceKind::Managed));
        assert!(matches!(tree.get(*pointee), mir::Type::Void));

        // assert the itab field type
        let itab_type = tree.get(itab_type);
        assert!(matches!(itab_type, mir::Type::Usize));
    });
}

/// Lower interface itab slots in declaration order for mixed members.
#[test]
fn test_lower_orders_interface_itab_slots() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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

    area(): int32 { return this.width; }
    paint(): int32 { return this.color; }
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

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

        assert!(matches!(shape_table.slots[0], mir::DispatchSlot::TypeTag));
        match &shape_table.slots[1] {
            mir::DispatchSlot::FieldOffset { field_name, offset } => {
                assert_eq!(strings.get(*field_name), "width");
                assert_eq!(*offset, 0);
            }
            _ => panic!("expected field offset slot for width"),
        }
        match &shape_table.slots[2] {
            mir::DispatchSlot::InterfaceMethod {
                interface_method, ..
            } => {
                let method_name = strings.get(tree.get(*interface_method).name);
                assert_eq!(method_name, "area");
            }
            _ => panic!("expected interface method slot for area"),
        }

        assert!(matches!(paint_table.slots[0], mir::DispatchSlot::TypeTag));
        match &paint_table.slots[1] {
            mir::DispatchSlot::FieldOffset { field_name, offset } => {
                assert_eq!(strings.get(*field_name), "color");
                assert_eq!(*offset, 4);
            }
            _ => panic!("expected field offset slot for color"),
        }
        match &paint_table.slots[2] {
            mir::DispatchSlot::InterfaceMethod {
                interface_method, ..
            } => {
                let method_name = strings.get(tree.get(*interface_method).name);
                assert_eq!(method_name, "paint");
            }
            _ => panic!("expected interface method slot for paint"),
        }
    });
}

/// Lower interface call metadata for interface dispatch.
#[test]
fn test_lower_interface_call_metadata() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // locate the interface call metadata
        let info = test.interface_call_info_by_name(tree, strings, "useDrawable");
        let interface_type = test.type_by_metadata_name(tree, strings, "test/test:Drawable#object");

        // assert the dispatch payload
        assert_eq!(info.slot_id, 2);
        assert_eq!(info.declaring_type, interface_type);
    });
}

/// Lower interface upcasts into fat pointer values in MIR.
#[test]
fn test_lower_interface_upcast() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
extern function @draw({ draw: fn() -> i32 }) -> i32
function @draw(v0: { value: i32 }) -> i32 {
block0(v0: { value: i32 }):
    v1 = field.get v0, 0
    return v1
}
function @castRenderable(v0: i32) -> { @object: ref<managed void>, @itab: usize } {
block0(v0: i32):
    v1 = struct { value: i32 } (v0)
    v2 = managed.alloc { value: i32 } -> ref<managed { value: i32 }>
    store v2, v1
    v3 = bitcast v2 -> ref<managed void>
    v4 = iconst 0u64
    v5 = bitcast v4 -> usize
    v6 = struct { @object: ref<managed void>, @itab: usize } (v3, v5)
    return v6
}
        "#,
    );
}

/// Execute an interface call through an interface-typed reference.
#[test]
fn test_lower_executes_interface_call() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
