use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::Arc;

use bytecode::{CodeBuilder, Parser, SymbolTag};
use destack_bytecode as bytecode;
use destack_core::Optional;
use destack_heap::{
    AllocationCache, AllocationPlan, DropId, Heap, HeapLimits, HeapOptions, SharedHeap,
    SharedHeapLimits, SharedHeapOptions, SharedMarkWorker,
};
use destack_memory::MemoryMap;
use destack_mir::{
    Access, Nullability, ReferenceKind, Space, TensorFormat, TensorViewFormat, TraceMap, TraceTable,
};
use destack_program as program;
use destack_program::{
    AllocationSite, BindingId, BreakpointId, CounterId, CounterSite, DispatchTableBuilder,
    DropEntry, DynamicEntry, DynamicTableBuilder, FrameLayoutBuilder, FrameLayoutId, FrameSlotId,
    FrameStateBuilder, FrameTableBuilder, FunctionBuilder, FunctionId, FunctionTableBuilder,
    InstructionStop, LayoutBuilder, LayoutId, LayoutShapeBuilder, MemoryAccess, MemorySite,
    MemoryStop, MemoryTarget, ObjectLayoutBuilder, Program, ProgramBuilder, ProgramPoint,
    ReferenceFlags, ReferenceLayout, SampleSite, SamplerId, ScalarFormat, Signature, SignatureId,
    SiteTableBuilder, StopReason, StopSet, TensorDimension, TensorLayoutBuilder,
    TensorViewLayoutBuilder, TypeDescriptorBuilder, TypeId, VirtualTableBuilder, WatchSet,
    WatchpointId, Word,
};
use destack_source::FileId;

use crate::machine::Activation;
use crate::{Machine, MachineLimits, Result};

const MEMORY_BYTES: usize = 512 * 1024 * 1024;
const MEMORY_FRAME_BYTES: usize = 16 * 1024 * 1024;
const TEST_GLOBAL_BYTES: usize = Word::BYTE_LEN;

/// Program tables used by one bytecode machine fixture.
pub(crate) struct TestProgram {
    /// Runtime bindings keyed by bytecode function name.
    bindings: HashMap<String, BindingId>,
    /// Program sites under test.
    sites: SiteTableBuilder,
    /// Virtual tables in dense runtime id order.
    virtual_tables: Vec<VirtualTableBuilder>,
    /// Dynamic tables in dense runtime id order.
    dynamic_tables: Vec<DynamicTableBuilder>,
    /// Dense dynamic table ids keyed by bytecode type pair.
    dynamic_table_ids: HashMap<(bytecode::TypeId, bytecode::TypeId), u32>,
    /// Concrete type layouts under test.
    layouts: Vec<TestLayout>,
}

/// One concrete test type layout.
#[derive(Debug, Clone, PartialEq)]
struct TestLayout {
    /// The dense test type id.
    ty: TypeId,
    /// The concrete layout shape.
    shape: LayoutShapeBuilder,
    /// The inline value byte length.
    byte_len: u32,
    /// The inline value alignment.
    alignment: u32,
}

