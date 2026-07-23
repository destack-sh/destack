use destack_program::Word;

use super::{TestMachine, TestProgram};

/// Capture register-backed frame state when bytecode yields.
#[test]
fn test_execute_yield() {
    let mut machine = TestMachine::parse(
        r#"
type Value

export function suspend(r0: int32) resume(int32): int32 {
    slot s0: Value = r0[1]

    r1: int32 = yield r0 => l0 | l1

l0:
    return r1

l1:
    unwind.resume
}
"#,
        TestProgram::new(),
    );

    let (continuation, value) = machine.run_to_yield("suspend", &[Word::int32(73)]);

    assert_eq!(value, vec![Word::int32(73)]);
    assert_eq!(continuation.frames.len(), 1);
    assert_eq!(
        continuation
            .frame_bytes(&continuation.frames[0])
            .expect("captured frame bytes"),
        &73u64.to_le_bytes()
    );

    // resume independent forks from the same canonical continuation bytes
    let first = machine.resume_to_completion(continuation.fork(), &[Word::int32(9)]);
    assert_eq!(first, vec![Word::int32(9)]);

    let second = machine.resume_to_completion(continuation, &[Word::int32(12)]);
    assert_eq!(second, vec![Word::int32(12)]);
}

/// Restore a suspended call chain and return through every caller frame.
#[test]
fn test_resume_nested_yield() {
    let mut machine = TestMachine::parse(
        r#"
type Value

function suspend(r0: int32) resume(int32): int32 {
    slot s0: Value = r0[1]

    r1: int32 = yield r0 => l0 | l1

l0:
    return r1

l1:
    unwind.resume
}

export function calculate(r0: int32): int32 {
    slot s0: Value = r0[1]

    r1: int32 = call suspend(r0)
    r2: int32 = int.add r0, r1
    return r2
}
"#,
        TestProgram::new(),
    );

    // suspend with both the caller and callee retained
    let (continuation, value) = machine.run_to_yield("calculate", &[Word::int32(5)]);
    assert_eq!(value, vec![Word::int32(5)]);
    assert_eq!(continuation.frames.len(), 2);

    // resume the callee and return its result through the restored caller
    let value = machine.resume_to_completion(continuation, &[Word::int32(7)]);
    assert_eq!(value, vec![Word::int32(12)]);
}
