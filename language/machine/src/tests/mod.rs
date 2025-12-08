mod arithmetic;
mod block;
mod gc;
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

/// Run a MIR function by name with the given arguments.
fn run_mir(mir: &str, function: &str, arguments: &[Value]) -> RuntimeResult<ExecutionOutput> {
    let mut interpreter = create_interpreter(mir);
    interpreter.run_function_by_name(function, arguments)
}

/// Run MIR and expect success, returning the output.
fn run_mir_ok(mir_text: &str, function: &str, args: &[Value]) -> ExecutionOutput {
    run_mir(mir_text, function, args).expect("execution failed")
}

/// Run MIR and expect a specific return value.
fn expect_evaluate_call(mir_text: &str, function: &str, args: &[Value], expected: Value) {
    let output = run_mir_ok(mir_text, function, args);
    assert_eq!(output.value, expected, "unexpected return value");
}

// Alias for backwards compatibility
use expect_evaluate_call as expect_evaluate_mir;
