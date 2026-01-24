use destack_vm::Value;

use crate::TestProgram;

/// Lower integer enum member values into constants.
#[test]
fn test_lower_lowers_enum_integer_members() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
function @statusValue() -> i32 {
block0:
    v0: i32 = iconst 4i32
    return v0
}
        "#,
    );

    test.assert_mir_function_output(module_id, "native", "statusValue", &[], Value::int32(4));
}

/// Lower enum equality using the backing integer type.
#[test]
fn test_lower_compares_enum_integer_values() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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

/// Lower string enum member values into constants.
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

    let mut interpreter = test.mir_isolate(module_id, "native");
    let output = interpreter
        .run_function_by_name("flavorValue", &[])
        .expect("execution failed");
    let actual = interpreter
        .string_value(output.value)
        .expect("string value");
    assert_eq!(actual, "sour");
}

/// Lower static enum method calls.
#[test]
fn test_lower_calls_enum_static_method() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
function @Status.isActive(v0: i32) -> bool {
block0(v0: i32):
    v1: i32 = iconst 1i32
    v2: bool = icmp_eq v0, v1
    return v2
}

function @checkStatic() -> bool {
block0:
    v0: i32 = iconst 1i32
    v1: bool = call @Status.isActive(v0) -> fn(i32) -> bool
    return v1
}
        "#,
    );

    test.assert_mir_function_output(module_id, "native", "checkStatic", &[], Value::bool(true));
}

/// Lower enum instance method calls.
#[test]
fn test_lower_calls_enum_instance_method() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
function @Status.isActive(v0: i32) -> bool {
block0(v0: i32):
    v1: i32 = iconst 1i32
    v2: bool = icmp_eq v0, v1
    return v2
}

function @checkInstance() -> bool {
block0:
    v0: i32 = iconst 1i32
    v1: bool = call @Status.isActive(v0) -> fn(i32) -> bool
    return v1
}
        "#,
    );

    test.assert_mir_function_output(module_id, "native", "checkInstance", &[], Value::bool(true));
}

/// Lower static enum fields to globals.
#[test]
fn test_lower_lowers_enum_static_field() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
global @Status.Default: i32 = 1i32 ; const

function @defaultValue() -> i32 {
block0:
    v0: i32 = global.const @Status.Default
    return v0
}
        "#,
    );

    test.assert_mir_function_output(module_id, "native", "defaultValue", &[], Value::int32(1));
}
