use destack_vm::Value;

use crate::{TestProgram, materialized_plain_value};

/// Lower integer enum member values into nominal enum constants.
#[test]
fn test_lower_lowers_enum_integer_members() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
enum Status {
    Idle
    Running = 3
    Done
}

function statusValue(): int32 {
    return Status.Done as int32;
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
type Status = newtype<int32>;

function statusValue(): int32 {
entry0:
    value0: int32 = 4int32
    value1: Status = cast.bit value0 -> Status
    value2: int32 = cast.bit value1 -> int32
    return value2
}
"#,
    );

    test.assert_mir_function_output(module_id, "native", "statusValue", &[], Value::int32(4));
}

/// Lower enum equality through nominal enum values.
#[test]
fn test_lower_compares_enum_integer_values() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
enum Status {
    Idle
    Active
}

function isActive(value: Status): boolean {
    return (value as int32) == (Status.Active as int32);
}

function checkActive(): boolean {
    return isActive(Status.Active);
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(module_id, "native", "checkActive", &[], Value::bool(true));
}

/// Lower string enum member values into nominal enum constants.
#[test]
fn test_lower_lowers_enum_string_members() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
enum Flavor {
    Sweet = "sweet"
    Sour = "sour"
}

function flavorValue(): string {
    return Flavor.Sour as string;
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
type Flavor = newtype<ref<String, managed, readonly>>;

function flavorValue(): ref<String, managed, readonly> {
entry0:
    value0: ref<String, managed, readonly> = global.const stringLiteralSour
    value1: Flavor = cast.bit value0 -> Flavor
    value2: ref<String, managed, readonly> = cast.bit value1 -> ref<String, managed, readonly>
    return value2
}"#,
    );

    let mut interpreter = test.mir_isolate(module_id, "native");
    let output = interpreter
        .run_function_by_name_output("flavorValue", &[])
        .expect("execution failed");
    let value = materialized_plain_value(&output.value);
    let actual = interpreter.string_value(value).expect("string value");
    assert_eq!(actual, "sour");
}

/// Lower static enum method calls with nominal enum parameters.
#[test]
fn test_lower_calls_enum_static_method() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
enum Status {
    Idle;
    Active;

    static isActive(value: Status): boolean {
        return (value as int32) == (Status.Active as int32);
    }
}

function checkStatic(): boolean {
    return Status.isActive(Status.Active);
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
type Status = newtype<int32>;

function checkStatic(): boolean {
entry0:
    value0: int32 = 1int32
    value1: Status = cast.bit value0 -> Status
    value2: boolean = call Status.isActive(value1): (Status) -> boolean
    return value2
}

function Status.isActive(value0: Status): boolean {
entry0(value0: Status):
    value1: int32 = cast.bit value0 -> int32
    value2: int32 = 1int32
    value3: Status = cast.bit value2 -> Status
    value4: int32 = cast.bit value3 -> int32
    value5: boolean = int.eq value1, value4
    return value5
}
"#,
    );

    test.assert_mir_function_output(module_id, "native", "checkStatic", &[], Value::bool(true));
}

/// Lower enum instance method calls with nominal enum receivers.
#[test]
fn test_lower_calls_enum_instance_method() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
enum Status {
    Idle;
    Active;

    isActive(): boolean {
        return (this as int32) == (Status.Active as int32);
    }
}

function checkInstance(): boolean {
    return Status.Active.isActive();
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
type Status = newtype<int32>;

function checkInstance(): boolean {
entry0:
    value0: int32 = 1int32
    value1: Status = cast.bit value0 -> Status
    value2: boolean = call Status.isActive(value1): (Status) -> boolean
    return value2
}

function Status.isActive(value0: Status): boolean {
entry0(value0: Status):
    value1: int32 = cast.bit value0 -> int32
    value2: int32 = 1int32
    value3: Status = cast.bit value2 -> Status
    value4: int32 = cast.bit value3 -> int32
    value5: boolean = int.eq value1, value4
    return value5
}
"#,
    );

    test.assert_mir_function_output(module_id, "native", "checkInstance", &[], Value::bool(true));
}

/// Lower static enum fields to nominal enum globals.
#[test]
fn test_lower_lowers_enum_static_field() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
enum Status {
    Idle;
    Active;

    static Default: Status = Status.Active;
}

function defaultValue(): int32 {
    return Status.Default as int32;
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
type Status = newtype<int32>;

global Status.Default: Status, readonly = 1int32

function defaultValue(): int32 {
entry0:
    value0: Status = global.const Status.Default
    value1: int32 = cast.bit value0 -> int32
    return value1
}
"#,
    );

    test.assert_mir_function_output(module_id, "native", "defaultValue", &[], Value::int32(1));
}
