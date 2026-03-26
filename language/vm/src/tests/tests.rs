use destack_heap::{GcStats, Heap, MemoryContext, SharedSpace, Value};
use destack_mir::parse::{ParseOptions, Parser};
use destack_source::FileId;

use crate::diagnostic::{Error, RuntimeResult};
use crate::{
    Continuation, ExecutionOutcome, ExecutionOutput, ExecutionYield, Isolate, IsolateOptions,
};

/// The isolate and authoritative heap used by one test runtime.
pub(crate) struct TestIsolate {
    /// The VM isolate under test.
    pub isolate: Isolate,
    /// The authoritative heap for the isolate.
    pub heap: Heap,
    /// The world-shared memory for the isolate.
    pub shared: SharedSpace,
}

impl TestIsolate {
    /// Build one test isolate from MIR text.
    pub(crate) fn new(mir_text: &str) -> Self {
        let (tree, strings) = Parser::parse(FileId::new(0), mir_text, ParseOptions::default())
            .expect("failed to parse MIR");
        let mut isolate = Isolate::build_with_options(tree, strings, IsolateOptions::test())
            .unwrap_or_else(|error| panic!("failed to initialize isolate: {error}"));
        let mut heap = Heap::new();
        let mut shared = SharedSpace::new();
        let mut memory = MemoryContext::new(&mut heap, &mut shared);

        // initialize isolate state against the authoritative heap
        isolate
            .initialize(&mut memory)
            .unwrap_or_else(|error| panic!("failed to initialize isolate globals: {error}"));

        Self {
            isolate,
            heap,
            shared,
        }
    }

    /// Create one aggregate value on the isolate heap.
    pub(crate) fn allocate_aggregate(&mut self, values: Vec<Value>) -> Value {
        self.isolate.allocate_aggregate(&mut self.heap, values)
    }