impl TestProgram {
    /// Create empty test Program tables.
    pub(crate) fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            sites: SiteTableBuilder::new(),
            virtual_tables: Vec::new(),
            dynamic_tables: Vec::new(),
            dynamic_table_ids: HashMap::new(),
            layouts: Vec::new(),
        }
    }

    /// Attach one runtime binding to a bytecode function.
    pub(crate) fn binding(mut self, function: &str, binding: &str) -> Self {
        self.bindings
            .insert(function.to_owned(), BindingId::from_name(binding));

        self
    }

    /// Set allocation sites.
    pub(crate) fn allocations(mut self, sites: impl IntoIterator<Item = AllocationSite>) -> Self {
        self.sites = self.sites.allocations(sites);

        self
    }

    /// Set memory sites.
    pub(crate) fn memory(mut self, sites: impl IntoIterator<Item = MemorySite>) -> Self {
        self.sites = self.sites.memory(sites);

        self
    }

    /// Set profile counter sites.
    pub(crate) fn counters(mut self, sites: impl IntoIterator<Item = CounterSite>) -> Self {
        self.sites = self.sites.counters(sites);

        self
    }

    /// Set profile sample sites.
    pub(crate) fn samples(mut self, sites: impl IntoIterator<Item = SampleSite>) -> Self {
        self.sites = self.sites.samples(sites);

        self
    }

    /// Append one virtual table in dense runtime id order.
    pub(crate) fn virtual_table(mut self, ty: u32, methods: impl IntoIterator<Item = u32>) -> Self {
        let methods = methods.into_iter().map(FunctionId);
        let table = VirtualTableBuilder::new(TypeId(ty)).methods(methods);
        self.virtual_tables.push(table);

        self
    }

    /// Append one dynamic table in dense runtime id order.
    pub(crate) fn dynamic_table(
        mut self,
        concrete: u32,
        constraint: u32,
        entries: impl IntoIterator<Item = DynamicEntry>,
    ) -> Self {
        let table = self.dynamic_tables.len() as u32;
        self.dynamic_table_ids.insert(
            (bytecode::TypeId(concrete), bytecode::TypeId(constraint)),
            table,
        );
        self.dynamic_tables
            .push(DynamicTableBuilder::new(TypeId(concrete), TypeId(constraint)).entries(entries));

        self
    }

    /// Set one dense owning tensor layout.
    pub(crate) fn tensor(
        mut self,
        ty: u32,
        element: u32,
        scalar: ScalarFormat,
        space: Space,
        dimensions: impl IntoIterator<Item = u64>,
    ) -> Self {
        let scalar_byte_len = scalar.byte_len();
        let dimensions = dimensions.into_iter().map(TensorDimension::fixed);
        let tensor = TensorLayoutBuilder::new(
            space,
            TypeId(element),
            TensorFormat::dense_row_major(),
            dimensions,
        );
        self.insert_layout(TestLayout {
            ty: TypeId(ty),
            shape: LayoutShapeBuilder::Tensor(tensor),
            byte_len: Word::BYTE_LEN as u32,
            alignment: Word::BYTE_LEN as u32,
        });
        self.insert_layout(TestLayout {
            ty: TypeId(element),
            shape: LayoutShapeBuilder::Scalar(scalar),
            byte_len: scalar_byte_len as u32,
            alignment: scalar_byte_len as u32,
        });

        self
    }

    /// Set one dense borrowed tensor-view layout.
    pub(crate) fn tensor_view(
        mut self,
        ty: u32,
        element: u32,
        scalar: ScalarFormat,
        space: Space,
        dimensions: impl IntoIterator<Item = u64>,
    ) -> Self {
        let scalar_byte_len = scalar.byte_len();
        let dimensions = dimensions
            .into_iter()
            .map(TensorDimension::fixed)
            .collect::<Vec<_>>();
        let rank = dimensions.len() as u16;
        let reference = ReferenceLayout {
            pointee: TypeId(element),
            flags: ReferenceFlags::new(
                ReferenceKind::Borrowed,
                space,
                Access::Mutable,
                Nullability::None,
            ),
        };
        let tensor = TensorViewLayoutBuilder::new(
            reference,
            TypeId(element),
            TensorViewFormat::dense_row_major(),
            dimensions,
        );
        let word_count = bytecode::ValueType::tensor_view_word_count(rank)
            .expect("test tensor view rank should fit one register range");
        self.insert_layout(TestLayout {
            ty: TypeId(ty),
            shape: LayoutShapeBuilder::TensorView(tensor),
            byte_len: u32::from(word_count) * Word::BYTE_LEN as u32,
            alignment: Word::BYTE_LEN as u32,
        });
        self.insert_layout(TestLayout {
            ty: TypeId(element),
            shape: LayoutShapeBuilder::Scalar(scalar),
            byte_len: scalar_byte_len as u32,
            alignment: scalar_byte_len as u32,
        });

        self
    }

    /// Set one object layout with a virtual dispatch word.
    pub(crate) fn virtual_object(mut self, ty: u32, byte_len: u32, dispatch: u32) -> Self {
        let object = ObjectLayoutBuilder::new([]).dispatch_offset(dispatch);
        self.insert_layout(TestLayout {
            ty: TypeId(ty),
            shape: LayoutShapeBuilder::Object(object),
            byte_len,
            alignment: Word::BYTE_LEN as u32,
        });

        self
    }

    /// Set one exact Program layout for a bytecode type.
    pub(crate) fn layout(
        mut self,
        ty: u32,
        shape: LayoutShapeBuilder,
        byte_len: u32,
        alignment: u32,
    ) -> Self {
        self.insert_layout(TestLayout {
            ty: TypeId(ty),
            shape,
            byte_len,
            alignment,
        });

        self
    }

    /// Insert one concrete test layout or require its existing definition to match.
    fn insert_layout(&mut self, layout: TestLayout) {
        if let Some(current) = self.layouts.iter().find(|current| current.ty == layout.ty) {
            assert_eq!(
                current, &layout,
                "test type layout must have one definition"
            );
        } else {
            self.layouts.push(layout);
        }
    }
}

