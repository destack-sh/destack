use crate::diagnostic::Error;
use crate::interpreter::ExecutionOutcome;
use crate::memory::Value;

/// Yield returns a value and resumes with the provided argument.
#[test]
fn test_yield_resume_value() {
    // define mir program
    let mir = r#"
function @yield_once(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 5i32
    yield v1, block1(v0)
block1(v2: i32, v3: i32):
    v4 = iadd v2, v3
    return v4
}
"#;

    // create interpreter
    let mut interpreter = super::create_interpreter(mir);

    // start coroutine and capture yield
    let outcome = interpreter
        .run_function_by_name_yielding("yield_once", &[Value::int32(7)])
        .expect("execution failed");

    // verify yielded value
    let (yielded_value, continuation) = match outcome {
        ExecutionOutcome::Yielded { yielded } => (yielded.value, yielded.continuation),
        ExecutionOutcome::Completed { .. } => panic!("expected yield"),
    };
    assert_eq!(yielded_value, Value::int32(5));

    // resume with a value and verify completion
    let outcome = interpreter
        .resume(continuation, Value::int32(11))
        .expect("resume failed");
    let output = match outcome {
        ExecutionOutcome::Completed { output } => output,
        ExecutionOutcome::Yielded { .. } => panic!("expected completion"),
    };
    assert_eq!(output.value, Value::int32(18));
}

/// Yield ignores the resume value when no slot is available.
#[test]
fn test_yield_resume_value_ignored() {
    // define mir program
    let mir = r#"
function @yield_ignore(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    yield v1, block1(v0)
block1(v2: i32):
    return v2
}
"#;

    // create interpreter
    let mut interpreter = super::create_interpreter(mir);

    // start coroutine and capture yield
    let outcome = interpreter
        .run_function_by_name_yielding("yield_ignore", &[Value::int32(9)])
        .expect("execution failed");

    // verify yielded value
    let (yielded_value, continuation) = match outcome {
        ExecutionOutcome::Yielded { yielded } => (yielded.value, yielded.continuation),
        ExecutionOutcome::Completed { .. } => panic!("expected yield"),
    };
    assert_eq!(yielded_value, Value::int32(1));

    // resume and verify the resume value is ignored
    let outcome = interpreter
        .resume(continuation, Value::int32(100))
        .expect("resume failed");
    let output = match outcome {
        ExecutionOutcome::Completed { output } => output,
        ExecutionOutcome::Yielded { .. } => panic!("expected completion"),
    };
    assert_eq!(output.value, Value::int32(9));
}

/// Yield can suspend multiple times and resume with new values.
#[test]
fn test_yield_multiple() {
    // define mir program
    let mir = r#"
function @yield_twice(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 2i32
    yield v1, block1(v0)
block1(v2: i32, v3: i32):
    v4 = iadd v2, v3
    yield v4, block2(v4)
block2(v5: i32, v6: i32):
    v7 = iadd v5, v6
    return v7
}
"#;

    // create interpreter
    let mut interpreter = super::create_interpreter(mir);

    // start coroutine and capture first yield
    let outcome = interpreter
        .run_function_by_name_yielding("yield_twice", &[Value::int32(4)])
        .expect("execution failed");

    // verify first yielded value
    let (yielded_value, continuation) = match outcome {
        ExecutionOutcome::Yielded { yielded } => (yielded.value, yielded.continuation),
        ExecutionOutcome::Completed { .. } => panic!("expected yield"),
    };
    assert_eq!(yielded_value, Value::int32(2));

    // resume for second yield
    let outcome = interpreter
        .resume(continuation, Value::int32(3))
        .expect("resume failed");

    // verify second yielded value
    let (yielded_value, continuation) = match outcome {
        ExecutionOutcome::Yielded { yielded } => (yielded.value, yielded.continuation),
        ExecutionOutcome::Completed { .. } => panic!("expected yield"),
    };
    assert_eq!(yielded_value, Value::int32(7));

    // resume for completion
    let outcome = interpreter
        .resume(continuation, Value::int32(10))
        .expect("resume failed");
    let output = match outcome {
        ExecutionOutcome::Completed { output } => output,
        ExecutionOutcome::Yielded { .. } => panic!("expected completion"),
    };
    assert_eq!(output.value, Value::int32(17));
}

