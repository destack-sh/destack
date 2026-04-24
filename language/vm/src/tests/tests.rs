use std::sync::Arc;

use destack_engine::MaterializedValue;
use destack_heap::{Allocator, GcStats, Heap, HeapLimits, HeapOptions, SharedHeapLimits};

use crate::SharedHeap;
use destack_mir::parse::{ParseOptions, Parser};
use destack_mir::{LayoutTable, Storage};
use destack_source::FileId;

use crate::diagnostic::{Error, RuntimeResult};
use crate::{Continuation, Isolate, IsolateOptions, RunOutcome, RunOutput, Value};

/// The isolate and authoritative heap used by one test runtime.
pub(crate) struct TestIsolate {
    /// The VM isolate under test.
    pub isolate: Isolate,
    /// The authoritative heap for the isolate.
    pub heap: Heap,
    /// The world-shared heap for the isolate.
    pub shared: SharedHeap,
}

/// Create one local test heap over explicit layouts.
pub(crate) fn create_test_heap(layouts: Arc<LayoutTable>) -> Heap {
    let options = HeapOptions::local();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_arena_bytes)
            .expect("test allocator should build"),
    );

    Heap::with_allocator_limits_layouts_and_options(
        allocator,
        layouts,
        HeapLimits::default(),
        options,
    )
    .expect("test heap should build")
}

/// Create one shared test heap over explicit layouts.
pub(crate) fn create_test_shared_heap(layouts: Arc<LayoutTable>) -> SharedHeap {
    let options = HeapOptions::shared();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_arena_bytes)
            .expect("test allocator should build"),
    );

    SharedHeap::with_allocator_limits_layouts_and_options(
        allocator,
        layouts,
        SharedHeapLimits::default(),
        options,
    )
    .expect("test shared heap should build")
}

/// Create one empty local test heap.
pub(crate) fn create_empty_test_heap() -> Heap {
    create_test_heap(Arc::new(LayoutTable::new()))
}

/// Create one empty shared test heap.
pub(crate) fn create_empty_test_shared_heap() -> SharedHeap {
    create_test_shared_heap(Arc::new(LayoutTable::new()))
}

impl TestIsolate {
    /// Build one test isolate from MIR text.
    pub(crate) fn new(mir_text: &str) -> Self {
        let (tree, strings) = Parser::parse(FileId::new(0), mir_text, ParseOptions::default())
            .validate()
            .expect("failed to parse MIR");

        let mut isolate = Isolate::build_with_options(tree, strings, IsolateOptions::test())
            .unwrap_or_else(|error| panic!("failed to initialize isolate: {error}"));
        let layouts = Arc::new(isolate.layout_table().clone());
        let mut heap = create_test_heap(layouts.clone());
        let mut shared = create_test_shared_heap(layouts);

        // initialize isolate state against the authoritative heap
        isolate
            .initialize(&mut heap, &mut shared)
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
            .function_id_by_name(function)
            .unwrap_or_else(|_| panic!("missing function '{function}'"));
        let function_node = self.isolate.tree().get(function_id);
        function_node
            .parameters
            .get(argument_index)
            .unwrap_or_else(|| panic!("missing argument {argument_index} for '{function}'"))
            .ty
            .ty()
            .expect("function parameter type should be concrete after validation")
    }

    /// Materialize one value for the given MIR type.
    pub(crate) fn materialize_value_for_type(
        &mut self,
        ty: destack_mir::LocalNodeId<destack_mir::Type>,
        values: Vec<Value>,
    ) -> Value {
        self.with_heaps(|vm, heap, shared| {
            vm.with_runtime_context(heap, shared, Default::default(), |context| {
                context
                    .materialize_storage_value(ty, values)
                    .unwrap_or_else(|error| {
                        panic!("failed to materialize typed composite: {error}")
                    })
            })
        })
    }

    /// Run one callback with the isolate heaps.
    pub(crate) fn with_heaps<R>(
        &mut self,
        run: impl FnOnce(&mut Isolate, &mut Heap, &SharedHeap) -> R,
    ) -> R {
        run(&mut self.isolate, &mut self.heap, &mut self.shared)
    }

