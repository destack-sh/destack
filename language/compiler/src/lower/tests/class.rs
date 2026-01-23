use destack_vm::Value;

use crate::TestProgram;

/// Lower class construction with `new`.
#[test]
fn test_lower_constructs_class_with_new() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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

    // lower the module
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
fn test_lower_allocates_class_with_new() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
type @test/test:Box = { value: i32 }
function @sumBox(v0: i32) -> i32 {
block0(v0: i32):
    v1: @test/test:Box = struct @test/test:Box (v0)
    v2: ref<managed @test/test:Box> = managed.alloc @test/test:Box
    store v2, v1
    v3: @test/test:Box = load v2
    v4: i32 = field.get v3, 0
    v5: i32 = iconst 1i32
    v6: i32 = trunc v5 -> i32
    v7: i32 = iadd v4, v6
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

/// Lower vtable headers into class allocations in MIR.
#[test]
fn test_lower_class_vtable_header() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
class PacketHeader {
    packetSize: int32 = 0;
}

class MessageHeader extends PacketHeader {
    ping(): int32 { return 1; }
}

function readPacketSize(value: int32): int32 {
    let header: PacketHeader = new PacketHeader(value);
    return header.packetSize;
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
global @test/test:PacketHeader#vtable: [ref?<raw void>; 2] = zeroinit ; const
global @test/test:MessageHeader#vtable: [ref?<raw void>; 3] = zeroinit ; const
function @ping(v0: ref<managed { @vtable: ref<raw void>, packetSize: i32 }>) -> i32 {
block0(v0: ref<managed { @vtable: ref<raw void>, packetSize: i32 }>):
    v1: i32 = iconst 1i32
    return v1
}
function @readPacketSize(v0: i32) -> i32 {
block0(v0: i32):
    v1: ref<raw [ref?<raw void>; 2]> = global.addr @test/test:PacketHeader#vtable
    v2: ref<raw void> = bitcast v1 -> ref<raw void>
    v3: { @vtable: ref<raw void>, packetSize: i32 } = struct { @vtable: ref<raw void>, packetSize: i32 } (v2, v0)
    v4: ref<managed { @vtable: ref<raw void>, packetSize: i32 }> = managed.alloc { @vtable: ref<raw void>, packetSize: i32 }
    store v4, v3
    v5: { @vtable: ref<raw void>, packetSize: i32 } = load v4
    v6: i32 = field.get v5, 1
    return v6
}
        "#,
    );
}

/// Lower explicit class constructors.
#[test]
fn test_lower_class_explicit_constructor() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
fn test_lower_class_method_returning_field() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
fn test_lower_class_method_with_parameters() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
fn test_lower_class_vtable_metadata() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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

        // resolve vtables by class metadata names
        let animal_type = test.type_by_metadata_name(tree, strings, "test/test:Animal");
        let animal_table_id = test.type_vtable_id(tree, animal_type);
        let animal_table = tree.type_table.dispatch_registry.table(animal_table_id);
        let dog_type = test.type_by_metadata_name(tree, strings, "test/test:Dog");
        let dog_table_id = test.type_vtable_id(tree, dog_type);
        let dog_table = tree.type_table.dispatch_registry.table(dog_table_id);

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

/// Lower vtable slot ordering across inheritance with multiple methods.
#[test]
fn test_lower_orders_class_vtable_slots() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Vehicle {
    start(): int32 { return 1; }
    stop(): int32 { return 2; }
}

class Car extends Vehicle {
    override start(): int32 { return 3; }
    honk(): int32 { return 4; }
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.with_mir_tree(module_id, "native", |tree, strings| {
        let base_type = test.type_by_metadata_name(tree, strings, "test/test:Vehicle");
        let derived_type = test.type_by_metadata_name(tree, strings, "test/test:Car");

        let base_table_id = test.type_vtable_id(tree, base_type);
        let derived_table_id = test.type_vtable_id(tree, derived_type);
        let base_table = tree.type_table.dispatch_registry.table(base_table_id);
        let derived_table = tree.type_table.dispatch_registry.table(derived_table_id);

        let base_methods = test.vtable_method_names(base_table, tree, strings);
        let derived_methods = test.vtable_method_names(derived_table, tree, strings);

        assert_eq!(base_methods, vec!["start", "stop"]);
        assert_eq!(derived_methods, vec!["start", "stop", "honk"]);
    });
}

/// Lower virtual call metadata for class dispatch.
#[test]
fn test_lower_class_virtual_call_metadata() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Logger {
    logLevel: int32 = 0;

    log(): int32 { 
        return 1; 
    }
}

class FileLogger extends Logger {
    fileMode: int32 = 0;

    override log(): int32 { 
        return 2; 
    }
}

function callLogger(base: Logger): int32 {
    return base.log();
}
"#,
    );

    // lower the module
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // inspect call metadata
    test.with_mir_tree(module_id, "native", |tree, strings| {
        let derived_type = test.type_by_metadata_name(tree, strings, "test/test:FileLogger");
        let base_type = test.type_parent(tree, derived_type);

        let call_logger_info = test.virtual_call_info_by_name(tree, strings, "callLogger");
        assert_eq!(call_logger_info.slot_id, 2);
        assert_eq!(call_logger_info.declaring_type, base_type);
    });
}

/// Lower virtual dispatch calls in MIR.
#[test]
fn test_lower_class_virtual_call() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Logger {
    log(): int32 { return 1; }
}

class FileLogger extends Logger {
    override log(): int32 { return 2; }
}

