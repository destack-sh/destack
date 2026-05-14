use destack_mir as mir;
use destack_workspace::{CheckFailurePolicy, CheckPolicy};

use crate::TestProgram;

/// Emit overflow checks for integer arithmetic when configured.
#[test]
fn test_lower_overflow_checks() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function sum(a: int32, b: int32): int32 {
    return a + b;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.checks.overflow = CheckPolicy::Always;
        target.checks.failure = CheckFailurePolicy::Panic;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    let integer_overflow_name = test.string_literal_global_name("integer overflow");
    let string_alias = test.string_type_alias_definition();
    let expected = r#"
${string_alias}
global ${integer_overflow}: ref<String, managed, readonly>, readonly = "integer overflow"
function sum(value0: int32, value1: int32): int32 {
entry0(value0: int32, value1: int32):
    value2: (int32, boolean) = intrinsic.math.arithmetic.overflowing.add(value0, value1)
    value3: int32 = field.get value2, 0
    value4: boolean = field.get value2, 1
    check int.add.overflow.s value0, value1 -> block2, block1
block1:
    value5: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${integer_overflow}
    value6: ref<String, managed, readonly> = load value5
    trap.panic value6
block2:
    return value3
}
        "#;
    let expected = expected.replace("${string_alias}", string_alias);
    let expected = expected.replace("${integer_overflow}", &integer_overflow_name);
    test.assert_mir(module_id, "native", &expected);
}

/// Emit overflow checks with abort failure policy.
#[test]
fn test_lower_overflow_checks_abort() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function sum(a: int32, b: int32): int32 {
    return a + b;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.checks.overflow = CheckPolicy::Always;
        target.checks.failure = CheckFailurePolicy::Abort;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function sum(value0: int32, value1: int32): int32 {
entry0(value0: int32, value1: int32):
    value2: (int32, boolean) = intrinsic.math.arithmetic.overflowing.add(value0, value1)
    value3: int32 = field.get value2, 0
    value4: boolean = field.get value2, 1
    check int.add.overflow.s value0, value1 -> block2, block1

block1:
    trap.abort

block2:
    return value3
}
"#,
    );
}

/// Emit unsigned overflow checks for integer arithmetic when configured.
#[test]
fn test_lower_overflow_checks_unsigned() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function sum(a: uint32, b: uint32): uint32 {
    return a + b;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.checks.overflow = CheckPolicy::Always;
        target.checks.failure = CheckFailurePolicy::Panic;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    let integer_overflow_name = test.string_literal_global_name("integer overflow");
    let string_alias = test.string_type_alias_definition();
    let expected = r#"
${string_alias}
global ${integer_overflow}: ref<String, managed, readonly>, readonly = "integer overflow"
function sum(value0: uint32, value1: uint32): uint32 {
entry0(value0: uint32, value1: uint32):
    value2: (uint32, boolean) = intrinsic.math.arithmetic.overflowing.add(value0, value1)
    value3: uint32 = field.get value2, 0
    value4: boolean = field.get value2, 1
    check int.add.overflow.u value0, value1 -> block2, block1
block1:
    value5: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${integer_overflow}
    value6: ref<String, managed, readonly> = load value5
    trap.panic value6
block2:
    return value3
}
        "#;
    let expected = expected.replace("${string_alias}", string_alias);
    let expected = expected.replace("${integer_overflow}", &integer_overflow_name);
    test.assert_mir(module_id, "native", &expected);
}

/// Skip overflow checks when disabled.
#[test]
fn test_lower_overflow_checks_disabled() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function sum(a: int32, b: int32): int32 {
    return a + b;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.checks.overflow = CheckPolicy::Never;
        target.checks.failure = CheckFailurePolicy::Panic;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function sum(value0: int32, value1: int32): int32 {
entry0(value0: int32, value1: int32):
    value2: int32 = int.add value0, value1
    return value2
}
"#,
    );
}

