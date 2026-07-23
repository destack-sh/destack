use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::Arc;

use bytecode::{CodeBuilder, Parser};
use destack_bytecode as bytecode;
use destack_core::{Optional, StringPool};
use destack_heap::{
    AllocationCache, AllocationPlan, Heap, HeapLimits, HeapOptions, SharedHeap, SharedHeapLimits,
    SharedHeapOptions, SharedMarkWorker,
};
use destack_memory::MemoryMap;
use destack_mir::{Space, TensorFormat, TraceMap, TraceTable};
use destack_program as program;
use destack_program::{
    AllocationSite, FunctionBuilder, FunctionId, FunctionTableBuilder, LayoutBuilder, LayoutId,
    LayoutShapeBuilder, ProgramActivation, ProgramBuilder, ProgramPoint, ProgramStorage,
    ScalarFormat, Signature, SignatureId, SiteTableBuilder, TensorDimension, TensorLayoutBuilder,
    TypeDescriptorBuilder, TypeId, Value, Word,
};
use destack_source::FileId;
use destack_vm::{Machine, MachineLimits};

const MEMORY_BYTES: usize = 512 * 1024 * 1024;
const MEMORY_FRAME_BYTES: usize = 16 * 1024 * 1024;

/// One direct-bytecode benchmark runtime.
pub(crate) struct Runtime {
    /// Immutable Program retained by the activation.
    program: Arc<program::Program>,
    /// Bytecode machine under measurement.
    machine: Machine,
    /// Runtime allocation plans indexed by Program allocation site id.
    allocation_plans: Arc<[Option<AllocationPlan>]>,
    /// Opaque runtime call state.
    state: Box<()>,
    /// Worker-local heap.
    heap: Heap,
    /// Runtime-shared heap.
    shared_heap: SharedHeap,
    /// Worker-local shared allocation cache.
    shared_cache: AllocationCache,
    /// Shared collector worker state.
    shared_mark_worker: SharedMarkWorker,
    /// Worker-local static bytes.
    local_static: program::StaticSpace,
    /// Runtime-shared static bytes.
    shared_static: program::StaticSpace,
}

impl Runtime {
    /// Build one direct-bytecode benchmark runtime.
    pub(crate) fn parse(source: &str) -> Self {
        Self::build(source, None)
    }

    /// Build one direct-bytecode tensor benchmark runtime.
    pub(crate) fn tensor(source: &str, dimensions: &[u64]) -> Self {
        Self::build(source, Some(dimensions))
    }

    /// Build one benchmark runtime with an optional tensor layout.
    fn build(source: &str, tensor_dimensions: Option<&[u64]>) -> Self {
        let object = Parser::new(FileId::from_source_bytes(source.as_bytes()), source)
            .parse()
            .expect("benchmark bytecode should parse");
        let program = Arc::new(Self::program(&object, tensor_dimensions));
        let memory = Arc::new(
            MemoryMap::reserve(MEMORY_BYTES, MEMORY_FRAME_BYTES)
                .expect("benchmark memory should reserve"),
        );
        let heap = Heap::new(memory.clone(), HeapLimits::default(), HeapOptions::local())
            .expect("benchmark heap should build");
        let shared_heap = SharedHeap::new(
            memory.clone(),
            SharedHeapLimits::default(),
            SharedHeapOptions::default(),
        )
        .expect("benchmark shared heap should build");
        let shared_cache = shared_heap.allocation_cache();
        let shared_mark_worker = shared_heap.register_mark_worker();
        let local_static = program
            .materialize_local_statics(memory.clone())
            .expect("benchmark local statics should materialize");
        let shared_static = program
            .materialize_shared_statics(memory.clone())
            .expect("benchmark shared statics should materialize");
        let allocation_plans = program
            .plan_allocations(heap.options(), shared_heap.options())
            .expect("benchmark allocation plans should build")
            .into();
        let machine = Machine::new(program.clone(), memory, MachineLimits::unbounded())
            .expect("benchmark machine should build");

        Self {
            program,
            machine,
            allocation_plans,
            state: Box::new(()),
            heap,
            shared_heap,
            shared_cache,
            shared_mark_worker,
            local_static,
            shared_static,
        }
    }

    /// Execute the benchmark entry with one iteration count.
    pub(crate) fn run(&mut self, iterations: i32) -> Value {
        let state = NonNull::from(self.state.as_mut()).cast::<c_void>();
        let context = ProgramActivation {
            state,
            storage: ProgramStorage {
                allocation_plans: &self.allocation_plans,
                heap: &mut self.heap,
                shared_heap: &self.shared_heap,
                shared_cache: &mut self.shared_cache,
                shared_mark_worker: &self.shared_mark_worker,
                local_static: &mut self.local_static,
                shared_static: &mut self.shared_static,
                constant_space: self.program.constants(),
            },
        };
        let arguments = [Value::int32(iterations)];
        let outcome = self
            .machine
            .run(context, FunctionId(0), &arguments, None, None, None)
            .expect("benchmark bytecode should execute");

        match outcome {
            program::Outcome::Completed { value } => value,
            program::Outcome::Yielded { .. } => panic!("benchmark bytecode should not yield"),
            program::Outcome::Stopped { .. } => panic!("benchmark bytecode should not stop"),
        }
    }

