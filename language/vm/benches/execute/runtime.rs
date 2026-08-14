use std::sync::Arc;

use bytecode::{CodeBuilder, Parser, RelocationTag};
use destack_bytecode as bytecode;
use destack_core::{Optional, StringId, StringPool};
use destack_heap::{
    AllocationCache, AllocationPlan, Heap, HeapLimits, HeapOptions, SharedHeap, SharedHeapLimits,
    SharedHeapOptions, SharedMarkWorker,
};
use destack_memory::{MemoryMap, MemoryRange};
use destack_mir::{
    Access, Nullability, ReferenceKind, Space, Storage, TensorFormat, TraceMap, TraceTable,
};
use destack_program as program;
use destack_program::{
    AllocationSite, FrameTableBuilder, FunctionBuilder, FunctionId, FunctionTableBuilder,
    LayoutBuilder, LayoutId, LayoutShapeBuilder, ProgramBuilder, ProgramPoint, ReferenceLayout,
    ScalarFormat, Signature, SignatureId, SiteTableBuilder, Symbol, TensorDimension,
    TensorLayoutBuilder, TypeDescriptorBuilder, TypeFingerprint, TypeId, TypeTableBuilder, Value,
    Word,
};
use destack_source::FileId;
use destack_vm::{Error, Machine, MachineLimits, Result};

/// Reserved address-space byte length for direct benchmark execution.
const MEMORY_BYTES: usize = 512 * 1024 * 1024;
/// Memory frame byte length for direct benchmark execution.
const MEMORY_FRAME_BYTES: usize = 16 * 1024 * 1024;

/// One direct-bytecode benchmark runtime.
pub(crate) struct Runtime {
    /// Immutable Program retained by the activation.
    program: Arc<program::Program>,
    /// Reusable execution fiber under measurement.
    fiber: destack_vm::Fiber,
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
    /// Runtime constant bytes.
    constant_space: program::StaticSpace,
    /// Runtime immortal object bytes.
    immortal_space: program::StaticSpace,
    /// Worker-local static bytes.
    local_static: program::StaticSpace,
    /// Runtime-shared static bytes.
    shared_static: program::StaticSpace,
}

/// Program runtime used by direct execution benchmarks.
#[derive(Debug, Default)]
struct BenchmarkRuntime;

impl program::Runtime for BenchmarkRuntime {
    type Error = Error;

    /// Return whether execution must yield at the current runtime poll.
    fn is_poll_requested(&self) -> bool {
        false
    }

    /// Continue benchmark execution after one impossible poll request.
    fn poll(&mut self, _memory: program::Memory<'_>) -> Result<program::Poll> {
        Ok(program::Poll::Continue)
    }

    /// Reject runtime bindings outside binding benchmarks.
    fn call_binding(
        &mut self,
        _memory: program::Memory<'_>,
        _context: program::Context,
        _fiber_id: Option<program::FiberId>,
        _binding: &program::Binding,
        _arguments: &[Word],
        _result: &mut [Word],
    ) -> Result<()> {
        unreachable!("direct execution benchmarks do not call runtime bindings")
    }

    /// Reject fiber parks outside asynchronous benchmarks.
    fn park(&mut self, _fiber: program::FiberId) -> Result<program::Park> {
        unreachable!("direct execution benchmarks do not park")
    }

    /// Reject detach boundaries outside asynchronous benchmarks.
    fn detach(&mut self) -> Result<program::FiberId> {
        unreachable!("direct execution benchmarks do not detach")
    }

