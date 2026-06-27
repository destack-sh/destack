use destack_program::Value;
use destack_mir as mir;

use crate::TestProgram;

/// Lower class construction with `new`.
#[test]
fn test_lower_constructs_class_with_new() {
    let test = TestProgram::memory_sequential_with_prelude();
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

/// Lower class construction to heap allocation in MIR.
#[test]
fn test_lower_allocates_class_with_new() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
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
type Box {
    value: int32;
}

function sumBox(v0: int32): int32 {
entry(v0: int32):
    v1: Box = struct Box (v0)
    v2: ref<Box, managed, readonly> = new.zeroed Box
    store v2, v1
    v3: Box = load v2
    v4: int32 = field.get v3, 0
    v5: int32 = 1
    v6: int32 = int.add v4, v5
    return v6
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
    let test = TestProgram::memory_sequential_with_prelude();
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
type PacketHeader {
    vtable: ref<void, raw, readonly, space(static)>;
    packetSize: int32;
}

readonly global PacketHeader#vtable: [ref<void, raw, readonly, nullable, space(static)>; 0], space(static) = zeroInit

readonly global MessageHeader#vtable: [ref<void, raw, readonly, nullable, space(static)>; 1], space(static) = zeroInit

function readPacketSize(v0: int32): int32 {
entry(v0: int32):
    v1: ref<[ref<void, raw, readonly, nullable, space(static)>; 0], raw, readonly, space(static)> = global.address PacketHeader#vtable
    v2: ref<void, raw, readonly, space(static)> = cast.bit v1 -> ref<void, raw, readonly, space(static)>
    v3: PacketHeader = struct PacketHeader (v2, v0)
    v4: ref<PacketHeader, managed, readonly> = new.zeroed PacketHeader
    store v4, v3
    v5: PacketHeader = load v4
    v6: int32 = field.get v5, 1
    return v6
}

function MessageHeader.ping(this0: ref<PacketHeader, managed, readonly>): int32 {
entry(this0: ref<PacketHeader, managed, readonly>):
    v1: int32 = 1
    return v1
}
"#,
    );
}

