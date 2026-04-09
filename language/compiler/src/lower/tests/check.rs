use destack_mir as mir;
use destack_workspace::{
    BoundsCheckPolicy, CheckFailurePolicy, DivisionCheckPolicy, OverflowCheckPolicy,
    ShiftCheckPolicy,
};

use crate::TestProgram;

/// Emit overflow checks for integer arithmetic when configured.
#[test]
fn test_lower_overflow_checks() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function sum(a: int32, b: int32): int32 {
    return a + b;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.overflow_checks = OverflowCheckPolicy::Always;
        target.check_failure = CheckFailurePolicy::Panic;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    let integer_overflow_name = test.string_literal_global_name("integer overflow");
    let string_alias = test.string_type_alias_definition();
    let expected = r#"
${string_alias}
global ${integer_overflow}: ref<String, managed, readonly>, readonly = "integer overflow"
function sum(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: (int32, boolean) = intrinsic.add.overflow(v0, v1)
    v3: int32 = field.get v2, 0
    v4: boolean = field.get v2, 1
    v5: boolean = int.not v4
    check int.add.overflow.s v0, v1 -> b2, b1
b1:
    v6: ref<String, managed, readonly> = global.const ${integer_overflow}
    trap.panic v6
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
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function sum(a: int32, b: int32): int32 {
    return a + b;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.overflow_checks = OverflowCheckPolicy::Always;
        target.check_failure = CheckFailurePolicy::Abort;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function sum(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: (int32, boolean) = intrinsic.add.overflow(v0, v1)
    v3: int32 = field.get v2, 0
    v4: boolean = field.get v2, 1
    v5: boolean = int.not v4
    check int.add.overflow.s v0, v1 -> b2, b1
b1:
    trap.abort
b2:
    return v3
}"#,
    );
}

/// Emit unsigned overflow checks for integer arithmetic when configured.
#[test]
fn test_lower_overflow_checks_unsigned() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function sum(a: uint32, b: uint32): uint32 {
    return a + b;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.overflow_checks = OverflowCheckPolicy::Always;
        target.check_failure = CheckFailurePolicy::Panic;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    let integer_overflow_name = test.string_literal_global_name("integer overflow");
    let string_alias = test.string_type_alias_definition();
    let expected = r#"
${string_alias}
global ${integer_overflow}: ref<String, managed, readonly>, readonly = "integer overflow"
function sum(v0: uint32, v1: uint32): uint32 {
b0(v0: uint32, v1: uint32):
    v2: (uint32, boolean) = intrinsic.add.overflow(v0, v1)
    v3: uint32 = field.get v2, 0
    v4: boolean = field.get v2, 1
    v5: boolean = int.not v4
    check int.add.overflow.u v0, v1 -> b2, b1
b1:
    v6: ref<String, managed, readonly> = global.const ${integer_overflow}
    trap.panic v6
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
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function sum(a: int32, b: int32): int32 {
    return a + b;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.overflow_checks = OverflowCheckPolicy::Never;
        target.check_failure = CheckFailurePolicy::Panic;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function sum(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}"#,
    );
}

/// Emit division checks for integer division when configured.
#[test]
fn test_lower_division_checks() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function quotient(a: int32, b: int32): int32 {
    return a / b;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.division_checks = DivisionCheckPolicy::Always;
        target.check_failure = CheckFailurePolicy::Panic;
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
function quotient(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = 0int32
    v3: boolean = int.ne v1, v2
    check zeroDivisor v1 -> b2, b1
b1:
    v4: ref<String, managed, readonly> = global.const ${division_by_zero}
    trap.panic v4
b2:
    v5: int32 = -2147483648int32
    v6: int32 = -1int32
    v7: boolean = int.eq v0, v5
    v8: boolean = int.eq v1, v6
    v9: boolean = int.and v7, v8
    v10: boolean = int.not v9
    check int.div.overflow.s v0, v1 -> b4, b3
b3:
    v11: ref<String, managed, readonly> = global.const ${division_overflow}
    trap.panic v11
b4:
    v12: int32 = int.div.s v0, v1
    return v12
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
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function quotient(a: int32, b: int32): int32 {
    return a / b;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.division_checks = DivisionCheckPolicy::Always;
        target.check_failure = CheckFailurePolicy::Trap;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function quotient(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = 0int32
    v3: boolean = int.ne v1, v2
    check zeroDivisor v1 -> b2, b1
b1:
    trap.abort
b2:
    v4: int32 = -2147483648int32
    v5: int32 = -1int32
    v6: boolean = int.eq v0, v4
    v7: boolean = int.eq v1, v5
    v8: boolean = int.and v6, v7
    v9: boolean = int.not v8
    check int.div.overflow.s v0, v1 -> b4, b3
b3:
    trap.abort
b4:
    v10: int32 = int.div.s v0, v1
    return v10
}"#,
    );
}

/// Emit unsigned division checks without overflow handling.
#[test]
fn test_lower_division_checks_unsigned() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function quotient(a: uint32, b: uint32): uint32 {
    return a / b;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.division_checks = DivisionCheckPolicy::Always;
        target.check_failure = CheckFailurePolicy::Panic;
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
function quotient(v0: uint32, v1: uint32): uint32 {
b0(v0: uint32, v1: uint32):
    v2: uint32 = 0uint32
    v3: boolean = int.ne v1, v2
    check zeroDivisor v1 -> b2, b1
b1:
    v4: ref<String, managed, readonly> = global.const ${division_by_zero}
    trap.panic v4
b2:
    v5: uint32 = int.div.u v0, v1
    return v5
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
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function shift(value: int32, amount: int32): int32 {
    return value << amount;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.shift_checks = ShiftCheckPolicy::Always;
        target.check_failure = CheckFailurePolicy::Panic;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    let shift_out_of_range_name = test.string_literal_global_name("shift out of range");
    let string_alias = test.string_type_alias_definition();
    let expected = r#"
${string_alias}
global ${shift_out_of_range}: ref<String, managed, readonly>, readonly = "shift out of range"
function shift(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = 32int32
    v3: int32 = 0int32
    v4: boolean = int.ge.s v1, v3
    v5: boolean = int.lt.s v1, v2
    v6: boolean = int.and v4, v5
    check shiftRange.s v1, 32 -> b2, b1
b1:
    v7: ref<String, managed, readonly> = global.const ${shift_out_of_range}
    trap.panic v7
b2:
    v8: int32 = int.shiftLeft v0, v1
    return v8
}
        "#;
    let expected = expected.replace("${string_alias}", string_alias);
    let expected = expected.replace("${shift_out_of_range}", &shift_out_of_range_name);
    test.assert_mir(module_id, "native", &expected);
}

/// Emit shift range checks with abort failure policy.
#[test]
fn test_lower_shift_checks_abort() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function shift(value: int32, amount: int32): int32 {
    return value << amount;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.shift_checks = ShiftCheckPolicy::Always;
        target.check_failure = CheckFailurePolicy::Abort;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function shift(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = 32int32
    v3: int32 = 0int32
    v4: boolean = int.ge.s v1, v3
    v5: boolean = int.lt.s v1, v2
    v6: boolean = int.and v4, v5
    check shiftRange.s v1, 32 -> b2, b1
b1:
    trap.abort
b2:
    v7: int32 = int.shiftLeft v0, v1
    return v7
}"#,
    );
}

/// Emit unsigned shift checks for unsigned shift amounts.
#[test]
fn test_lower_shift_checks_unsigned() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function shift(value: uint32, amount: uint32): uint32 {
    return value << amount;
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.shift_checks = ShiftCheckPolicy::Always;
        target.check_failure = CheckFailurePolicy::Panic;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    let shift_out_of_range_name = test.string_literal_global_name("shift out of range");
    let string_alias = test.string_type_alias_definition();
    let expected = r#"
${string_alias}
global ${shift_out_of_range}: ref<String, managed, readonly>, readonly = "shift out of range"
function shift(v0: uint32, v1: uint32): uint32 {
b0(v0: uint32, v1: uint32):
    v2: uint32 = 32uint32
    v3: boolean = int.lt.u v1, v2
    check shiftRange.u v1, 32 -> b2, b1
b1:
    v4: ref<String, managed, readonly> = global.const ${shift_out_of_range}
    trap.panic v4
b2:
    v5: uint32 = int.shiftLeft v0, v1
    return v5
}
        "#;
    let expected = expected.replace("${string_alias}", string_alias);
    let expected = expected.replace("${shift_out_of_range}", &shift_out_of_range_name);
    test.assert_mir(module_id, "native", &expected);
}

/// Emit bounds checks for array access when configured.
#[test]
fn test_lower_bounds_checks() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function element(values: int32[4], index: int32): int32 {
    return values[index];
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
function element(v0: int32[4], v1: int32): int32 {
b0(v0: int32[4], v1: int32):
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
    v8: int32 = element.get v0, v1
    return v8
}
        "#;
    let expected = expected.replace("${string_alias}", string_alias);
    let expected = expected.replace("${bounds_check_failed}", &bounds_check_name);
    test.assert_mir(module_id, "native", &expected);
}

/// Emit bounds checks with trap failure policy.
#[test]
fn test_lower_bounds_checks_trap() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function element(values: int32[4], index: int32): int32 {
    return values[index];
}
"#,
    );

    test.configure_target(module_id, "native", |target| {
        target.bounds_checks = BoundsCheckPolicy::Always;
        target.check_failure = CheckFailurePolicy::Trap;
    });
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function element(v0: int32[4], v1: int32): int32 {
b0(v0: int32[4], v1: int32):
    v2: int32 = 4int32
    v3: int32 = 0int32
    v4: boolean = int.ge.s v1, v3
    v5: boolean = int.lt.s v1, v2
    v6: boolean = int.and v4, v5
    check bounds.s v1, v2, v0 -> b2, b1
b1:
    trap.abort
b2:
    v7: int32 = element.get v0, v1
    return v7
}"#,
    );
}

/// Emit unsigned bounds checks for unsigned indices.
#[test]
fn test_lower_bounds_checks_unsigned() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function element(values: int32[4], index: uint32): int32 {
    return values[index];
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
function element(v0: int32[4], v1: uint32): int32 {
b0(v0: int32[4], v1: uint32):
    v2: uint32 = 4uint32
    v3: boolean = int.lt.u v1, v2
    check bounds.u v1, v2, v0 -> b2, b1
b1:
    v4: ref<String, managed, readonly> = global.const ${bounds_check_failed}
    trap.panic v4
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
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
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

/// Apply stack only allocation mode when requested by decorators.
#[test]
fn test_lower_sets_stack_only_allocation_mode() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
@stackOnly
function work(): void {}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.with_mir_tree(module_id, "native", |tree, strings| {
        let function = test.function_by_name(tree, strings, "work");
        assert_eq!(function.allocation, mir::AllocationMode::StackOnly);
    });
}

/// Do not force no managed allocation mode from profile flags.
#[test]
fn test_lower_sets_profile_no_managed_allocation_mode() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function work(): void {}
"#,
    );

    test.apply_destack_config(module_id, r#"{ "compilerOptions": { "noManaged": true } }"#);
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.with_mir_tree(module_id, "native", |tree, strings| {
        let function = test.function_by_name(tree, strings, "work");
        assert_eq!(function.allocation, mir::AllocationMode::Any);
    });
}
