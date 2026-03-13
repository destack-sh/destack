use destack_mir as mir;
use destack_mir::parse::{ParseOptions, Parser};
use destack_source::FileId;

use crate::diagnostic::Error;
use crate::tests::{create_isolate, run_mir, run_mir_expect, run_mir_ok};
use crate::{Isolate, IsolateOptions};
use destack_heap::{Heap, ManagedHeap, RawHeap, Value};

/// Branch instruction takes the true path when condition is true.
#[test]
fn test_branch_true() {
    let mir = r#"
function @select(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block1:
    v1: i32 = iconst 1i32
    return v1
block2:
    v2: i32 = iconst 0i32
    return v2
}"#;
    run_mir_expect(mir, "select", &[Value::bool(true)], Value::int32(1));
}

/// Branch instruction takes the false path when condition is false.
#[test]
fn test_branch_false() {
    let mir = r#"
function @select(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block1:
    v1: i32 = iconst 1i32
    return v1
block2:
    v2: i32 = iconst 0i32
    return v2
}"#;
    run_mir_expect(mir, "select", &[Value::bool(false)], Value::int32(0));
}

/// Jump instruction transfers control to target block.
#[test]
fn test_jump() {
    let mir = r#"
function @jump_test() -> i32 {
block0:
    v0: i32 = iconst 42i32
    jump block1
block1:
    return v0
}"#;
    run_mir_expect(mir, "jump_test", &[], Value::int32(42));
}

/// Switch instruction dispatches to the correct case or default.
#[test]
fn test_switch() {
    let mir = r#"
function @switch_test(v0: i32) -> i32 {
block0(v0: i32):
    switch v0, block3, 0 => block1, 1 => block2
block1:
    v1: i32 = iconst 100i32
    return v1
block2:
    v2: i32 = iconst 200i32
    return v2
block3:
    v3: i32 = iconst 0i32
    return v3
}"#;
    run_mir_expect(mir, "switch_test", &[Value::int32(0)], Value::int32(100));
    run_mir_expect(mir, "switch_test", &[Value::int32(1)], Value::int32(200));
    run_mir_expect(mir, "switch_test", &[Value::int32(99)], Value::int32(0)); // default
}

/// Function calls pass arguments and return values correctly.
#[test]
fn test_simple_call() {
    let mir = r#"
function @add(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2: i32 = iadd v0, v1
    return v2
}

function @caller() -> i32 {
block0:
    v0: i32 = iconst 10i32
    v1: i32 = iconst 20i32
    v2: i32 = call @add(v0, v1)
    return v2
}"#;
    run_mir_expect(mir, "caller", &[], Value::int32(30));
}

/// Recursive calls compute factorial correctly.
#[test]
fn test_recursive_factorial() {
    let mir = r#"
function @factorial(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 1i32
    v2: bool = icmp_sle v0, v1
    branch v2, block1, block2
block1:
    return v1
block2:
    v3: i32 = isub v0, v1
    v4: i32 = call @factorial(v3)
    v5: i32 = imul v0, v4
    return v5
}"#;
    run_mir_expect(mir, "factorial", &[Value::int32(5)], Value::int32(120));
}

/// Infinite recursion triggers stack overflow error.
#[test]
fn test_stack_overflow() {
    let mir = r#"
function @infinite() -> i32 {
block0:
    v0: i32 = call @infinite()
    return v0
}"#;
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
    v0: i32 = iconst 0i32
    jump block0
}"#;
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
}"#;
    run_mir_expect(mir, "noop", &[], Value::VOID);
}

/// Caller's local values are preserved across nested calls.
#[test]
fn test_call_stack_preserved() {
    let mir = r#"
function @inner() -> i32 {
block0:
    v0: i32 = iconst 10i32
    return v0
}

function @outer() -> i32 {
block0:
    v0: i32 = iconst 5i32
    v1: i32 = call @inner()
    v2: i32 = iadd v0, v1
    return v2
}"#;
    run_mir_expect(mir, "outer", &[], Value::int32(15));
}

/// Sequential function calls work correctly.
#[test]
fn test_multiple_function_calls() {
    let mir = r#"
function @double(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 2i32
    v2: i32 = imul v0, v1
    return v2
}

function @caller() -> i32 {
block0:
    v0: i32 = iconst 3i32
    v1: i32 = call @double(v0)
    v2: i32 = call @double(v1)
    return v2
}"#;
    run_mir_expect(mir, "caller", &[], Value::int32(12));
}

/// CallIndirect calls through a function pointer.
#[test]
fn test_call_indirect() {
    let mir_text = r#"
function @double(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 2i32
    v2: i32 = imul v0, v1
    return v2
}

function @caller(v0: fn(i32) -> i32, v1: i32) -> i32 {
block0(v0: fn(i32) -> i32, v1: i32):
    v2: i32 = call.indirect v0(v1) -> fn(i32) -> i32
    return v2
}"#;
    // parse the MIR
    let (tree, strings) = Parser::parse(FileId::new(0), mir_text, ParseOptions::default())
        .expect("failed to parse MIR");

    // find the @double function id
    let double_id = tree
        .iter_nodes::<mir::Function>()
        .find(|(_, f)| strings.get(f.name) == "double")
        .map(|(id, _)| id)
        .expect("double not found");

    // create isolate and run
    let mut isolate = Isolate::build_with_options(tree, strings, IsolateOptions::test())
        .unwrap_or_else(|error| panic!("failed to initialize isolate: {error}"));
    let mut heap = Heap::new();

    // initialize isolate state against the authoritative heap
    isolate
        .initialize(&mut heap)
        .unwrap_or_else(|error| panic!("failed to initialize isolate globals: {error}"));

    let result = isolate
        .run_function_by_name(
            &mut heap,
            "caller",
            &[Value::function_pointer(double_id), Value::int32(21)],
        )
        .expect("execution failed");

    assert_eq!(result.value, Value::int32(42));
}

