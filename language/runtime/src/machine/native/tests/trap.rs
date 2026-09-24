use destack_native::abi;
use destack_repository::RuntimeOptions;

use crate::binding::BindingTable;
use crate::diagnostic::RuntimeError;
use crate::tests::{TestProgram, TestWorker};

use super::super::{Error, Loader, Platform};

/// Report a native integer division by zero as a trap.
#[test]
fn test_trap_on_native_division_by_zero() {
    let error = run_native(
        r#"
export function task(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = div v0, v1
    return v2
}
"#,
    );

    assert_eq!(error, trapped(abi::Trap::DivisionByZero));
}

/// Report unbounded native recursion as a stack overflow trap.
#[test]
fn test_trap_on_native_stack_overflow() {
    let error = run_native(
        r#"
function descend(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call descend(v0): (int32) => int32
    v2: int32 = add v1, v0
    return v2
}

export function task(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call descend(v0): (int32) => int32
    return v1
}
"#,
    );

    assert_eq!(error, trapped(abi::Trap::StackOverflow));
}

/// Report a stack overflow in a callee whose frame exceeds the stack its recursive caller left.
#[test]
fn test_trap_on_a_callee_frame_beyond_the_native_stack() {
    let error = run_native(
        r#"
function spill(v0: int32): int32 {
    local l0: [int32; 32768]

entry(v0: int32):
    return v0
}

function descend(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call spill(v0): (int32) => int32
    v2: int32 = call descend(v1): (int32) => int32
    v3: int32 = add v2, v0
    return v3
}

export function task(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call descend(v0): (int32) => int32
    return v1
}
"#,
    );

    assert_eq!(error, trapped(abi::Trap::StackOverflow));
}

/// Run one native MIR program's task and return its failure.
fn run_native(source: &str) -> RuntimeError {
    let program = TestProgram::mir(source).compile_native().build();
    let code = Platform
        .load(&program)
        .expect("native test program should load");
    let mut worker = TestWorker::native(
        &RuntimeOptions::default(),
        program,
        BindingTable::new(),
        code,
    );

    *worker
        .run_entrypoint("task", 1)
        .expect_err("native task should trap")
}

/// Return the runtime failure for one native trap.
fn trapped(trap: abi::Trap) -> RuntimeError {
    RuntimeError::Native(Box::new(Error::Trapped { trap }))
}
