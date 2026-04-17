use crate::diagnostic::Error;
use crate::tests::{
    assert_execution_completed, assert_execution_yielded, assert_runtime_error_matches,
    create_isolate,
};
use destack_heap::Value;

/// Yield returns a value and resumes with the provided argument.
#[test]
fn test_yield_resume_value() {
    // define mir program
    let mir = r#"
function yieldOnce(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 5int32
    yield v1, b1(v0)
b1(v2: int32, v3: int32):
    v4: int32 = int.add v2, v3
    return v4
}"#;

    // create isolate
    let mut isolate = create_isolate(mir);

    // start coroutine and capture yield
    let yielded = assert_execution_yielded(
        isolate.run_function_by_name_yielding("yieldOnce", &[Value::int32(7)]),
    );

    // verify yielded value
    assert_eq!(yielded.value, Value::int32(5));
    let continuation = yielded.continuation;

    // resume with a value and verify completion
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(11)));
    assert_eq!(output.value, Value::int32(18));
}

/// Yield ignores the resume value when no slot is available.
#[test]
fn test_yield_resume_value_ignored() {
    // define mir program
    let mir = r#"
function yieldIgnore(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 1int32
    yield v1, b1(v0)
b1(v2: int32, v3: int32):
    return v2
}"#;

    // create isolate
    let mut isolate = create_isolate(mir);

    // start coroutine and capture yield
    let yielded = assert_execution_yielded(
        isolate.run_function_by_name_yielding("yieldIgnore", &[Value::int32(9)]),
    );

    // verify yielded value
    assert_eq!(yielded.value, Value::int32(1));
    let continuation = yielded.continuation;

    // resume and verify the resume value is ignored
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(100)));
    assert_eq!(output.value, Value::int32(9));
}

/// Yield can suspend multiple times and resume with new values.
#[test]
fn test_yield_multiple() {
    // define mir program
    let mir = r#"
function yieldTwice(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 2int32
    yield v1, b1(v0)
b1(v2: int32, v3: int32):
    v4: int32 = int.add v2, v3
    yield v4, b2(v4)
b2(v5: int32, v6: int32):
    v7: int32 = int.add v5, v6
    return v7
}"#;

    // create isolate
    let mut isolate = create_isolate(mir);

    // start coroutine and capture first yield
    let yielded = assert_execution_yielded(
        isolate.run_function_by_name_yielding("yieldTwice", &[Value::int32(4)]),
    );

    // verify first yielded value
    assert_eq!(yielded.value, Value::int32(2));
    let continuation = yielded.continuation;

    // resume for second yield
    let yielded = assert_execution_yielded(isolate.resume(continuation, Value::int32(3)));

    // verify second yielded value
    assert_eq!(yielded.value, Value::int32(7));
    let continuation = yielded.continuation;

    // resume for completion
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(10)));
    assert_eq!(output.value, Value::int32(17));
}

/// Yield resumes without explicit resume arguments.
#[test]
fn test_yield_resume_no_args() {
    // define mir program
    let mir = r#"
function yieldNoArgs(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 4int32
    yield v1, b1
b1(v2: int32):
    return v2
}"#;

    // create isolate
    let mut isolate = create_isolate(mir);

    // start coroutine and capture yield
    let yielded = assert_execution_yielded(
        isolate.run_function_by_name_yielding("yieldNoArgs", &[Value::int32(3)]),
    );

    // verify yielded value
    assert_eq!(yielded.value, Value::int32(4));
    let continuation = yielded.continuation;

    // resume and verify resumed value is returned
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(9)));
    assert_eq!(output.value, Value::int32(9));
}

