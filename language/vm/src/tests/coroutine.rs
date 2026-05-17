use crate::diagnostic::Error;
use crate::tests::{
    assert_execution_completed, assert_execution_yielded, assert_runtime_error_matches,
    create_isolate, create_isolate_with_id,
};
use crate::{IsolateId, Value};

/// Yield returns a value and resumes with the provided argument.
#[test]
fn test_yield_resume_value() {
    let mir = r#"
function yieldOnce(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 5int32
    yield v1, b1(v0)
b1(v2: int32, v3: int32):
    v4: int32 = int.add v2, v3
    return v4
}"#;
    let mut isolate = create_isolate(mir);
    let (continuation, value) = assert_execution_yielded(
        isolate.run_function_by_name_yielding("yieldOnce", &[Value::int32(7)]),
    );
    assert_eq!(value, Value::int32(5));
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(11)));
    assert_eq!(output, Value::int32(18));
}

/// Yield ignores the resume value when no slot is available.
#[test]
fn test_yield_resume_value_ignored() {
    let mir = r#"
function yieldIgnore(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 1int32
    yield v1, b1(v0)
b1(v2: int32, v3: int32):
    return v2
}"#;
    let mut isolate = create_isolate(mir);
    let (continuation, value) = assert_execution_yielded(
        isolate.run_function_by_name_yielding("yieldIgnore", &[Value::int32(9)]),
    );
    assert_eq!(value, Value::int32(1));
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(100)));
    assert_eq!(output, Value::int32(9));
}

/// Yield can suspend multiple times and resume with new values.
#[test]
fn test_yield_multiple() {
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
    let mut isolate = create_isolate(mir);
    let (continuation, value) = assert_execution_yielded(
        isolate.run_function_by_name_yielding("yieldTwice", &[Value::int32(4)]),
    );
    assert_eq!(value, Value::int32(2));
    let (continuation, value) =
        assert_execution_yielded(isolate.resume(continuation, Value::int32(3)));
    assert_eq!(value, Value::int32(7));
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(10)));
    assert_eq!(output, Value::int32(17));
}

/// Yield resumes without explicit resume arguments.
#[test]
fn test_yield_resume_no_args() {
    let mir = r#"
function yieldNoArgs(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 4int32
    yield v1, b1
b1(v2: int32):
    return v2
}"#;
    let mut isolate = create_isolate(mir);
    let (continuation, value) = assert_execution_yielded(
        isolate.run_function_by_name_yielding("yieldNoArgs", &[Value::int32(3)]),
    );
    assert_eq!(value, Value::int32(4));
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(9)));
    assert_eq!(output, Value::int32(9));
}

/// Yield preserves locals across suspension.
#[test]
fn test_yield_preserves_locals() {
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
    let mut isolate = create_isolate(mir);
    let (continuation, value) = assert_execution_yielded(
        isolate.run_function_by_name_yielding("yieldWithLocal", &[Value::int32(1)]),
    );
    assert_eq!(value, Value::int32(4));
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(6)));
    assert_eq!(output, Value::int32(10));
}

/// Yield resumes with explicit arguments and a trailing resume value.
#[test]
fn test_yield_resume_arguments_prefix() {
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
    let mut isolate = create_isolate(mir);
    let (continuation, value) = assert_execution_yielded(
        isolate.run_function_by_name_yielding("yieldPrefix", &[Value::int32(5)]),
    );
    assert_eq!(value, Value::int32(10));
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(7)));
    assert_eq!(output, Value::int32(32));
}

/// Yield clears trailing resume parameters when no argument is provided.
#[test]
fn test_yield_clears_trailing_params() {
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
    let mut isolate = create_isolate(mir);
    let (continuation, value) = assert_execution_yielded(
        isolate.run_function_by_name_yielding("yieldTrailing", &[Value::int32(2)]),
    );
    assert_eq!(value, Value::int32(3));
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(0)));
    assert_eq!(output, Value::int32(0));
}

/// Yield in a nested call resumes back to the caller.
#[test]
fn test_yield_nested_call() {
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
    let mut isolate = create_isolate(mir);
    let (continuation, value) = assert_execution_yielded(
        isolate.run_function_by_name_yielding("outer", &[Value::int32(5)]),
    );
    assert_eq!(value, Value::int32(5));
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(7)));
    assert_eq!(output, Value::int32(13));
}