    /// Run one MIR function by name with the given arguments.
    pub(crate) fn run_function_by_name(
        &mut self,
        function: &str,
        arguments: &[Value],
    ) -> RuntimeResult<RunOutput> {
        self.isolate
            .run_function_by_name(&mut self.heap, &mut self.shared, function, arguments)
    }

    /// Run one MIR function by name with yield support.
    pub(crate) fn run_function_by_name_yielding(
        &mut self,
        function: &str,
        arguments: &[Value],
    ) -> RuntimeResult<crate::RunOutcome> {
        self.isolate.run_function_by_name_yielding(
            &mut self.heap,
            &mut self.shared,
            function,
            arguments,
        )
    }

    /// Resume one yielded continuation.
    pub(crate) fn resume(
        &mut self,
        continuation: Continuation,
        resume_value: Value,
    ) -> RuntimeResult<crate::RunOutcome> {
        let resume_value = resume_value
            .try_into()
            .expect("test resume value should materialize");

        self.isolate
            .resume(&mut self.heap, &mut self.shared, continuation, resume_value)
    }

    /// Collect garbage and return one GC summary.
    pub(crate) fn collect_garbage(&mut self) -> GcStats {
        self.collect_garbage_with_continuations(&[])
    }

    /// Collect garbage with continuation roots and return one GC summary.
    pub(crate) fn collect_garbage_with_continuations(
        &mut self,
        continuations: &[Continuation],
    ) -> GcStats {
        let roots = self
            .isolate
            .root_set(continuations)
            .expect("failed to collect root set");
        let mut heap_roots = roots.heap;
        let mut stats = self
            .heap
            .collect_full(&mut heap_roots)
            .expect("failed to collect heap");
        let shared_stats = self
            .shared
            .collect_full(roots.shared.iter().copied())
            .expect("failed to collect shared heap");

        stats.freed_allocations += shared_stats.freed_allocations;
        stats.live_allocations += shared_stats.live_allocations;
        stats.freed_bytes += shared_stats.freed_bytes;
        stats.allocated_bytes += shared_stats.allocated_bytes;
        stats.active_bytes += shared_stats.active_bytes;

        stats
    }
}

/// Parse MIR text and create one test isolate.
pub(crate) fn create_isolate(mir_text: &str) -> TestIsolate {
    TestIsolate::new(mir_text)
}

/// Parse MIR text and create one test isolate with explicit storage metadata.
pub(crate) fn create_isolate_with_storage(mir_text: &str, storage: Storage) -> TestIsolate {
    let (tree, strings) = Parser::parse(
        FileId::new(0),
        mir_text,
        ParseOptions {
            pointer_bytes: storage.native_pointer_bytes,
        },
    )
    .validate()
    .expect("failed to parse MIR");

    // keep the helper honest: parse must produce the requested layout directly
    assert_eq!(tree.metadata.layout.storage, storage);

    let mut isolate = Isolate::build_with_options(tree, strings, IsolateOptions::test())
        .unwrap_or_else(|error| panic!("failed to initialize isolate: {error}"));
    let layouts = Arc::new(isolate.layout_table().clone());
    let mut heap = create_test_heap(layouts.clone());
    let mut shared = create_test_shared_heap(layouts);

    // initialize isolate state against the authoritative heap
    isolate
        .initialize(&mut heap, &mut shared)
        .unwrap_or_else(|error| panic!("failed to initialize isolate globals: {error}"));

    TestIsolate {
        isolate,
        heap,
        shared,
    }
}

/// Run one MIR function by name with the given arguments.
pub(crate) fn run_mir(mir: &str, function: &str, arguments: &[Value]) -> RuntimeResult<RunOutput> {
    let mut isolate = create_isolate(mir);

    isolate.run_function_by_name(function, arguments)
}

/// Run MIR with access to one heap-owning isolate before execution.
pub(crate) fn run_mir_with<F>(mir_text: &str, function: &str, setup: F) -> RuntimeResult<RunOutput>
where
    F: FnOnce(&mut TestIsolate) -> Vec<Value>,
{
    let mut isolate = create_isolate(mir_text);
    let args = setup(&mut isolate);

    isolate.run_function_by_name(function, &args)
}