/// Yield resumes without explicit resume arguments.
#[test]
fn test_yield_resume_no_args() {
    // define mir program
    let mir = r#"
function @yield_no_args(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 4i32
    yield v1, block1
block1(v2: i32):
    return v2
}
"#;

    // create interpreter
    let mut interpreter = super::create_interpreter(mir);

    // start coroutine and capture yield
    let outcome = interpreter
        .run_function_by_name_yielding("yield_no_args", &[Value::int32(3)])
        .expect("execution failed");

    // verify yielded value
    let (yielded_value, continuation) = match outcome {
        ExecutionOutcome::Yielded { yielded } => (yielded.value, yielded.continuation),
        ExecutionOutcome::Completed { .. } => panic!("expected yield"),
    };
    assert_eq!(yielded_value, Value::int32(4));

    // resume and verify resumed value is returned
    let outcome = interpreter
        .resume(continuation, Value::int32(9))
        .expect("resume failed");
    let output = match outcome {
        ExecutionOutcome::Completed { output } => output,
        ExecutionOutcome::Yielded { .. } => panic!("expected completion"),
    };
    assert_eq!(output.value, Value::int32(9));
}

/// Yield preserves locals across suspension.
#[test]
fn test_yield_preserves_locals() {
    // define mir program
    let mir = r#"
function @yield_with_local(v0: i32) -> i32 {
    local0: i32 ; owned, mut

block0(v0: i32):
    v1 = iconst 4i32
    local.set local0, v1
    yield v1, block1
block1(v2: i32):
    v3 = local.get local0
    v4 = iadd v3, v2
    return v4
}
"#;

    // create interpreter
    let mut interpreter = super::create_interpreter(mir);

    // start coroutine and capture yield
    let outcome = interpreter
        .run_function_by_name_yielding("yield_with_local", &[Value::int32(1)])
        .expect("execution failed");

    // verify yielded value
    let (yielded_value, continuation) = match outcome {
        ExecutionOutcome::Yielded { yielded } => (yielded.value, yielded.continuation),
        ExecutionOutcome::Completed { .. } => panic!("expected yield"),
    };
    assert_eq!(yielded_value, Value::int32(4));

    // resume and verify local survives
    let outcome = interpreter
        .resume(continuation, Value::int32(6))
        .expect("resume failed");
    let output = match outcome {
        ExecutionOutcome::Completed { output } => output,
        ExecutionOutcome::Yielded { .. } => panic!("expected completion"),
    };
    assert_eq!(output.value, Value::int32(10));
}

/// Yield resumes with explicit arguments and a trailing resume value.
#[test]
fn test_yield_resume_arguments_prefix() {
    // define mir program
    let mir = r#"
function @yield_prefix(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 10i32
    v2 = iconst 20i32
    yield v1, block1(v0, v2)
block1(v3: i32, v4: i32, v5: i32):
    v6 = iadd v3, v4
    v7 = iadd v6, v5
    return v7
}
"#;

    // create interpreter
    let mut interpreter = super::create_interpreter(mir);

    // start coroutine and capture yield
    let outcome = interpreter
        .run_function_by_name_yielding("yield_prefix", &[Value::int32(5)])
        .expect("execution failed");

    // verify yielded value
    let (yielded_value, continuation) = match outcome {
        ExecutionOutcome::Yielded { yielded } => (yielded.value, yielded.continuation),
        ExecutionOutcome::Completed { .. } => panic!("expected yield"),
    };
    assert_eq!(yielded_value, Value::int32(10));

    // resume and verify argument ordering
    let outcome = interpreter
        .resume(continuation, Value::int32(7))
        .expect("resume failed");
    let output = match outcome {
        ExecutionOutcome::Completed { output } => output,
        ExecutionOutcome::Yielded { .. } => panic!("expected completion"),
    };
    assert_eq!(output.value, Value::int32(32));
}