/// Lower explicit class constructors.
#[test]
fn test_lower_class_explicit_constructor() {
    let test = TestProgram::memory_sequential_with_prelude();
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
    let test = TestProgram::memory_sequential_with_prelude();
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
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Adder {
    base: int32 = 0;

    add(n: int32): int32 {
        return this.value + n;
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

/// Lower class dispatch tables with override reuse.
#[test]
fn test_lower_class_vtable_entries() {
    let test = TestProgram::memory_sequential_with_prelude();
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

    // assert the lowered mir
    test.assert_mir(
        module_id,
        "native",
        r#"
type Animal {
    vtable: ref<void, raw, readonly, space(static)>;
    name: int32;
}

type Dog {
    vtable: ref<void, raw, readonly, space(static)>;
    name: int32;
    breed: int32;
}

readonly global Animal#vtable: [ref<void, raw, readonly, nullable, space(static)>; 1], space(static) = zeroInit

readonly global Dog#vtable: [ref<void, raw, readonly, nullable, space(static)>; 1], space(static) = zeroInit

function useDog(v0: ref<Dog, managed, readonly>): int32 {
entry(v0: ref<Dog, managed, readonly>):
    v1: int32 = call.virtual v0, Dog, 0(v0): (ref<Dog, managed, readonly>) => int32
    return v1
}

function Animal.speak(this0: ref<Animal, managed, readonly>): int32 {
entry(this0: ref<Animal, managed, readonly>):
    v1: int32 = 1
    return v1
}

function Dog.speak(this0: ref<Dog, managed, readonly>): int32 {
entry(this0: ref<Dog, managed, readonly>):
    v1: int32 = 2
    return v1
}
"#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        // collect the class vtables
        let vtables = test.class_dispatch_tables(tree);
        assert_eq!(vtables.len(), 2);

        // resolve vtables by class names
        let animal_type = test.type_by_metadata_name(tree, strings, "test/test:Animal");
        let animal_table = test.type_vtable(tree, animal_type);
        let dog_type = test.type_by_metadata_name(tree, strings, "test/test:Dog");
        let dog_table = test.type_vtable(tree, dog_type);

        // count the class method slots
        let animal_methods = test.count_vtable_methods(animal_table);
        let dog_methods = test.count_vtable_methods(dog_table);

        // assert the override reuse
        assert_eq!(animal_methods, 1);
        assert_eq!(dog_methods, 1);
    });
}

/// Lower virtual dispatch slot ordering across inheritance with multiple methods.
#[test]
fn test_lower_orders_class_vtable_slots() {
    let test = TestProgram::memory_sequential_with_prelude();
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

    // assert the lowered mir
    test.assert_mir(
        module_id,
        "native",
        r#"
type Vehicle {
    vtable: ref<void, raw, readonly, space(static)>;
}

readonly global Vehicle#vtable: [ref<void, raw, readonly, nullable, space(static)>; 2], space(static) = zeroInit

readonly global Car#vtable: [ref<void, raw, readonly, nullable, space(static)>; 3], space(static) = zeroInit

function Vehicle.start(this0: ref<Vehicle, managed, readonly>): int32 {
entry(this0: ref<Vehicle, managed, readonly>):
    v1: int32 = 1
    return v1
}

function Vehicle.stop(this0: ref<Vehicle, managed, readonly>): int32 {
entry(this0: ref<Vehicle, managed, readonly>):
    v1: int32 = 2
    return v1
}

function Car.start(this0: ref<Vehicle, managed, readonly>): int32 {
entry(this0: ref<Vehicle, managed, readonly>):
    v1: int32 = 3
    return v1
}

function Car.honk(this0: ref<Vehicle, managed, readonly>): int32 {
entry(this0: ref<Vehicle, managed, readonly>):
    v1: int32 = 4
    return v1
}
"#,
    );

    test.with_mir_tree(module_id, "native", |tree, strings| {
        let base_type = test.type_by_metadata_name(tree, strings, "test/test:Vehicle");
        let derived_type = test.type_by_metadata_name(tree, strings, "test/test:Car");

        let base_table = test.type_vtable(tree, base_type);
        let derived_table = test.type_vtable(tree, derived_type);

        let base_methods = test.vtable_method_names(base_table, tree, strings);
        let derived_methods = test.vtable_method_names(derived_table, tree, strings);

        assert_eq!(base_methods, vec!["Vehicle.start", "Vehicle.stop"]);
        assert_eq!(
            derived_methods,
            vec!["Car.start", "Vehicle.stop", "Car.honk"]
        );
    });
}

/// Lower virtual call entries for virtual dispatch.
#[test]
fn test_lower_class_call_entries() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
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

    // assert the lowered mir
    test.assert_mir(
        module_id,
        "native",
        r#"
type Logger {
    vtable: ref<void, raw, readonly, space(static)>;
    logLevel: int32;
}

type FileLogger {
    vtable: ref<void, raw, readonly, space(static)>;
    logLevel: int32;
    fileMode: int32;
}

readonly global Logger#vtable: [ref<void, raw, readonly, nullable, space(static)>; 1], space(static) = zeroInit

readonly global FileLogger#vtable: [ref<void, raw, readonly, nullable, space(static)>; 1], space(static) = zeroInit

function callLogger(v0: ref<Logger, managed, readonly>): int32 {
entry(v0: ref<Logger, managed, readonly>):
    v1: int32 = call.virtual v0, Logger, 0(v0): (ref<Logger, managed, readonly>) => int32
    return v1
}

function Logger.log(this0: ref<Logger, managed, readonly>): int32 {
entry(this0: ref<Logger, managed, readonly>):
    v1: int32 = 1
    return v1
}

function FileLogger.log(this0: ref<FileLogger, managed, readonly>): int32 {
entry(this0: ref<FileLogger, managed, readonly>):
    v1: int32 = 2
    return v1
}
"#,
    );

    // inspect call tables
    test.with_mir_tree(module_id, "native", |tree, strings| {
        let derived_type = test.type_by_metadata_name(tree, strings, "test/test:FileLogger");
        let base_type = test.type_parent(tree, derived_type);

        let call_logger_info = test.class_call_info_by_name(tree, strings, "callLogger");
        assert_eq!(call_logger_info.slot, mir::DispatchSlot::new(0));
        assert_eq!(call_logger_info.class, base_type);
    });
}

/// Lower virtual dispatch calls in MIR.
#[test]
fn test_lower_class_call() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
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
type Logger {
    vtable: ref<void, raw, readonly, space(static)>;
}

readonly global Logger#vtable: [ref<void, raw, readonly, nullable, space(static)>; 1], space(static) = zeroInit

readonly global FileLogger#vtable: [ref<void, raw, readonly, nullable, space(static)>; 1], space(static) = zeroInit

function callLogger(v0: ref<Logger, managed, readonly>): int32 {
entry(v0: ref<Logger, managed, readonly>):
    v1: int32 = call.virtual v0, Logger, 0(v0): (ref<Logger, managed, readonly>) => int32
    return v1
}

function Logger.log(this0: ref<Logger, managed, readonly>): int32 {
entry(this0: ref<Logger, managed, readonly>):
    v1: int32 = 1
    return v1
}

function FileLogger.log(this0: ref<Logger, managed, readonly>): int32 {
entry(this0: ref<Logger, managed, readonly>):
    v1: int32 = 2
    return v1
}
"#,
    );
}

