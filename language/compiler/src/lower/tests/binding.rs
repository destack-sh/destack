use destack_engine::Value;
use destack_workspace::{CheckFailurePolicy, CheckPolicy};

use crate::TestProgram;

/// Verify module-level const declarations are lowered correctly.
#[test]
fn test_lower_module_constants() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
const PI = 3.141592653589793;
const TWO = 2;

function getCircumference(radius: number): number {
    return TWO * PI * radius;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "getCircumference",
        &[Value::float64(1.0)],
        Value::float64(std::f64::consts::TAU),
    );
}

/// Verify let bindings and reassignments produce correct SSA form.
#[test]
fn test_lower_let_and_assign() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function assignReturn(x: number): number {
    let y = x + 1.0;
    y = y + 2.0;
    y = y + x;
    return y;
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
function assignReturn(value0: float64): float64 {
entry0(value0: float64):
    value1: float64 = 1float64
    value2: float64 = float.add value0, value1
    value3: float64 = 2float64
    value4: float64 = float.add value2, value3
    value5: float64 = float.add value4, value0
    return value5
}
"#,
    );

    test.assert_mir_function_output(
        module_id,
        "native",
        "assignReturn",
        &[Value::float64(10.0)],
        Value::float64(23.0),
    );
}

/// Verify borrowing a local produces a local addr instruction.
#[test]
fn test_lower_borrows_local_binding() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function borrowLocal(x: int32): &int32 {
    let y = x;
    return &y;
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
function borrowLocal(value0: int32): ref<int32, borrowed> {
    local local0: int32, owned

entry0(value0: int32):
    local.set local0, value0
    value1: ref<int32, borrowed, space(frame)> = local.address local0
    return value1
}
"#,
    );
}

/// Verify borrowing an rvalue spills to a temporary local.
#[test]
fn test_lower_borrows_rvalue() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function borrowTemp(x: int32): &int32 {
    return &(x + 1);
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
function borrowTemp(value0: int32): ref<int32, borrowed> {
    local local0: int32, owned

entry0(value0: int32):
    value1: int32 = 1int32
    value2: int32 = int.add value0, value1
    local.set local0, value2
    value3: ref<int32, borrowed, space(frame)> = local.address local0
    return value3
}
"#,
    );
}

/// Verify borrowing a member emits a field address.
#[test]
fn test_lower_borrows_member_field() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Point {
    x: int32;
    y: int32;
}

function borrowField(point: &Point): &int32 {
    return &point.x;
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
type Point {
    x: int32;
    y: int32;
}

function borrowField(value0: ref<Point, borrowed>): ref<int32, borrowed> {
entry0(value0: ref<Point, borrowed>):
    value1: ref<int32, borrowed, space(frame)> = field.address value0, 0
    return value1
}
"#,
    );
}

/// Verify borrowing an array element emits element addr without checks when disabled.
#[test]
fn test_lower_borrows_array_element() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function borrowElement(values: [int32; 4]): &int32 {
    return &values[2];
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.checks.bounds = CheckPolicy::Never;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function borrowElement(value0: [int32; 4]): ref<int32, borrowed> {
entry0(value0: [int32; 4]):
    value1: int32 = 2int32
    value2: ref<int32, borrowed, space(frame)> = element.address value0, value1
    return value2
}
"#,
    );
}

/// Verify borrowing an array element emits bounds checks when enabled.
#[test]
fn test_lower_borrows_array_element_checked() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function borrowElementChecked(values: [int32; 4]): &int32 {
    return &values[2];
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.checks.bounds = CheckPolicy::Always;
        target.checks.failure = CheckFailurePolicy::Panic;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    let bounds_check_name = test.string_literal_global_name("bounds check failed");
    let string_alias = test.string_type_alias_definition();
    let expected = r#"
${string_alias}
readonly global ${bounds_check_failed}: ref<String, managed, readonly> = "bounds check failed"
function borrowElementChecked(value0: [int32; 4]): ref<int32, borrowed> {
entry0(value0: [int32; 4]):
    value1: int32 = 2int32
    value2: int32 = 4int32
    check bounds.s value1, value2, value0 -> block2, block1
block1:
    value3: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${bounds_check_failed}
    value4: ref<String, managed, readonly> = load value3
    panic value4
block2:
    value5: ref<int32, borrowed, space(frame)> = element.address value0, value1
    return value5
}
        "#;
    let expected = expected.replace("${string_alias}", string_alias);
    let expected = expected.replace("${bounds_check_failed}", &bounds_check_name);
    test.assert_mir(module_id, "native", &expected);
}

