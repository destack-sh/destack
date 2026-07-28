use std::sync::Arc;

use bytecode::{CodeBuilder, Parser, RelocationTag};
use destack_bytecode as bytecode;
use destack_core::{Optional, StringId, StringPool};
use destack_heap::{
    AllocationCache, AllocationPlan, Heap, HeapLimits, HeapOptions, SharedHeap, SharedHeapLimits,
    SharedHeapOptions, SharedMarkWorker,
};
use destack_memory::MemoryMap;
use destack_mir::{Space, TensorFormat, TraceMap, TraceTable};
use destack_program as program;
use destack_program::{
    AllocationSite, FrameTableBuilder, FunctionBuilder, FunctionId, FunctionTableBuilder,
    LayoutBuilder, LayoutId, LayoutShapeBuilder, ProgramBuilder, ProgramPoint, ScalarFormat,
    Signature, SignatureId, SiteTableBuilder, TensorDimension, TensorLayoutBuilder,
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
    /// Worker-local continuation storage.
    continuations: program::ContinuationTable,
    /// Bytecode machine under measurement.
    machine: Machine,
    /// Runtime allocation plans indexed by Program allocation site id.
    allocation_plans: Arc<[Option<AllocationPlan>]>,
    /// Program runtime operations.
    runtime: BenchmarkRuntime,
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

/// Program runtime used by direct execution benchmarks.
#[derive(Debug, Default)]
struct BenchmarkRuntime;

impl program::Runtime for BenchmarkRuntime {
    /// Reject waiter settlement outside asynchronous benchmarks.
    fn queue_waiter(&mut self, _waiter: program::Waiter, _value: Value) -> program::Result<bool> {
        unreachable!("direct execution benchmarks do not await")
    }

    /// Reject waiter cancellation outside asynchronous benchmarks.
    fn cancel_waiter(&mut self, _waiter: program::Waiter) -> program::Result<bool> {
        unreachable!("direct execution benchmarks do not await")
    }

    /// Reject resolved tasks outside asynchronous benchmarks.
    fn resolve_task(&mut self, _value: Value) -> program::Task {
        unreachable!("direct execution benchmarks do not create tasks")
    }

    /// Reject eager tasks outside asynchronous benchmarks.
    fn start_task(&mut self) -> program::Task {
        unreachable!("direct execution benchmarks do not create tasks")
    }

    /// Reject task cancellation requests outside asynchronous benchmarks.
    fn cancel_task(&mut self, _task: program::Task) -> program::Result<()> {
        unreachable!("direct execution benchmarks do not create tasks")
    }

    /// Reject task suspension outside asynchronous benchmarks.
    fn suspend_task(
        &mut self,
        _task: program::Task,
        _continuation: program::Continuation,
    ) -> program::Result<program::Waiter> {
        unreachable!("direct execution benchmarks do not create tasks")
    }

    /// Reject task waiting outside asynchronous benchmarks.
    fn park_task(&mut self, _task: program::Task, _waiter: program::Waiter) -> program::Result<()> {
        unreachable!("direct execution benchmarks do not create tasks")
    }

    /// Reject task cancellation queries outside asynchronous benchmarks.
    fn is_task_cancelled(&mut self, _task: program::Task) -> program::Result<bool> {
        unreachable!("direct execution benchmarks do not create tasks")
    }

    /// Reject task detachment outside asynchronous benchmarks.
    fn detach_task(&mut self, _task: program::Task) -> program::Result<()> {
        unreachable!("direct execution benchmarks do not create tasks")
    }

    /// Reject terminal task outcomes outside asynchronous benchmarks.
    fn finish_task(
        &mut self,
        _task: program::Task,
        _outcome: program::TaskOutcome,
    ) -> program::Result<()> {
        unreachable!("direct execution benchmarks do not create tasks")
    }
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
            continuations: program::ContinuationTable::default(),
            machine,
            allocation_plans,
            runtime: BenchmarkRuntime,
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
        let activation = program::Activation {
            runtime: &mut self.runtime,
            memory: program::Memory {
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
        let parameter = self
            .program
            .function_parameters(FunctionId(0))
            .and_then(|parameters| parameters.first())
            .copied()
            .expect("benchmark entry should accept its iteration count");
        let argument = self
            .program
            .value(parameter, [Word::int32(iterations)])
            .expect("benchmark argument should match its Program type");
        let arguments = [argument];
        let outcome = self
            .machine
            .run(
                &mut self.continuations,
                activation,
                FunctionId(0),
                None,
                &arguments,
                None,
                None,
                None,
            )
            .expect("benchmark bytecode should execute");

        match outcome {
            program::Outcome::Completed { value } => value,
            program::Outcome::Cancelled => panic!("benchmark bytecode should not cancel"),
            program::Outcome::Stopped { .. } => panic!("benchmark bytecode should not stop"),
            program::Outcome::Awaited { .. } | program::Outcome::Yielded { .. } => {
                panic!("benchmark bytecode should not suspend")
            }
        }
    }

    /// Build the immutable Program consumed by one benchmark machine.
    fn program(object: &bytecode::Object, tensor_dimensions: Option<&[u64]>) -> program::Program {
        let (strings, string_ids, functions) = Self::functions(object, tensor_dimensions.is_some());
        let code = Self::code(object);
        let (types, layouts, traces) = Self::types(tensor_dimensions);
        let sites = Self::sites(object, tensor_dimensions.is_some());

        ProgramBuilder::new(Default::default(), code)
            .strings(&strings, string_ids)
            .types(types)
            .layouts(layouts)
            .traces(traces)
            .functions(functions)
            .frames(FrameTableBuilder::new())
            .sites(sites)
            .build()
    }

    /// Link one parsed bytecode object into Program code.
    fn code(object: &bytecode::Object) -> CodeBuilder {
        let mut code = object.code().to_vec();

        // resolve object-local identities into the equal benchmark identities
        for relocation in object.relocations() {
            let start = relocation.byte_offset as usize;
            let end = start + size_of::<u32>();
            let bytes = code[start..end]
                .try_into()
                .expect("benchmark relocation should be complete");
            let value = u32::from_le_bytes(bytes);
            let value = match relocation.tag {
                RelocationTag::LAYOUT => value + 1,
                RelocationTag::TYPE
                | RelocationTag::FUNCTION
                | RelocationTag::GLOBAL
                | RelocationTag::DYNAMIC
                | RelocationTag::ALLOCATION
                | RelocationTag::COUNTER
                | RelocationTag::SAMPLER => value,
                _ => panic!("unknown benchmark relocation"),
            };
            code[start..end].copy_from_slice(&value.to_le_bytes());
        }

        CodeBuilder::new()
            .functions(object.functions().iter().copied())
            .frames(object.frames().iter().copied())
            .registers(object.registers().iter().copied())
            .operations(object.operations().iter().copied())
            .code(code)
    }

    /// Build the object-local, void, and signed 32-bit Program types used by benchmarks.
    fn types(
        tensor_dimensions: Option<&[u64]>,
    ) -> (Vec<TypeDescriptorBuilder>, Vec<LayoutBuilder>, TraceTable) {
        let mut traces = TraceTable::new();
        let trace = traces.insert(TraceMap::empty());
        let object_type_count = usize::from(tensor_dimensions.is_some());
        let mut types = Vec::with_capacity(object_type_count + 2);
        let mut layouts = Vec::with_capacity(object_type_count + 2);
        let int32_type = TypeId(object_type_count as u32 + 1);

        // materialize object-local types before execution representations
        for index in 0..object_type_count {
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

        (types, layouts, traces)
    }

    /// Build one signed 32-bit benchmark function signature.
    fn functions(
        object: &bytecode::Object,
        has_tensor: bool,
    ) -> (StringPool, Vec<StringId>, FunctionTableBuilder) {
        let strings = StringPool::new();
        let int32_type = TypeId(u32::from(has_tensor) + 1);
        let mut names = Vec::with_capacity(object.functions().len());
        let mut signatures = Vec::with_capacity(object.functions().len());
        let mut functions = Vec::with_capacity(object.functions().len());

        // preserve physical function order as dense Program identity
        for (index, _) in object.functions().iter().enumerate() {
            let name = strings.intern(&format!("f{index}"));
            let signature = SignatureId(index as u32);
            names.push(name);
            signatures.push(Signature {
                parameters: vec![int32_type],
                result: int32_type,
            });
            functions.push(FunctionBuilder::new(name, signature));
        }

        let table = FunctionTableBuilder::new()
            .signatures(signatures)
            .functions(functions);

        (strings, names, table)
    }

    /// Build allocation sites for tensor-producing benchmark operations.
    fn sites(object: &bytecode::Object, has_tensor: bool) -> SiteTableBuilder {
        if !has_tensor {
            return SiteTableBuilder::new();
        }
        let function = &object.functions()[0];
        let operation_count = function.operations(object.operations()).len();
        let allocations = (0..operation_count).filter_map(|operation| {
            let instruction = object
                .operation(bytecode::FunctionId(0), operation as u32)
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
                layout: LayoutId::new(1),
                virtual_table: Optional::none(),
            })
        });

        SiteTableBuilder::new().allocations(allocations)
    }
}