/// Yield preserves locals across suspension.
#[test]
fn test_yield_preserves_locals() {
    // define mir program
    let mir = r#"
function yieldWithLocal(v0: int32): int32 {
    local local0: int32, owned
b0(v0: int32):
    v1: int32 = 4int32
    local.set local0, v1
    yield v1, b1
b1(v2: int32):
    v3: int32 = local.get local0
    v4: int32 = int.add v3, v2
    return v4
}"#;

    // create isolate
    let mut isolate = create_isolate(mir);

    // start coroutine and capture yield
    let yielded = assert_execution_yielded(
        isolate.run_function_by_name_yielding("yieldWithLocal", &[Value::int32(1)]),
    );

    // verify yielded value
    assert_eq!(yielded.value, Value::int32(4));
    let continuation = yielded.continuation;

    // resume and verify local survives
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(6)));
    assert_eq!(output.value, Value::int32(10));
}

/// Yield resumes with explicit arguments and a trailing resume value.
#[test]
fn test_yield_resume_arguments_prefix() {
    // define mir program
    let mir = r#"
function yieldPrefix(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 10int32
    v2: int32 = 20int32
    yield v1, b1(v0, v2)
b1(v3: int32, v4: int32, v5: int32):
    v6: int32 = int.add v3, v4
    v7: int32 = int.add v6, v5
    return v7
}"#;

    // create isolate
    let mut isolate = create_isolate(mir);

    // start coroutine and capture yield
    let yielded = assert_execution_yielded(
        isolate.run_function_by_name_yielding("yieldPrefix", &[Value::int32(5)]),
    );

    // verify yielded value
    assert_eq!(yielded.value, Value::int32(10));
    let continuation = yielded.continuation;

    // resume and verify argument ordering
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(7)));
    assert_eq!(output.value, Value::int32(32));
}

/// Yield clears trailing resume parameters when no argument is provided.
#[test]
fn test_yield_clears_trailing_params() {
    // define mir program
    let mir = r#"
function yieldTrailing(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 1int32
    v2: int32 = 99int32
    jump b1(v0, v1, v2)
b1(v3: int32, v4: int32, v5: int32):
    v6: int32 = 0int32
    v7: boolean = int.eq v5, v6
    branch v7, b3(v5), b2(v3, v4, v5)
b2(v8: int32, v9: int32, v10: int32):
    v11: int32 = int.add v8, v9
    yield v11, b1(v8, v9)
b3(v12: int32):
    return v12
}"#;

    // create isolate
    let mut isolate = create_isolate(mir);

    // start coroutine and capture yield
    let yielded = assert_execution_yielded(
        isolate.run_function_by_name_yielding("yieldTrailing", &[Value::int32(2)]),
    );

    // verify yielded value
    assert_eq!(yielded.value, Value::int32(3));
    let continuation = yielded.continuation;

    // resume with a value that triggers the return path
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(0)));
    assert_eq!(output.value, Value::int32(0));
}

/// Yield in a nested call resumes back to the caller.
#[test]
fn test_yield_nested_call() {
    // define mir program
    let mir = r#"
function yieldInner(v0: int32): int32 {
b0(v0: int32):
    yield v0, b1
b1(v1: int32):
    v2: int32 = 1int32
    v3: int32 = int.add v1, v2
    return v3
}
function outer(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = call yieldInner(v0): (int32) -> int32
    v2: int32 = int.add v1, v0
    return v2
}"#;

    // create isolate
    let mut isolate = create_isolate(mir);

    // start coroutine and capture yield
    let yielded = assert_execution_yielded(
        isolate.run_function_by_name_yielding("outer", &[Value::int32(5)]),
    );

    // verify yielded value
    assert_eq!(yielded.value, Value::int32(5));
    let continuation = yielded.continuation;

    // resume and verify completion
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(7)));
    assert_eq!(output.value, Value::int32(13));
}

