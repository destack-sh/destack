use destack_engine::Value;

use crate::TestProgram;

/// Lower `is` checks for tagged unions.
#[test]
fn test_lower_type_binary_union_checks() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Circle { radius: int32 }

struct Square { width: int32 }

function isCircle(value: Circle | Square): boolean {
    return value is Circle;
}

function useIsCircle(): boolean {
    return isCircle(Circle { radius: 3 });
}

function useIsCircleNegative(): boolean {
    return isCircle(Square { width: 4 });
}

"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(module_id, "native", "useIsCircle", &[], Value::bool(true));

    test.assert_mir_function_output(
        module_id,
        "native",
        "useIsCircleNegative",
        &[],
        Value::bool(false),
    );
}

/// Lower `is` checks for tagged unions.
#[test]
fn test_lower_union_type_binary_is() {
    // set up the test program
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Circle {
    radius: int32;
}

struct Square {
    width: int32;
}

function isCircle(value: Circle | Square): boolean {
    return value is Circle;
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
type isCircle.value#union { tag: uint8, payload: usize[1] }

function isCircle(value0: isCircle.value#union): boolean {
entry0(value0: isCircle.value#union):
    value1: uint8 = 0uint8
    value2: uint8 = field.get value0, 0
    value3: boolean = int.eq value2, value1
    return value3
}
        "#,
    );
}
