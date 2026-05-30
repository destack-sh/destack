use std::sync::{Arc, OnceLock};

use destack_engine::{StaticSpace, Value};
use destack_heap::{
    AllocationCache, Allocator, GcStats, GcWorker, Heap, HeapLimits, HeapOptions, HeapReference,
    SharedHeapLimits, SharedHeapOptions,
};

use crate::SharedHeap;
use destack_mir::parse::{ParseOptions, Parser};
use destack_mir::{DataLayout, TraceTable};
use destack_source::FileId;

use crate::diagnostic::{Error, RuntimeResult};
use crate::program::{Layout, encode_word_bytes};
use crate::{Continuation, Isolate, IsolateId, IsolateOptions, Outcome, Word};

/// The virtual heap-space width used by ordinary VM tests.
const TEST_LOCAL_SPACE_SIZE_BYTES: usize = 16 * 1024 * 1024;
static TRACE_TABLE: OnceLock<TraceTable> = OnceLock::new();

/// Return the shared empty trace table for VM tests.
pub(crate) fn trace_table() -> &'static TraceTable {
    TRACE_TABLE.get_or_init(TraceTable::new)
}

/// The isolate and authoritative heap used by one test runtime.
pub(crate) struct TestIsolate {
    /// The VM isolate under test.
    pub isolate: Isolate,
    /// The worker static space used by the isolate.
    pub statics: StaticSpace,
    /// The authoritative heap for the isolate.
    pub heap: Heap,
    /// The runtime-shared heap for the isolate.
    pub shared_heap: SharedHeap,
    /// The shared collector worker used by this isolate.
    pub shared_gc: GcWorker,
    /// The worker-local shared allocation cache.
    pub shared_cache: AllocationCache,
}

/// Create one local test heap.
pub(crate) fn create_test_heap() -> Heap {
    let options = test_local_heap_options();
    let allocator = Arc::new(
        Allocator::try_new(options.page_size_bytes, options.allocator_chunk_size_bytes)
            .expect("test allocator should build"),
    );

    Heap::with_allocator_limits_and_options(allocator, HeapLimits::default(), options)
        .expect("test heap should build")
}

/// Create one shared test heap.
pub(crate) fn create_test_shared_heap() -> SharedHeap {
    let options = test_shared_heap_options();
    let allocator = Arc::new(
        Allocator::try_new(options.page_size_bytes, options.allocator_chunk_size_bytes)
            .expect("test allocator should build"),
    );

    SharedHeap::with_allocator_limits_and_options(allocator, SharedHeapLimits::default(), options)
        .expect("test shared heap should build")
}

/// Create heap options for ordinary local VM tests.
fn test_local_heap_options() -> HeapOptions {
    HeapOptions {
        address_space_size_bytes: TEST_LOCAL_SPACE_SIZE_BYTES,
        ..HeapOptions::local()
    }
}

/// Create heap options for ordinary shared VM tests.
pub(crate) fn test_shared_heap_options() -> SharedHeapOptions {
    SharedHeapOptions {
        address_space_size_bytes: TEST_LOCAL_SPACE_SIZE_BYTES,
        ..SharedHeapOptions::default()
    }
}

impl TestIsolate {
    /// Build one test isolate from MIR text.
    pub(crate) fn new(mir_text: &str) -> Self {
        Self::with_id(mir_text, IsolateId::new(1))
    }