    /// Build the immutable Program consumed by one benchmark machine.
    fn program(object: &bytecode::Object, tensor_dimensions: Option<&[u64]>) -> program::Program {
        let strings = Self::strings(object);
        let string_ids = object.strings().iter().map(|entry| entry.id);
        let code = CodeBuilder::new()
            .value_types(object.value_types().iter().copied())
            .frame_slots(object.frame_slots().iter().copied())
            .constants(object.constants().iter().map(|constant| constant.value))
            .constant_bytes(object.constant_bytes())
            .bodies(object.functions().iter().map(|function| function.body))
            .code(object.code())
            .operation_offsets(object.operation_offsets().iter().copied());
        let (types, layouts, traces, int32_type) = Self::types(object, tensor_dimensions);
        let functions = Self::functions(object, int32_type);
        let sites = Self::sites(object, tensor_dimensions.is_some());

        ProgramBuilder::new(Default::default(), code)
            .strings(&strings, string_ids)
            .types(types)
            .layouts(layouts)
            .traces(traces)
            .functions(functions)
            .sites(sites)
            .build()
    }

    /// Build the stable string pool carried by one bytecode object.
    fn strings(object: &bytecode::Object) -> StringPool {
        let strings =
            StringPool::with_capacity(object.strings().len(), object.string_bytes().len());

        for entry in object.strings() {
            let text = object
                .string(entry.id)
                .expect("benchmark object string should decode");
            let id = strings.intern(text);
            assert_eq!(id, entry.id, "benchmark string identity should match");
        }

        strings
    }

    /// Build the nominal, void, and signed 32-bit Program types used by benchmarks.
    fn types(
        object: &bytecode::Object,
        tensor_dimensions: Option<&[u64]>,
    ) -> (
        Vec<TypeDescriptorBuilder>,
        Vec<LayoutBuilder>,
        TraceTable,
        TypeId,
    ) {
        let mut traces = TraceTable::new();
        let trace = traces.insert(TraceMap::empty());
        let mut types = Vec::with_capacity(object.types().len() + 2);
        let mut layouts = Vec::with_capacity(object.types().len() + 2);
        let int32_type = TypeId(object.types().len() as u32 + 1);

        // materialize nominal types before execution representations
        for index in 0..object.types().len() {
            let layout = LayoutId::new(index as u32 + 1);
            let shape = if index == 0
                && let Some(dimensions) = tensor_dimensions
            {
                let dimensions = dimensions.iter().copied().map(TensorDimension::fixed);
                let tensor = TensorLayoutBuilder::new(
                    Space::Local,
                    int32_type,
                    TensorFormat::dense_row_major(),
                    dimensions,
                );

                LayoutShapeBuilder::Tensor(tensor)
            } else {
                LayoutShapeBuilder::Struct(Vec::new())
            };
            types.push(TypeDescriptorBuilder::new(layout));
            layouts.push(LayoutBuilder::new(
                shape,
                Word::BYTE_LEN as u32,
                Word::BYTE_LEN as u32,
                trace,
            ));
        }

        // append the representations used by the public machine call
        let void_layout = LayoutId::new(layouts.len() as u32 + 1);
        types.push(TypeDescriptorBuilder::new(void_layout));
        layouts.push(LayoutBuilder::new(LayoutShapeBuilder::None, 0, 1, trace));
        let int32_layout = LayoutId::new(layouts.len() as u32 + 1);
        types.push(TypeDescriptorBuilder::new(int32_layout));
        layouts.push(LayoutBuilder::new(
            LayoutShapeBuilder::Scalar(ScalarFormat::int(i32::BITS as u16, true)),
            size_of::<i32>() as u32,
            align_of::<i32>() as u32,
            trace,
        ));

        (types, layouts, traces, int32_type)
    }

    /// Build one signed 32-bit benchmark function signature.
    fn functions(object: &bytecode::Object, int32_type: TypeId) -> FunctionTableBuilder {
        let signature = Signature {
            parameters: vec![int32_type],
            result: int32_type,
        };
        let functions = object
            .functions()
            .iter()
            .map(|function| FunctionBuilder::new(function.name, SignatureId(0)));

        FunctionTableBuilder::new()
            .signatures([signature])
            .functions(functions)
    }

    /// Build allocation sites for tensor-producing benchmark operations.
    fn sites(object: &bytecode::Object, has_tensor: bool) -> SiteTableBuilder {
        if !has_tensor {
            return SiteTableBuilder::new();
        }
        let function = &object.functions()[0];
        let operation_count = function
            .body
            .operation_offsets(object.operation_offsets())
            .len();
        let allocations = (0..operation_count).filter_map(|operation| {
            let instruction = object
                .instruction(bytecode::FunctionId(0), operation as u32)
                .expect("benchmark instruction should decode")
                .expect("benchmark operation should exist");
            let tensor = instruction.opcode().tensor_operation()?;
            if !matches!(
                tensor,
                bytecode::TensorOperation::Splat | bytecode::TensorOperation::Element
            ) {
                return None;
            }

            Some(AllocationSite {
                point: ProgramPoint::new(FunctionId(0), operation as u32),
                space: Space::Local,
                result_type: TypeId(0),
                storage_type: TypeId(0),
                virtual_table: Optional::none(),
            })
        });

        SiteTableBuilder::new().allocations(allocations)
    }
}
