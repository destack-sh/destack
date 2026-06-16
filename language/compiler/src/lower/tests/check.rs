use destack_mir as mir;
use destack_repository::{CheckFailurePolicy, CheckPolicy};

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
readonly global ${integer_overflow}: ref<String, managed, readonly> = "integer overflow"
function sum(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: (int32, boolean) = intrinsic.math.arithmetic.overflowing.add(v0, v1)
    v3: int32 = field.get v2, 0
    v4: boolean = field.get v2, 1
    check int.add.overflow.s v0, v1 => b2, b1
b1:
    v5: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${integer_overflow}
    v6: ref<String, managed, readonly> = load v5
    panic v6
b2:
    return v3
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
function sum(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: (int32, boolean) = intrinsic.math.arithmetic.overflowing.add(v0, v1)
    v3: int32 = field.get v2, 0
    v4: boolean = field.get v2, 1
    check int.add.overflow.s v0, v1 => b2, b1

b1:
    trap.abort

b2:
    return v3
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
readonly global ${integer_overflow}: ref<String, managed, readonly> = "integer overflow"
function sum(v0: uint32, v1: uint32): uint32 {
entry(v0: uint32, v1: uint32):
    v2: (uint32, boolean) = intrinsic.math.arithmetic.overflowing.add(v0, v1)
    v3: uint32 = field.get v2, 0
    v4: boolean = field.get v2, 1
    check int.add.overflow.u v0, v1 => b2, b1
b1:
    v5: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${integer_overflow}
    v6: ref<String, managed, readonly> = load v5
    panic v6
b2:
    return v3
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
function sum(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
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
readonly global ${division_by_zero}: ref<String, managed, readonly> = "division by zero"
readonly global ${division_overflow}: ref<String, managed, readonly> = "division overflow"
function quotient(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    check div.zero v1 => b2, b1
b1:
    v2: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${division_by_zero}
    v3: ref<String, managed, readonly> = load v2
    panic v3
b2:
    check int.div.overflow.s v0, v1 => b4, b3
b3:
    v4: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${division_overflow}
    v5: ref<String, managed, readonly> = load v4
    panic v5
b4:
    v6: int32 = int.div.s v0, v1
    return v6
}
        "#;
    let expected = expected.replace("${string_alias}", string_alias);
    let expected = expected
        .replace("${division_by_zero}", &division_by_zero_name)
        .replace("${division_overflow}", &division_overflow_name);
    test.assert_mir(module_id, "native", &expected);
}

/// Emit division checks with abort failure policy.
#[test]
fn test_lower_division_checks_abort() {
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
        target.checks.failure = CheckFailurePolicy::Abort;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function quotient(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    check div.zero v1 => b2, b1

b1:
    trap.abort

b2:
    check int.div.overflow.s v0, v1 => b4, b3

b3:
    trap.abort

b4:
    v2: int32 = int.div.s v0, v1
    return v2
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
readonly global ${division_by_zero}: ref<String, managed, readonly> = "division by zero"
readonly global ${division_overflow}: ref<String, managed, readonly> = "division overflow"
function quotient(v0: uint32, v1: uint32): uint32 {
entry(v0: uint32, v1: uint32):
    check div.zero v1 => b2, b1
b1:
    v2: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${division_by_zero}
    v3: ref<String, managed, readonly> = load v2
    panic v3
b2:
    v4: uint32 = int.div.u v0, v1
    return v4
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
readonly global ${shift_out_of_range}: ref<String, managed, readonly> = "shift out of range"
function shift(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    check shift.range.s v1, 32 => b2, b1
b1:
    v2: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${shift_out_of_range}
    v3: ref<String, managed, readonly> = load v2
    panic v3
b2:
    v4: int32 = int.shl v0, v1
    return v4
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
function shift(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    check shift.range.s v1, 32 => b2, b1

b1:
    trap.abort

b2:
    v2: int32 = int.shl v0, v1
    return v2
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
readonly global ${shift_out_of_range}: ref<String, managed, readonly> = "shift out of range"
function shift(v0: uint32, v1: uint32): uint32 {
entry(v0: uint32, v1: uint32):
    check shift.range.u v1, 32 => b2, b1
b1:
    v2: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${shift_out_of_range}
    v3: ref<String, managed, readonly> = load v2
    panic v3
b2:
    v4: uint32 = int.shl v0, v1
    return v4
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
function element(values: [int32; 4], index: int32): int32 {
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
readonly global ${bounds_check_failed}: ref<String, managed, readonly> = "bounds check failed"
function element(v0: [int32; 4], v1: int32): int32 {
entry(v0: [int32; 4], v1: int32):
    v2: int32 = 4int32
    check bounds.s v1, v2, v0 => b2, b1
b1:
    v3: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${bounds_check_failed}
    v4: ref<String, managed, readonly> = load v3
    panic v4
b2:
    v5: int32 = element.get v0, v1
    return v5
}
        "#;
    let expected = expected.replace("${string_alias}", string_alias);
    let expected = expected.replace("${bounds_check_failed}", &bounds_check_name);
    test.assert_mir(module_id, "native", &expected);
}

/// Emit bounds checks with abort failure policy.
#[test]
fn test_lower_bounds_checks_abort() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function element(values: [int32; 4], index: int32): int32 {
    return values[index];
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.checks.bounds = CheckPolicy::Always;
        target.checks.failure = CheckFailurePolicy::Abort;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function element(v0: [int32; 4], v1: int32): int32 {
entry(v0: [int32; 4], v1: int32):
    v2: int32 = 4int32
    check bounds.s v1, v2, v0 => b2, b1

b1:
    trap.abort

b2:
    v3: int32 = element.get v0, v1
    return v3
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
function element(values: [int32; 4], index: uint32): int32 {
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
readonly global ${bounds_check_failed}: ref<String, managed, readonly> = "bounds check failed"
function element(v0: [int32; 4], v1: uint32): int32 {
entry(v0: [int32; 4], v1: uint32):
    v2: uint32 = 4uint32
    check bounds.u v1, v2, v0 => b2, b1
b1:
    v3: ref<ref<String, managed, readonly>, raw, readonly> = global.address ${bounds_check_failed}
    v4: ref<String, managed, readonly> = load v3
    panic v4
b2:
    v5: int32 = element.get v0, v1
    return v5
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

    test.apply_destack_config(
        module_id,
        r#"{ "compiler": { "restrictions": { "noManaged": "deny" } } }"#,
    );
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.with_mir_tree(module_id, "native", |tree, strings| {
        let function = test.function_by_name(tree, strings, "work");
        assert_eq!(function.allocation, mir::AllocationMode::Any);
    });
}