/// Yield preserves one pending exceptional call continuation across suspension.
#[test]
fn test_yield_preserves_exceptional_call_continuation() {
    // define mir program
    let mir = r#"
function worker(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 5int32
    yield v1, b1(v0)
b1(v2: int32, v3: int32):
    v4: int32 = int.add v2, v3
    return v4
}

function caller(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 10int32
    invoke worker(v0): (int32) -> int32 -> b1(v1), catch b2
b1(v2: int32, v3: int32):
    v4: int32 = int.add v2, v3
    return v4
b2(v5: ref<void, managed, readonly>):
    v6: int32 = 0int32
    return v6
}"#;

    // create isolate
    let mut isolate = create_isolate(mir);

    // start coroutine and capture the inner yield
    let yielded = assert_execution_yielded(
        isolate.run_function_by_name_yielding("caller", &[Value::int32(7)]),
    );

    // verify yielded value
    assert_eq!(yielded.value, Value::int32(5));
    let continuation = yielded.continuation;

    // resume and verify the outer success continuation still runs
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(3)));
    assert_eq!(output.value, Value::int32(20));
}

/// Yield preserves live frame-local stack storage in the yielded frame.
#[test]
fn test_yield_preserves_stack_alloc_in_current_frame() {
    // define mir program
    let mir = r#"
function yieldStackLocal(): int32 {
b0:
    v0: ref<int32, raw, readonly, addressSpace(stack)> = stack.alloc int32
    v1: int32 = 1int32
    yield v1, b1
b1(v2: int32):
    return v2
}"#;

    // create isolate
    let mut isolate = create_isolate(mir);

    // suspend and resume with live stack-local storage
    let yielded =
        assert_execution_yielded(isolate.run_function_by_name_yielding("yieldStackLocal", &[]));
    assert_eq!(yielded.value, Value::int32(1));

    let output = assert_execution_completed(isolate.resume(yielded.continuation, Value::int32(7)));
    assert_eq!(output.value, Value::int32(7));
}

/// Yield accepts stack allocation after the lifetime is explicitly ended.
#[test]
fn test_yield_allows_retired_stack_alloc_in_current_frame() {
    // define mir program
    let mir = r#"
function yieldRetiredStackLocal(): int32 {
b0:
    v0: ref<int32, raw, readonly, addressSpace(stack)> = stack.alloc int32
    drop v0
    v1: int32 = 1int32
    yield v1, b1
b1(v2: int32):
    return v2
}"#;

    // create isolate
    let mut isolate = create_isolate(mir);

    // allow suspension after the stack allocation is retired
    let yielded = assert_execution_yielded(
        isolate.run_function_by_name_yielding("yieldRetiredStackLocal", &[]),
    );
    assert_eq!(yielded.value, Value::int32(1));
}

/// Yield preserves live frame-local stack storage in suspended caller frames.
#[test]
fn test_yield_preserves_stack_alloc_in_caller_frame() {
    // define mir program
    let mir = r#"
function yieldInner(v0: int32): int32 {
b0(v0: int32):
    yield v0, b1
b1(v1: int32):
    return v1
}
function outerWithStackLocal(v0: int32): int32 {
b0(v0: int32):
    v1: ref<int32, raw, readonly, addressSpace(stack)> = stack.alloc int32
    v2: int32 = call yieldInner(v0): (int32) -> int32
    return v2
}"#;

    // create isolate
    let mut isolate = create_isolate(mir);

    // suspend and resume with caller-owned stack-local storage
    let yielded = assert_execution_yielded(
        isolate.run_function_by_name_yielding("outerWithStackLocal", &[Value::int32(5)]),
    );
    assert_eq!(yielded.value, Value::int32(5));

    let output = assert_execution_completed(isolate.resume(yielded.continuation, Value::int32(9)));
    assert_eq!(output.value, Value::int32(9));
}

/// Yield preserves live frame-local pointers in the yielded frame.
#[test]
fn test_yield_preserves_local_pointer_in_current_frame() {
    // define mir program
    let mir = r#"
function yieldLocalPointer(): int32 {
    local local0: int32, owned
b0:
    v0: int32 = 1int32
    local.set local0, v0
    v1: ref<int32, borrowed, addressSpace(stack)> = local.address local0
    v2: int32 = 2int32
    yield v2, b1
b1(v3: int32):
    v4: int32 = load v1
    return v4
}"#;

    // create isolate
    let mut isolate = create_isolate(mir);

    // suspend and resume with live frame-local pointers
    let yielded =
        assert_execution_yielded(isolate.run_function_by_name_yielding("yieldLocalPointer", &[]));
    assert_eq!(yielded.value, Value::int32(2));

    let output = assert_execution_completed(isolate.resume(yielded.continuation, Value::int32(11)));
    assert_eq!(output.value, Value::int32(1));
}

