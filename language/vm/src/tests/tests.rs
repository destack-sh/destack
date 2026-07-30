use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use destack_compiler::ProgramLinker;
use destack_core::{SectionDirectory, SectionImage, SectionPacker, SectionStorage, StringPool};
use destack_heap::{
    AllocationCache, AllocationPlan, AllocationShape, GcStats, Heap, HeapError, HeapLimits,
    HeapOptions, HeapReference, SharedHeap, SharedHeapLimits, SharedHeapOptions, SharedMarkWorker,
    TraceTable, TraceView,
};
use destack_memory::MemoryMap;
use destack_mir as mir;
use destack_program::{
    Layout, LayoutShape, Profile, ProfileOptions, Program, StaticSpace, StopReason, TypeId, Value,
    WatchSet,
};
use destack_source::{DiagnosticSeverity, File, FileId, FileType, PackageId, Uri};
use mir::parse::{ParseOptions, Parser};
use mir::{
    DropTable, LocalNodeId, TargetLayout, TensorDimension, TraceMap, Type, VariantCaseLayout,
    VariantEncoding, VariantLayout,
};

use crate::diagnostic::{Error, RuntimeResult};
use crate::{Continuation, Machine, MachineOptions, Outcome};
use destack_program::vm::{Cell, encode_cell_bytes, tensor_element_count};

/// The virtual heap-space width used by ordinary VM tests.
const TEST_LOCAL_SPACE_SIZE_BYTES: usize = 16 * 1024 * 1024;

/// Section-backed trace table used by VM tests.
struct TestTraceTable {
    /// Packed section directory.
    sections: SectionDirectory,
    /// Packed section storage.
    storage: SectionStorage,
    /// Packed heap trace table.
    traces: TraceTable,
}

/// Return the shared empty trace table for VM tests.
pub(crate) fn trace_view() -> TraceView<'static> {
    static TRACE_FIXTURE: OnceLock<TestTraceTable> = OnceLock::new();

    TRACE_FIXTURE.get_or_init(TestTraceTable::new).view()
}

impl TestTraceTable {
    /// Build one empty section-backed trace table.
    fn new() -> Self {
        let mut sections = SectionPacker::new();
        let traces = TraceTable::pack(&mut sections, &mir::TraceTable::new());
        let (sections, storage) = sections.finish();

        Self {
            sections,
            storage,
            traces,
        }
    }