/// Verify borrowing a reference array element skips local spilling.
#[test]
fn test_lower_borrows_array_reference_element() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function borrowElementRef(values: &[int32; 4]): &int32 {
    return &values[2];
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.checks.bounds = CheckPolicy::Never;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function borrowElementRef(value0: ref<[int32; 4], borrowed>): ref<int32, borrowed> {
entry0(value0: ref<[int32; 4], borrowed>):
    value1: int32 = 2int32
    value2: ref<int32, borrowed, space(frame)> = element.address value0, value1
    return value2
}
"#,
    );
}

/// Verify borrowing a reference array element emits bounds checks when enabled.
#[test]
fn test_lower_borrows_array_reference_element_checked() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function borrowElementRefChecked(values: &[int32; 4]): &int32 {
    return &values[2];
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.checks.bounds = CheckPolicy::Always;
        target.checks.failure = CheckFailurePolicy::Panic;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    let bounds_check_name = test.string_literal_global_name("bounds check failed");
    let string_alias = test.string_type_alias_definition();
    let expected = r#"
${string_alias}
readonly global ${bounds_check_failed}: ref<String, managed, readonly> = "bounds check failed"
function borrowElementRefChecked(value0: ref<[int32; 4], borrowed>): ref<int32, borrowed> {
entry0(value0: ref<[int32; 4], borrowed>):
    value1: int32 = 2int32
    value2: int32 = 4int32
    check bounds.s value1, value2, value0 -> block2, block1
block1:
    value3: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${bounds_check_failed}
    value4: ref<String, managed, readonly> = load value3
    panic value4
block2:
    value5: ref<int32, borrowed, space(frame)> = element.address value0, value1
    return value5
}
        "#;
    let expected = expected.replace("${string_alias}", string_alias);
    let expected = expected.replace("${bounds_check_failed}", &bounds_check_name);
    test.assert_mir(module_id, "native", &expected);
}

/// Verify borrowing an interface value produces a local addr on the fat pointer.
#[test]
fn test_lower_borrows_interface_value() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Greeter {
    greet(): int32;
}

function borrowGreeter(greeter: Greeter): &Greeter {
    return &greeter;
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
type Greeter.function = () => int32;

type Greeter.object {
    greet: Greeter.function;
}

type Greeter {
    value: ref<void, managed, readonly>;
    table: ref<void, raw, readonly, space(static)>;
}

external function Greeter.greet(Greeter.object): int32

function borrowGreeter(value0: Greeter): ref<Greeter, borrowed> {
    local local0: Greeter, owned

entry0(value0: Greeter):
    local.set local0, value0
    value1: ref<Greeter, borrowed, space(frame)> = local.address local0
    return value1
}
"#,
    );
}

/// Verify borrowing a this field emits a field address.
#[test]
fn test_lower_borrows_this_field() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Counter {
    value: int32 = 0;

    borrowValue(): &int32 {
        return &this.value;
    }
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
type Counter {
    vtable: ref<void, raw, readonly, space(static)>;
    value: int32;
}

readonly global Counter#vtable: [ref<void, raw, readonly, space(static), nullable>; 3], space(static) = zeroInit

function Counter.borrowValue(this0: ref<Counter, managed, readonly>): ref<int32, borrowed> {
entry0(this0: ref<Counter, managed, readonly>):
    value1: ref<int32, borrowed> = field.address this0, 1
    return value1
}
"#,
    );
}
