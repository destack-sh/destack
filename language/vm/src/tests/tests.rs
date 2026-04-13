use destack_core::ImmutableStringPool;
use destack_heap::{GcStats, Heap, MemoryContext, SharedSpace, Value};
use destack_mir::parse::{ParseOptions, Parser};
use destack_mir::{NodeTree, TypeAlias};
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
        let (mut tree, strings) = Parser::parse(FileId::new(0), mir_text, ParseOptions::default())
            .validate()
            .expect("failed to parse MIR");

        // keep raw MIR tests explicit about the well known String contract
        stamp_well_known_string_type_for_tests(&mut tree, &strings);
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

    /// Resolve one function parameter type by name and position.
    pub(crate) fn parameter_type(
        &self,
        function: &str,
        argument_index: usize,
    ) -> destack_mir::LocalNodeId<destack_mir::Type> {
        let function_id = self
            .isolate
            .lookup_function_id(function)
            .unwrap_or_else(|| panic!("missing function '{function}'"));
        let function_node = self.isolate.tree().get(function_id);
        function_node
            .parameters
            .get(argument_index)
            .unwrap_or_else(|| panic!("missing argument {argument_index} for '{function}'"))
            .ty
            .ty()
            .expect("function parameter type should be concrete after validation")
    }

    /// Materialize one typed value for the given MIR type.
    pub(crate) fn materialize_value_for_type(
        &mut self,
        ty: destack_mir::LocalNodeId<destack_mir::Type>,
        values: Vec<Value>,
    ) -> Value {
        self.with_memory(|vm, memory| {
            vm.with_runtime_context(memory, |context| {
                context
                    .materialize_storage_value_for_type(ty, values)
                    .unwrap_or_else(|error| {
                        panic!("failed to materialize typed composite: {error}")
                    })
            })
        })
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
            .expect("failed to collect garbage")
    }

    /// Collect garbage with continuation roots and return one GC summary.
    pub(crate) fn collect_garbage_with_continuations(
        &mut self,
        continuations: &[Continuation],
    ) -> GcStats {
        self.isolate
            .collect_garbage_with_continuations(&mut self.heap, &mut self.shared, continuations)
            .expect("failed to collect garbage")
    }
}

/// Stamp the canonical well known string type for raw MIR test modules when present.
pub(crate) fn stamp_well_known_string_type_for_tests(
    tree: &mut NodeTree,
    strings: &ImmutableStringPool,
) {
    // find the explicit String alias used by VM test MIR fixtures
    let string_type = tree.iter_nodes::<TypeAlias>().find_map(|(_, type_alias)| {
        if strings.get(type_alias.name) == "String" {
            type_alias.ty.ty()
        } else {
            None
        }
    });

    if let Some(string_type) = string_type {
        tree.metadata.layout.set_string_type(string_type);
    }
}

/// Parse MIR text and create one test isolate.
pub(crate) fn create_isolate(mir_text: &str) -> TestIsolate {
    TestIsolate::new(mir_text)
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
type Box {
    value: int32;
}

function sumBox(v0: int32): int32 {
b0(v0: int32):
    v1: Box = struct Box (v0)
    v2: ref<Box, managed, readonly> = managed.alloc Box
    store v2, v1
    v3: Box = load v2
    v4: int32 = field.get v3, 0
    v5: int32 = 1int32
    v6: int32 = int.add v4, v5
    return v6
}"#;

    let output = run_mir_ok(mir_text, "sumBox", &[Value::int32(9)]);

    assert_eq!(output.value, Value::int32(10));
}

/// Direct calls preserve managed receiver storage for callee loads.
#[test]
fn test_execute_direct_call_with_managed_receiver_field_load() {
    let mir_text = r#"
type Box {
    value: int32;
}

function readValueClass(v0: int32): int32 {
b0(v0: int32):
    v1: Box = struct Box (v0)
    v2: ref<Box, managed, readonly> = managed.alloc Box
    store v2, v1
    v3: int32 = call Box.get(v2): (ref<Box, managed, readonly>) -> int32
    return v3
}

function Box.get(v0: ref<Box, managed, readonly>): int32 {
b0(v0: ref<Box, managed, readonly>):
    v1: Box = load v0
    v2: int32 = field.get v1, 0
    return v2
}"#;

    let output = run_mir_ok(mir_text, "readValueClass", &[Value::int32(9)]);

    assert_eq!(output.value, Value::int32(9));
}