function callLogger(base: Logger): int32 {
    return base.log();
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
type @Struct0 = { @vtable: ref<raw void> }
global @test/test:Logger#vtable: [ref?<raw void>; 3] = zeroinit ; const
global @test/test:FileLogger#vtable: [ref?<raw void>; 3] = zeroinit ; const
function @log(v0: ref<managed @Struct0>) -> i32 {
block0(v0: ref<managed @Struct0>):
    v1: i32 = iconst 1i32
    return v1
}
function @log#1(v0: ref<managed @Struct0>) -> i32 {
block0(v0: ref<managed @Struct0>):
    v1: i32 = iconst 2i32
    return v1
}
function @callLogger(v0: ref<managed @Struct0>) -> i32 {
block0(v0: ref<managed @Struct0>):
    v1: i32 = call.virtual v0, @Struct0, 2, @log(v0) -> fn(ref<managed @Struct0>) -> i32
    return v1
}
        "#,
    );
}

/// Execute a virtual call through a base-typed reference.
#[test]
fn test_lower_executes_class_virtual_call() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Notifier {
    notify(): int32 { 
        return 1; 
    }
}

class SmsNotifier extends Notifier {
    override notify(): int32 { 
        return 2; 
    }
}

function callNotification(): int32 {
    let d: SmsNotifier = new SmsNotifier();
    let b: Notifier = d;
    return b.notify();
}
"#,
    );

    // lower the module
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "callNotification",
        &[],
        Value::int32(2),
    );
}

/// Lower class lineage metadata for inheritance and interfaces.
#[test]
fn test_lower_class_lineage_metadata() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Taggable {
    tag: int32;
}

class WidgetBase {
    baseId: int32 = 0;
}

class TaggedWidget extends WidgetBase implements Taggable {
    widgetId: int32 = 0;
    tag: int32 = 0;
}
"#,
    );

    // lower the module
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // inspect the lowered mir metadata
    test.with_mir_tree(module_id, "native", |tree, strings| {
        // locate the base and derived payload types
        let base_type = test.type_by_metadata_name(tree, strings, "test/test:WidgetBase");
        let derived_type = test.type_by_metadata_name(tree, strings, "test/test:TaggedWidget");

        // assert lineage metadata for the derived type
        let lineage = test.type_lineage(tree, derived_type);
        assert_eq!(lineage.parent, Some(base_type));
        assert_eq!(lineage.interfaces.len(), 1);
        assert!(!lineage.is_interface);
        assert!(!lineage.is_abstract);

        // assert interface metadata is flagged correctly
        let interface_type = lineage.interfaces[0];
        let interface_lineage = test.type_lineage(tree, interface_type);
        assert!(interface_lineage.is_interface);
        assert!(interface_lineage.is_abstract);
    });
}

/// Lower class layout metadata for inherited fields.
#[test]
fn test_lower_class_layout_inheritance() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Account {
    balance: int32 = 0;
}

class SavingsAccount extends Account {
    bonus: int64 = 0;

    ping(): int32 { return 1; }
}
"#,
    );

    // lower the module
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // inspect layout metadata
    test.with_mir_tree(module_id, "native", |tree, strings| {
        // resolve the derived type
        let derived_type = test.type_by_metadata_name(tree, strings, "test/test:SavingsAccount");

        // resolve the base type from lineage metadata
        let base_type = test.type_parent(tree, derived_type);

        // collect base and derived field offsets
        let base_offset =
            test.expect_struct_field_offset_by_name(tree, strings, base_type, "balance");
        let derived_base_offset =
            test.expect_struct_field_offset_by_name(tree, strings, derived_type, "balance");
        let derived_offset =
            test.expect_struct_field_offset_by_name(tree, strings, derived_type, "bonus");

        // ensure derived layouts preserve base offsets
        assert_eq!(base_offset, derived_base_offset);
        assert!(derived_offset > base_offset);
    });
}

/// Lower vtable headers for base classes in polymorphic hierarchies.
#[test]
fn test_lower_propagates_class_vtable_header() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
class AssetHeader {
    assetId: int32 = 0;
}

class TextureHeader extends AssetHeader {
    textureId: int32 = 0;

    ping(): int32 { 
        return 1; 
    }
}

function useHeader(value: TextureHeader): int32 {
    return value.ping();
}
"#,
    );

    // lower the module
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // inspect vtable header offsets
    test.with_mir_tree(module_id, "native", |tree, strings| {
        let derived_type = test.type_by_metadata_name(tree, strings, "test/test:TextureHeader");
        let base_type = test.type_parent(tree, derived_type);

        let base_vtable_offset =
            test.expect_struct_field_offset_by_name(tree, strings, base_type, "@vtable");
        let derived_vtable_offset =
            test.expect_struct_field_offset_by_name(tree, strings, derived_type, "@vtable");

        assert_eq!(base_vtable_offset, 0);
        assert_eq!(derived_vtable_offset, 0);
    });
}

/// Lower headerless layouts for non-polymorphic classes.
#[test]
fn test_lower_omits_header_for_non_polymorphic_class() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
class PlainRecord {
    recordValue: int32 = 0;
}

function usePlain(value: PlainRecord): int32 {
    return value.recordValue;
}
"#,
    );

    // lower the module
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    // inspect layout for absence of vtable header
    test.with_mir_tree(module_id, "native", |tree, strings| {
        let class_type = test.type_by_metadata_name(tree, strings, "test/test:PlainRecord");
        let vtable_offset = test.struct_field_offset_by_name(tree, strings, class_type, "@vtable");
        assert!(vtable_offset.is_none());
    });
}
