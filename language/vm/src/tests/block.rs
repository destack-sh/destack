use crate::diagnostic::Error;
use crate::tests::{
    assert_runtime_error, assert_runtime_error_matches, create_isolate, run_mir, run_mir_expect,
    run_mir_ok,
};
use crate::{FunctionPointer, Value, Word};

/// Branch instruction takes the true path when condition is true.
#[test]
fn test_branch_true() {
    let mir = r#"
function select(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b1, b2
b1:
    v1: int32 = 1int32
    return v1
b2:
    v2: int32 = 0int32
    return v2
}"#;
    run_mir_expect(mir, "select", &[Value::bool(true)], Value::int32(1));
}

/// Branch instruction takes the false path when condition is false.
#[test]
fn test_branch_false() {
    let mir = r#"
function select(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b1, b2
b1:
    v1: int32 = 1int32
    return v1
b2:
    v2: int32 = 0int32
    return v2
}"#;
    run_mir_expect(mir, "select", &[Value::bool(false)], Value::int32(0));
}

/// Jump instruction transfers control to target block.
#[test]
fn test_jump() {
    let mir = r#"
function jumpTest(): int32 {
b0:
    v0: int32 = 42int32
    jump b1
b1:
    return v0
}"#;
    run_mir_expect(mir, "jumpTest", &[], Value::int32(42));
}

/// Switch instruction dispatches to the correct case or default.
#[test]
fn test_switch() {
    let mir = r#"
function switchTest(v0: int32): int32 {
b0(v0: int32):
    switch v0, b3, 0 => b1, 1 => b2
b1:
    v1: int32 = 100int32
    return v1
b2:
    v2: int32 = 200int32
    return v2
b3:
    v3: int32 = 0int32
    return v3
}"#;
    run_mir_expect(mir, "switchTest", &[Value::int32(0)], Value::int32(100));
    run_mir_expect(mir, "switchTest", &[Value::int32(1)], Value::int32(200));
    run_mir_expect(mir, "switchTest", &[Value::int32(99)], Value::int32(0));
}

/// Function calls pass arguments and return values correctly.
#[test]
fn test_simple_call() {
    let mir = r#"
function add(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}
function caller(): int32 {
b0:
    v0: int32 = 10int32
    v1: int32 = 20int32
    v2: int32 = call add(v0, v1): (int32, int32) -> int32
    return v2
}"#;
    run_mir_expect(mir, "caller", &[], Value::int32(30));
}

/// Recursive calls compute factorial correctly.
#[test]
fn test_recursive_factorial() {
    let mir = r#"
function factorial(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 1int32
    v2: boolean = int.le.s v0, v1
    branch v2, b1, b2
b1:
    return v1
b2:
    v3: int32 = int.sub v0, v1
    v4: int32 = call factorial(v3): (int32) -> int32
    v5: int32 = int.mul v0, v4
    return v5
}"#;
    run_mir_expect(mir, "factorial", &[Value::int32(5)], Value::int32(120));
}

/// Infinite recursion triggers stack overflow error.
#[test]
fn test_stack_overflow() {
    let mir = r#"
function infinite(): int32 {
b0:
    v0: int32 = call infinite(): () -> int32
    return v0
}"#;
    let result = run_mir(mir, "infinite", &[]);

    assert_runtime_error(result, Error::StackOverflow);
}

/// Infinite loop triggers step limit exceeded error.
#[test]
fn test_step_limit() {
    let mir = r#"
function infiniteLoop(): int32 {
b0:
    v0: int32 = 0int32
    jump b0
}"#;
    let result = run_mir(mir, "infiniteLoop", &[]);

    assert_runtime_error(result, Error::StepLimitExceeded);
}

/// Void functions return without a value.
#[test]
fn test_void_return() {
    let mir = r#"
function noop(): void {
b0:
    return
}"#;
    run_mir_expect(mir, "noop", &[], Value::VOID);
}