/// CallIndirect with wrong type produces an error.
#[test]
fn test_call_indirect_type_mismatch() {
    let mir_text = r#"
function @caller(v0: fn(i32) -> i32, v1: i32) -> i32 {
block0(v0: fn(i32) -> i32, v1: i32):
    v2: i32 = call.indirect v0(v1) -> fn(i32) -> i32
    return v2
}"#;
    let (tree, strings) = Parser::parse(FileId::new(0), mir_text, ParseOptions::default())
        .expect("failed to parse MIR");
    let mut isolate = Isolate::build_with_options(tree, strings, IsolateOptions::test())
        .unwrap_or_else(|error| panic!("failed to initialize isolate: {error}"));
    let mut heap = Heap::new();

    // initialize isolate state against the authoritative heap
    isolate
        .initialize(&mut heap)
        .unwrap_or_else(|error| panic!("failed to initialize isolate globals: {error}"));

    // pass an integer instead of a function pointer
    let result =
        isolate.run_function_by_name(&mut heap, "caller", &[Value::int32(999), Value::int32(21)]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.error, Error::TypeMismatch { .. }));
}

/// Tail calls reuse the current frame without growing the stack.
#[test]
fn test_tail_call_direct() {
    let mir = r#"
function @countdown(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2: i32 = iconst 0i32
    v3: bool = icmp_eq v0, v2
    branch v3, block2, block1
block1:
    v4: i32 = iconst 1i32
    v5: i32 = isub v0, v4
    v6: i32 = iadd v1, v4
    tailcall @countdown(v5, v6)
block2:
    return v1
}

function @entry(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 0i32
    tailcall @countdown(v0, v1)
}"#;

    // run tail call with depth beyond the test stack limit
    let output = run_mir_ok(mir, "entry", &[Value::int32(200)]);
    assert_eq!(output.value, Value::int32(200));
    assert_eq!(output.statistics.max_stack_depth, 1);
}

/// Tail call indirect reuses the current frame without growing the stack.
#[test]
fn test_tail_call_indirect() {
    let mir_text = r#"
function @countdown(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2: i32 = iconst 0i32
    v3: bool = icmp_eq v0, v2
    branch v3, block2, block1
block1:
    v4: i32 = iconst 1i32
    v5: i32 = isub v0, v4
    v6: i32 = iadd v1, v4
    tailcall @countdown(v5, v6)
block2:
    return v1
}

function @entry(v0: i32, v1: fn(i32, i32) -> i32) -> i32 {
block0(v0: i32, v1: fn(i32, i32) -> i32):
    v2: i32 = iconst 0i32
    tailcall.indirect v1(v0, v2) -> fn(i32, i32) -> i32
}"#;

    // parse the MIR
    let (tree, strings) = Parser::parse(FileId::new(0), mir_text, ParseOptions::default())
        .expect("failed to parse MIR");

    // find the @countdown function id
    let countdown_id = tree
        .iter_nodes::<mir::Function>()
        .find(|(_, f)| strings.get(f.name) == "countdown")
        .map(|(id, _)| id)
        .expect("countdown not found");

    // create isolate and run
    let mut isolate = create_isolate(mir_text);
    let result = isolate
        .run_function_by_name(
            "entry",
            &[Value::int32(200), Value::function_pointer(countdown_id)],
        )
        .expect("execution failed");

    assert_eq!(result.value, Value::int32(200));
    assert_eq!(result.statistics.max_stack_depth, 1);
}

/// Block parameters are correctly passed via jump.
#[test]
fn test_block_parameters_jump() {
    let mir = r#"
function @block_params() -> i32 {
block0:
    v0: i32 = iconst 10i32
    v1: i32 = iconst 20i32
    jump block1(v0, v1)
block1(v2: i32, v3: i32):
    v4: i32 = iadd v2, v3
    return v4
}"#;
    run_mir_expect(mir, "block_params", &[], Value::int32(30));
}

/// Block parameters are correctly passed via branch.
#[test]
fn test_block_parameters_branch() {
    let mir = r#"
function @branch_params(v0: bool) -> i32 {
block0(v0: bool):
    v1: i32 = iconst 100i32
    v2: i32 = iconst 200i32
    branch v0, block1(v1), block1(v2)
block1(v3: i32):
    return v3
}"#;
    run_mir_expect(
        mir,
        "branch_params",
        &[Value::bool(true)],
        Value::int32(100),
    );
    run_mir_expect(
        mir,
        "branch_params",
        &[Value::bool(false)],
        Value::int32(200),
    );
}

/// Unreachable terminator produces an error.
#[test]
fn test_unreachable() {
    let mir = r#"
function @unreachable_fn() -> i32 {
block0:
    unreachable
}"#;
    let result = run_mir(mir, "unreachable_fn", &[]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.error, Error::Unreachable));
}