/// Execute a virtual call through a base-typed reference.
#[test]
fn test_lower_executes_class_call() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
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

/// Lower class lineage for inheritance and interfaces.
#[test]
fn test_lower_class_lineage_entries() {
    let test = TestProgram::memory_sequential_with_prelude();
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

    // inspect the lowered mir tables
    test.with_mir_tree(module_id, "native", |tree, strings| {
        // locate the base and derived payload types
        let base_type = test.type_by_metadata_name(tree, strings, "test/test:WidgetBase");
        let derived_type = test.type_by_metadata_name(tree, strings, "test/test:TaggedWidget");

        // assert lineage tables for the derived type
        let lineage = test.type_lineage(tree, derived_type);
        assert_eq!(lineage.parent, Some(base_type));
        assert_eq!(lineage.interfaces.len(), 1);
        assert!(!lineage.is_interface);
        assert!(!lineage.is_abstract);

        // assert interface lineage is flagged correctly
        let interface_type = lineage.interfaces[0];
        let interface_lineage = test.type_lineage(tree, interface_type);
        assert!(interface_lineage.is_interface);
        assert!(interface_lineage.is_abstract);
    });
}

/// Lower class layout entries for inherited fields.
#[test]
fn test_lower_class_layout_entries() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
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

    // inspect layout entries
    test.with_mir_tree(module_id, "native", |tree, strings| {
        // resolve the derived type
        let derived_type = test.type_by_metadata_name(tree, strings, "test/test:SavingsAccount");

        // resolve the base type from lineage tables
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
    let test = TestProgram::memory_sequential_with_prelude();
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
            test.expect_struct_field_offset_by_name(tree, strings, base_type, "vtable");
        let derived_vtable_offset =
            test.expect_struct_field_offset_by_name(tree, strings, derived_type, "vtable");

        assert_eq!(base_vtable_offset, 0);
        assert_eq!(derived_vtable_offset, 0);
    });
}

/// Lower headerless layouts for non-polymorphic classes.
#[test]
fn test_lower_omits_header_for_non_polymorphic_class() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
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
        let vtable_offset = test.struct_field_offset_by_name(tree, strings, class_type, "vtable");
        assert!(vtable_offset.is_none());
    });
}
