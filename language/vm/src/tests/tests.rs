use destack_heap::{GcStats, Heap, ManagedHeap, RawHeap, Value};
use destack_mir::parse::{ParseOptions, Parser};
use destack_source::FileId;

use crate::diagnostic::{Error, RuntimeResult};
use crate::{Continuation, ExecutionOutput, Isolate, IsolateOptions};

/// The isolate and authoritative heap used by one test runtime.
pub(crate) struct TestIsolate {
    /// The VM isolate under test.
    pub isolate: Isolate,
    /// The authoritative heap for the isolate.
    pub heap: Heap,
}

impl TestIsolate {
    /// Build one test isolate from MIR text.
    pub(crate) fn new(mir_text: &str) -> Self {
        let (tree, strings) = Parser::parse(FileId::new(0), mir_text, ParseOptions::default())
            .expect("failed to parse MIR");
        let mut isolate = Isolate::build_with_options(tree, strings, IsolateOptions::test())
            .unwrap_or_else(|error| panic!("failed to initialize isolate: {error}"));
        let heap = Heap::new(ManagedHeap::new(), RawHeap::new());
        let mut heap = heap;

        // initialize isolate state against the authoritative heap
        isolate
            .initialize(&mut heap)
            .unwrap_or_else(|error| panic!("failed to initialize isolate globals: {error}"));

        Self { isolate, heap }
    }

    /// Create one aggregate value on the isolate heap.
    pub(crate) fn allocate_aggregate(&mut self, values: Vec<Value>) -> Value {
        self.isolate.allocate_aggregate(&mut self.heap, values)
    }

    /// Run one MIR function by name with the given arguments.
    pub(crate) fn run_function_by_name(
        &mut self,
        function: &str,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutput> {
        self.isolate
            .run_function_by_name(&mut self.heap, function, arguments)
    }

    /// Collect garbage and return one GC summary.
    pub(crate) fn collect_garbage(&mut self) -> GcStats {
        self.isolate.collect_garbage(&mut self.heap)
    }

    /// Collect garbage with continuation roots and return one GC summary.
    pub(crate) fn collect_garbage_with_continuations(
        &mut self,
        continuations: &[Continuation],
    ) -> GcStats {
        self.isolate
            .collect_garbage_with_continuations(&mut self.heap, continuations)
    }
}

/// Parse MIR text and create one test isolate.
pub(crate) fn create_isolate(mir_text: &str) -> TestIsolate {
    TestIsolate::new(mir_text)
}

/// Create one aggregate value on the isolate heap.
pub(crate) fn create_aggregate(isolate: &mut TestIsolate, values: Vec<Value>) -> Value {
    isolate.allocate_aggregate(values)
}

/// Run one MIR function by name with the given arguments.
pub(crate) fn run_mir(
    mir: &str,
    function: &str,
    arguments: &[Value],
) -> RuntimeResult<ExecutionOutput> {
    let mut isolate = create_isolate(mir);

    isolate.run_function_by_name(function, arguments)
}

/// Run MIR with access to one heap-owning isolate before execution.
pub(crate) fn run_mir_with<F>(
    mir_text: &str,
    function: &str,
    setup: F,
) -> RuntimeResult<ExecutionOutput>
where
    F: FnOnce(&mut TestIsolate) -> Vec<Value>,
{
    let mut isolate = create_isolate(mir_text);
    let args = setup(&mut isolate);

    isolate.run_function_by_name(function, &args)
}

/// Run MIR with setup, expecting success.
pub(crate) fn run_mir_with_ok<F>(mir_text: &str, function: &str, setup: F) -> ExecutionOutput
where
    F: FnOnce(&mut TestIsolate) -> Vec<Value>,
{
    run_mir_with(mir_text, function, setup).expect("execution failed")
}

/// Run MIR and expect success, returning the output.
pub(crate) fn run_mir_ok(mir_text: &str, function: &str, args: &[Value]) -> ExecutionOutput {
    run_mir(mir_text, function, args).expect("execution failed")
}

/// Run MIR and expect one specific return value.
pub(crate) fn run_mir_expect(mir_text: &str, function: &str, args: &[Value], expected: Value) {
    let output = run_mir_ok(mir_text, function, args);

    assert_eq!(output.value, expected, "unexpected return value");
}

/// Run MIR and expect one specific runtime error.
pub(crate) fn run_mir_expect_error(
    mir_text: &str,
    function: &str,
    args: &[Value],
    expected: Error,
) {
    let error = run_mir(mir_text, function, args).expect_err("expected execution error");

    assert_eq!(error.error, expected, "unexpected execution error");
}