/// Yield preserves one pending call terminator continuation across suspension.
#[test]
fn test_yield_preserves_call_terminator_continuation() {
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
    call worker(v0): (int32) -> int32 -> b1(v1)
b1(v2: int32, v3: int32):
    v4: int32 = int.add v2, v3
    return v4
}"#;
    let mut isolate = create_isolate(mir);
    let (continuation, value) = assert_execution_yielded(
        isolate.run_function_by_name_yielding("caller", &[Value::int32(7)]),
    );
    assert_eq!(value, Value::int32(5));
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(3)));
    assert_eq!(output, Value::int32(20));
}

/// Yield preserves live frame-local stack memory in the yielded frame.
#[test]
fn test_yield_preserves_frame_alloc_in_current_frame() {
    let mir = r#"
function yieldStackLocal(): int32 {
b0:
    v0: ref<int32, raw, readonly, space(frame)> = frame.alloc int32
    v1: int32 = 1int32
    yield v1, b1
b1(v2: int32):
    return v2
}"#;
    let mut isolate = create_isolate(mir);
    let (continuation, value) =
        assert_execution_yielded(isolate.run_function_by_name_yielding("yieldStackLocal", &[]));
    assert_eq!(value, Value::int32(1));

    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(7)));
    assert_eq!(output, Value::int32(7));
}

/// Yield accepts frame-local stack memory that is not live across suspension.
#[test]
fn test_yield_allows_retired_frame_alloc_in_current_frame() {
    let mir = r#"
function yieldRetiredStackLocal(): int32 {
b0:
    v0: ref<int32, raw, readonly, space(frame)> = frame.alloc int32
    v1: int32 = 1int32
    yield v1, b1
b1(v2: int32):
    return v2
}"#;
    let mut isolate = create_isolate(mir);
    let (_continuation, value) = assert_execution_yielded(
        isolate.run_function_by_name_yielding("yieldRetiredStackLocal", &[]),
    );
    assert_eq!(value, Value::int32(1));
}

/// Yield preserves live frame-local stack memory in suspended caller frames.
#[test]
fn test_yield_preserves_frame_alloc_in_caller_frame() {
    let mir = r#"
function yieldInner(v0: int32): int32 {
b0(v0: int32):
    yield v0, b1
b1(v1: int32):
    return v1
}
function outerWithStackLocal(v0: int32): int32 {
b0(v0: int32):
    v1: ref<int32, raw, readonly, space(frame)> = frame.alloc int32
    v2: int32 = call yieldInner(v0): (int32) -> int32
    return v2
}"#;
    let mut isolate = create_isolate(mir);
    let (continuation, value) = assert_execution_yielded(
        isolate.run_function_by_name_yielding("outerWithStackLocal", &[Value::int32(5)]),
    );
    assert_eq!(value, Value::int32(5));

    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(9)));
    assert_eq!(output, Value::int32(9));
}

/// Yield preserves live frame pointers in the yielded frame.
#[test]
fn test_yield_preserves_frame_pointer_in_current_frame() {
    let mir = r#"
function yieldFramePointer(): int32 {
    local local0: int32, owned
b0:
    v0: int32 = 1int32
    local.set local0, v0
    v1: ref<int32, borrowed, space(frame)> = local.address local0
    v2: int32 = 2int32
    yield v2, b1
b1(v3: int32):
    v4: int32 = load v1
    return v4
}"#;
    let mut isolate = create_isolate(mir);
    let (continuation, value) =
        assert_execution_yielded(isolate.run_function_by_name_yielding("yieldFramePointer", &[]));
    assert_eq!(value, Value::int32(2));

    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(11)));
    assert_eq!(output, Value::int32(1));
}

/// Running a coroutine with the non-yielding entry reports an error.
#[test]
fn test_run_function_rejects_yield() {
    let mir = r#"
function yieldOnce(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 5int32
    yield v1, b1(v0)
b1(v2: int32, v3: int32):
    v4: int32 = int.add v2, v3
    return v4
}"#;
    let mut isolate = create_isolate(mir);
    let result = isolate.run_function_by_name("yieldOnce", &[Value::int32(7)]);
    assert_runtime_error_matches!(result, Error::UnexpectedYield);
}

