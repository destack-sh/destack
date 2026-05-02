use std::sync::Arc;

use destack_engine::{StaticSpace, Value};
use destack_heap::{
    Allocator, GcStats, Heap, HeapLimits, HeapOptions, SharedAllocator, SharedGcWorker,
    SharedHeapLimits,
};

use crate::SharedHeap;
use destack_mir::Storage;
use destack_mir::parse::{ParseOptions, Parser};
use destack_source::FileId;

use crate::diagnostic::{Error, RuntimeResult};
use crate::{Continuation, Isolate, IsolateId, IsolateOptions, Outcome, RootSet, Word};

/// The isolate and authoritative heap used by one test runtime.
pub(crate) struct TestIsolate {
    /// The VM isolate under test.
    pub isolate: Isolate,
    /// The worker static space used by the isolate.
    pub statics: StaticSpace,
    /// The authoritative heap for the isolate.
    pub heap: Heap,
    /// The world-shared heap for the isolate.
    pub shared_heap: SharedHeap,
    /// The shared collector worker used by this isolate.
    pub shared_gc: SharedGcWorker,
    /// The worker-local shared heap allocator.
    pub shared_allocator: SharedAllocator,
}

/// Create one local test heap.
pub(crate) fn create_test_heap() -> Heap {
    let options = HeapOptions::local();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("test allocator should build"),
    );

    Heap::with_allocator_limits_and_options(allocator, HeapLimits::default(), options)
        .expect("test heap should build")
}

/// Create one shared test heap.
pub(crate) fn create_test_shared_heap() -> SharedHeap {
    let options = HeapOptions::shared();
    let allocator = Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("test allocator should build"),
    );

    SharedHeap::with_allocator_limits_and_options(allocator, SharedHeapLimits::default(), options)
        .expect("test shared heap should build")
}

impl TestIsolate {
    /// Build one test isolate from MIR text.
    pub(crate) fn new(mir_text: &str) -> Self {
        Self::with_id(mir_text, IsolateId::new(1))
    }

