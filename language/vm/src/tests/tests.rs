use destack_mir::parse::Parser;

use crate::diagnostic::RuntimeResult;
use crate::memory::Value;
use crate::{ExecutionOutput, Isolate, IsolateOptions};

/// Parse MIR text and create an isolate.
pub(crate) fn create_isolate(mir_text: &str) -> Isolate {
    let (tree, strings) = Parser::parse(mir_text).expect("failed to parse MIR");
    Isolate::with_options(tree, strings, IsolateOptions::test())
}

/// Create an aggregate value on the isolate heap.
pub(crate) fn create_aggregate(isolate: &mut Isolate, values: Vec<Value>) -> Value {
    isolate.allocate_aggregate(values)
}

/// Run a MIR function by name with the given arguments.
pub(crate) fn run_mir(
    mir: &str,
    function: &str,
    arguments: &[Value],
) -> RuntimeResult<ExecutionOutput> {
    let mut isolate = create_isolate(mir);
    isolate.run_function_by_name(function, arguments)
}

/// Run MIR with access to interpreter (for creating aggregates before execution).
pub(crate) fn run_mir_with<F>(
    mir_text: &str,
    function: &str,
    setup: F,
) -> RuntimeResult<ExecutionOutput>
where
    F: FnOnce(&mut Isolate) -> Vec<Value>,
{
    let mut isolate = create_isolate(mir_text);
    let args = setup(&mut isolate);
    isolate.run_function_by_name(function, &args)
}

/// Run MIR with interpreter setup, expecting success.
pub(crate) fn run_mir_with_ok<F>(mir_text: &str, function: &str, setup: F) -> ExecutionOutput
where
    F: FnOnce(&mut Isolate) -> Vec<Value>,
{
    run_mir_with(mir_text, function, setup).expect("execution failed")
}

/// Run MIR and expect success, returning the output.
pub(crate) fn run_mir_ok(mir_text: &str, function: &str, args: &[Value]) -> ExecutionOutput {
    run_mir(mir_text, function, args).expect("execution failed")
}

/// Run MIR and expect a specific return value.
pub(crate) fn run_mir_expect(mir_text: &str, function: &str, args: &[Value], expected: Value) {
    let output = run_mir_ok(mir_text, function, args);
    assert_eq!(output.value, expected, "unexpected return value");
}