/// Emit division checks for integer division when configured.
#[test]
fn test_lower_division_checks() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function quotient(a: int32, b: int32): int32 {
    return a / b;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.checks.division = CheckPolicy::Always;
        target.checks.failure = CheckFailurePolicy::Panic;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    let division_by_zero_name = test.string_literal_global_name("division by zero");
    let division_overflow_name = test.string_literal_global_name("division overflow");
    let string_alias = test.string_type_alias_definition();
    let expected = r#"
${string_alias}
global ${division_by_zero}: ref<String, managed, readonly>, readonly = "division by zero"
global ${division_overflow}: ref<String, managed, readonly>, readonly = "division overflow"
function quotient(value0: int32, value1: int32): int32 {
entry0(value0: int32, value1: int32):
    check zeroDivisor value1 -> block2, block1
block1:
    value2: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${division_by_zero}
    value3: ref<String, managed, readonly> = load value2
    trap.panic value3
block2:
    check int.div.overflow.s value0, value1 -> block4, block3
block3:
    value4: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${division_overflow}
    value5: ref<String, managed, readonly> = load value4
    trap.panic value5
block4:
    value6: int32 = int.div.s value0, value1
    return value6
}
        "#;
    let expected = expected.replace("${string_alias}", string_alias);
    let expected = expected
        .replace("${division_by_zero}", &division_by_zero_name)
        .replace("${division_overflow}", &division_overflow_name);
    test.assert_mir(module_id, "native", &expected);
}

/// Emit division checks with trap failure policy.
#[test]
fn test_lower_division_checks_trap() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function quotient(a: int32, b: int32): int32 {
    return a / b;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.checks.division = CheckPolicy::Always;
        target.checks.failure = CheckFailurePolicy::Trap;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function quotient(value0: int32, value1: int32): int32 {
entry0(value0: int32, value1: int32):
    check zeroDivisor value1 -> block2, block1

block1:
    trap.abort

block2:
    check int.div.overflow.s value0, value1 -> block4, block3

block3:
    trap.abort

block4:
    value2: int32 = int.div.s value0, value1
    return value2
}
"#,
    );
}

/// Emit unsigned division checks without overflow handling.
#[test]
fn test_lower_division_checks_unsigned() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function quotient(a: uint32, b: uint32): uint32 {
    return a / b;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.checks.division = CheckPolicy::Always;
        target.checks.failure = CheckFailurePolicy::Panic;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    let division_by_zero_name = test.string_literal_global_name("division by zero");
    let division_overflow_name = test.string_literal_global_name("division overflow");
    let string_alias = test.string_type_alias_definition();
    let expected = r#"
${string_alias}
global ${division_by_zero}: ref<String, managed, readonly>, readonly = "division by zero"
global ${division_overflow}: ref<String, managed, readonly>, readonly = "division overflow"
function quotient(value0: uint32, value1: uint32): uint32 {
entry0(value0: uint32, value1: uint32):
    check zeroDivisor value1 -> block2, block1
block1:
    value2: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${division_by_zero}
    value3: ref<String, managed, readonly> = load value2
    trap.panic value3
block2:
    value4: uint32 = int.div.u value0, value1
    return value4
}
        "#;
    let expected = expected.replace("${string_alias}", string_alias);
    let expected = expected
        .replace("${division_by_zero}", &division_by_zero_name)
        .replace("${division_overflow}", &division_overflow_name);
    test.assert_mir(module_id, "native", &expected);
}

/// Emit shift range checks for integer shifts when configured.
#[test]
fn test_lower_shift_checks() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function shift(value: int32, amount: int32): int32 {
    return value << amount;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.checks.shift = CheckPolicy::Always;
        target.checks.failure = CheckFailurePolicy::Panic;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    let shift_out_of_range_name = test.string_literal_global_name("shift out of range");
    let string_alias = test.string_type_alias_definition();
    let expected = r#"
${string_alias}
global ${shift_out_of_range}: ref<String, managed, readonly>, readonly = "shift out of range"
function shift(value0: int32, value1: int32): int32 {
entry0(value0: int32, value1: int32):
    check shiftRange.s value1, 32 -> block2, block1
block1:
    value2: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${shift_out_of_range}
    value3: ref<String, managed, readonly> = load value2
    trap.panic value3
block2:
    value4: int32 = int.shl value0, value1
    return value4
}
        "#;
    let expected = expected.replace("${string_alias}", string_alias);
    let expected = expected.replace("${shift_out_of_range}", &shift_out_of_range_name);
    test.assert_mir(module_id, "native", &expected);
}

/// Emit shift range checks with abort failure policy.
#[test]
fn test_lower_shift_checks_abort() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function shift(value: int32, amount: int32): int32 {
    return value << amount;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.checks.shift = CheckPolicy::Always;
        target.checks.failure = CheckFailurePolicy::Abort;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function shift(value0: int32, value1: int32): int32 {
entry0(value0: int32, value1: int32):
    check shiftRange.s value1, 32 -> block2, block1

block1:
    trap.abort

block2:
    value2: int32 = int.shl value0, value1
    return value2
}
"#,
    );
}

