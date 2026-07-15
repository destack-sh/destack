use std::collections::HashMap;

use destack_core::SectionPacker;
use destack_heap as heap;
use destack_mir as mir;
use destack_mir::TraceTable;
use destack_program::vm::{self, CallTarget, SideTableBuilder};
use destack_program::{
    AllocationSite, CallSite, ContinuationSite, CounterSite, EdgeSite, FrameLayout, FrameLayoutId,
    FrameSlot, FrameStateId, FrameTable, FunctionId, GlobalId, MemorySite, SampleSite, TypeId,
};

use crate::LinkResult;

use super::super::ProgramLinker;
use super::lower::FunctionLowerer;
use super::resume::InvokeStates;
use super::{FrameLinker, LoweredFunction, ResumeLinker, StorageLayout};

/// Linked VM code and execution metadata.
#[derive(Debug)]
pub(crate) struct Code {
    /// Executable VM code.
    pub(crate) program: vm::Code,
    /// Runtime frame layouts and materialization tables.
    pub(crate) frames: FrameTable,
    /// Executable heap allocation sites.
    pub(crate) allocation_sites: Vec<AllocationSite>,
    /// Executable memory access sites.
    pub(crate) memory_sites: Vec<MemorySite>,
    /// Executable call sites.
    pub(crate) call_sites: Vec<CallSite>,
    /// Executable control-flow edge sites.
    pub(crate) edge_sites: Vec<EdgeSite>,
    /// Executable continuation sites.
    pub(crate) continuation_sites: Vec<ContinuationSite>,
    /// Explicit counter sites.
    pub(crate) counter_sites: Vec<CounterSite>,
    /// Explicit sample sites.
    pub(crate) sample_sites: Vec<SampleSite>,
}

/// Link MIR functions into VM code and frame metadata.
#[derive(Debug)]
pub(crate) struct Linker<'a> {
    /// MIR tree being linked.
    tree: &'a mir::Tree,
    /// Target ABI layout for this program.
    target_layout: &'a mir::TargetLayout,
    /// MIR type table for names, lineage, and type attachments.
    types: &'a mir::TypeTable,
    /// MIR layout table produced by lower and optimization.
    layouts: &'a mir::LayoutTable,
    /// Local heap options baked into the program.
    heap_options: &'a heap::HeapOptions,
    /// Shared heap options baked into the program.
    shared_heap_options: &'a heap::SharedHeapOptions,
    /// Program linker owning dense program id projection.
    program: &'a ProgramLinker,
    /// Lowered value storage layouts keyed by MIR type id.
    storage: &'a HashMap<mir::TypeId, StorageLayout>,
    /// Executable trace table.
    traces: &'a TraceTable,
    /// MIR function ids that lower to VM code.
    lowering_order: Vec<mir::FunctionId>,
    /// Executable call targets keyed by program function id.
    call_targets: Vec<Option<CallTarget>>,
    /// Linked frame layouts and materialization entries.
    frames: FrameLinker<'a>,
    /// Linked resume states.
    resume: ResumeLinker<'a>,
    /// Executable heap allocation sites.
    allocation_sites: Vec<AllocationSite>,
    /// Executable memory access sites.
    memory_sites: Vec<MemorySite>,
    /// Executable call sites.
    call_sites: Vec<CallSite>,
    /// Executable control-flow edge sites.
    edge_sites: Vec<EdgeSite>,
    /// Executable continuation sites.
    continuation_sites: Vec<ContinuationSite>,
    /// Explicit counter sites.
    counter_sites: Vec<CounterSite>,
    /// Explicit sample sites.
    sample_sites: Vec<SampleSite>,
}

impl<'a> Linker<'a> {
    /// Create one VM linker.
    pub(crate) fn new(
        tree: &'a mir::Tree,
        target_layout: &'a mir::TargetLayout,
        types: &'a mir::TypeTable,
        layouts: &'a mir::LayoutTable,
        heap_options: &'a heap::HeapOptions,
        shared_heap_options: &'a heap::SharedHeapOptions,
        program: &'a ProgramLinker,
        storage: &'a HashMap<mir::TypeId, StorageLayout>,
        traces: &'a TraceTable,
    ) -> Self {
        let mut linker = Self {
            tree,
            target_layout,
            types,
            layouts,
            heap_options,
            shared_heap_options,
            program,
            storage,
            traces,
            lowering_order: Vec::new(),
            call_targets: Vec::new(),
            frames: FrameLinker::new(tree, program, storage),
            resume: ResumeLinker::new(tree, program),
            allocation_sites: Vec::new(),
            memory_sites: Vec::new(),
            call_sites: Vec::new(),
            edge_sites: Vec::new(),
            continuation_sites: Vec::new(),
            counter_sites: Vec::new(),
            sample_sites: Vec::new(),
        };

        linker.build_call_targets();

        linker
    }

