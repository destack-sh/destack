use destack_vm::Value;

use crate::TestProgram;

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
b0:
    v0: int32 = 4int32
    v1: Status = cast.bit v0 -> Status
    v2: int32 = cast.bit v1 -> int32
    return v2
}"#,
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
b0:
    v0: ref<String, managed, readonly> = global.const stringLiteralSour
    v1: Flavor = cast.bit v0 -> Flavor
    v2: ref<String, managed, readonly> = cast.bit v1 -> ref<String, managed, readonly>
    return v2
}"#,
    );

    let mut interpreter = test.mir_isolate(module_id, "native");
    let output = interpreter
        .run_function_by_name_output("flavorValue", &[])
        .expect("execution failed");
    let actual = interpreter
        .string_value(output.value)
        .expect("string value");
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
b0:
    v0: int32 = 1int32
    v1: Status = cast.bit v0 -> Status
    v2: boolean = call Status.isActive(v1): (Status) -> boolean
    return v2
}

function Status.isActive(v0: Status): boolean {
b0(v0: Status):
    v1: int32 = cast.bit v0 -> int32
    v2: int32 = 1int32
    v3: Status = cast.bit v2 -> Status
    v4: int32 = cast.bit v3 -> int32
    v5: boolean = int.eq v1, v4
    return v5
}"#,
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
b0:
    v0: int32 = 1int32
    v1: Status = cast.bit v0 -> Status
    v2: boolean = call Status.isActive(v1): (Status) -> boolean
    return v2
}

function Status.isActive(v0: Status): boolean {
b0(v0: Status):
    v1: int32 = cast.bit v0 -> int32
    v2: int32 = 1int32
    v3: Status = cast.bit v2 -> Status
    v4: int32 = cast.bit v3 -> int32
    v5: boolean = int.eq v1, v4
    return v5
}"#,
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
b0:
    v0: Status = global.const Status.Default
    v1: int32 = cast.bit v0 -> int32
    return v1
}"#,
    );

    test.assert_mir_function_output(module_id, "native", "defaultValue", &[], Value::int32(1));
}