/// Resume with a continuation from another isolate reports an error.
#[test]
fn test_resume_invalid_continuation() {
    let mir = r#"
function yieldOnce(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 5int32
    yield v1, b1(v0)
b1(v2: int32, v3: int32):
    v4: int32 = int.add v2, v3
    return v4
}"#;
    let mut isolate = create_isolate(mir);
    let (continuation, _value) = assert_execution_yielded(
        isolate.run_function_by_name_yielding("yieldOnce", &[Value::int32(7)]),
    );
    let mut other_isolate = create_isolate_with_id(mir, IsolateId::new(2));
    let result = other_isolate.resume(continuation, Value::int32(0));
    assert_runtime_error_matches!(result, Error::InvalidContinuation);
}

/// Continuations can be cloned for multi-shot resumption.
#[test]
fn test_continuation_clone_for_fork() {
    let mir = r#"
function yieldOnce(): int32 {
b0:
    v0: int32 = 1int32
    yield v0, b1
b1(v1: int32):
    return v1
}"#;
    let mut isolate = create_isolate(mir);
    let (continuation, value) =
        assert_execution_yielded(isolate.run_function_by_name_yielding("yieldOnce", &[]));
    assert_eq!(value, Value::int32(1));
    let forked = continuation
        .clone_for_fork()
        .expect("continuation should fork");
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(5)));
    assert_eq!(output, Value::int32(5));
    let output = assert_execution_completed(isolate.resume(forked, Value::int32(9)));
    assert_eq!(output, Value::int32(9));
}

/// Continuation roots keep heap allocations alive across yields.
#[test]
fn test_continuation_roots_keep_allocations() {
    let mir = r#"
type Pair {
    ref<int32, managed, readonly>;
}

function yieldAlloc(): int32 {
b0:
    v0: ref<int32, managed, readonly> = new int32
    v1: int32 = 1int32
    store v0, v1
    v2: Pair = struct Pair (v0)
    yield v1, b1(v2)
b1(v3: Pair, v4: int32):
    v5: ref<int32, managed, readonly> = field.get v3, 0
    v6: int32 = load v5
    return v6
}"#;
    let mut isolate = create_isolate(mir);
    let (mut continuation, _value) =
        assert_execution_yielded(isolate.run_function_by_name_yielding("yieldAlloc", &[]));
    let stats = isolate.collect_garbage_with_continuations(std::slice::from_mut(&mut continuation));
    assert_eq!(stats.live_allocations, 1);
    let output = assert_execution_completed(isolate.resume(continuation, Value::int32(7)));
    assert_eq!(output, Value::int32(1));
    let stats = isolate.collect_garbage();
    assert_eq!(stats.live_allocations, 0);
}

/// Continuation images keep heap allocations alive across yields.
#[test]
fn test_continuation_image_roots_keep_allocations() {
    let mir = r#"
type Pair {
    ref<int32, managed, readonly>;
}

function yieldAlloc(): int32 {
b0:
    v0: ref<int32, managed, readonly> = new int32
    v1: int32 = 1int32
    store v0, v1
    v2: Pair = struct Pair (v0)
    yield v1, b1(v2)
b1(v3: Pair, v4: int32):
    v5: ref<int32, managed, readonly> = field.get v3, 0
    v6: int32 = load v5
    return v6
}"#;

    let mut isolate = create_isolate(mir);
    let (continuation, _value) =
        assert_execution_yielded(isolate.run_function_by_name_yielding("yieldAlloc", &[]));

    let mut image = isolate
        .isolate
        .continuation_image(&continuation)
        .expect("continuation image should capture");
    let mut roots = crate::RootSet::default();
    isolate
        .isolate
        .visit_image_root_slots(&mut image, &mut |slot| {
            let root = slot.load()?;
            destack_heap::RootSink::push(&mut roots, root);

            Ok(())
        })
        .expect("continuation image roots should collect");
    let mut heap_roots = roots.heap;
    let stats = isolate
        .heap
        .collect_full(&mut heap_roots)
        .expect("heap should collect");

    assert_eq!(stats.live_allocations, 1);
}