/// One bytecode machine fixture backed by a linked Program.
pub(crate) struct TestMachine {
    /// The linked Program under test.
    program: Arc<Program>,
    /// Dense linked function ids keyed by bytecode symbol name.
    function_ids: HashMap<String, FunctionId>,
    /// Dense linked type ids keyed by bytecode symbol name.
    type_ids: HashMap<String, TypeId>,
    /// The bytecode machine under test.
    machine: Machine,
    /// Runtime allocation plans indexed by Program allocation site id.
    allocation_plans: Arc<[Option<AllocationPlan>]>,
    /// Runtime state passed through binding calls.
    state: Box<()>,
    /// Worker heap used by allocation instructions.
    heap: Heap,
    /// Runtime heap used by shared allocation instructions.
    shared_heap: SharedHeap,
    /// Worker-local shared allocation cache.
    shared_cache: AllocationCache,
    /// Shared collector worker state.
    shared_mark_worker: SharedMarkWorker,
    /// Worker-local static memory.
    local_static: program::StaticSpace,
    /// Runtime-shared static memory.
    shared_static: program::StaticSpace,
}

impl TestMachine {
    /// Create one dense test program point.
    pub(crate) const fn point(function: u32, operation: u32) -> ProgramPoint {
        ProgramPoint::new(FunctionId(function), operation)
    }

    /// Create one word-sized allocation site.
    pub(crate) const fn value_allocation(
        function: u32,
        operation: u32,
        space: Space,
        ty: u32,
    ) -> AllocationSite {
        AllocationSite {
            point: Self::point(function, operation),
            space,
            result_type: TypeId(ty),
            storage_type: TypeId(ty),
            virtual_table: Optional::none(),
        }
    }

    /// Create one virtual object allocation site.
    pub(crate) const fn virtual_allocation(
        function: u32,
        operation: u32,
        space: Space,
        ty: u32,
        table: u32,
    ) -> AllocationSite {
        AllocationSite {
            point: Self::point(function, operation),
            space,
            result_type: TypeId(ty),
            storage_type: TypeId(ty),
            virtual_table: Optional::some(program::VirtualTableId(table)),
        }
    }

    /// Create one dynamically sized slice allocation site.
    pub(crate) const fn slice_allocation(
        function: u32,
        operation: u32,
        space: Space,
        element: u32,
    ) -> AllocationSite {
        AllocationSite {
            point: Self::point(function, operation),
            space,
            result_type: TypeId(element),
            storage_type: TypeId(element),
            virtual_table: Optional::none(),
        }
    }

    /// Create one dynamically sized tensor allocation site.
    pub(crate) const fn tensor_allocation(
        function: u32,
        operation: u32,
        space: Space,
        result_type: u32,
    ) -> AllocationSite {
        AllocationSite {
            point: Self::point(function, operation),
            space,
            result_type: TypeId(result_type),
            storage_type: TypeId(result_type),
            virtual_table: Optional::none(),
        }
    }