    /// Link VM code and frame metadata.
    pub(crate) fn link(mut self, sections: &mut SectionPacker) -> LinkResult<Code> {
        let mut side_table = SideTableBuilder::default();

        // lower VM code and finish side tables
        let functions = self.build_functions(&mut side_table)?;
        let side_table = side_table.pack(sections);
        let functions = vm::FunctionTable::pack(sections, functions, self.call_targets);
        let program = vm::Code::new(functions, side_table, self.resume.finish(sections)?);

        Ok(Code {
            program,
            frames: self.frames.finish(sections),
            allocation_sites: self.allocation_sites,
            memory_sites: self.memory_sites,
            call_sites: self.call_sites,
            edge_sites: self.edge_sites,
            continuation_sites: self.continuation_sites,
            counter_sites: self.counter_sites,
            sample_sites: self.sample_sites,
        })
    }

    /// Return the dense program id projection.
    pub(crate) fn program(&self) -> &ProgramLinker {
        self.program
    }

    /// Return the MIR tree being linked.
    pub(crate) fn tree(&self) -> &mir::Tree {
        self.tree
    }

    /// Return the target ABI layout.
    pub(crate) fn target_layout(&self) -> &mir::TargetLayout {
        self.target_layout
    }

    /// Return the MIR type table.
    pub(crate) fn types(&self) -> &mir::TypeTable {
        self.types
    }

    /// Return the MIR layout table.
    pub(crate) fn layout_table(&self) -> &mir::LayoutTable {
        self.layouts
    }

    /// Return the worker-local heap options.
    pub(crate) fn heap_options(&self) -> &heap::HeapOptions {
        self.heap_options
    }

    /// Return the runtime-shared heap options.
    pub(crate) fn shared_heap_options(&self) -> &heap::SharedHeapOptions {
        self.shared_heap_options
    }

    /// Return the lowered storage layouts.
    pub(crate) fn storage_layouts(&self) -> &HashMap<mir::TypeId, StorageLayout> {
        self.storage
    }

    /// Return the compiler trace maps.
    pub(crate) fn traces(&self) -> &TraceTable {
        self.traces
    }

    /// Return the call target table.
    pub(crate) fn call_targets(&self) -> &[Option<CallTarget>] {
        &self.call_targets
    }

    /// Return the program function id for one MIR function.
    pub(crate) fn function_id(&self, function: mir::FunctionId) -> FunctionId {
        self.program.function_id(function)
    }

    /// Return the program type id for one MIR type.
    pub(crate) fn type_id(&self, ty: mir::TypeId) -> TypeId {
        self.program.type_id(ty)
    }

    /// Return the input type id for one program type id.
    pub(crate) fn type_by_id(&self, ty: TypeId) -> Option<mir::TypeId> {
        self.program.type_by_id(ty)
    }

    /// Return the program global id for one MIR global.
    pub(crate) fn global_id(&self, global: mir::GlobalId) -> GlobalId {
        self.program.global_id(global)
    }

    /// Build the lowered function order and call target map.
    fn build_call_targets(&mut self) {
        // size the dense target table to the program function id space
        let function_count = self.program.function_count();
        self.call_targets.resize(function_count, None);

        // collect imported and lowerable functions
        for (function_id, function) in self.tree.iter_nodes::<mir::Function>() {
            if function.is_import() {
                let function = self.function_id(function_id);
                self.call_targets[function.index()] = Some(CallTarget::BINDING);
                continue;
            }

            if function.entry().is_none() {
                continue;
            }

            self.lowering_order.push(function_id);
        }

        // assign stable lowered indices in build order
        for (slot, function_id) in self.lowering_order.iter().enumerate() {
            let function = self.function_id(*function_id);
            self.call_targets[function.index()] = Some(CallTarget::local(slot as u32));
        }
    }

    /// Collect lowered runtime value types for one function.
    fn value_types(&self, function: &mir::Function) -> LinkResult<Vec<mir::TypeId>> {
        let mut value_types = Vec::with_capacity(function.value_types().len());

        for (index, ty) in function.value_types().iter().enumerate() {
            let Some(ty) = *ty else {
                return Err(self
                    .program
                    .invalid_input(format!("type for value v{index}")));
            };

            value_types.push(ty);
        }

        Ok(value_types)
    }