/// Caller's local values are preserved across nested calls.
#[test]
fn test_call_stack_preserved() {
    let mir = r#"
function inner(): int32 {
b0:
    v0: int32 = 10int32
    return v0
}

function outer(): int32 {
b0:
    v0: int32 = 5int32
    v1: int32 = call inner(): () -> int32
    v2: int32 = int.add v0, v1
    return v2
}"#;
    run_mir_expect(mir, "outer", &[], Value::int32(15));
}

/// Sequential function calls work correctly.
#[test]
fn test_multiple_function_calls() {
    let mir = r#"
function double(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 2int32
    v2: int32 = int.mul v0, v1
    return v2
}

function caller(): int32 {
b0:
    v0: int32 = 3int32
    v1: int32 = call double(v0): (int32) -> int32
    v2: int32 = call double(v1): (int32) -> int32
    return v2
}"#;
    run_mir_expect(mir, "caller", &[], Value::int32(12));
}

/// CallIndirect calls through a function pointer.
#[test]
fn test_call_indirect() {
    let mir_text = r#"
function double(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 2int32
    v2: int32 = int.mul v0, v1
    return v2
}

function caller(v0: (int32) -> int32, v1: int32): int32 {
b0(v0: (int32) -> int32, v1: int32):
    v2: int32 = call.indirect v0(v1): (int32) -> int32
    return v2
}"#;

    let mut isolate = create_isolate(mir_text);
    let double_id = isolate
        .isolate
        .function_id_by_name("double")
        .expect("double should exist");
    let result = isolate
        .run_frame_function_by_name(
            "caller",
            &[
                Word::function_pointer(FunctionPointer::from_bits(double_id.id as usize)),
                Word::int32(21),
            ],
        )
        .expect("execution failed");

    assert_eq!(result.value, Value::int32(42));
}

/// CallIndirect with wrong type produces an error.
#[test]
fn test_call_indirect_type_mismatch() {
    let mir_text = r#"
function wrong(v0: int64): int32 {
b0(v0: int64):
    v1: int32 = 0int32
    return v1
}

function caller(v0: (int32) -> int32, v1: int32): int32 {
b0(v0: (int32) -> int32, v1: int32):
    v2: int32 = call.indirect v0(v1): (int32) -> int32
    return v2
}"#;

    let mut isolate = create_isolate(mir_text);
    let wrong_id = isolate
        .isolate
        .function_id_by_name("wrong")
        .expect("wrong should exist");
    let result = isolate.run_frame_function_by_name(
        "caller",
        &[
            Word::function_pointer(FunctionPointer::from_bits(wrong_id.id as usize)),
            Word::int32(21),
        ],
    );

    assert_runtime_error_matches!(result, Error::TypeMismatch { .. });
}

/// Tail calls reuse the current frame without growing the stack.
#[test]
fn test_tail_call_direct() {
    let mir = r#"
function countdown(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = 0int32
    v3: boolean = int.eq v0, v2
    branch v3, b2, b1
b1:
    v4: int32 = 1int32
    v5: int32 = int.sub v0, v4
    v6: int32 = int.add v1, v4
    tailCall countdown(v5, v6): (int32, int32) -> int32
b2:
    return v1
}

function entry(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    tailCall countdown(v0, v1): (int32, int32) -> int32
}"#;
    let output = run_mir_ok(mir, "entry", &[Value::int32(200)]);
    assert_eq!(output.value, Value::int32(200));
}

/// Self tail calls release stack allocations before re-entering.
#[test]
fn test_tail_call_self_releases_stack_allocations() {
    let mir = r#"
function countdown(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    v2: boolean = int.eq v0, v1
    branch v2, b2, b1
b1:
    v3: int32 = 1int32
    v4: ref<int32[512], raw, space(stack)> = stack.alloc int32[512]
    v5: int32 = int.sub v0, v3
    tailCall countdown(v5): (int32) -> int32
b2:
    return v0
}"#;
    let output = run_mir_ok(mir, "countdown", &[Value::int32(600)]);
    assert_eq!(output.value, Value::int32(0));
}

