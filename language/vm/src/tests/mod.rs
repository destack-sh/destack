mod arithmetic;
mod block;
mod cast;
mod gc;
mod global;
mod intrinsic;
mod memory;

use destack_mir::parse::Parser;

use crate::diagnostic::RuntimeResult;
use crate::interpreter::{ExecutionOutput, Interpreter, MachineOptions};
use crate::memory::Value;

/// Parse MIR text and create an interpreter.
fn create_interpreter(mir_text: &str) -> Interpreter {
    let (tree, strings) = Parser::parse(mir_text).expect("failed to parse MIR");
    Interpreter::with_options(tree, strings, MachineOptions::test())
}

/// Create an aggregate value on the interpreter's heap.
fn create_aggregate(interpreter: &mut Interpreter, values: Vec<Value>) -> Value {
    interpreter.allocate_aggregate(values)
}

/// Run a MIR function by name with the given arguments.
fn run_mir(mir: &str, function: &str, arguments: &[Value]) -> RuntimeResult<ExecutionOutput> {
    let mut interpreter = create_interpreter(mir);
    interpreter.run_function_by_name(function, arguments)
}

/// Run MIR with access to interpreter (for creating aggregates before execution).
fn run_mir_with<F>(mir_text: &str, function: &str, setup: F) -> RuntimeResult<ExecutionOutput>
where
    F: FnOnce(&mut Interpreter) -> Vec<Value>,
{
    let mut interpreter = create_interpreter(mir_text);
    let args = setup(&mut interpreter);
    interpreter.run_function_by_name(function, &args)
}

/// Run MIR with interpreter setup, expecting success.
fn run_mir_with_ok<F>(mir_text: &str, function: &str, setup: F) -> ExecutionOutput
where
    F: FnOnce(&mut Interpreter) -> Vec<Value>,
{
    run_mir_with(mir_text, function, setup).expect("execution failed")
}

/// Run MIR and expect success, returning the output.
fn run_mir_ok(mir_text: &str, function: &str, args: &[Value]) -> ExecutionOutput {
    run_mir(mir_text, function, args).expect("execution failed")
}

/// Run MIR and expect a specific return value.
fn run_mir_expect(mir_text: &str, function: &str, args: &[Value], expected: Value) {
    let output = run_mir_ok(mir_text, function, args);
    assert_eq!(output.value, expected, "unexpected return value");
}