/// Stored function values preserve their function pointer payload through nominal storage.
#[test]
fn test_execute_stored_function_value_roundtrip() {
    let mir_text = r#"
type Fn = () -> int32;
type Holder {
    action: Fn;
}

function target(): int32 {
b0:
    v0: int32 = 7int32
    return v0
}

function run(): int32 {
b0:
    v0: Fn = function.address target
    v1: ref<Holder, managed, readonly> = managed.alloc Holder
    v2: Holder = struct Holder (v0)
    store v1, v2
    v3: Holder = load v1
    v4: Fn = field.get v3, 0
    v5: int32 = call.indirect v4(): () -> int32
    return v5
}"#;

    let output = run_mir_ok(mir_text, "run", &[]);

    assert_eq!(output.value, Value::int32(7));
}

/// Interface dispatch forwards the concrete object receiver to the selected method.
#[ignore = "raw MIR fixtures cannot declare interface itab metadata"]
#[test]
fn test_execute_interface_call_with_concrete_object_receiver() {
    let mir_text = r#"
type Greeter {
    object: ref<void, managed, readonly>;
    itab: usize;
}
type GreeterImpl {
    vtable: ref<void, raw, readonly, addressSpace(global)>;
    value: int32;
}
type Greeter#object {
    greet: () => int32;
}

global GreeterImpl#vtable: ref?<void, raw, readonly, addressSpace(global)>[3], readonly = zeroInit

extern function Greeter.greet(Greeter#object): int32

function callInterface(v0: Greeter): int32 {
b0(v0: Greeter):
    v1: ref<void, managed, readonly> = field.get v0, 0
    v2: int32 = call.interface v0, Greeter#object, 1(v1): (Greeter#object) -> int32
    return v2
}

function runInterface(): int32 {
b0:
    v0: int32 = 41int32
    v1: ref<GreeterImpl, managed, readonly> = call GreeterImpl.constructor(v0): (int32) -> ref<GreeterImpl, managed, readonly>
    v2: ref<void, managed, readonly> = cast.bit v1 -> ref<void, managed, readonly>
    v3: uint64 = 0uint64
    v4: usize = cast.bit v3 -> usize
    v5: Greeter = struct Greeter (v2, v4)
    v6: int32 = call callInterface(v5): (Greeter) -> int32
    return v6
}

function GreeterImpl.constructor(v0: int32): ref<GreeterImpl, managed, readonly> {
b0(v0: int32):
    v1: ref<GreeterImpl, managed, readonly> = managed.alloc GreeterImpl
    v2: ref<ref?<void, raw, readonly, addressSpace(global)>[3], raw, readonly, addressSpace(global)> = global.address GreeterImpl#vtable
    v3: ref<void, raw, readonly, addressSpace(global)> = cast.bit v2 -> ref<void, raw, readonly, addressSpace(global)>
    v4: int32 = 0int32
    v5: GreeterImpl = struct GreeterImpl (v3, v4)
    store v1, v5
    v6: GreeterImpl = load v1
    v7: GreeterImpl = field.set v6, 1, v0
    store v1, v7
    return v1
}

function GreeterImpl.greet(v0: ref<GreeterImpl, managed, readonly>): int32 {
b0(v0: ref<GreeterImpl, managed, readonly>):
    v1: GreeterImpl = load v0
    v2: int32 = field.get v1, 1
    v3: int32 = 1int32
    v4: int32 = int.add v2, v3
    return v4
}"#;

    let output = run_mir_ok(mir_text, "runInterface", &[]);

    assert_eq!(output.value, Value::int32(42));
}