    /// Return the packed trace view.
    fn view(&'static self) -> TraceView<'static> {
        let sections = SectionImage::load(&self.sections, &self.storage)
            .expect("test trace sections should load");

        self.traces.view(sections)
    }
}

/// The machine and authoritative heap used by one test runtime.
pub(crate) struct TestMachine {
    /// The MIR tree used to build this fixture.
    pub tree: mir::Tree,
    /// Program type ids keyed by MIR type id.
    type_ids: HashMap<LocalNodeId<Type>, TypeId>,
    /// MIR type ids keyed by program type id.
    type_nodes: Vec<LocalNodeId<Type>>,
    /// The VM machine under test.
    pub machine: Machine,
    /// The worker-local static space used by the machine.
    pub local_static: StaticSpace,
    /// The runtime-shared static space used by the machine.
    pub shared_static: StaticSpace,
    /// The authoritative heap for the machine.
    pub heap: Heap,
    /// The runtime-shared heap for the machine.
    pub shared_heap: SharedHeap,
    /// The shared mark worker used by this machine.
    pub shared_mark_worker: SharedMarkWorker,
    /// The worker-local shared allocation cache.
    pub shared_cache: AllocationCache,
}

/// Create one world memory map for VM tests.
pub(crate) fn create_test_memory() -> Arc<MemoryMap> {
    Arc::new(
        MemoryMap::reserve(
            TEST_LOCAL_SPACE_SIZE_BYTES,
            test_local_heap_options().page_size_bytes,
        )
        .expect("test memory should build"),
    )
}

/// Create one local test heap in world memory.
pub(crate) fn create_test_heap(memory: Arc<MemoryMap>) -> Heap {
    let options = test_local_heap_options();

    Heap::new(memory, HeapLimits::default(), options).expect("test heap should build")
}

/// Create one shared test heap in world memory.
pub(crate) fn create_test_shared_heap(memory: Arc<MemoryMap>) -> SharedHeap {
    let options = test_shared_heap_options();

    SharedHeap::new(memory, SharedHeapLimits::default(), options)
        .expect("test shared heap should build")
}

/// Create one test machine with one explicit physical variant layout.
pub(crate) fn create_machine_with_variant_layout(
    mir_text: &str,
    encoding: VariantEncoding,
    payload_offsets: &[u32],
    size: u32,
    alignment: u32,
) -> TestMachine {
    let (tree, target_layout, types, mut layouts, dispatch, drops, strings) =
        parse_test_mir(mir_text, ParseOptions::default());
    let variants = tree
        .iter_nodes::<Type>()
        .filter_map(|(ty, value)| match value {
            Type::Variant { .. } => Some((ty, value.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        !variants.is_empty(),
        "test input should contain a variant type"
    );

    // attach the explicit representation to every parsed occurrence
    for (variant_type, variant) in variants {
        let Type::Variant {
            discriminant,
            storage,
            cases,
            ..
        } = variant
        else {
            unreachable!();
        };
        assert_eq!(cases.len(), payload_offsets.len());
        let cases = cases
            .iter()
            .zip(payload_offsets)
            .map(|(case, payload_offset)| VariantCaseLayout {
                discriminant: match case.discriminant {
                    mir::Constant::Int { value, .. } => (value as u128).into(),
                    mir::Constant::UInt { value, .. } => value.into(),
                    _ => panic!("variant test discriminants should be integer"),
                },
                ty: case.ty,
                payload_offset: *payload_offset,
            })
            .collect();
        let layout = mir::Layout {
            shape: mir::LayoutShape::Variant(VariantLayout {
                discriminant,
                storage,
                encoding,
                cases,
            }),
            size,
            alignment,
            trace_map: TraceMap::empty(),
        };
        let layout_id = layouts.insert(layout);
        layouts.set_layout_id(variant_type, layout_id);
    }

    TestMachine::build(
        tree,
        target_layout,
        types,
        layouts,
        dispatch,
        drops,
        strings,
    )
}

/// Build one explicit local heap allocation plan for VM tests.
pub(crate) fn local_allocation_plan(heap: &Heap, shape: &AllocationShape) -> AllocationPlan {
    heap.options().allocation_plan(shape)
}

/// Build one explicit shared heap allocation plan for VM tests.
pub(crate) fn shared_allocation_plan(heap: &SharedHeap, shape: &AllocationShape) -> AllocationPlan {
    heap.options().allocation_plan(shape)
}

/// Allocate one zeroed local heap payload for VM tests.
pub(crate) fn allocate_local_zeroed(
    heap: &mut Heap,
    shape: AllocationShape,
) -> destack_heap::HeapResult<HeapReference> {
    let plan = local_allocation_plan(heap, &shape);

    heap.allocate_zeroed(plan, &shape.trace_map)
}

/// Allocate one byte-initialized local heap payload for VM tests.
pub(crate) fn allocate_local_bytes(
    heap: &mut Heap,
    shape: AllocationShape,
    bytes: &[u8],
) -> destack_heap::HeapResult<HeapReference> {
    let plan = local_allocation_plan(heap, &shape);

    heap.allocate_bytes(plan, &shape.trace_map, bytes)
}

/// Create heap options for ordinary local VM tests.
fn test_local_heap_options() -> HeapOptions {
    HeapOptions::local()
}

/// Create heap options for ordinary shared VM tests.
pub(crate) fn test_shared_heap_options() -> SharedHeapOptions {
    SharedHeapOptions::default()
}

/// Create machine options matching ordinary VM test heaps.
fn test_machine_options() -> MachineOptions {
    MachineOptions {
        heap: test_local_heap_options(),
        shared_heap: test_shared_heap_options(),
        ..MachineOptions::test()
    }
}

/// Return the package id used by VM tests.
fn test_package_id() -> PackageId {
    PackageId::from_uri(&Uri::logical("test/vm"))
}

/// Parse one MIR test program and preserve all executable tables.
fn parse_test_mir(
    mir_text: &str,
    options: ParseOptions,
) -> (
    mir::Tree,
    TargetLayout,
    mir::TypeTable,
    mir::LayoutTable,
    mir::DispatchTable,
    DropTable,
    StringPool,
) {
    let file_id = FileId::from_source_bytes(mir_text.as_bytes());
    let file = File::from_text(
        file_id,
        "test.mir".to_string(),
        Uri::from_string("test.mir"),
        None,
        FileType::Text,
        mir_text.to_string(),
    );
    let parsed = Parser::parse(&file, options).expect("test MIR should be text");
    let (tree, target_layout, types, layouts, dispatch, drops, _, _, _, strings, diagnostics) =
        parsed.into_parts();

    if diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
        let diagnostic = diagnostics
            .iter()
            .next()
            .unwrap_or_else(|| panic!("parser emitted an empty error set"));

        panic!("failed to parse MIR: {diagnostic:?}");
    }

    (
        tree,
        target_layout,
        types,
        layouts,
        dispatch,
        drops,
        strings,
    )
}

/// Build one executable program from MIR test input.
fn build_test_program(
    tree: mir::Tree,
    target_layout: TargetLayout,
    types: mir::TypeTable,
    layouts: mir::LayoutTable,
    dispatch: mir::DispatchTable,
    drops: DropTable,
    strings: StringPool,
) -> Program {
    let options = test_machine_options();

    ProgramLinker::new(
        test_package_id(),
        tree,
        target_layout,
        types,
        layouts,
        dispatch,
        drops,
        strings,
        options.heap,
        options.shared_heap,
    )
    .build()
    .unwrap_or_else(|error| panic!("failed to lower test program: {error:?}"))
}

/// Assign test-local type id maps for one MIR tree.
fn type_maps(tree: &mir::Tree) -> (HashMap<LocalNodeId<Type>, TypeId>, Vec<LocalNodeId<Type>>) {
    let type_nodes = tree
        .iter_nodes::<Type>()
        .map(|(ty, _)| ty)
        .collect::<Vec<_>>();
    let type_ids = type_nodes
        .iter()
        .copied()
        .enumerate()
        .map(|(index, ty)| (ty, TypeId::from(index as u32)))
        .collect();

    (type_ids, type_nodes)
}

impl TestMachine {
    /// Build one test machine from MIR text.
    pub(crate) fn new(mir_text: &str) -> Self {
        let (tree, target_layout, types, layouts, dispatch, drops, strings) =
            parse_test_mir(mir_text, ParseOptions::default());

        Self::build(
            tree,
            target_layout,
            types,
            layouts,
            dispatch,
            drops,
            strings,
        )
    }

    /// Build one test machine with one dynamic method table.
    pub(crate) fn dynamic(
        mir_text: &str,
        concrete_name: &str,
        constraint_name: &str,
        method_names: &[&str],
    ) -> Self {
        let (tree, target_layout, types, layouts, mut dispatch, drops, strings) =
            parse_test_mir(mir_text, ParseOptions::default());

        // resolve the concrete and constraint types by their MIR display names
        let concrete = Self::named_type(&tree, &types, &strings, concrete_name);
        let constraint = Self::named_type(&tree, &types, &strings, constraint_name);

        // resolve dynamic methods in slot order
        let mut entries = Vec::with_capacity(method_names.len());
        for method_name in method_names {
            let function = Self::named_function(&tree, &strings, method_name);
            entries.push(mir::DynamicEntry::Function { function });
        }
        dispatch.insert_dynamic_table(mir::DynamicTable {
            concrete,
            constraint,
            entries,
            names: Vec::new(),
        });

        Self::build(
            tree,
            target_layout,
            types,
            layouts,
            dispatch,
            drops,
            strings,
        )
    }

    /// Build one test machine with one destructor.
    pub(crate) fn with_drop(mir_text: &str, type_name: &str, function_name: &str) -> Self {
        let (tree, target_layout, types, layouts, dispatch, mut drops, strings) =
            parse_test_mir(mir_text, ParseOptions::default());
        let ty = Self::named_type(&tree, &types, &strings, type_name);
        let function = Self::named_function(&tree, &strings, function_name);
        drops.set_destructor(ty, function);

        Self::build(
            tree,
            target_layout,
            types,
            layouts,
            dispatch,
            drops,
            strings,
        )
    }

    /// Build one test machine from parsed MIR state.
    fn build(
        tree: mir::Tree,
        target_layout: TargetLayout,
        types: mir::TypeTable,
        layouts: mir::LayoutTable,
        dispatch: mir::DispatchTable,
        drops: DropTable,
        strings: StringPool,
    ) -> Self {
        let (type_ids, type_nodes) = type_maps(&tree);
        let source_tree = tree.clone();

        let program = build_test_program(
            tree,
            target_layout,
            types,
            layouts,
            dispatch,
            drops,
            strings,
        );
        let memory = create_test_memory();
        let program = Arc::new(program);
        let local_static = program
            .materialize_local_statics(memory.clone())
            .expect("local test statics should build");
        let shared_static = program
            .materialize_shared_statics(memory.clone())
            .expect("shared test statics should build");
        let machine = Machine::new(program, memory.clone(), test_machine_options())
            .unwrap_or_else(|error| panic!("failed to initialize machine: {error}"));

        let heap = create_test_heap(memory.clone());
        let shared_heap = create_test_shared_heap(memory);
        let shared_mark_worker = shared_heap.register_mark_worker();
        let shared_cache = shared_heap.allocation_cache();

        machine
            .require_heap_compatibility(&heap, &shared_heap)
            .unwrap_or_else(|error| panic!("failed to initialize machine globals: {error}"));

        Self {
            tree: source_tree,
            type_ids,
            type_nodes,
            machine,
            local_static,
            shared_static,
            heap,
            shared_heap,
            shared_mark_worker,
            shared_cache,
        }
    }

    /// Return one MIR type by its display name.
    fn named_type(
        tree: &mir::Tree,
        types: &mir::TypeTable,
        strings: &StringPool,
        expected: &str,
    ) -> mir::TypeId {
        for (ty, _) in tree.iter_nodes::<mir::Type>() {
            let Some(name) = types.display_name(ty) else {
                continue;
            };

            if strings.get(name) == expected {
                return ty;
            }
        }

        panic!("missing test type '{expected}'")
    }

    /// Return one MIR function by its name.
    fn named_function(tree: &mir::Tree, strings: &StringPool, expected: &str) -> mir::FunctionId {
        for (function, node) in tree.iter_nodes::<mir::Function>() {
            if strings.get(node.name) == expected {
                return function;
            }
        }

        panic!("missing test function '{expected}'")
    }

    /// Return the MIR type for one program type.
    pub(crate) fn mir_type(&self, ty: TypeId) -> mir::LocalNodeId<mir::Type> {
        self.type_nodes
            .get(ty.index())
            .copied()
            .unwrap_or_else(|| panic!("missing MIR type id for program type {ty:?}"))
    }

    /// Return the program type for one MIR type.
    pub(crate) fn program_type(&self, ty: mir::LocalNodeId<mir::Type>) -> TypeId {
        self.type_ids[&ty]
    }

    /// Resolve one function parameter type by name and position.
    pub(crate) fn parameter_type(&self, function: &str, argument_index: usize) -> TypeId {
        let function_id = self
            .machine
            .function_id_by_name(function)
            .unwrap_or_else(|_| panic!("missing function '{function}'"));
        let parameters = self
            .machine
            .program
            .function_parameters(function_id)
            .unwrap_or_else(|| panic!("missing function tables for '{function}'"));
        parameters
            .get(argument_index)
            .unwrap_or_else(|| panic!("missing argument {argument_index} for '{function}'"))
            .to_owned()
    }

    /// Materialize one value for the given program type.
    pub(crate) fn materialize_value_for_type(&mut self, ty: TypeId, values: Vec<Cell>) -> Cell {
        // tensors are handles to heap payloads
        let mir_type = self.tree.repr_type(self.mir_type(ty));
        let ty = self.program_type(mir_type);

        if matches!(self.tree.get(mir_type), Type::Tensor { .. }) {
            return self.materialize_tensor_for_type(ty, values);
        }

        let layout = self
            .machine
            .layout(ty)
            .unwrap_or_else(|| panic!("missing layout for type {ty:?}"));

        // scalars travel directly as VM cells
        if self.machine.program.is_cell_type(ty) {
            assert_eq!(values.len(), 1, "scalar materialization expects one value");

            return values[0];
        }

        let bytes = materialize_value_bytes(&self.machine, &self.heap, layout, &values);
        let layout_id = self
            .machine
            .layout_id_for_type(ty)
            .unwrap_or_else(|| panic!("missing layout id for type {ty:?}"));
        let shape = self
            .machine
            .allocation_shape(layout_id)
            .unwrap_or_else(|error| panic!("failed to resolve allocation shape: {error}"));
        let reference = allocate_local_bytes(&mut self.heap, shape, &bytes)
            .unwrap_or_else(|error| panic!("failed to allocate materialized value: {error}"));

        Cell::heap_reference(reference)
    }

    /// Materialize one tensor value as a local heap payload.
    fn materialize_tensor_for_type(&mut self, ty: TypeId, values: Vec<Cell>) -> Cell {
        let mir_type = self.tree.repr_type(self.mir_type(ty));
        let ty = self.program_type(mir_type);

        let Type::Tensor { element, shape, .. } = self.tree.get(mir_type) else {
            panic!("type {ty:?} is not a tensor");
        };
        let element = self.program_type(*element);
        let shape = static_tensor_shape(shape);
        let element_count = tensor_element_count(&shape);
        assert_eq!(
            values.len(),
            element_count,
            "tensor materialization expects one value per element"
        );

        let element_layout = self
            .machine
            .layout(element)
            .unwrap_or_else(|| panic!("missing tensor element layout for type {element:?}"));
        let stride = align_to(element_layout.byte_len(), element_layout.alignment as usize);
        let byte_len = stride
            .checked_mul(element_count)
            .expect("tensor materialization byte length overflow");
        let mut bytes = vec![0u8; byte_len];

        for (index, value) in values.iter().copied().enumerate() {
            let start = stride * index;
            write_materialized_value(
                &self.machine,
                &self.heap,
                element,
                value,
                &mut bytes[start..],
            );
        }

        let trace_map = TraceMap::empty();
        let shape =
            AllocationShape::new(byte_len, element_layout.alignment as usize, None, trace_map);
        let reference = allocate_local_bytes(&mut self.heap, shape, &bytes)
            .unwrap_or_else(|error| panic!("failed to allocate materialized tensor: {error}"));

        Cell::heap_reference(reference)
    }

    /// Run one MIR function by name with the given arguments.
    pub(crate) fn run_function_by_name(
        &mut self,
        function: &str,
        arguments: &[Value],
    ) -> RuntimeResult<Value> {
        let function = self.machine.function_id_by_name(function)?;

        self.machine.run_function(
            &mut self.local_static,
            &mut self.shared_static,
            &mut self.heap,
            &self.shared_heap,
            &mut self.shared_cache,
            &self.shared_mark_worker,
            None,
            None,
            None,
            function,
            arguments,
        )
    }

    /// Run one MIR function by name while recording a runtime profile.
    pub(crate) fn run_function_by_name_profiled(
        &mut self,
        function: &str,
        arguments: &[Value],
        profile: &mut Profile,
    ) -> RuntimeResult<Value> {
        let function = self.machine.function_id_by_name(function)?;

        self.machine.run_function(
            &mut self.local_static,
            &mut self.shared_static,
            &mut self.heap,
            &self.shared_heap,
            &mut self.shared_cache,
            &self.shared_mark_worker,
            None,
            None,
            Some(profile),
            function,
            arguments,
        )
    }

    /// Create one empty runtime profile for this machine program.
    pub(crate) fn empty_profile(&self) -> Profile {
        self.empty_profile_with_options(ProfileOptions::STANDARD)
    }

    /// Create one empty runtime profile with explicit options.
    pub(crate) fn empty_profile_with_options(&self, options: ProfileOptions) -> Profile {
        Profile::new(self.machine.program(), options)
    }

    /// Run one MIR function by name with yield support.
    pub(crate) fn run_function_by_name_yielding(
        &mut self,
        function: &str,
        arguments: &[Value],
    ) -> RuntimeResult<Outcome> {
        let function = self.machine.function_id_by_name(function)?;

        self.machine.run_function_yielding(
            &mut self.local_static,
            &mut self.shared_static,
            &mut self.heap,
            &self.shared_heap,
            &mut self.shared_cache,
            &self.shared_mark_worker,
            None,
            None,
            None,
            function,
            arguments,
        )
    }

    /// Run one MIR function by name with yield support and profile recording.
    pub(crate) fn run_function_by_name_yielding_profiled(
        &mut self,
        function: &str,
        arguments: &[Value],
        profile: &mut Profile,
    ) -> RuntimeResult<Outcome> {
        let function = self.machine.function_id_by_name(function)?;

        self.machine.run_function_yielding(
            &mut self.local_static,
            &mut self.shared_static,
            &mut self.heap,
            &self.shared_heap,
            &mut self.shared_cache,
            &self.shared_mark_worker,
            None,
            None,
            Some(profile),
            function,
            arguments,
        )
    }

    /// Run one MIR function by name with watchpoints.
    pub(crate) fn run_function_by_name_watched(
        &mut self,
        function: &str,
        arguments: &[Value],
        watch_points: &WatchSet,
    ) -> RuntimeResult<Outcome> {
        let function = self.machine.function_id_by_name(function)?;

        self.machine.run_function_yielding(
            &mut self.local_static,
            &mut self.shared_static,
            &mut self.heap,
            &self.shared_heap,
            &mut self.shared_cache,
            &self.shared_mark_worker,
            None,
            Some(watch_points),
            None,
            function,
            arguments,
        )
    }

    /// Run one MIR function by name with frame cells.
    pub(crate) fn run_frame_function_by_name(
        &mut self,
        function: &str,
        arguments: &[Cell],
    ) -> RuntimeResult<Value> {
        let function = self.machine.function_id_by_name(function)?;

        self.machine.run_function_cells(
            &mut self.local_static,
            &mut self.shared_static,
            &mut self.heap,
            &self.shared_heap,
            &mut self.shared_cache,
            &self.shared_mark_worker,
            None,
            None,
            None,
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
        self.machine.resume(
            &mut self.local_static,
            &mut self.shared_static,
            &mut self.heap,
            &self.shared_heap,
            &mut self.shared_cache,
            &self.shared_mark_worker,
            None,
            None,
            None,
            continuation,
            resume_value,
        )
    }

    /// Resume one yielded continuation with profile recording.
    pub(crate) fn resume_profiled(
        &mut self,
        continuation: Continuation,
        resume_value: Value,
        profile: &mut Profile,
    ) -> RuntimeResult<Outcome> {
        self.machine.resume(
            &mut self.local_static,
            &mut self.shared_static,
            &mut self.heap,
            &self.shared_heap,
            &mut self.shared_cache,
            &self.shared_mark_worker,
            None,
            None,
            Some(profile),
            continuation,
            resume_value,
        )
    }

    /// Continue one stopped continuation.
    pub(crate) fn continue_continuation(
        &mut self,
        continuation: Continuation,
    ) -> RuntimeResult<Outcome> {
        self.machine.continue_continuation(
            &mut self.local_static,
            &mut self.shared_static,
            &mut self.heap,
            &self.shared_heap,
            &mut self.shared_cache,
            &self.shared_mark_worker,
            None,
            None,
            None,
            None,
            continuation,
        )
    }

    /// Continue one stopped continuation with watchpoints.
    pub(crate) fn continue_continuation_watched(
        &mut self,
        continuation: Continuation,
        watch_points: &WatchSet,
    ) -> RuntimeResult<Outcome> {
        self.machine.continue_continuation(
            &mut self.local_static,
            &mut self.shared_static,
            &mut self.heap,
            &self.shared_heap,
            &mut self.shared_cache,
            &self.shared_mark_worker,
            None,
            Some(watch_points),
            None,
            None,
            continuation,
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
        self.machine
            .visit_root_slots(&mut self.local_static, continuations, &mut |slot| {
                let root = slot.load()?;
                destack_heap::RootSink::push(&mut shared_roots, root);

                Ok(())
            })
            .expect("failed to collect shared roots");
        let program = self.machine.program_handle();
        let mut heap_roots =
            |visit: &mut dyn FnMut(destack_heap::RootSlot<'_>) -> destack_heap::HeapResult<()>| {
                self.machine
                    .visit_root_slots(&mut self.local_static, continuations, visit)
                    .expect("failed to collect mutable root slots");

                Ok::<(), HeapError>(())
            };
        let mut stats = self
            .heap
            .collect_full(&mut heap_roots, program.trace_view(), &mut |_| {
                Ok::<(), HeapError>(())
            })
            .expect("failed to collect heap");

        // publish this worker's shared allocations before tracing shared roots
        self.shared_heap
            .flush_allocation_cache(&mut self.shared_cache);
        let shared_stats = self
            .shared_heap
            .collect_full(&shared_roots, program.trace_view(), &mut |_| {
                Ok::<(), HeapError>(())
            })
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
    machine: &Machine,
    heap: &Heap,
    layout: &Layout,
    values: &[Cell],
) -> Vec<u8> {
    let mut bytes = vec![0u8; layout.byte_len()];

    // fields
    if let Some(field_count) = machine.program.layout_field_count(layout) {
        assert_eq!(
            values.len(),
            field_count,
            "field materialization expects one value per field"
        );
        for (index, value) in values.iter().copied().enumerate() {
            let field = machine
                .program
                .layout_field_at(layout, index as u32)
                .unwrap_or_else(|| panic!("missing field {index} for layout {layout:?}"));
            write_materialized_value(
                machine,
                heap,
                field.ty,
                value,
                &mut bytes[field.offset as usize..],
            );
        }

        return bytes;
    }

    // elements
    let elements = match &layout.shape {
        LayoutShape::Array(elements) | LayoutShape::Vector(elements) => elements,
        _ => panic!("layout {layout:?} is not materializable as an aggregate"),
    };
    let element_count = elements.count;
    assert_eq!(
        values.len(),
        element_count as usize,
        "element materialization expects one value per element"
    );
    for (index, value) in values.iter().copied().enumerate() {
        let start = elements.stride as usize * index;
        write_materialized_value(machine, heap, elements.element, value, &mut bytes[start..]);
    }

    bytes
}
/// Write one materialized field or element value.
fn write_materialized_value(
    machine: &Machine,
    heap: &Heap,
    ty: TypeId,
    value: Cell,
    destination: &mut [u8],
) {
    let layout = machine
        .layout(ty)
        .unwrap_or_else(|| panic!("missing layout for nested type {ty:?}"));

    // scalar values encode inline
    if is_cell_type(machine, ty) {
        let cell_layout = machine
            .program
            .cell_layout(ty)
            .unwrap_or_else(|| panic!("missing cell layout for nested type {ty:?}"));
        let encoded = encode_cell_bytes(cell_layout, value, machine.program.pointer_bytes());
        destination[..encoded.len()].copy_from_slice(encoded.as_slice());

        return;
    }

    // aggregate values are already heap payloads
    let source = value.as_heap_reference();
    let byte_len = layout.byte_len();
    let payload = read_heap_payload(heap, source, byte_len);
    destination[..byte_len].copy_from_slice(&payload);
}

/// Return whether one type is represented by one VM cell.
fn is_cell_type(machine: &Machine, ty: TypeId) -> bool {
    machine.program.is_cell_type(ty)
}

/// Return the static tensor shape.
fn static_tensor_shape(shape: &[TensorDimension]) -> Vec<u64> {
    let mut values = Vec::with_capacity(shape.len());

    for dimension in shape {
        match dimension {
            TensorDimension::Static(value) => values.push(*value),
            TensorDimension::Dynamic => panic!("dynamic test tensor shape is not materializable"),
            TensorDimension::Symbol(name) => {
                panic!("symbolic test tensor shape {name} is not materializable")
            }
        }
    }

    values
}

/// Align one byte width up to the given alignment.
fn align_to(value: usize, alignment: usize) -> usize {
    let alignment = alignment.max(1);
    let remainder = value % alignment;

    if remainder == 0 {
        value
    } else {
        value + alignment - remainder
    }
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

/// Parse MIR text and create one test machine.
pub(crate) fn create_machine(mir_text: &str) -> TestMachine {
    TestMachine::new(mir_text)
}

/// Parse MIR text and create one test machine with an explicit target layout.
pub(crate) fn create_machine_with_target_layout(
    mir_text: &str,
    target_layout: TargetLayout,
) -> TestMachine {
    let options = ParseOptions {
        pointer_bytes: target_layout.pointer_bytes(),
    };
    let (tree, parsed_target_layout, types, layouts, dispatch, drops, strings) =
        parse_test_mir(mir_text, options);
    assert_eq!(parsed_target_layout, target_layout);
    let (type_ids, type_nodes) = type_maps(&tree);
    let source_tree = tree.clone();

    let program = build_test_program(
        tree,
        parsed_target_layout,
        types,
        layouts,
        dispatch,
        drops,
        strings,
    );
    let memory = create_test_memory();
    let program = Arc::new(program);
    let local_static = program
        .materialize_local_statics(memory.clone())
        .expect("local test statics should build");
    let shared_static = program
        .materialize_shared_statics(memory.clone())
        .expect("shared test statics should build");
    let machine = Machine::new(program, memory.clone(), test_machine_options())
        .unwrap_or_else(|error| panic!("failed to initialize machine: {error}"));
    let heap = create_test_heap(memory.clone());
    let shared = create_test_shared_heap(memory);
    let shared_mark_worker = shared.register_mark_worker();
    let shared_cache = shared.allocation_cache();
    machine
        .require_heap_compatibility(&heap, &shared)
        .unwrap_or_else(|error| panic!("failed to initialize machine globals: {error}"));

    TestMachine {
        tree: source_tree,
        type_ids,
        type_nodes,
        machine,
        local_static,
        shared_static,
        heap,
        shared_heap: shared,
        shared_mark_worker,
        shared_cache,
    }
}

/// Run one MIR function by name with the given arguments.
pub(crate) fn run_mir(mir: &str, function: &str, arguments: &[Value]) -> RuntimeResult<Value> {
    let mut machine = create_machine(mir);

    machine.run_function_by_name(function, arguments)
}

/// Run MIR with access to one heap-owning machine and frame cells.
pub(crate) fn run_mir_with_frame<F>(
    mir_text: &str,
    function: &str,
    setup: F,
) -> RuntimeResult<Value>
where
    F: FnOnce(&mut TestMachine) -> Vec<Cell>,
{
    let mut machine = create_machine(mir_text);
    let args = setup(&mut machine);

    machine.run_frame_function_by_name(function, &args)
}

/// Run MIR with frame cells, expecting success.
pub(crate) fn run_mir_with_frame_ok<F>(mir_text: &str, function: &str, setup: F) -> Value
where
    F: FnOnce(&mut TestMachine) -> Vec<Cell>,
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
        Outcome::Stopped { .. } => panic!("expected yield"),
    }
}

/// Assert that one execution result stopped.
pub(crate) fn assert_execution_stopped(
    result: RuntimeResult<Outcome>,
) -> (Continuation, StopReason) {
    let outcome = result.expect("execution failed");

    match outcome {
        Outcome::Stopped {
            continuation,
            reason,
        } => (continuation, reason),
        Outcome::Completed { .. } => panic!("expected stop"),
        Outcome::Yielded { .. } => panic!("expected stop"),
    }
}

/// Assert that one execution result completed.
pub(crate) fn assert_execution_completed(result: RuntimeResult<Outcome>) -> Value {
    let outcome = result.expect("execution failed");

    match outcome {
        Outcome::Completed { value } => value,
        Outcome::Yielded { .. } => panic!("expected completion"),
        Outcome::Stopped { .. } => panic!("expected completion"),
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
entry(v0: int32):
    v1: Box = aggregate (v0)
    v2: ref<Box, managed, readonly> = new.zeroed Box
    store v2, v1
    v3: ref<int32, managed, readonly> = field.address v2, 0
    v4: int32 = load v3
    v5: int32 = 1
    v6: int32 = int.add v4, v5
    return v6
}
"#;

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
entry(v0: int32):
    v1: Box = aggregate (v0)
    v2: ref<Box, managed, readonly> = new.zeroed Box
    store v2, v1
    v3: int32 = call Box.get(v2)
    return v3
}

function Box.get(v0: ref<Box, managed, readonly>): int32 {
entry(v0: ref<Box, managed, readonly>):
    v1: ref<int32, managed, readonly> = field.address v0, 0
    v2: int32 = load v1
    return v2
}
"#;

    let output = run_mir_ok(mir_text, "readValueClass", &[Value::int32(9)]);

    assert_eq!(output, Value::int32(9));
}

/// Stored function values preserve their function pointer through managed structs.
#[test]
fn test_stored_function_roundtrips() {
    let mir_text = r#"
type Fn = fn() => int32;

type Holder {
    action: Fn;
}

function target(): int32 {
entry:
    v0: int32 = 7
    return v0
}

function run(): int32 {
entry:
    v0: Fn = function.address target
    v1: ref<Holder, managed, readonly> = new.zeroed Holder
    v2: Holder = aggregate (v0)
    store v1, v2
    v3: ref<Fn, managed, readonly> = field.address v1, 0
    v4: Fn = load v3
    v5: int32 = call.indirect v4(): () => int32
    return v5
}
"#;

    let output = run_mir_ok(mir_text, "run", &[]);

    assert_eq!(output, Value::int32(7));
}
