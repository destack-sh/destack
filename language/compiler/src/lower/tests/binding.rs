use destack_vm::Value;
use destack_workspace::{BoundsCheckPolicy, CheckFailurePolicy};

use crate::TestProgram;

/// Verify module-level const declarations are lowered correctly.
#[test]
fn test_lower_module_constants() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
function @assignReturn(v0: f64) -> f64 {
block0(v0: f64):
    v1: f64 = iconst 1f64
    v2: f64 = fadd v0, v1
    v3: f64 = iconst 2f64
    v4: f64 = fadd v2, v3
    v5: f64 = fadd v4, v0
    return v5
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
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
function @borrowLocal(v0: i32) -> ref<borrowed i32> {
    local0: i32 ; owned
block0(v0: i32):
    local.set local0, v0
    v1: ref<borrowed i32> = local.addr local0
    return v1
}
        "#,
    );
}

/// Verify borrowing an rvalue spills to a temporary local.
#[test]
fn test_lower_borrows_rvalue() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
function @borrowTemp(v0: i32) -> ref<borrowed i32> {
    local0: i32 ; owned
block0(v0: i32):
    v1: i32 = iconst 1i32
    v2: i32 = iadd v0, v1
    local.set local0, v2
    v3: ref<borrowed i32> = local.addr local0
    return v3
}
        "#,
    );
}

/// Verify borrowing a member emits a field address.
#[test]
fn test_lower_borrows_member_field() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
type @Point = { x: i32, y: i32 }

function @borrowField(v0: ref<borrowed @Point>) -> ref<borrowed i32> {
block0(v0: ref<borrowed @Point>):
    v1: ref<borrowed i32> = field.addr v0, 0
    return v1
}
        "#,
    );
}

/// Verify borrowing an array element emits element addr without checks when disabled.
#[test]
fn test_lower_borrows_array_element() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
function @borrowElement(v0: [i32; 4]) -> ref<borrowed i32> {
    local0: [i32; 4] ; owned, readonly
block0(v0: [i32; 4]):
    local.set local0, v0
    v1: [i32; 4] = local.get local0
    v2: i32 = iconst 2i32
    v3: ref<borrowed i32> = element.addr v1, v2
    return v3
}
        "#,
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
global @${bounds_check_failed}: ref<managed readonly @String> = "bounds check failed" ; readonly

function @borrowElementChecked(v0: [i32; 4]) -> ref<borrowed i32> {
    local0: [i32; 4] ; owned, readonly
block0(v0: [i32; 4]):
    local.set local0, v0
    v1: [i32; 4] = local.get local0
    v2: i32 = iconst 2i32
    v3: i32 = iconst 4i32
    v4: i32 = iconst 0i32
    v5: bool = icmp_sge v2, v4
    v6: bool = icmp_slt v2, v3
    v7: bool = band v5, v6
    check v7, bounds.signed v2, v3, v1, block2, block1
block1:
    v8: ref<managed readonly @String> = global.const @${bounds_check_failed}
    trap panic v8
block2:
    v9: ref<borrowed i32> = element.addr v1, v2
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
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
function @borrowElementRef(v0: ref<borrowed [i32; 4]>) -> ref<borrowed i32> {
block0(v0: ref<borrowed [i32; 4]>):
    v1: i32 = iconst 2i32
    v2: ref<borrowed i32> = element.addr v0, v1
    return v2
}
        "#,
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
global @${bounds_check_failed}: ref<managed readonly @String> = "bounds check failed" ; readonly

function @borrowElementRefChecked(v0: ref<borrowed [i32; 4]>) -> ref<borrowed i32> {
block0(v0: ref<borrowed [i32; 4]>):
    v1: i32 = iconst 2i32
    v2: i32 = iconst 4i32
    v3: i32 = iconst 0i32
    v4: bool = icmp_sge v1, v3
    v5: bool = icmp_slt v1, v2
    v6: bool = band v4, v5
    check v6, bounds.signed v1, v2, v0, block2, block1
block1:
    v7: ref<managed readonly @String> = global.const @${bounds_check_failed}
    trap panic v7
block2:
    v8: ref<borrowed i32> = element.addr v0, v1
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
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
type @Greeter = { @object: ref<managed readonly void>, @itab: usize }

extern function @Greeter.greet({ greet: fnvalue<fn() -> i32, ref?<managed void>> }) -> i32

function @borrowGreeter(v0: @Greeter) -> ref<borrowed @Greeter> {
    local0: @Greeter ; owned, readonly
block0(v0: @Greeter):
    local.set local0, v0
    v1: ref<borrowed @Greeter> = local.addr local0
    return v1
}
        "#,
    );
}

/// Verify borrowing a this field emits a field address.
#[test]
fn test_lower_borrows_this_field() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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
type @Counter = { @vtable: ref<raw addrspace(global) readonly void>, value: i32 }

global @Counter#vtable: [ref?<raw addrspace(global) readonly void>; 3] = zeroinit ; readonly

function @Counter.borrowValue(v0: ref<managed readonly @Counter>) -> ref<borrowed i32> {
block0(v0: ref<managed readonly @Counter>):
    v1: ref<borrowed i32> = field.addr v0, 1
    return v1
}
        "#,
    );
}