    /// Run one callback with the isolate execution memory.
    pub(crate) fn with_memory<R>(
        &mut self,
        run: impl FnOnce(&mut Isolate, &mut MemoryContext<'_>) -> R,
    ) -> R {
        let mut memory = MemoryContext::new(&mut self.heap, &mut self.shared);
        run(&mut self.isolate, &mut memory)
    }

    /// Run one MIR function by name with the given arguments.
    pub(crate) fn run_function_by_name(
        &mut self,
        function: &str,
        arguments: &[Value],
    ) -> RuntimeResult<ExecutionOutput> {
        let mut memory = MemoryContext::new(&mut self.heap, &mut self.shared);
        self.isolate
            .run_function_by_name(&mut memory, function, arguments)
    }

    /// Run one MIR function by name with yield support.
    pub(crate) fn run_function_by_name_yielding(
        &mut self,
        function: &str,
        arguments: &[Value],
    ) -> RuntimeResult<crate::ExecutionOutcome> {
        let mut memory = MemoryContext::new(&mut self.heap, &mut self.shared);
        self.isolate
            .run_function_by_name_yielding(&mut memory, function, arguments)
    }

    /// Resume one yielded continuation.
    pub(crate) fn resume(
        &mut self,
        continuation: Continuation,
        resume_value: Value,
    ) -> RuntimeResult<crate::ExecutionOutcome> {
        let mut memory = MemoryContext::new(&mut self.heap, &mut self.shared);
        self.isolate.resume(&mut memory, continuation, resume_value)
    }

    /// Collect garbage and return one GC summary.
    pub(crate) fn collect_garbage(&mut self) -> GcStats {
        self.isolate
            .collect_garbage(&mut self.heap, &mut self.shared)
    }

    /// Collect garbage with continuation roots and return one GC summary.
    pub(crate) fn collect_garbage_with_continuations(
        &mut self,
        continuations: &[Continuation],
    ) -> GcStats {
        self.isolate.collect_garbage_with_continuations(
            &mut self.heap,
            &mut self.shared,
            continuations,
        )
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

/// Unwrap one runtime result into its VM error.
pub(crate) fn unwrap_runtime_error<T>(result: RuntimeResult<T>) -> Error {
    match result {
        Ok(_) => panic!("expected execution error"),
        Err(error) => error.error,
    }
}

/// Assert that one runtime result failed with the expected VM error.
pub(crate) fn assert_runtime_error<T>(result: RuntimeResult<T>, expected: Error) {
    let error = unwrap_runtime_error(result);

    assert_eq!(error, expected, "unexpected execution error");
}

/// Assert that one runtime result failed with an error matching a pattern.
macro_rules! assert_runtime_error_matches {
    ($result:expr, $pattern:pat $(if $guard:expr)? $(,)?) => {{
        let error = $crate::tests::unwrap_runtime_error($result);

        assert!(
            matches!(error, $pattern $(if $guard)?),
            "unexpected execution error: {error:?}"
        );
    }};
}

pub(crate) use assert_runtime_error_matches;

/// Assert that one execution result yielded.
pub(crate) fn assert_execution_yielded(result: RuntimeResult<ExecutionOutcome>) -> ExecutionYield {
    let outcome = result.expect("execution failed");

    match outcome {
        ExecutionOutcome::Yielded { yielded } => yielded,
        ExecutionOutcome::Completed { .. } => panic!("expected yield"),
    }
}

/// Assert that one execution result completed.
pub(crate) fn assert_execution_completed(
    result: RuntimeResult<ExecutionOutcome>,
) -> ExecutionOutput {
    let outcome = result.expect("execution failed");

    match outcome {
        ExecutionOutcome::Completed { output } => output,
        ExecutionOutcome::Yielded { .. } => panic!("expected completion"),
    }
}

/// Execute one managed nominal allocation and field load.
#[test]
fn test_execute_managed_nominal_field_load() {
    let mir_text = r#"
type @Box = { value: i32 }

function @sumBox(v0: i32) -> i32 {
block0(v0: i32):
    v1: @Box = struct @Box (v0)
    v2: ref<managed readonly @Box> = managed.alloc @Box
    store v2, v1
    v3: @Box = load v2
    v4: i32 = field.get v3, 0
    v5: i32 = iconst 1i32
    v6: i32 = iadd v4, v5
    return v6
}
"#;

    let output = run_mir_ok(mir_text, "sumBox", &[Value::int32(9)]);

    assert_eq!(output.value, Value::int32(10));
}

/// Direct calls preserve managed receiver storage for callee loads.
#[test]
fn test_execute_direct_call_with_managed_receiver_field_load() {
    let mir_text = r#"
type @Box = { value: i32 }

function @readValueClass(v0: i32) -> i32 {
block0(v0: i32):
    v1: @Box = struct @Box (v0)
    v2: ref<managed readonly @Box> = managed.alloc @Box
    store v2, v1
    v3: i32 = call @Box.get(v2) -> fn(ref<managed readonly @Box>) -> i32
    return v3
}

function @Box.get(v0: ref<managed readonly @Box>) -> i32 {
block0(v0: ref<managed readonly @Box>):
    v1: @Box = load v0
    v2: i32 = field.get v1, 0
    return v2
}
"#;

    let output = run_mir_ok(mir_text, "readValueClass", &[Value::int32(9)]);

    assert_eq!(output.value, Value::int32(9));
}

/// Stored function values preserve their function pointer payload through nominal storage.
#[test]
fn test_execute_stored_function_value_roundtrip() {
    let mir_text = r#"
type @Fn = fnvalue<fn() -> i32>
type @Holder = { action: @Fn }

function @target() -> i32 {
block0:
    v0: i32 = iconst 7i32
    return v0
}

function @run() -> i32 {
block0:
    v0: fn() -> i32 = function.addr @target
    v1: u64 = iconst 0u64
    v2: ref?<managed void> = int_to_ptr v1 -> ref?<managed void>
    v3: @Fn = struct @Fn (v0, v2)
    v4: ref<managed readonly @Holder> = managed.alloc @Holder
    v5: @Holder = struct @Holder (v3)
    store v4, v5
    v6: @Holder = load v4
    v7: @Fn = field.get v6, 0
    v8: i32 = call.indirect v7() -> @Fn
    return v8
}
"#;

    let output = run_mir_ok(mir_text, "run", &[]);

    assert_eq!(output.value, Value::int32(7));
}

/// Interface dispatch forwards the concrete object receiver to the selected method.
#[ignore = "raw MIR fixtures cannot declare interface itab metadata"]
#[test]
fn test_execute_interface_call_with_concrete_object_receiver() {
    let mir_text = r#"
type @Greeter = { @object: ref<managed readonly void>, @itab: usize }
type @GreeterImpl = { @vtable: ref<raw addrspace(global) readonly void>, value: i32 }
type @Greeter#object = { greet: fnvalue<fn() -> i32> }

global @GreeterImpl#vtable: [ref?<raw addrspace(global) readonly void>; 3] = zeroinit ; readonly

extern function @Greeter.greet(@Greeter#object) -> i32

function @callInterface(v0: @Greeter) -> i32 {
block0(v0: @Greeter):
    v1: ref<managed readonly void> = field.get v0, 0
    v2: i32 = call.interface v0, @Greeter#object, 1(v1) -> fn(@Greeter#object) -> i32
    return v2
}

function @runInterface() -> i32 {
block0:
    v0: i32 = iconst 41i32
    v1: ref<managed readonly @GreeterImpl> = call @GreeterImpl.constructor(v0) -> fn(i32) -> ref<managed readonly @GreeterImpl>
    v2: ref<managed readonly void> = bitcast v1 -> ref<managed readonly void>
    v3: u64 = iconst 0u64
    v4: usize = bitcast v3 -> usize
    v5: @Greeter = struct @Greeter (v2, v4)
    v6: i32 = call @callInterface(v5) -> fn(@Greeter) -> i32
    return v6
}

function @GreeterImpl.constructor(v0: i32) -> ref<managed readonly @GreeterImpl> {
block0(v0: i32):
    v1: ref<managed readonly @GreeterImpl> = managed.alloc @GreeterImpl
    v2: ref<raw addrspace(global) readonly [ref?<raw addrspace(global) readonly void>; 3]> = global.addr @GreeterImpl#vtable
    v3: ref<raw addrspace(global) readonly void> = bitcast v2 -> ref<raw addrspace(global) readonly void>
    v4: i32 = iconst 0i32
    v5: @GreeterImpl = struct @GreeterImpl (v3, v4)
    store v1, v5
    v6: @GreeterImpl = load v1
    v7: @GreeterImpl = field.set v6, 1, v0
    store v1, v7
    return v1
}

function @GreeterImpl.greet(v0: ref<managed readonly @GreeterImpl>) -> i32 {
block0(v0: ref<managed readonly @GreeterImpl>):
    v1: @GreeterImpl = load v0
    v2: i32 = field.get v1, 1
    v3: i32 = iconst 1i32
    v4: i32 = iadd v2, v3
    return v4
}
"#;

    let output = run_mir_ok(mir_text, "runInterface", &[]);

    assert_eq!(output.value, Value::int32(42));
}