/// Tail call indirect reuses the current frame without growing the stack.
#[test]
fn test_tail_call_indirect() {
    let mir_text = r#"
function countdown(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = 0int32
    v3: boolean = int.eq v0, v2
    branch v3, b2, b1
b1:
    v4: int32 = 1int32
    v5: int32 = int.sub v0, v4
    v6: int32 = int.add v1, v4
    tailCall countdown(v5, v6): (int32, int32) -> int32
b2:
    return v1
}

function entry(v0: int32, v1: (int32, int32) -> int32): int32 {
b0(v0: int32, v1: (int32, int32) -> int32):
    v2: int32 = 0int32
    tailCall.indirect v1(v0, v2): (int32, int32) -> int32
}"#;

    let mut isolate = create_isolate(mir_text);
    let countdown_id = isolate
        .isolate
        .function_id_by_name("countdown")
        .expect("countdown should exist");
    let result = isolate
        .run_frame_function_by_name(
            "entry",
            &[
                Word::int32(200),
                Word::function_pointer(FunctionPointer::from_bits(countdown_id.id as usize)),
            ],
        )
        .expect("execution failed");

    assert_eq!(result.value, Value::int32(200));
}

/// Block parameters are correctly passed via jump.
#[test]
fn test_block_parameters_jump() {
    let mir = r#"
function blockParams(): int32 {
b0:
    v0: int32 = 10int32
    v1: int32 = 20int32
    jump b1(v0, v1)
b1(v2: int32, v3: int32):
    v4: int32 = int.add v2, v3
    return v4
}"#;
    run_mir_expect(mir, "blockParams", &[], Value::int32(30));
}

/// Block parameters are correctly passed via branch.
#[test]
fn test_block_parameters_branch() {
    let mir = r#"
function branchParams(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 100int32
    v2: int32 = 200int32
    branch v0, b1(v1), b1(v2)
b1(v3: int32):
    return v3
}"#;
    run_mir_expect(mir, "branchParams", &[Value::bool(true)], Value::int32(100));
    run_mir_expect(
        mir,
        "branchParams",
        &[Value::bool(false)],
        Value::int32(200),
    );
}

/// Array block parameters stay correct across ordinary CFG jumps.
#[test]
fn test_block_parameters_jump_array() {
    let mir = r#"
function arrayParams(): int32 {
b0:
    v0: int32 = 10int32
    v1: int32 = 20int32
    v2: int32 = 30int32
    v3: int32[3] = array int32[3] (v0, v1, v2)
    jump b1(v3)
b1(v4: int32[3]):
    v5: int64 = 2int64
    v6: int32 = element.get v4, v5
    return v6
}"#;
    run_mir_expect(mir, "arrayParams", &[], Value::int32(30));
}

/// Updated local arrays stay decomposed across ordinary CFG jumps.
#[test]
fn test_block_parameters_jump_updated_array() {
    let mir = r#"
function updatedArrayParams(v0: int64, v1: int32): int32 {
b0(v0: int64, v1: int32):
    v2: int32 = 10int32
    v3: int32 = 20int32
    v4: int32 = 30int32
    v5: int32[3] = array int32[3] (v2, v3, v4)
    v6: int32[3] = element.set v5, v0, v1
    jump b1(v6, v0)
b1(v7: int32[3], v8: int64):
    v9: int32 = element.get v7, v8
    return v9
}"#;

    run_mir_expect(
        mir,
        "updatedArrayParams",
        &[Value::uint64(2), Value::int32(99)],
        Value::int32(99),
    );
}

/// Unreachable terminator produces an error.
#[test]
fn test_unreachable() {
    let mir = r#"
function unreachableFunction(): int32 {
b0:
    unreachable
}"#;
    let result = run_mir(mir, "unreachableFunction", &[]);

    assert_runtime_error(result, Error::Unreachable);
}