/// Running a coroutine with the non-yielding entry reports an error.
#[test]
fn test_run_function_rejects_yield() {
    // define mir program
    let mir = r#"
function yieldOnce(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 5int32
    yield v1, b1(v0)
b1(v2: int32, v3: int32):
    v4: int32 = int.add v2, v3
    return v4
}"#;

    // create isolate
    let mut isolate = create_isolate(mir);

    // run via the non-yielding entry
    let result = isolate.run_function_by_name("yieldOnce", &[Value::int32(7)]);

    // verify unexpected yield error
    assert_runtime_error_matches!(result, Error::UnexpectedYield);
}

/// Resume with a continuation from another isolate reports an error.
#[test]
fn test_resume_invalid_continuation() {
    // define mir program
    let mir = r#"
function yieldOnce(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 5int32
    yield v1, b1(v0)
b1(v2: int32, v3: int32):
    v4: int32 = int.add v2, v3
    return v4
}"#;

    // create isolate
    let mut isolate = create_isolate(mir);

    // start coroutine and capture continuation
    let yielded = assert_execution_yielded(
        isolate.run_function_by_name_yielding("yieldOnce", &[Value::int32(7)]),
    );
    let continuation = yielded.continuation;

    // create a different isolate
    let mut other_isolate = create_isolate(mir);

    // resume on a different isolate
    let result = other_isolate.resume(continuation, Value::int32(0));

    // validate error
    assert_runtime_error_matches!(result, Error::InvalidContinuation);
}

/// Continuations can be cloned for multi-shot resumption.
#[test]
fn test_continuation_clone_for_fork() {
    // define mir program
    let mir = r#"
function yieldOnce(): int32 {
b0:
    v0: int32 = 1int32
    yield v0, b1
b1(v1: int32):
    return v1
}"#;

    // create isolate
    let mut isolate = create_isolate(mir);

    // start coroutine and capture continuation
    let yielded = assert_execution_yielded(isolate.run_function_by_name_yielding("yieldOnce", &[]));
    assert_eq!(yielded.value, Value::int32(1));
    let continuation = yielded.continuation;

    // clone the continuation for a forked resume
    let forked = continuation.clone_for_fork();

    // resume the original continuation
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(5)));
    assert_eq!(output.value, Value::int32(5));

    // resume the forked continuation
    let output = assert_execution_completed(isolate.resume(forked, Value::int32(9)));
    assert_eq!(output.value, Value::int32(9));
}

/// Continuation roots keep managed allocations alive across yields.
#[test]
fn test_continuation_roots_keep_allocations() {
    // define mir program
    let mir = r#"
type Pair {
    ref<int32, managed, readonly>;
}
function yieldAlloc(): int32 {
b0:
    v0: ref<int32, managed, readonly> = managed.alloc int32
    v1: int32 = 1int32
    store v0, v1
    v2: Pair = struct Pair (v0)
    yield v1, b1(v2)
b1(v3: Pair, v4: int32):
    v5: ref<int32, managed, readonly> = field.get v3, 0
    v6: int32 = load v5
    return v6
}"#;

    // create isolate
    let mut isolate = create_isolate(mir);

    // start coroutine and capture continuation
    let yielded =
        assert_execution_yielded(isolate.run_function_by_name_yielding("yieldAlloc", &[]));
    let continuation = yielded.continuation;

    // collect garbage while continuation is suspended
    let stats = isolate.collect_garbage_with_continuations(std::slice::from_ref(&continuation));
    assert_eq!(stats.live_allocations, 1);

    // resume and complete the coroutine
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(7)));
    assert_eq!(output.value, Value::int32(1));

    // collect garbage after completion
    let stats = isolate.collect_garbage();
    assert_eq!(stats.live_allocations, 0);
}
