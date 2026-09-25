use std::sync::Arc;

use bytecode::{CodeBuilder, Parser, RelocationTag};
use tspp_bytecode as bytecode;
use tspp_core::{StringId, StringPool};
use tspp_heap::{
    AllocationCache, AllocationPlan, Heap, HeapLimits, HeapOptions, SharedHeap, SharedHeapLimits,
    SharedHeapOptions, SharedMarkWorker,
};
use tspp_memory::{MemoryMap, MemoryRange};
use tspp_mir::{TraceMap, TraceTable};
use tspp_program as program;
use tspp_program::{
    FrameTableBuilder, FunctionBuilder, FunctionId, FunctionTableBuilder, LayoutBuilder, LayoutId,
    LayoutShapeBuilder, ProgramBuilder, ScalarFormat, Signature, SignatureId, SiteTableBuilder,
    Symbol, TypeDescriptorBuilder, TypeFingerprint, TypeId, TypeTableBuilder, Value, Word,
};
use tspp_source::FileId;
use tspp_vm::{Error, Machine, MachineLimits, Result};

/// Reserved address-space byte length for direct benchmark execution.
const MEMORY_BYTES: usize = 512 * 1024 * 1024;
/// Memory frame byte length for direct benchmark execution.
const MEMORY_FRAME_BYTES: usize = 16 * 1024 * 1024;

/// One direct-bytecode benchmark runtime.
pub(crate) struct Runtime {
    /// Immutable Program retained by the activation.
    program: Arc<program::Program>,
    /// Reusable execution fiber under measurement.
    fiber: tspp_vm::Fiber,
    /// Bytecode machine under measurement.
    machine: Machine,
    /// Runtime allocation plans indexed by Program allocation site id.
    allocation_plans: Arc<[AllocationPlan]>,
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
    /// Worker-local static bytes.
    local_static: program::StaticSpace,
    /// Runtime-shared static bytes.
    shared_static: program::StaticSpace,
    /// The request word the machine polls at safepoints.
    handshake: program::Handshake,
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
}

impl Runtime {
    /// Build one direct-bytecode benchmark runtime.
    pub(crate) fn parse(source: &str) -> Self {
        let object = Parser::new(FileId::from_source_bytes(source.as_bytes()), source)
            .parse()
            .expect("benchmark bytecode should parse");
        let program = Arc::new(Self::program(&object));
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
        let (constant_space, shared_static) = program
            .materialize_runtime_statics(memory.clone())
            .expect("benchmark runtime statics should materialize");
        let local_static = program
            .materialize_local_statics(memory.clone(), &constant_space, &shared_static)
            .expect("benchmark local statics should materialize");
        shared_heap.set_constant_range(MemoryRange {
            offset: constant_space.offset(),
            byte_len: constant_space.byte_len(),
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
            handshake: program::Handshake::new(),
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
                constants: &self.constant_space,
                handshake: &self.handshake,
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
    fn program(object: &bytecode::Object) -> program::Program {
        let (strings, string_ids, functions) = Self::functions(object);
        let code = Self::code(object);
        let (types, layouts, traces) = Self::types();
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
            .sites(SiteTableBuilder::new())
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

    /// Build the void and signed 32-bit Program types used by benchmarks.
    fn types() -> (Vec<TypeDescriptorBuilder>, Vec<LayoutBuilder>, TraceTable) {
        let mut traces = TraceTable::new();
        let trace = traces.insert(TraceMap::empty());
        let mut types = Vec::with_capacity(2);
        let mut layouts = Vec::with_capacity(2);

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
    fn functions(object: &bytecode::Object) -> (StringPool, Vec<StringId>, FunctionTableBuilder) {
        let strings = StringPool::new();
        let int32_type = TypeId(1);
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
}