    /// Build one test isolate from MIR text with one explicit isolate id.
    pub(crate) fn with_id(mir_text: &str, isolate_id: IsolateId) -> Self {
        let (tree, strings) = Parser::parse(FileId::new(0), mir_text, ParseOptions::default())
            .finish()
            .expect("failed to parse MIR");

        let mut isolate =
            Isolate::build_with_options(isolate_id, tree, strings, IsolateOptions::test())
                .unwrap_or_else(|error| panic!("failed to initialize isolate: {error}"));

        let mut statics = StaticSpace::empty();
        let heap = create_test_heap();
        let shared_heap = create_test_shared_heap();
        let shared_gc = shared_heap.register_collector_worker();
        let shared_cache = shared_heap.allocation_cache();

        isolate
            .initialize(&heap, &shared_heap, &mut statics)
            .unwrap_or_else(|error| panic!("failed to initialize isolate globals: {error}"));

        Self {
            isolate,
            statics,
            heap,
            shared_heap,
            shared_gc,
            shared_cache,
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
            .expect("function parameter type should be concrete after parsing")
    }

    /// Materialize one value for the given MIR type.
    pub(crate) fn materialize_value_for_type(
        &mut self,
        ty: destack_mir::LocalNodeId<destack_mir::Type>,
        values: Vec<Word>,
    ) -> Word {
        let layout = self
            .isolate
            .layout(ty)
            .unwrap_or_else(|| panic!("missing layout for type {ty:?}"));

        // scalars travel directly as VM words
        if layout.is_word() {
            assert_eq!(values.len(), 1, "scalar materialization expects one value");

            return values[0];
        }

        let bytes = materialize_value_bytes(&self.isolate, &self.heap, ty, layout, &values);
        let layout_id = self
            .isolate
            .layout_id_for_type(ty)
            .unwrap_or_else(|| panic!("missing layout id for type {ty:?}"));
        let shape = self
            .isolate
            .allocation_shape(layout_id)
            .unwrap_or_else(|error| panic!("failed to resolve allocation shape: {error}"));
        let reference = self
            .heap
            .allocate_dynamic_bytes(shape, &bytes)
            .unwrap_or_else(|error| panic!("failed to allocate materialized value: {error}"));

        Word::heap_reference(reference)
    }

    /// Run one MIR function by name with the given arguments.
    pub(crate) fn run_function_by_name(
        &mut self,
        function: &str,
        arguments: &[Value],
    ) -> RuntimeResult<Value> {
        let function = self.isolate.function_id_by_name(function)?;

        self.isolate.run_function(
            &mut self.statics,
            &mut self.heap,
            &self.shared_heap,
            &mut self.shared_cache,
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
        let function = self.isolate.function_id_by_name(function)?;

        self.isolate.run_function_yielding(
            &mut self.statics,
            &mut self.heap,
            &self.shared_heap,
            &mut self.shared_cache,
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
        let function = self.isolate.function_id_by_name(function)?;

        self.isolate.run_function_words(
            &mut self.statics,
            &mut self.heap,
            &self.shared_heap,
            &mut self.shared_cache,
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
            &mut self.shared_cache,
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
        let mut shared_roots = Vec::new();
        self.isolate
            .visit_root_slots(&mut self.statics, continuations, &mut |slot| {
                let root = slot.load()?;
                destack_heap::RootSink::push(&mut shared_roots, root);

                Ok(())
            })
            .expect("failed to collect shared roots");
        let trace_table = self.isolate.trace_table();
        let mut heap_roots =
            |visit: &mut dyn FnMut(destack_heap::RootSlot<'_>) -> destack_heap::HeapResult<()>| {
                self.isolate
                    .visit_root_slots(&mut self.statics, continuations, visit)
                    .expect("failed to collect mutable root slots");

                Ok::<(), destack_heap::HeapError>(())
            };
        let mut stats = self
            .heap
            .collect_full(&mut heap_roots, trace_table.as_ref())
            .expect("failed to collect heap");
        let shared_stats = self
            .shared_heap
            .collect_full(&shared_roots, trace_table.as_ref())
            .expect("failed to collect shared heap");

        stats.freed_allocations += shared_stats.freed_allocations;
        stats.live_allocations += shared_stats.live_allocations;
        stats.freed_bytes += shared_stats.freed_bytes;
        stats.allocated_bytes += shared_stats.allocated_bytes;
        stats.retained_bytes += shared_stats.retained_bytes;

        stats
    }
}

/// Materialize one VM aggregate into heap payload bytes.
fn materialize_value_bytes(
    isolate: &Isolate,
    heap: &Heap,
    ty: destack_mir::LocalNodeId<destack_mir::Type>,
    layout: &Layout,
    values: &[Word],
) -> Vec<u8> {
    let mut bytes = vec![0u8; layout.byte_len];

    // fields
    if let Some(field_count) = layout.field_count() {
        assert_eq!(
            values.len(),
            field_count,
            "field materialization expects one value per field"
        );
        for (index, value) in values.iter().copied().enumerate() {
            let field = layout
                .field(index as u32)
                .unwrap_or_else(|| panic!("missing field {index} for type {ty:?}"));
            write_materialized_value(isolate, heap, field.ty, value, &mut bytes[field.offset..]);
        }

        return bytes;
    }

    // elements
    let element = layout
        .element()
        .unwrap_or_else(|| panic!("type {ty:?} is not materializable as an aggregate"));
    let element_count = layout
        .element_count()
        .unwrap_or_else(|| panic!("type {ty:?} has no element count"));
    assert_eq!(
        values.len(),
        element_count,
        "element materialization expects one value per element"
    );
    for (index, value) in values.iter().copied().enumerate() {
        let start = element.stride * index;
        write_materialized_value(isolate, heap, element.ty, value, &mut bytes[start..]);
    }

    bytes
}

/// Write one materialized field or element value.
fn write_materialized_value(
    isolate: &Isolate,
    heap: &Heap,
    ty: destack_mir::LocalNodeId<destack_mir::Type>,
    value: Word,
    destination: &mut [u8],
) {
    let layout = isolate
        .layout(ty)
        .unwrap_or_else(|| panic!("missing layout for nested type {ty:?}"));

    // scalar values encode inline
    if layout.is_word() {
        let encoded = encode_word_bytes(isolate.tree(), ty, value)
            .unwrap_or_else(|error| panic!("failed to encode materialized word: {error}"));
        destination[..encoded.len()].copy_from_slice(encoded.as_slice());

        return;
    }

    // aggregate values are already heap payloads
    let source = value.as_heap_reference();
    let payload = read_heap_payload(heap, source, layout.byte_len);
    destination[..layout.byte_len].copy_from_slice(&payload);
}

/// Read one heap payload for test materialization.
fn read_heap_payload(heap: &Heap, reference: HeapReference, byte_len: usize) -> Vec<u8> {
    assert!(
        heap.is_heap_live(reference),
        "materialized aggregate source is not live"
    );
    let mut bytes = vec![0u8; byte_len];
    let address = heap.heap_base_address() + reference.offset();

    unsafe {
        std::ptr::copy_nonoverlapping(address as *const u8, bytes.as_mut_ptr(), byte_len);
    }

    bytes
}

/// Parse MIR text and create one test isolate.
pub(crate) fn create_isolate(mir_text: &str) -> TestIsolate {
    TestIsolate::new(mir_text)
}

/// Parse MIR text and create one test isolate with an explicit isolate id.
pub(crate) fn create_isolate_with_id(mir_text: &str, isolate_id: IsolateId) -> TestIsolate {
    TestIsolate::with_id(mir_text, isolate_id)
}

/// Parse MIR text and create one test isolate with explicit data layout.
pub(crate) fn create_isolate_with_data_layout(
    mir_text: &str,
    data_layout: DataLayout,
) -> TestIsolate {
    let (tree, strings) = Parser::parse(
        FileId::new(0),
        mir_text,
        ParseOptions {
            pointer_bytes: data_layout.pointer_bytes,
        },
    )
    .finish()
    .expect("failed to parse MIR");
    assert_eq!(tree.metadata.data_layout, data_layout);

    let mut isolate =
        Isolate::build_with_options(IsolateId::new(1), tree, strings, IsolateOptions::test())
            .unwrap_or_else(|error| panic!("failed to initialize isolate: {error}"));
    let mut statics = StaticSpace::empty();
    let heap = create_test_heap();
    let shared = create_test_shared_heap();
    let shared_gc = shared.register_collector_worker();
    let shared_cache = shared.allocation_cache();
    isolate
        .initialize(&heap, &shared, &mut statics)
        .unwrap_or_else(|error| panic!("failed to initialize isolate globals: {error}"));

    TestIsolate {
        isolate,
        statics,
        heap,
        shared_heap: shared,
        shared_gc,
        shared_cache,
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
    v2: ref<Box, managed, readonly> = new.zeroed Box
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
    v2: ref<Box, managed, readonly> = new.zeroed Box
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
fn test_stored_closure_roundtrips() {
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
    v1: ref<Holder, managed, readonly> = new.zeroed Holder
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

/// The dynamic dispatch forwards the concrete object receiver to the selected method.
#[ignore = "text MIR fixtures cannot declare dynamic table metadata"]
#[test]
fn test_interface_call_forwards_concrete_receiver() {
    let mir_text = r#"
type Greeter {
    value: ref<void, managed, readonly>;
    table: ref<void, raw, readonly, space(static)>;
}
type GreeterImpl {
    vtable: ref<void, raw, readonly, space(local)>;
    value: int32;
}
type Greeter#object {
    greet: () => int32;
}

readonly global GreeterImpl#vtable: [ref<void, raw, readonly, space(local), nullable>; 3] = zeroInit

external function Greeter.greet(Greeter#object): int32

function callInterface(v0: Greeter): int32 {
b0(v0: Greeter):
    v1: ref<void, managed, readonly> = field.get v0, 0
    v2: int32 = call.dynamic v0, Greeter#object, 1(v1): (Greeter#object) -> int32
    return v2
}

function runInterface(): int32 {
b0:
    v0: int32 = 41int32
    v1: ref<GreeterImpl, managed, readonly> = call GreeterImpl.constructor(v0): (int32) -> ref<GreeterImpl, managed, readonly>
    v2: ref<void, managed, readonly> = cast.bit v1 -> ref<void, managed, readonly>
    v3: uint64 = 0uint64
    v4: ref<void, raw, readonly, space(static)> = cast.bit v3 -> ref<void, raw, readonly, space(static)>
    v5: Greeter = struct Greeter (v2, v4)
    v6: int32 = call callInterface(v5): (Greeter) -> int32
    return v6
}

function GreeterImpl.constructor(v0: int32): ref<GreeterImpl, managed, readonly> {
b0(v0: int32):
    v1: ref<GreeterImpl, managed, readonly> = new.zeroed GreeterImpl
    v2: ref<[ref<void, raw, readonly, space(local), nullable>; 3], raw, readonly, space(local)> = global.address GreeterImpl#vtable
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