/// Run MIR with setup, expecting success.
pub(crate) fn run_mir_with_ok<F>(mir_text: &str, function: &str, setup: F) -> RunOutput
where
    F: FnOnce(&mut TestIsolate) -> Vec<Value>,
{
    run_mir_with(mir_text, function, setup).expect("execution failed")
}

/// Run MIR and expect success, returning the output.
pub(crate) fn run_mir_ok(mir_text: &str, function: &str, args: &[Value]) -> RunOutput {
    run_mir(mir_text, function, args).expect("execution failed")
}

/// Run MIR and expect one specific return value.
pub(crate) fn run_mir_expect(mir_text: &str, function: &str, args: &[Value], expected: Value) {
    let output = run_mir_ok(mir_text, function, args);

    assert_materialized_plain_eq(&output.value, expected);
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
pub(crate) fn assert_execution_yielded(
    result: RuntimeResult<RunOutcome>,
) -> (Continuation, MaterializedValue) {
    let outcome = result.expect("execution failed");

    match outcome {
        RunOutcome::Yielded {
            continuation,
            value,
        } => (continuation, value),
        RunOutcome::Completed { .. } => panic!("expected yield"),
    }
}

/// Return one plain runtime value from one materialized boundary value.
pub(crate) fn assert_materialized_plain(value: &MaterializedValue) -> Value {
    match value {
        MaterializedValue::Void => Value::VOID,
        MaterializedValue::Bool(value) => Value::bool(*value),
        MaterializedValue::Int { value, width } => Value::int(*value, *width),
        MaterializedValue::UInt { value, width } => Value::uint(*value, *width),
        MaterializedValue::Float32 { bits } => Value::float32(f32::from_bits(*bits)),
        MaterializedValue::Float64 { bits } => Value::float64(f64::from_bits(*bits)),
        MaterializedValue::Char(value) => Value::char(*value),
        MaterializedValue::HeapReference(reference) => Value::heap_reference(*reference),
        MaterializedValue::SharedHeapReference(reference) => {
            Value::shared_heap_reference(*reference)
        }
        MaterializedValue::RawPointer(pointer) => Value::raw_pointer(*pointer),
        MaterializedValue::SharedRawPointer(pointer) => Value::shared_raw_pointer(*pointer),

        MaterializedValue::Undefined
        | MaterializedValue::Aggregate { .. }
        | MaterializedValue::FrameAddress(_)
        | MaterializedValue::GlobalAddress(_)
        | MaterializedValue::Function(_) => {
            panic!("expected plain materialized value, got {value:?}")
        }
    }
}

/// Assert one materialized boundary value equals one plain runtime value.
pub(crate) fn assert_materialized_plain_eq(value: &MaterializedValue, expected: Value) {
    let value = assert_materialized_plain(value);

    assert_eq!(value, expected, "unexpected materialized value");
}

/// Assert that one execution result completed.
pub(crate) fn assert_execution_completed(result: RuntimeResult<RunOutcome>) -> RunOutput {
    let outcome = result.expect("execution failed");

    match outcome {
        RunOutcome::Completed { output } => output,
        RunOutcome::Yielded { .. } => panic!("expected completion"),
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
    v2: ref<Box, managed, readonly> = new Box
    store v2, v1
    v3: Box = load v2
    v4: int32 = field.get v3, 0
    v5: int32 = 1int32
    v6: int32 = int.add v4, v5
    return v6
}"#;

    let output = run_mir_ok(mir_text, "sumBox", &[Value::int32(9)]);

    assert_eq!(assert_materialized_plain(&output.value), Value::int32(10));
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
    v2: ref<Box, managed, readonly> = new Box
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

    assert_eq!(assert_materialized_plain(&output.value), Value::int32(9));
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
    v1: ref<Holder, managed, readonly> = new Holder
    v2: Holder = struct Holder (v0)
    store v1, v2
    v3: Holder = load v1
    v4: Fn = field.get v3, 0
    v5: int32 = call.indirect v4(): () -> int32
    return v5
}"#;

    let output = run_mir_ok(mir_text, "run", &[]);

    assert_eq!(assert_materialized_plain(&output.value), Value::int32(7));
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
    v1: ref<GreeterImpl, managed, readonly> = new GreeterImpl
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

    assert_eq!(assert_materialized_plain(&output.value), Value::int32(42));
}