    /// Return whether one program frame slot is stored as one VM cell.
    pub(crate) fn frame_slot_is_cell(&self, slot: &FrameSlot) -> bool {
        self.frames.slot_is_cell(slot)
    }

    /// Return one value slot inside one frame layout.
    pub(crate) fn frame_value_slot(&self, layout: &FrameLayout, value: u32) -> Option<&FrameSlot> {
        self.frames.value_slot(layout, value)
    }

    /// Return one local slot inside one frame layout.
    pub(crate) fn frame_local_slot(&self, layout: &FrameLayout, local: u32) -> Option<&FrameSlot> {
        self.frames.local_slot(layout, local)
    }

    /// Build the lowered functions and append their execution tables.
    fn build_functions(
        &mut self,
        side_table: &mut SideTableBuilder,
    ) -> LinkResult<Vec<vm::FunctionBuilder>> {
        let lowering_order = std::mem::take(&mut self.lowering_order);
        let mut functions = Vec::with_capacity(lowering_order.len());

        // build one lowered function at a time
        for function_id in lowering_order {
            let function = self.build_function(function_id, side_table)?;
            functions.push(function);
        }

        Ok(functions)
    }

    /// Build one lowered function and append its execution tables.
    fn build_function(
        &mut self,
        function_id: mir::FunctionId,
        side_table: &mut SideTableBuilder,
    ) -> LinkResult<vm::FunctionBuilder> {
        let function = self.tree.get(function_id);
        let program_function = self.function_id(function_id);
        let value_types = self.value_types(function)?;

        // derive the logical frame shape before lowering
        let frame_layout_id = self.frames.next_layout_id();
        let frame_layout = self.frames.build_layout(function, &value_types)?;
        let liveness = mir::FunctionLiveness::build(function, self.tree);
        let (yield_frame_states, invoke_frame_states) = self.resume.build_entries(
            function_id,
            frame_layout_id,
            &frame_layout,
            &liveness,
            &mut self.frames,
        )?;

        // lower the function with the preassigned yield resume ids
        let lowered = self
            .lower_function(
                function_id,
                frame_layout_id,
                &frame_layout,
                &yield_frame_states,
                &invoke_frame_states,
                &value_types,
                side_table,
            )?
            .ok_or_else(|| {
                self.program
                    .invalid_input(format!("function {function_id:?} did not lower"))
            })?;
        let LoweredFunction {
            function,
            source_points,
            allocation_sites,
            memory_sites,
            call_sites,
            edge_sites,
            continuation_sites,
            counter_sites,
            sample_sites,
        } = lowered;

        // append the frame layout before assigning resume states
        self.frames.push_layout(frame_layout);

        // resolve block-entry states from lowered operation starts
        self.resume
            .resolve_entry_points(program_function, &source_points)?;

        // append states for every lowered instruction point
        self.resume.append_source_points(
            program_function,
            frame_layout_id,
            &frame_layout,
            &liveness,
            &source_points,
            &mut self.frames,
        )?;
        self.allocation_sites.extend(allocation_sites);
        self.memory_sites.extend(memory_sites);
        self.call_sites.extend(call_sites);
        self.edge_sites.extend(edge_sites);
        self.continuation_sites.extend(continuation_sites);
        self.counter_sites.extend(counter_sites);
        self.sample_sites.extend(sample_sites);

        Ok(function)
    }

    /// Lower one MIR function into the VM function form.
    fn lower_function(
        &self,
        function: mir::FunctionId,
        frame_layout_id: FrameLayoutId,
        frame_layout: &FrameLayout,
        yield_frame_states: &HashMap<mir::LocalNodeId<mir::Block>, FrameStateId>,
        invoke_frame_states: &HashMap<mir::LocalNodeId<mir::Block>, InvokeStates>,
        value_types: &[mir::LocalNodeId<mir::Type>],
        side_table: &mut SideTableBuilder,
    ) -> LinkResult<Option<LoweredFunction>> {
        let function = FunctionLowerer::new(
            self,
            function,
            frame_layout_id,
            frame_layout,
            yield_frame_states,
            invoke_frame_states,
            value_types,
            side_table,
        )?;

        function.map(FunctionLowerer::lower).transpose()
    }
}