/// Emit unsigned shift checks for unsigned shift amounts.
#[test]
fn test_lower_shift_checks_unsigned() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function shift(value: uint32, amount: uint32): uint32 {
    return value << amount;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.checks.shift = CheckPolicy::Always;
        target.checks.failure = CheckFailurePolicy::Panic;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    let shift_out_of_range_name = test.string_literal_global_name("shift out of range");
    let string_alias = test.string_type_alias_definition();
    let expected = r#"
${string_alias}
global ${shift_out_of_range}: ref<String, managed, readonly>, readonly = "shift out of range"
function shift(value0: uint32, value1: uint32): uint32 {
entry0(value0: uint32, value1: uint32):
    check shiftRange.u value1, 32 -> block2, block1
block1:
    value2: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${shift_out_of_range}
    value3: ref<String, managed, readonly> = load value2
    trap.panic value3
block2:
    value4: uint32 = int.shl value0, value1
    return value4
}
        "#;
    let expected = expected.replace("${string_alias}", string_alias);
    let expected = expected.replace("${shift_out_of_range}", &shift_out_of_range_name);
    test.assert_mir(module_id, "native", &expected);
}

/// Emit bounds checks for array access when configured.
#[test]
fn test_lower_bounds_checks() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function element(values: int32[4], index: int32): int32 {
    return values[index];
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
global ${bounds_check_failed}: ref<String, managed, readonly>, readonly = "bounds check failed"
function element(value0: int32[4], value1: int32): int32 {
entry0(value0: int32[4], value1: int32):
    value2: int32 = 4int32
    check bounds.s value1, value2, value0 -> block2, block1
block1:
    value3: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${bounds_check_failed}
    value4: ref<String, managed, readonly> = load value3
    trap.panic value4
block2:
    value5: int32 = element.get value0, value1
    return value5
}
        "#;
    let expected = expected.replace("${string_alias}", string_alias);
    let expected = expected.replace("${bounds_check_failed}", &bounds_check_name);
    test.assert_mir(module_id, "native", &expected);
}

/// Emit bounds checks with trap failure policy.
#[test]
fn test_lower_bounds_checks_trap() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function element(values: int32[4], index: int32): int32 {
    return values[index];
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.checks.bounds = CheckPolicy::Always;
        target.checks.failure = CheckFailurePolicy::Trap;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function element(value0: int32[4], value1: int32): int32 {
entry0(value0: int32[4], value1: int32):
    value2: int32 = 4int32
    check bounds.s value1, value2, value0 -> block2, block1

block1:
    trap.abort

block2:
    value3: int32 = element.get value0, value1
    return value3
}
"#,
    );
}

/// Emit unsigned bounds checks for unsigned indices.
#[test]
fn test_lower_bounds_checks_unsigned() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function element(values: int32[4], index: uint32): int32 {
    return values[index];
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
global ${bounds_check_failed}: ref<String, managed, readonly>, readonly = "bounds check failed"
function element(value0: int32[4], value1: uint32): int32 {
entry0(value0: int32[4], value1: uint32):
    value2: uint32 = 4uint32
    check bounds.u value1, value2, value0 -> block2, block1
block1:
    value3: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${bounds_check_failed}
    value4: ref<String, managed, readonly> = load value3
    trap.panic value4
block2:
    value5: int32 = element.get value0, value1
    return value5
}
        "#;
    let expected = expected.replace("${string_alias}", string_alias);
    let expected = expected.replace("${bounds_check_failed}", &bounds_check_name);
    test.assert_mir(module_id, "native", &expected);
}

/// Apply no managed allocation mode when requested by decorators.
#[test]
fn test_lower_sets_no_managed_allocation_mode() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
@noManaged
function work(): void {}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.with_mir_tree(module_id, "native", |tree, strings| {
        let function = test.function_by_name(tree, strings, "work");
        assert_eq!(function.allocation, mir::AllocationMode::NoManaged);
    });
}

/// Apply no heap allocation mode when requested by decorators.
#[test]
fn test_lower_sets_no_heap_allocation_mode() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
@noHeap
function work(): void {}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.with_mir_tree(module_id, "native", |tree, strings| {
        let function = test.function_by_name(tree, strings, "work");
        assert_eq!(function.allocation, mir::AllocationMode::NoHeap);
    });
}

/// Do not force no managed allocation mode from profile flags.
#[test]
fn test_lower_sets_profile_no_managed_allocation_mode() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function work(): void {}
"#,
    );

    test.apply_destack_config(module_id, r#"{ "compiler": { "noManaged": true } }"#);
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.with_mir_tree(module_id, "native", |tree, strings| {
        let function = test.function_by_name(tree, strings, "work");
        assert_eq!(function.allocation, mir::AllocationMode::Any);
    });
}