    /// Build one test isolate from MIR text with one explicit isolate id.
    pub(crate) fn with_id(mir_text: &str, isolate_id: IsolateId) -> Self {
        let (tree, strings) = Parser::parse(FileId::new(0), mir_text, ParseOptions::default())
            .validate()
            .expect("failed to parse MIR");

        let mut isolate =
            Isolate::build_with_options(isolate_id, tree, strings, IsolateOptions::test())
                .unwrap_or_else(|error| panic!("failed to initialize isolate: {error}"));
        let mut statics = StaticSpace::empty();
        let heap = create_test_heap();
        let shared_heap = create_test_shared_heap();
        let shared_gc = shared_heap.register_collector_worker();
        let shared_allocator = shared_heap.allocator();
        isolate
            .initialize(&heap, &shared_heap, &mut statics)
            .unwrap_or_else(|error| panic!("failed to initialize isolate globals: {error}"));

        Self {
            isolate,
            statics,
            heap,
            shared_heap,
            shared_gc,
            shared_allocator,
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
        values: Vec<Word>,
    ) -> Word {
        self.with_heaps(|vm, heap, shared| {
            vm.with_runtime_context(heap, shared, Default::default(), |context| {
                context.materialize_heap_value(ty, values)
            })
            .unwrap_or_else(|error| panic!("failed to materialize heap value: {error}"))
        })
    }

    /// Run one callback with the isolate heaps.
    pub(crate) fn with_heaps<R>(
        &mut self,
        run: impl FnOnce(&mut Isolate, &mut Heap, &SharedHeap) -> R,
    ) -> R {
        run(&mut self.isolate, &mut self.heap, &mut self.shared_heap)
    }

    /// Run one MIR function by name with the given arguments.
    pub(crate) fn run_function_by_name(
        &mut self,
        function: &str,
        arguments: &[Value],
    ) -> RuntimeResult<Value> {
        self.isolate.run_function_by_name(
            &mut self.statics,
            &mut self.heap,
            &self.shared_heap,
            &mut self.shared_allocator,
            &self.shared_gc,
            function,
            arguments,
        )
    }

    /// Run one MIR function by name with yield support.
    pub(crate) fn run_function_by_name_yielding(
        &mut self,
        function: &str,
        arguments: &[Value],
    ) -> RuntimeResult<Outcome> {
        self.isolate.run_function_by_name_yielding(
            &mut self.statics,
            &mut self.heap,
            &self.shared_heap,
            &mut self.shared_allocator,
            &self.shared_gc,
            function,
            arguments,
        )
    }

    /// Run one MIR function by name with interpreter frame words.
    pub(crate) fn run_frame_function_by_name(
        &mut self,
        function: &str,
        arguments: &[Word],
    ) -> RuntimeResult<Value> {
        self.isolate.run_function_by_name_words(
            &mut self.statics,
            &mut self.heap,
            &self.shared_heap,
            &mut self.shared_allocator,
            &self.shared_gc,
            function,
            arguments,
        )
    }

    /// Resume one yielded continuation.
    pub(crate) fn resume(
        &mut self,
        continuation: Continuation,
        resume_value: Value,
    ) -> RuntimeResult<Outcome> {
        self.isolate.resume(
            &mut self.statics,
            &mut self.heap,
            &self.shared_heap,
            &mut self.shared_allocator,
            &self.shared_gc,
            continuation,
            resume_value,
        )
    }

    /// Collect garbage and return one GC summary.
    pub(crate) fn collect_garbage(&mut self) -> GcStats {
        self.collect_garbage_with_continuations(&mut [])
    }

    /// Collect garbage with continuation roots and return one GC summary.
    pub(crate) fn collect_garbage_with_continuations(
        &mut self,
        continuations: &mut [Continuation],
    ) -> GcStats {
        let mut roots = RootSet::default();
        self.isolate
            .visit_state_roots(&self.statics, continuations, &mut roots)
            .expect("failed to collect root set");
        let mut heap_roots =
            |visit: &mut dyn FnMut(destack_heap::RootSlot<'_>) -> destack_heap::HeapResult<()>| {
                self.isolate
                    .visit_root_slots(&mut self.statics, continuations, visit)
                    .expect("failed to collect mutable root slots");

                Ok::<(), destack_heap::HeapError>(())
            };
        let mut stats = self
            .heap
            .collect_full(&mut heap_roots)
            .expect("failed to collect heap");
        let shared_stats = self
            .shared_heap
            .collect_full(&roots.shared_heap)
            .expect("failed to collect shared heap");

        stats.freed_allocations += shared_stats.freed_allocations;
        stats.live_allocations += shared_stats.live_allocations;
        stats.freed_bytes += shared_stats.freed_bytes;
        stats.allocated_bytes += shared_stats.allocated_bytes;
        stats.retained_bytes += shared_stats.retained_bytes;

        stats
    }
}

/// Parse MIR text and create one test isolate.
pub(crate) fn create_isolate(mir_text: &str) -> TestIsolate {
    TestIsolate::new(mir_text)
}

/// Parse MIR text and create one test isolate with an explicit isolate id.
pub(crate) fn create_isolate_with_id(mir_text: &str, isolate_id: IsolateId) -> TestIsolate {
    TestIsolate::with_id(mir_text, isolate_id)
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
    assert_eq!(tree.metadata.layout.storage, storage);

    let mut isolate =
        Isolate::build_with_options(IsolateId::new(1), tree, strings, IsolateOptions::test())
            .unwrap_or_else(|error| panic!("failed to initialize isolate: {error}"));
    let mut statics = StaticSpace::empty();
    let heap = create_test_heap();
    let shared = create_test_shared_heap();
    let shared_gc = shared.register_collector_worker();
    let shared_allocator = shared.allocator();
    isolate
        .initialize(&heap, &shared, &mut statics)
        .unwrap_or_else(|error| panic!("failed to initialize isolate globals: {error}"));

    TestIsolate {
        isolate,
        statics,
        heap,
        shared_heap: shared,
        shared_gc,
        shared_allocator,
    }
}

/// Run one MIR function by name with the given arguments.
pub(crate) fn run_mir(mir: &str, function: &str, arguments: &[Value]) -> RuntimeResult<Value> {
    let mut isolate = create_isolate(mir);

    isolate.run_function_by_name(function, arguments)
}

/// Run MIR with access to one heap-owning isolate and interpreter frame words.
pub(crate) fn run_mir_with_frame<F>(
    mir_text: &str,
    function: &str,
    setup: F,
) -> RuntimeResult<Value>
where
    F: FnOnce(&mut TestIsolate) -> Vec<Word>,
{
    let mut isolate = create_isolate(mir_text);
    let args = setup(&mut isolate);

    isolate.run_frame_function_by_name(function, &args)
}

/// Run MIR with interpreter frame words, expecting success.
pub(crate) fn run_mir_with_frame_ok<F>(mir_text: &str, function: &str, setup: F) -> Value
where
    F: FnOnce(&mut TestIsolate) -> Vec<Word>,
{
    run_mir_with_frame(mir_text, function, setup).expect("execution failed")
}

/// Run MIR and expect success, returning the output.
pub(crate) fn run_mir_ok(mir_text: &str, function: &str, args: &[Value]) -> Value {
    run_mir(mir_text, function, args).expect("execution failed")
}

/// Run MIR and expect one specific return value.
pub(crate) fn run_mir_expect(mir_text: &str, function: &str, args: &[Value], expected: Value) {
    let output = run_mir_ok(mir_text, function, args);

    assert_eq!(output, expected, "unexpected value");
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
pub(crate) fn assert_execution_yielded(result: RuntimeResult<Outcome>) -> (Continuation, Value) {
    let outcome = result.expect("execution failed");

    match outcome {
        Outcome::Yielded {
            continuation,
            value,
        } => (continuation, value),
        Outcome::Completed { .. } => panic!("expected yield"),
    }
}

/// Assert that one execution result completed.
pub(crate) fn assert_execution_completed(result: RuntimeResult<Outcome>) -> Value {
    let outcome = result.expect("execution failed");

    match outcome {
        Outcome::Completed { value } => value,
        Outcome::Yielded { .. } => panic!("expected completion"),
    }
}

/// Managed nominal allocation stores and loads one field.
#[test]
fn test_managed_nominal_allocation_loads_field() {
    let mir_text = r#"
type Box {
    value: int32;
}

function sumBox(v0: int32): int32 {
b0(v0: int32):
    v1: Box = struct Box (v0)
    v2: ref<Box, managed, readonly> = new Box
    store v2, v1
    v3: ref<int32, managed, readonly> = field.address v2, 0
    v4: int32 = load v3
    v5: int32 = 1int32
    v6: int32 = int.add v4, v5
    return v6
}"#;

    let output = run_mir_ok(mir_text, "sumBox", &[Value::int32(9)]);

    assert_eq!(output, Value::int32(10));
}

/// Direct calls preserve managed receivers for callee loads.
#[test]
fn test_direct_call_loads_managed_receiver_field() {
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
    v1: ref<int32, managed, readonly> = field.address v0, 0
    v2: int32 = load v1
    return v2
}"#;

    let output = run_mir_ok(mir_text, "readValueClass", &[Value::int32(9)]);

    assert_eq!(output, Value::int32(9));
}

/// Stored function values preserve their function pointer through managed structs.
#[test]
fn test_stored_callable_roundtrips() {
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
    v3: ref<Fn, managed, readonly> = field.address v1, 0
    v4: Fn = load v3
    v5: int32 = call.indirect v4(): () -> int32
    return v5
}"#;

    let output = run_mir_ok(mir_text, "run", &[]);

    assert_eq!(output, Value::int32(7));
}

/// Interface dispatch forwards the concrete object receiver to the selected method.
#[ignore = "raw MIR fixtures cannot declare interface itab metadata"]
#[test]
fn test_interface_call_forwards_concrete_receiver() {
    let mir_text = r#"
type Greeter {
    object: ref<void, managed, readonly>;
    itab: usize;
}
type GreeterImpl {
    vtable: ref<void, raw, readonly, space(local)>;
    value: int32;
}
type Greeter#object {
    greet: () => int32;
}

global GreeterImpl#vtable: ref?<void, raw, readonly, space(local)>[3], readonly = zeroInit

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
    v2: ref<ref?<void, raw, readonly, space(local)>[3], raw, readonly, space(local)> = global.address GreeterImpl#vtable
    v3: ref<void, raw, readonly, space(local)> = cast.bit v2 -> ref<void, raw, readonly, space(local)>
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

    assert_eq!(output, Value::int32(42));
}
