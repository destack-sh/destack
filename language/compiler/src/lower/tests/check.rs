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

    test.assert_mir(
        module_id,
        "native",
        r#"
function @sum(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = intrinsic.add.overflow(v0, v1)
    v3 = field.get v2, 0
    v4 = field.get v2, 1
    v5 = bnot v4
    check v5, overflow.signed.iadd v0, v1, block2, block1
block1:
    v6 = iconst "integer overflow"
    intrinsic.panic(v6)
    unreachable
block2:
    return v3
}
        "#,
    );
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
function @sum(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = intrinsic.add.overflow(v0, v1)
    v3 = field.get v2, 0
    v4 = field.get v2, 1
    v5 = bnot v4
    check v5, overflow.signed.iadd v0, v1, block2, block1
block1:
    intrinsic.abort()
    unreachable
block2:
    return v3
}
        "#,
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

    test.assert_mir(
        module_id,
        "native",
        r#"
function @sum(v0: u32, v1: u32) -> u32 {
block0(v0: u32, v1: u32):
    v2 = intrinsic.add.overflow(v0, v1)
    v3 = field.get v2, 0
    v4 = field.get v2, 1
    v5 = bnot v4
    check v5, overflow.unsigned.iadd v0, v1, block2, block1
block1:
    v6 = iconst "integer overflow"
    intrinsic.panic(v6)
    unreachable
block2:
    return v3
}
        "#,
    );
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
function @sum(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}
        "#,
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

    test.assert_mir(
        module_id,
        "native",
        r#"
function @quotient(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iconst 0i32
    v3 = icmp_ne v1, v2
    check v3, div_zero v1, block2, block1
block1:
    v4 = iconst "division by zero"
    intrinsic.panic(v4)
    unreachable
block2:
    v5 = iconst -2147483648i32
    v6 = iconst -1i32
    v7 = icmp_eq v0, v5
    v8 = icmp_eq v1, v6
    v9 = band v7, v8
    v10 = bnot v9
    check v10, overflow.signed.sdiv v0, v1, block4, block3
block3:
    v11 = iconst "division overflow"
    intrinsic.panic(v11)
    unreachable
block4:
    v12 = sdiv v0, v1
    return v12
}
        "#,
    );
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
function @quotient(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iconst 0i32
    v3 = icmp_ne v1, v2
    check v3, div_zero v1, block2, block1
block1:
    unreachable
block2:
    v4 = iconst -2147483648i32
    v5 = iconst -1i32
    v6 = icmp_eq v0, v4
    v7 = icmp_eq v1, v5
    v8 = band v6, v7
    v9 = bnot v8
    check v9, overflow.signed.sdiv v0, v1, block4, block3
block3:
    unreachable
block4:
    v10 = sdiv v0, v1
    return v10
}
        "#,
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

    test.assert_mir(
        module_id,
        "native",
        r#"
function @quotient(v0: u32, v1: u32) -> u32 {
block0(v0: u32, v1: u32):
    v2 = iconst 0u32
    v3 = icmp_ne v1, v2
    check v3, div_zero v1, block2, block1
block1:
    v4 = iconst "division by zero"
    intrinsic.panic(v4)
    unreachable
block2:
    v5 = udiv v0, v1
    return v5
}
        "#,
    );
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

    test.assert_mir(
        module_id,
        "native",
        r#"
function @shift(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iconst 32i32
    v3 = iconst 0i32
    v4 = icmp_sge v1, v3
    v5 = icmp_slt v1, v2
    v6 = band v4, v5
    check v6, shift.signed v1, 32, block2, block1
block1:
    v7 = iconst "shift out of range"
    intrinsic.panic(v7)
    unreachable
block2:
    v8 = ishl v0, v1
    return v8
}
        "#,
    );
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
function @shift(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iconst 32i32
    v3 = iconst 0i32
    v4 = icmp_sge v1, v3
    v5 = icmp_slt v1, v2
    v6 = band v4, v5
    check v6, shift.signed v1, 32, block2, block1
block1:
    intrinsic.abort()
    unreachable
block2:
    v7 = ishl v0, v1
    return v7
}
        "#,
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

    test.assert_mir(
        module_id,
        "native",
        r#"
function @shift(v0: u32, v1: u32) -> u32 {
block0(v0: u32, v1: u32):
    v2 = iconst 32u32
    v3 = icmp_ult v1, v2
    check v3, shift.unsigned v1, 32, block2, block1
block1:
    v4 = iconst "shift out of range"
    intrinsic.panic(v4)
    unreachable
block2:
    v5 = ishl v0, v1
    return v5
}
        "#,
    );
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

    test.assert_mir(
        module_id,
        "native",
        r#"
function @element(v0: [i32; 4], v1: i32) -> i32 {
block0(v0: [i32; 4], v1: i32):
    v2 = iconst 4i32
    v3 = iconst 0i32
    v4 = icmp_sge v1, v3
    v5 = icmp_slt v1, v2
    v6 = band v4, v5
    check v6, bounds.signed v1, v2, v0, block2, block1
block1:
    v7 = iconst "bounds check failed"
    intrinsic.panic(v7)
    unreachable
block2:
    v8 = element.get v0, v1
    return v8
}
        "#,
    );
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
function @element(v0: [i32; 4], v1: i32) -> i32 {
block0(v0: [i32; 4], v1: i32):
    v2 = iconst 4i32
    v3 = iconst 0i32
    v4 = icmp_sge v1, v3
    v5 = icmp_slt v1, v2
    v6 = band v4, v5
    check v6, bounds.signed v1, v2, v0, block2, block1
block1:
    unreachable
block2:
    v7 = element.get v0, v1
    return v7
}
        "#,
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

    test.assert_mir(
        module_id,
        "native",
        r#"
function @element(v0: [i32; 4], v1: u32) -> i32 {
block0(v0: [i32; 4], v1: u32):
    v2 = iconst 4u32
    v3 = icmp_ult v1, v2
    check v3, bounds.unsigned v1, v2, v0, block2, block1
block1:
    v4 = iconst "bounds check failed"
    intrinsic.panic(v4)
    unreachable
block2:
    v5 = element.get v0, v1
    return v5
}
        "#,
    );
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

/// Apply no managed allocation mode when configured in profiles.
#[test]
fn test_lower_sets_profile_no_managed_allocation_mode() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function work(): void {}
"#,
    );

    test.apply_dsconfig(module_id, r#"{ "compilerOptions": { "noManaged": true } }"#);
    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.with_mir_tree(module_id, "native", |tree, strings| {
        let function = test.function_by_name(tree, strings, "work");
        assert_eq!(function.allocation, mir::AllocationMode::NoManaged);
    });
}
