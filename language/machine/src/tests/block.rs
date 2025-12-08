use crate::diagnostic::Error;
use crate::memory::Value;
use crate::tests::{expect_evaluate_mir, run_mir};

/// Branch instruction takes the true path when condition is true.
#[test]
fn test_branch_true() {
    let mir = r#"
function @select(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block1:
    v1 = iconst 1i32
    return v1
block2:
    v2 = iconst 0i32
    return v2
}
"#;
    expect_evaluate_mir(mir, "select", &[Value::Bool(true)], Value::int32(1));
}

/// Branch instruction takes the false path when condition is false.
#[test]
fn test_branch_false() {
    let mir = r#"
function @select(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block1:
    v1 = iconst 1i32
    return v1
block2:
    v2 = iconst 0i32
    return v2
}
"#;
    expect_evaluate_mir(mir, "select", &[Value::Bool(false)], Value::int32(0));
}

/// Jump instruction transfers control to target block.
#[test]
fn test_jump() {
    let mir = r#"
function @jump_test() -> i32 {
block0:
    v0 = iconst 42i32
    jump block1
block1:
    return v0
}
"#;
    expect_evaluate_mir(mir, "jump_test", &[], Value::int32(42));
}

/// Switch instruction dispatches to the correct case or default.
#[test]
fn test_switch() {
    let mir = r#"
function @switch_test(v0: i32) -> i32 {
block0(v0: i32):
    switch v0, block3, 0 => block1, 1 => block2
block1:
    v1 = iconst 100i32
    return v1
block2:
    v2 = iconst 200i32
    return v2
block3:
    v3 = iconst 0i32
    return v3
}
"#;
    expect_evaluate_mir(mir, "switch_test", &[Value::int32(0)], Value::int32(100));
    expect_evaluate_mir(mir, "switch_test", &[Value::int32(1)], Value::int32(200));
    expect_evaluate_mir(mir, "switch_test", &[Value::int32(99)], Value::int32(0)); // default
}

/// Function calls pass arguments and return values correctly.
#[test]
fn test_simple_call() {
    let mir = r#"
function @add(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}

function @caller() -> i32 {
block0:
    v0 = iconst 10i32
    v1 = iconst 20i32
    v2 = call @add(v0, v1)
    return v2
}
"#;
    expect_evaluate_mir(mir, "caller", &[], Value::int32(30));
}

/// Recursive calls compute factorial correctly.
#[test]
fn test_recursive_factorial() {
    let mir = r#"
function @factorial(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    v2 = icmp_sle v0, v1
    branch v2, block1, block2
block1:
    return v1
block2:
    v3 = isub v0, v1
    v4 = call @factorial(v3)
    v5 = imul v0, v4
    return v5
}
"#;
    expect_evaluate_mir(mir, "factorial", &[Value::int32(5)], Value::int32(120));
}

/// Infinite recursion triggers stack overflow error.
#[test]
fn test_stack_overflow() {
    let mir = r#"
function @infinite() -> i32 {
block0:
    v0 = call @infinite()
    return v0
}
"#;
    let result = run_mir(mir, "infinite", &[]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.error, Error::StackOverflow));
}

/// Infinite loop triggers step limit exceeded error.
#[test]
fn test_step_limit() {
    let mir = r#"
function @infinite_loop() -> i32 {
block0:
    v0 = iconst 0i32
    jump block0
}
"#;
    let result = run_mir(mir, "infinite_loop", &[]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.error, Error::StepLimitExceeded));
}

/// Void functions return without a value.
#[test]
fn test_void_return() {
    let mir = r#"
function @noop() -> void {
block0:
    return
}
"#;
    expect_evaluate_mir(mir, "noop", &[], Value::Void);
}

/// Caller's local values are preserved across nested calls.
#[test]
fn test_call_stack_preserved() {
    let mir = r#"
function @inner() -> i32 {
block0:
    v0 = iconst 10i32
    return v0
}

function @outer() -> i32 {
block0:
    v0 = iconst 5i32
    v1 = call @inner()
    v2 = iadd v0, v1
    return v2
}
"#;
    expect_evaluate_mir(mir, "outer", &[], Value::int32(15));
}

/// Sequential function calls work correctly.
#[test]
fn test_multiple_function_calls() {
    let mir = r#"
function @double(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 2i32
    v2 = imul v0, v1
    return v2
}

function @caller() -> i32 {
block0:
    v0 = iconst 3i32
    v1 = call @double(v0)
    v2 = call @double(v1)
    return v2
}
"#;
    expect_evaluate_mir(mir, "caller", &[], Value::int32(12));
}