/// Yield clears trailing resume parameters when no argument is provided.
#[test]
fn test_yield_clears_trailing_params() {
    // define mir program
    let mir = r#"
function @yield_trailing(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    v2 = iconst 99i32
    jump block1(v0, v1, v2)
block1(v3: i32, v4: i32, v5: i32):
    v6 = iconst 0i32
    v7 = icmp_eq v4, v6
    branch v7, block3(v5), block2(v3, v4, v5)
block2(v8: i32, v9: i32, v10: i32):
    v11 = iadd v8, v9
    yield v11, block1(v8)
block3(v12: i32):
    return v12
}
"#;

    // create interpreter
    let mut interpreter = super::create_interpreter(mir);

    // start coroutine and capture yield
    let outcome = interpreter
        .run_function_by_name_yielding("yield_trailing", &[Value::int32(2)])
        .expect("execution failed");

    // verify yielded value
    let (yielded_value, continuation) = match outcome {
        ExecutionOutcome::Yielded { yielded } => (yielded.value, yielded.continuation),
        ExecutionOutcome::Completed { .. } => panic!("expected yield"),
    };
    assert_eq!(yielded_value, Value::int32(3));

    // resume with a value that triggers the return path
    let outcome = interpreter
        .resume(continuation, Value::int32(0))
        .expect("resume failed");
    let output = match outcome {
        ExecutionOutcome::Completed { output } => output,
        ExecutionOutcome::Yielded { .. } => panic!("expected completion"),
    };
    assert_eq!(output.value, Value::VOID);
}

/// Yield in a nested call resumes back to the caller.
#[test]
fn test_yield_nested_call() {
    // define mir program
    let mir = r#"
function @yield_inner(v0: i32) -> i32 {
block0(v0: i32):
    yield v0, block1
block1(v1: i32):
    v2 = iconst 1i32
    v3 = iadd v1, v2
    return v3
}

function @outer(v0: i32) -> i32 {
block0(v0: i32):
    v1 = call @yield_inner(v0)
    v2 = iadd v1, v0
    return v2
}
"#;

    // create interpreter
    let mut interpreter = super::create_interpreter(mir);

    // start coroutine and capture yield
    let outcome = interpreter
        .run_function_by_name_yielding("outer", &[Value::int32(5)])
        .expect("execution failed");

    // verify yielded value
    let (yielded_value, continuation) = match outcome {
        ExecutionOutcome::Yielded { yielded } => (yielded.value, yielded.continuation),
        ExecutionOutcome::Completed { .. } => panic!("expected yield"),
    };
    assert_eq!(yielded_value, Value::int32(5));

    // resume and verify completion
    let outcome = interpreter
        .resume(continuation, Value::int32(7))
        .expect("resume failed");
    let output = match outcome {
        ExecutionOutcome::Completed { output } => output,
        ExecutionOutcome::Yielded { .. } => panic!("expected completion"),
    };
    assert_eq!(output.value, Value::int32(13));
}

/// Running a coroutine with the non-yielding entry reports an error.
#[test]
fn test_run_function_rejects_yield() {
    // define mir program
    let mir = r#"
function @yield_once(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 5i32
    yield v1, block1(v0)
block1(v2: i32, v3: i32):
    v4 = iadd v2, v3
    return v4
}
"#;

    // create interpreter
    let mut interpreter = super::create_interpreter(mir);

    // run via the non-yielding entry
    let result = interpreter.run_function_by_name("yield_once", &[Value::int32(7)]);

    // verify unexpected yield error
    let err = result.unwrap_err();
    assert!(matches!(err.error, Error::UnexpectedYield));
}