    /// Reject boundary retirement outside asynchronous benchmarks.
    fn retire(&mut self, _fiber: program::FiberId) -> Result<()> {
        unreachable!("direct execution benchmarks do not detach")
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
        let mut shared_heap = SharedHeap::new(
            memory.clone(),
            SharedHeapLimits::default(),
            SharedHeapOptions::default(),
        )
        .expect("benchmark shared heap should build");
        let shared_cache = shared_heap.allocation_cache();
        let shared_mark_worker = shared_heap.register_mark_worker();
        let (constant_space, immortal_space, shared_static) = program
            .materialize_runtime_statics(memory.clone())
            .expect("benchmark runtime statics should materialize");
        let local_static = program
            .materialize_local_statics(
                memory.clone(),
                &constant_space,
                &immortal_space,
                &shared_static,
            )
            .expect("benchmark local statics should materialize");
        shared_heap.set_immortal_range(MemoryRange {
            offset: immortal_space.offset(),
            byte_len: immortal_space.byte_len(),
        });
        let allocation_plans = program
            .plan_allocations(heap.options(), shared_heap.options())
            .expect("benchmark allocation plans should build")
            .into();
        let machine = Machine::new(program.clone(), MachineLimits::unbounded())
            .expect("benchmark machine should build");
        let fiber = machine
            .reserve_fiber(memory)
            .expect("benchmark fiber should reserve");

        Self {
            program,
            fiber,
            machine,
            allocation_plans,
            runtime: BenchmarkRuntime,
            heap,
            shared_heap,
            shared_cache,
            shared_mark_worker,
            constant_space,
            local_static,
            shared_static,
            immortal_space,
        }
    }

    /// Execute the benchmark entry with one iteration count.
    pub(crate) fn run(&mut self, iterations: i32) -> Value {
        let mut context = program::Context::empty();
        let activation = program::Activation {
            runtime: &mut self.runtime,
            context: &mut context,
            memory: program::Memory {
                allocation_plans: &self.allocation_plans,
                local_heap: &mut self.heap,
                shared_heap: &self.shared_heap,
                shared_cache: &mut self.shared_cache,
                shared_mark_worker: &self.shared_mark_worker,
                local_statics: &mut self.local_static,
                shared_statics: &mut self.shared_static,
                immortals: &self.immortal_space,
                constants: &self.constant_space,
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
                &mut self.fiber,
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
            program::Outcome::Parked => panic!("benchmark bytecode should not park"),
        }
    }

    /// Build the immutable Program consumed by one benchmark machine.
    fn program(object: &bytecode::Object, tensor_dimensions: Option<&[u64]>) -> program::Program {
        let (strings, string_ids, functions) = Self::functions(object, tensor_dimensions.is_some());
        let code = Self::code(object);
        let (types, layouts, traces) = Self::types(tensor_dimensions);
        let sites = Self::sites(object, tensor_dimensions.is_some());
        let types = TypeTableBuilder::new().types(
            (0..types.len())
                .map(|index| TypeFingerprint::from_raw(index as u128))
                .zip(types),
        );

        ProgramBuilder::new(Default::default())
            .bytecode(code)
            .strings(&strings, string_ids)
            .types(types)
            .layouts(layouts)
            .traces(traces)
            .functions(functions)
            .frames(FrameTableBuilder::new())
            .sites(sites)
            .build()
            .expect("bench program should build")
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
            let (shape, layout_trace) = if index == 0
                && let Some(dimensions) = tensor_dimensions
            {
                let dimensions = dimensions.iter().copied().map(TensorDimension::fixed);
                let reference = ReferenceLayout::new(
                    int32_type,
                    ReferenceKind::Managed,
                    Storage::Heap(Space::Local),
                    Access::Mutable,
                    Nullability::None,
                );
                let tensor = TensorLayoutBuilder::new(
                    reference,
                    TensorFormat::dense_row_major(),
                    dimensions,
                );
                let tensor_trace = traces.insert(TraceMap::Fixed {
                    local_offsets: Box::new([0]),
                    shared_offsets: Box::new([]),
                    frame_offsets: Box::new([]),
                });

                (LayoutShapeBuilder::Tensor(tensor), tensor_trace)
            } else {
                (LayoutShapeBuilder::Struct(Vec::new()), trace)
            };
            types.push(TypeDescriptorBuilder::new(layout));
            layouts.push(LayoutBuilder::new(
                shape,
                Word::BYTE_LEN as u32,
                Word::BYTE_LEN as u32,
                layout_trace,
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

        let functions = (0..functions.len())
            .map(|index| Symbol::from_raw(index as u64))
            .zip(functions);
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