    /// Create one word-sized memory site.
    pub(crate) const fn memory(
        function: u32,
        operation: u32,
        access: MemoryAccess,
        space: Space,
    ) -> MemorySite {
        MemorySite {
            point: Self::point(function, operation),
            access,
            space,
            value_type: TypeId(0),
        }
    }

    /// Create one counter site.
    pub(crate) const fn counter(function: u32, operation: u32, counter: u32) -> CounterSite {
        CounterSite {
            point: Self::point(function, operation),
            counter: CounterId(counter),
        }
    }

    /// Create one word-sized sample site.
    pub(crate) const fn sample(function: u32, operation: u32, sampler: u32) -> SampleSite {
        SampleSite {
            point: Self::point(function, operation),
            sampler: SamplerId(sampler),
            value_type: TypeId(0),
        }
    }

    /// Create one runtime breakpoint.
    pub(crate) const fn breakpoint(
        function: u32,
        operation: u32,
        breakpoint: u64,
    ) -> InstructionStop {
        let point = Self::point(function, operation);
        let reason = StopReason::Breakpoint {
            breakpoint_id: BreakpointId::new(breakpoint),
            point,
        };

        InstructionStop::new(point, reason)
    }

    /// Create one point watchpoint.
    pub(crate) const fn watchpoint(
        function: u32,
        operation: u32,
        watchpoint: u64,
        access: MemoryAccess,
    ) -> MemoryStop {
        let point = Self::point(function, operation);

        MemoryStop::new(
            WatchpointId::new(watchpoint),
            access,
            MemoryTarget::Point(point),
        )
    }

