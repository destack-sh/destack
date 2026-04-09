use destack_vm::Value;
use destack_workspace::{BoundsCheckPolicy, CheckFailurePolicy};

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
function assignReturn(v0: float64): float64 {
b0(v0: float64):
    v1: float64 = 1float64
    v2: float64 = float.add v0, v1
    v3: float64 = 2float64
    v4: float64 = float.add v2, v3
    v5: float64 = float.add v4, v0
    return v5
}"#,
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
function borrowLocal(v0: int32): ref<int32, borrowed> {
    local local0: int32, owned
b0(v0: int32):
    local.set local0, v0
    v1: ref<int32, borrowed> = local.address local0
    return v1
}"#,
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
function borrowTemp(v0: int32): ref<int32, borrowed> {
    local local0: int32, owned
b0(v0: int32):
    v1: int32 = 1int32
    v2: int32 = int.add v0, v1
    local.set local0, v2
    v3: ref<int32, borrowed> = local.address local0
    return v3
}"#,
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
function borrowField(v0: ref<Point, borrowed>): ref<int32, borrowed> {
b0(v0: ref<Point, borrowed>):
    v1: ref<int32, borrowed> = field.address v0, 0
    return v1
}"#,
    );
}

/// Verify borrowing an array element emits element addr without checks when disabled.
#[test]
fn test_lower_borrows_array_element() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function borrowElement(values: int32[4]): &int32 {
    return &values[2];
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.bounds_checks = BoundsCheckPolicy::Never;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function borrowElement(v0: int32[4]): ref<int32, borrowed> {
    local local0: int32[4], owned, readonly
b0(v0: int32[4]):
    local.set local0, v0
    v1: int32[4] = local.get local0
    v2: int32 = 2int32
    v3: ref<int32, borrowed> = element.address v1, v2
    return v3
}"#,
    );
}

/// Verify borrowing an array element emits bounds checks when enabled.
#[test]
fn test_lower_borrows_array_element_checked() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function borrowElementChecked(values: int32[4]): &int32 {
    return &values[2];
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.bounds_checks = BoundsCheckPolicy::Always;
        target.check_failure = CheckFailurePolicy::Panic;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    let bounds_check_name = test.string_literal_global_name("bounds check failed");
    let string_alias = test.string_type_alias_definition();
    let expected = r#"
${string_alias}
global ${bounds_check_failed}: ref<String, managed, readonly>, readonly = "bounds check failed"
function borrowElementChecked(v0: int32[4]): ref<int32, borrowed> {
    local local0: int32[4], owned, readonly
b0(v0: int32[4]):
    local.set local0, v0
    v1: int32[4] = local.get local0
    v2: int32 = 2int32
    v3: int32 = 4int32
    v4: int32 = 0int32
    v5: boolean = int.ge.s v2, v4
    v6: boolean = int.lt.s v2, v3
    v7: boolean = int.and v5, v6
    check bounds.s v2, v3, v1 -> b2, b1
b1:
    v8: ref<String, managed, readonly> = global.const ${bounds_check_failed}
    trap.panic v8
b2:
    v9: ref<int32, borrowed> = element.address v1, v2
    return v9
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
function borrowElementRef(values: &int32[4]): &int32 {
    return &values[2];
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.bounds_checks = BoundsCheckPolicy::Never;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function borrowElementRef(v0: ref<int32[4], borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32[4], borrowed>):
    v1: int32 = 2int32
    v2: ref<int32, borrowed> = element.address v0, v1
    return v2
}"#,
    );
}

/// Verify borrowing a reference array element emits bounds checks when enabled.
#[test]
fn test_lower_borrows_array_reference_element_checked() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function borrowElementRefChecked(values: &int32[4]): &int32 {
    return &values[2];
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.bounds_checks = BoundsCheckPolicy::Always;
        target.check_failure = CheckFailurePolicy::Panic;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    let bounds_check_name = test.string_literal_global_name("bounds check failed");
    let string_alias = test.string_type_alias_definition();
    let expected = r#"
${string_alias}
global ${bounds_check_failed}: ref<String, managed, readonly>, readonly = "bounds check failed"
function borrowElementRefChecked(v0: ref<int32[4], borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32[4], borrowed>):
    v1: int32 = 2int32
    v2: int32 = 4int32
    v3: int32 = 0int32
    v4: boolean = int.ge.s v1, v3
    v5: boolean = int.lt.s v1, v2
    v6: boolean = int.and v4, v5
    check bounds.s v1, v2, v0 -> b2, b1
b1:
    v7: ref<String, managed, readonly> = global.const ${bounds_check_failed}
    trap.panic v7
b2:
    v8: ref<int32, borrowed> = element.address v0, v1
    return v8
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
type Greeter {
    object: ref<void, managed, readonly>;
    itab: usize;
}
extern function Greeter.greet({ greet: closure() -> int32 }): int32
function borrowGreeter(v0: Greeter): ref<Greeter, borrowed> {
    local local0: Greeter, owned, readonly
b0(v0: Greeter):
    local.set local0, v0
    v1: ref<Greeter, borrowed> = local.address local0
    return v1
}"#,
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
    vtable: ref<void, raw, readonly, addressSpace(global)>;
    value: int32;
}
global Counter#vtable: ref?<void, raw, readonly, addressSpace(global)>[3], readonly = zeroInit
function Counter.borrowValue(v0: ref<Counter, managed, readonly>): ref<int32, borrowed> {
b0(v0: ref<Counter, managed, readonly>):
    v1: ref<int32, borrowed> = field.address v0, 1
    return v1
}"#,
    );
}