/// Resume with a continuation from another interpreter reports an error.
#[test]
fn test_resume_invalid_continuation() {
    // define mir program
    let mir = r#"
function @yield_once(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 5i32
    yield v1, block1(v0)
block1(v2: i32, v3: i32):
    v4 = iadd v2, v3
    return v4
}
"#;

    // create interpreter
    let mut interpreter = super::create_interpreter(mir);

    // start coroutine and capture continuation
    let outcome = interpreter
        .run_function_by_name_yielding("yield_once", &[Value::int32(7)])
        .expect("execution failed");
    let (_, continuation) = match outcome {
        ExecutionOutcome::Yielded { yielded } => (yielded.value, yielded.continuation),
        ExecutionOutcome::Completed { .. } => panic!("expected yield"),
    };

    // create a different interpreter
    let mut other_interpreter = super::create_interpreter(mir);

    // resume on a different interpreter
    let result = other_interpreter.resume(continuation, Value::int32(0));

    // validate error
    let err = result.unwrap_err();
    assert!(matches!(err.error, Error::InvalidContinuation));
}

/// Continuations can be cloned for multi-shot resumption.
#[test]
fn test_continuation_clone_for_fork() {
    // define mir program
    let mir = r#"
function @yield_once() -> i32 {
block0:
    v0 = iconst 1i32
    yield v0, block1
block1(v1: i32):
    return v1
}
"#;

    // create interpreter
    let mut interpreter = super::create_interpreter(mir);

    // start coroutine and capture continuation
    let outcome = interpreter
        .run_function_by_name_yielding("yield_once", &[])
        .expect("execution failed");
    let (yielded_value, continuation) = match outcome {
        ExecutionOutcome::Yielded { yielded } => (yielded.value, yielded.continuation),
        ExecutionOutcome::Completed { .. } => panic!("expected yield"),
    };
    assert_eq!(yielded_value, Value::int32(1));

    // clone the continuation for a forked resume
    let forked = continuation.clone_for_fork();

    // resume the original continuation
    let outcome = interpreter
        .resume(continuation, Value::int32(5))
        .expect("resume failed");
    let output = match outcome {
        ExecutionOutcome::Completed { output } => output,
        ExecutionOutcome::Yielded { .. } => panic!("expected completion"),
    };
    assert_eq!(output.value, Value::int32(5));

    // resume the forked continuation
    let outcome = interpreter
        .resume(forked, Value::int32(9))
        .expect("resume failed");
    let output = match outcome {
        ExecutionOutcome::Completed { output } => output,
        ExecutionOutcome::Yielded { .. } => panic!("expected completion"),
    };
    assert_eq!(output.value, Value::int32(9));
}

/// Continuation roots keep managed allocations alive across yields.
#[test]
fn test_continuation_roots_keep_allocations() {
    // define mir program
    let mir = r#"
type @Pair = { i32, i32 }

function @yield_alloc() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = struct @Pair (v0, v1)
    yield v0, block1(v2)
block1(v3: @Pair, v4: i32):
    return v4
}
"#;

    // create interpreter
    let mut interpreter = super::create_interpreter(mir);

    // start coroutine and capture continuation
    let outcome = interpreter
        .run_function_by_name_yielding("yield_alloc", &[])
        .expect("execution failed");
    let (_, continuation) = match outcome {
        ExecutionOutcome::Yielded { yielded } => (yielded.value, yielded.continuation),
        ExecutionOutcome::Completed { .. } => panic!("expected yield"),
    };

    // collect garbage while continuation is suspended
    let stats = interpreter.collect_garbage_with_continuations(std::slice::from_ref(&continuation));
    assert_eq!(stats.live_cells, 1);

    // resume and complete the coroutine
    let outcome = interpreter
        .resume(continuation, Value::int32(7))
        .expect("resume failed");
    let output = match outcome {
        ExecutionOutcome::Completed { output } => output,
        ExecutionOutcome::Yielded { .. } => panic!("expected completion"),
    };
    assert_eq!(output.value, Value::int32(7));

    // collect garbage after completion
    let stats = interpreter.collect_garbage();
    assert_eq!(stats.live_cells, 0);
}