    /// Parse and link one self-contained bytecode module.
    pub(crate) fn parse(source: &str, test: TestProgram) -> Self {
        let object = Parser::new(FileId::new(0), source)
            .parse()
            .expect("test bytecode should parse");
        assert!(
            object.constant_relocations().is_empty()
                && object.instruction_relocations().iter().all(|relocation| {
                    let symbol = relocation.symbol;

                    match symbol.tag {
                        SymbolTag::TYPE => symbol.index < object.types().len() as u32,
                        SymbolTag::GLOBAL => symbol.index < object.globals().len() as u32,
                        SymbolTag::FUNCTION => symbol.index < object.functions().len() as u32,
                        SymbolTag::CONSTANT => symbol.index < object.constants().len() as u32,
                        _ => false,
                    }
                }),
            "test bytecode must be self-contained"
        );

        // retain source-facing symbols for readable execution assertions
        let function_ids = object
            .functions()
            .iter()
            .enumerate()
            .map(|(index, function)| {
                let name = object.string(function.name).expect("test function name");

                (name.to_owned(), FunctionId(index as u32))
            })
            .collect();
        let type_ids = object
            .types()
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let name = object.string(*name).expect("test type name");

                (name.to_owned(), TypeId(index as u32))
            })
            .collect();

        // resolve dynamic implementation pairs into dense runtime table ids
        let mut code = object.code().to_vec();
        for relocation in object.dynamic_relocations() {
            let table = test.dynamic_table_ids[&(relocation.concrete, relocation.constraint)];
            let start = relocation.byte_offset as usize;
            let end = start + size_of::<u32>();
            code[start..end].copy_from_slice(&table.to_le_bytes());
        }

        // retain the exact linked bytecode arrays in one Program image
        let code = CodeBuilder::new()
            .value_types(object.value_types().iter().copied())
            .frame_slots(object.frame_slots().iter().copied())
            .constants(object.constants().iter().map(|constant| constant.value))
            .constant_bytes(object.constant_bytes())
            .bodies(object.functions().iter().map(|function| function.body))
            .code(code)
            .operation_offsets(object.operation_offsets().iter().copied());
        let memory = Arc::new(
            MemoryMap::reserve(MEMORY_BYTES, MEMORY_FRAME_BYTES)
                .expect("test memory should reserve"),
        );
        let (globals, constant_space, shared_static_space, local_static_space) =
            TestProgram::globals(&object);
        let (types, layouts, traces, drops) = test.types(&object);
        let (frames, functions, continuations) = test.frames(&object);
        let sites = test.sites.continuations(continuations);
        let program = Arc::new(
            ProgramBuilder::new(Default::default(), code)
                .frames(frames)
                .functions(functions)
                .types(types)
                .drops(drops)
                .layouts(layouts)
                .traces(traces)
                .sites(sites)
                .dispatch(
                    DispatchTableBuilder::new()
                        .virtual_tables(test.virtual_tables)
                        .dynamic_tables(test.dynamic_tables),
                )
                .globals(globals)
                .constant_space(constant_space)
                .shared_static_space(shared_static_space)
                .local_static_space(local_static_space)
                .build(),
        );

        // build the real runtime storage consumed by one activation
        let heap = Heap::new(memory.clone(), HeapLimits::default(), HeapOptions::local())
            .expect("test heap should build");
        let shared_heap = SharedHeap::new(
            memory.clone(),
            SharedHeapLimits::default(),
            SharedHeapOptions::default(),
        )
        .expect("test shared heap should build");
        let shared_cache = shared_heap.allocation_cache();
        let shared_mark_worker = shared_heap.register_mark_worker();
        let local_static = program
            .materialize_local_statics(memory.clone())
            .expect("test local statics should materialize");
        let shared_static = program
            .materialize_shared_statics(memory.clone())
            .expect("test shared statics should materialize");
        let allocation_plans = program
            .plan_allocations(heap.options(), shared_heap.options())
            .expect("test allocation plans should build")
            .into();
        let machine = Machine::new(program.clone(), memory, MachineLimits::test())
            .expect("test machine should build");

        Self {
            program,
            function_ids,
            type_ids,
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

    /// Execute one function and require normal completion.
    pub(crate) fn complete(&mut self, function: &str, arguments: &[Word]) -> Vec<Word> {
        let outcome = self
            .run(function, arguments, None, None, None)
            .unwrap_or_else(|error| panic!("{function} should execute: {error}"));

        Self::completion(function, outcome)
    }

    /// Execute one profiled function and require normal completion.
    pub(crate) fn complete_profiled(
        &mut self,
        function: &str,
        arguments: &[Word],
        profile: &mut program::Profile,
    ) -> Vec<Word> {
        let outcome = self
            .run(function, arguments, None, None, Some(profile))
            .unwrap_or_else(|error| panic!("{function} should execute: {error}"));

        Self::completion(function, outcome)
    }

    /// Execute one function and return its raw machine outcome.
    pub(crate) fn run(
        &mut self,
        function: &str,
        arguments: &[Word],
        stop_points: Option<&StopSet>,
        watch_points: Option<&WatchSet>,
        profile: Option<&mut program::Profile>,
    ) -> Result<program::Outcome<program::Continuation, Vec<Word>>> {
        let function = self.function_id(function);

        self.activation(stop_points, watch_points, profile, None)
            .run(function, arguments)
    }

    /// Execute one function and require one language yield.
    pub(crate) fn run_to_yield(
        &mut self,
        function: &str,
        arguments: &[Word],
    ) -> (program::Continuation, Vec<Word>) {
        let outcome = self
            .run(function, arguments, None, None, None)
            .unwrap_or_else(|error| panic!("{function} should execute: {error}"));
        let program::Outcome::Yielded {
            continuation,
            value,
        } = outcome
        else {
            panic!("{function} should yield");
        };

        (continuation, value)
    }

    /// Execute one function and require one debugger stop.
    pub(crate) fn run_to_stop(
        &mut self,
        function: &str,
        arguments: &[Word],
        stop_points: Option<&StopSet>,
        watch_points: Option<&WatchSet>,
    ) -> (program::Continuation, StopReason) {
        let outcome = self
            .run(function, arguments, stop_points, watch_points, None)
            .unwrap_or_else(|error| panic!("{function} should execute: {error}"));
        let program::Outcome::Stopped {
            continuation,
            reason,
        } = outcome
        else {
            panic!("{function} should stop");
        };

        (continuation, reason)
    }

    /// Resume one suspended continuation.
    pub(crate) fn resume(
        &mut self,
        continuation: program::Continuation,
        received: &[Word],
        profile: Option<&mut program::Profile>,
    ) -> Result<program::Outcome<program::Continuation, Vec<Word>>> {
        self.activation(None, None, profile, None)
            .resume(continuation, received)
    }

    /// Resume one suspended continuation and require normal completion.
    pub(crate) fn resume_to_completion(
        &mut self,
        continuation: program::Continuation,
        received: &[Word],
    ) -> Vec<Word> {
        let outcome = self
            .resume(continuation, received, None)
            .unwrap_or_else(|error| panic!("continuation should resume: {error}"));

        Self::completion("continuation", outcome)
    }

    /// Continue one stopped continuation.
    pub(crate) fn continue_execution(
        &mut self,
        continuation: program::Continuation,
        stop_points: Option<&StopSet>,
        watch_points: Option<&WatchSet>,
        profile: Option<&mut program::Profile>,
        resume_skip: Option<program::ResumeSkip>,
    ) -> Result<program::Outcome<program::Continuation, Vec<Word>>> {
        self.activation(stop_points, watch_points, profile, resume_skip)
            .continue_execution(continuation)
    }

    /// Continue one stopped continuation and require normal completion.
    pub(crate) fn continue_to_completion(
        &mut self,
        continuation: program::Continuation,
        stop_points: Option<&StopSet>,
        watch_points: Option<&WatchSet>,
        resume_skip: Option<program::ResumeSkip>,
    ) -> Vec<Word> {
        let outcome = self
            .continue_execution(continuation, stop_points, watch_points, None, resume_skip)
            .unwrap_or_else(|error| panic!("continuation should execute: {error}"));

        Self::completion("continuation", outcome)
    }

    /// Return one completed value or fail the current test.
    fn completion(
        operation: &str,
        outcome: program::Outcome<program::Continuation, Vec<Word>>,
    ) -> Vec<Word> {
        let program::Outcome::Completed { value } = outcome else {
            panic!("{operation} should complete");
        };

        value
    }

    /// Bind one clean activation to this test machine.
    fn activation<'machine, 'run>(
        &'machine mut self,
        stop_points: Option<&'run StopSet>,
        watch_points: Option<&'run WatchSet>,
        profile: Option<&'run mut program::Profile>,
        resume_skip: Option<program::ResumeSkip>,
    ) -> Activation<'machine, 'run>
    where
        'machine: 'run,
    {
        let context = NonNull::from(self.state.as_mut()).cast::<c_void>();
        let call = program::Activation {
            context,
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

        Activation::new(
            &mut self.machine,
            call,
            stop_points,
            watch_points,
            profile,
            resume_skip,
        )
    }

    /// Return the linked Program.
    pub(crate) fn program(&self) -> &Program {
        &self.program
    }

    /// Resolve one linked type id by its bytecode symbol name.
    pub(crate) fn type_id(&self, name: &str) -> TypeId {
        self.type_ids
            .get(name)
            .copied()
            .unwrap_or_else(|| panic!("undefined test type {name}"))
    }

    /// Resolve one linked function id by its bytecode symbol name.
    pub(crate) fn function_id(&self, name: &str) -> FunctionId {
        self.function_ids
            .get(name)
            .copied()
            .unwrap_or_else(|| panic!("undefined test function {name}"))
    }
}

impl TestProgram {
    /// Build one word-sized no-scan Program type for every bytecode type symbol.
    fn types(
        &self,
        object: &bytecode::Object,
    ) -> (
        Vec<TypeDescriptorBuilder>,
        Vec<LayoutBuilder>,
        TraceTable,
        Vec<DropEntry>,
    ) {
        let mut traces = TraceTable::new();
        let trace = traces.insert(TraceMap::empty());
        let mut types = Vec::with_capacity(object.types().len());
        let mut layouts = Vec::with_capacity(object.types().len());
        let mut drops = Vec::new();

        // preserve object type order so local ids are already linked ids
        for (index, name) in object.types().iter().enumerate() {
            let layout = LayoutId::new(index as u32 + 1);
            let mut descriptor = TypeDescriptorBuilder::new(layout);

            // link the compiler's generated destructor symbol when present
            let type_name = object.string(*name).expect("test type name");
            let destructor_name = format!("{type_name}.destruct");
            if let Some(function) = object
                .functions()
                .iter()
                .position(|function| object.string(function.name) == Some(&destructor_name))
            {
                let drop = DropId::from_index(drops.len() as u32);
                descriptor = descriptor.drop(drop);
                drops.push(DropEntry {
                    function: FunctionId(function as u32),
                });
            }

            types.push(descriptor);
            let layout = self
                .layouts
                .iter()
                .find(|layout| layout.ty == TypeId(index as u32));
            let layout = if let Some(layout) = layout {
                LayoutBuilder::new(
                    layout.shape.clone(),
                    layout.byte_len,
                    layout.alignment,
                    trace,
                )
            } else {
                LayoutBuilder::new(
                    LayoutShapeBuilder::Struct(Vec::new()),
                    Word::BYTE_LEN as u32,
                    Word::BYTE_LEN as u32,
                    trace,
                )
            };
            layouts.push(layout);
        }

        (types, layouts, traces, drops)
    }

    /// Build dense Program global storage from one self-contained bytecode object.
    fn globals(object: &bytecode::Object) -> (Vec<program::Global>, Vec<u8>, Vec<u8>, Vec<u8>) {
        let mut globals = Vec::with_capacity(object.globals().len());
        let mut constant_space = Vec::new();
        let mut shared_static_space = Vec::new();
        let mut local_static_space = Vec::new();

        // preserve object global order so local ids are already linked ids
        for source in object.globals() {
            let (location, bytes) = match source.location {
                bytecode::GlobalLocation::CONSTANT => {
                    (program::GlobalLocation::Constant, &mut constant_space)
                }
                bytecode::GlobalLocation::SHARED_STATIC => (
                    program::GlobalLocation::SharedStatic,
                    &mut shared_static_space,
                ),
                bytecode::GlobalLocation::LOCAL_STATIC => (
                    program::GlobalLocation::LocalStatic,
                    &mut local_static_space,
                ),
                _ => unreachable!("the parser only produces defined global locations"),
            };
            let offset = bytes.len();
            bytes.resize(offset + TEST_GLOBAL_BYTES, 0);
            globals.push(program::Global::new(
                location,
                offset,
                TEST_GLOBAL_BYTES,
                program::TypeId(source.ty.0),
                source.is_mutable(),
            ));
        }

        (
            globals,
            constant_space,
            shared_static_space,
            local_static_space,
        )
    }

    /// Build canonical Program frame rows from one self-contained bytecode object.
    fn frames(
        &self,
        object: &bytecode::Object,
    ) -> (
        FrameTableBuilder,
        FunctionTableBuilder,
        Vec<program::ContinuationSite>,
    ) {
        let mut layouts = Vec::with_capacity(object.functions().len());
        let mut states = Vec::new();
        let mut functions = Vec::with_capacity(object.functions().len());
        let mut continuations = Vec::new();

        // preserve function and slot order across bytecode and Program tables
        for (function_index, function) in object.functions().iter().enumerate() {
            let frame_slots = function.body.frame_slots(object.frame_slots());
            let mut byte_len = 0_u32;
            let slots = frame_slots
                .iter()
                .map(|slot| {
                    let ty = TypeId(slot.ty.0);
                    let layout = self.layouts.iter().find(|layout| layout.ty == ty);
                    let slot_byte_len = layout
                        .map(|layout| layout.byte_len)
                        .unwrap_or(Word::BYTE_LEN as u32);
                    let alignment = layout
                        .map(|layout| layout.alignment)
                        .unwrap_or(Word::BYTE_LEN as u32);
                    byte_len = byte_len.next_multiple_of(alignment);
                    let slot = program::FrameSlot {
                        offset: byte_len,
                        byte_len: slot_byte_len,
                        alignment: alignment as u16,
                        ty,
                    };
                    byte_len += slot_byte_len;

                    slot
                })
                .collect::<Vec<_>>();
            let frame_alignment = slots
                .iter()
                .map(|slot| u32::from(slot.alignment))
                .max()
                .unwrap_or(Word::BYTE_LEN as u32);
            let byte_len = byte_len.next_multiple_of(frame_alignment);
            let frame_layout = FrameLayoutId(function_index as u32);
            layouts.push(FrameLayoutBuilder::new(byte_len, frame_alignment).slots(slots));
            let name = object.string(function.name).expect("test function name");
            let mut entry =
                FunctionBuilder::new(function.name, SignatureId(0)).frame_layout(frame_layout);
            if let Some(binding) = self.bindings.get(name).copied() {
                entry = entry.binding(binding);
            }
            functions.push(entry);

            // materialize every operation used by continuation and debugger tests
            let operation_count = function
                .body
                .operation_offsets(object.operation_offsets())
                .len();
            let state_start = states.len() as u32;
            let live_slots = (0..frame_slots.len())
                .map(|slot| FrameSlotId(slot as u32))
                .collect::<Vec<_>>();
            for operation in 0..operation_count {
                let point = ProgramPoint::new(FunctionId(function_index as u32), operation as u32);
                states.push(
                    FrameStateBuilder::new(point, frame_layout).live_slots(live_slots.clone()),
                );
            }

            // connect each yield state to its normal and unwind destinations
            for operation in 0..operation_count {
                let instruction = object
                    .instruction(
                        bytecode::FunctionId(function_index as u32),
                        operation as u32,
                    )
                    .expect("test operation should decode")
                    .expect("test operation should exist");
                if instruction.opcode() != bytecode::Opcode::YIELD {
                    continue;
                }
                let frame_state = program::FrameStateId(state_start + operation as u32);
                let point = ProgramPoint::new(FunctionId(function_index as u32), operation as u32);

                // project encoded branch displacements into engine-neutral Program points
                let mut operands = instruction.operands();
                let _received = operands.range().expect("yield result range");
                let _yielded = operands.range().expect("yielded value range");
                let _yielded_type = operands.value_type().expect("yielded value type");
                let resume = operands.i32().expect("yield resume branch");
                let unwind = operands.i32().expect("yield unwind branch");
                let instruction_offset = function
                    .body
                    .operation_offset(object.operation_offsets(), operation as u32)
                    .expect("yield operation offset");
                let instruction_end = instruction_offset.0 + instruction.byte_len() as u32;
                let resume_offset =
                    bytecode::CodeOffset(instruction_end.wrapping_add_signed(resume));
                let unwind_offset =
                    bytecode::CodeOffset(instruction_end.wrapping_add_signed(unwind));
                let offsets = function.body.operation_offsets(object.operation_offsets());
                let resume = offsets
                    .binary_search(&resume_offset)
                    .expect("resume operation") as u32;
                let unwind = offsets
                    .binary_search(&unwind_offset)
                    .expect("unwind operation") as u32;
                continuations.push(program::ContinuationSite {
                    point,
                    resume: ProgramPoint::new(FunctionId(function_index as u32), resume),
                    unwind: Some(ProgramPoint::new(FunctionId(function_index as u32), unwind))
                        .into(),
                    frame_state,
                    yielded_type: TypeId(0),
                    resumed_type: TypeId(0),
                });
            }
        }

        let frames = FrameTableBuilder::new().layouts(layouts).states(states);
        let functions = FunctionTableBuilder::new()
            .signatures([Signature {
                parameters: Vec::new(),
                result: TypeId(0),
            }])
            .functions(functions);
        (frames, functions, continuations)
    }
}
