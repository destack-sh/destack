use std::collections::HashMap;

use destack_mir as mir;

use destack_program::vm::{
    Block, CallTarget, FunctionBuilder, Instruction, MoveSlot, SideTableBuilder,
};
use destack_program::{
    AllocationSite, CallSite, FrameLayout, FrameLayoutId, FrameSlot, FrameStateId, MemorySite,
};

use crate::{LinkError, LinkResult};

use super::block::{BlockOrder, FunctionContext};
use super::layout::StorageLayout;
use super::linker::Linker;
use super::pool::Pool;
use super::value::OperandMap;

/// One whole-function lowerer.
pub(super) struct FunctionLowerer<'a, 'table> {
    /// Shared immutable function lowering context.
    context: FunctionContext<'a>,
    /// MIR function being lowered.
    function: &'a mir::Function,
    /// Executable frame layout id for this function.
    frame_layout_id: FrameLayoutId,
    /// Shared side-table builder for this function.
    pool: Pool<'a, 'table>,
}

impl<'a, 'table> FunctionLowerer<'a, 'table> {
    /// Create one function lowerer for the given MIR function.
    pub(super) fn new(
        program: &'a Linker<'_>,
        func_id: mir::LocalNodeId<mir::Function>,
        frame_layout_id: FrameLayoutId,
        frame_layout: &'a FrameLayout,
        yield_frame_states: &'a HashMap<mir::LocalNodeId<mir::Block>, FrameStateId>,
        call_frame_states: &'a HashMap<mir::LocalNodeId<mir::Block>, FrameStateId>,
        value_types: &'a [mir::LocalNodeId<mir::Type>],
        side_table: &'table mut SideTableBuilder,
    ) -> LinkResult<Option<Self>> {
        let tree = program.tree();
        let function = tree.get(func_id);

        if function.is_import() {
            return Ok(None);
        }

        let Some(entry) = function.entry() else {
            return Ok(None);
        };

        // build function-local maps in VM block order
        let block_order = BlockOrder::new(tree, entry, program.program())?;
        let value_types = value_types.to_vec();
        let value_count = value_types.len();
        let operand_map = OperandMap::lower(
            tree,
            program.target_layout(),
            program.types(),
            program,
            &value_types,
        );
        let value_use_counts = Self::value_use_counts(program, &block_order.blocks, value_count)?;
        let local_index_by_id = Self::local_index_map(function);
        let block_parameters = Self::block_parameters(tree, &block_order.blocks)?;
        let entry_block = block_order.index_by_id[&entry] as u32;

        // assemble immutable lowering context for all blocks
        let context = FunctionContext {
            tree,
            target_layout: program.target_layout(),
            types: program.types(),
            function_id: func_id,
            entry_block,
            yield_frame_states,
            call_frame_states,
            call_targets: program.call_targets(),
            program,
            frame_layout,
            operand_map,
            value_types,
            layouts: program.storage_layouts(),
            layout_table: program.layout_table(),
            heap_options: program.heap_options(),
            shared_heap_options: program.shared_heap_options(),
            block_index_by_id: block_order.index_by_id,
            block_parameters,
            local_index_by_id,
            value_use_counts,
        };

        Ok(Some(Self {
            context,
            function,
            frame_layout_id,
            pool: Pool::new(side_table, frame_layout, program, program.traces()),
        }))
    }

    /// Lower the function into program form.
    pub(super) fn lower(mut self) -> LinkResult<LoweredFunction> {
        let parameter_values = self
            .function
            .parameters
            .iter()
            .map(|parameter| parameter.value)
            .collect::<Vec<_>>();
        let parameter = self.pool.argument_range(&parameter_values)?;
        let mut ordered_blocks = self
            .context
            .block_index_by_id
            .iter()
            .map(|(block_id, index)| (*index, *block_id))
            .collect::<Vec<_>>();
        ordered_blocks.sort_unstable_by_key(|(index, _)| *index);
        let mut code = Vec::new();
        let mut blocks = Vec::with_capacity(ordered_blocks.len());
        let mut source_points = Vec::with_capacity(ordered_blocks.len());
        let mut allocation_sites = Vec::new();
        let mut memory_sites = Vec::new();
        let mut call_sites = Vec::new();

        for (_, block_id) in ordered_blocks {
            let start = code.len() as u32;
            let block = self.lower_block(block_id, start)?;
            let len = block.instructions.len() as u32;

            code.extend(block.instructions);
            blocks.push(Block { start, len });
            source_points.push(SourceBlock {
                block: block_id,
                start,
                point_by_pc: block.point_by_pc,
            });
            allocation_sites.extend(block.allocation_sites);
            memory_sites.extend(block.memory_sites);
            call_sites.extend(block.call_sites);
        }

        let (argument_pool, move_pool) = self.pool.finish();
        let function = FunctionBuilder {
            function: self.context.program_function(self.context.function_id),
            frame_layout: self.frame_layout_id,
            parameters: parameter,
            entry: self.context.entry_block,
            code,
            blocks,
            argument_pool,
            move_pool,
        };

        Ok(LoweredFunction {
            function,
            source_points,
            allocation_sites,
            memory_sites,
            call_sites,
        })
    }

    /// Lower one MIR block into program form.
    fn lower_block(
        &mut self,
        block_id: mir::LocalNodeId<mir::Block>,
        block_start: u32,
    ) -> LinkResult<LoweredBlock> {
        let lowerer = BlockLowerer {
            function: &self.context,
            block_id,
            block: self.context.tree.get(block_id),
            block_start,
        };

        lowerer.lower(&mut self.pool)
    }

    /// Build the lowered local index map for the function.
    fn local_index_map(function: &mir::Function) -> HashMap<mir::LocalNodeId<mir::Local>, u32> {
        debug_assert!(
            function.locals().len() <= u32::MAX as usize,
            "too many locals for lowered function indices"
        );

        let mut local_index_by_id = HashMap::with_capacity(function.locals().len());

        for (index, local) in function.locals().iter().enumerate() {
            local_index_by_id.insert(*local, index as u32);
        }

        local_index_by_id
    }

    /// Collect block parameter values in lowered block order.
    fn block_parameters(
        tree: &mir::Tree,
        blocks: &[mir::LocalNodeId<mir::Block>],
    ) -> LinkResult<Vec<Vec<mir::Value>>> {
        let mut block_parameters = Vec::with_capacity(blocks.len());

        for block_id in blocks {
            let block = tree.get(*block_id);
            let parameter = block
                .parameters
                .iter()
                .map(|parameter| parameter.value)
                .collect::<Vec<_>>();
            block_parameters.push(parameter);
        }

        Ok(block_parameters)
    }

    /// Compute SSA value use counts across the function.
    fn value_use_counts(
        program: &Linker<'_>,
        blocks: &[mir::LocalNodeId<mir::Block>],
        value_count: usize,
    ) -> LinkResult<Vec<u32>> {
        let tree = program.tree();
        let mut uses = vec![0u32; value_count];

        // scan instructions and terminators for value uses
        for block_id in blocks {
            let block = tree.get(*block_id);
            let terminator = tree.get(block.terminator);
            for inst_id in &block.instructions {
                let inst = tree.get(*inst_id);
                for value in inst.uses() {
                    Self::record_value_use(program, &mut uses, value)?;
                }
                if let Some(arguments) = inst.argument_slice() {
                    for argument in tree.get_values(arguments) {
                        Self::record_value_use(program, &mut uses, *argument)?;
                    }
                }
            }
            for value in terminator.uses(tree) {
                Self::record_value_use(program, &mut uses, value)?;
            }
        }

        Ok(uses)
    }

    /// Record one SSA value use count.
    fn record_value_use(
        program: &Linker<'_>,
        uses: &mut [u32],
        value: mir::Value,
    ) -> LinkResult<()> {
        let Some(count) = uses.get_mut(value.0 as usize) else {
            return Err(program
                .program()
                .invalid_input(format!("use count for {value:?}")));
        };

        *count += 1;

        Ok(())
    }
}

/// One lowered function plus transient source mapping.
pub(crate) struct LoweredFunction {
    /// The lowered VM function.
    pub(crate) function: FunctionBuilder,
    /// Source block and instruction points in VM block order.
    pub(crate) source_points: Vec<SourceBlock>,
    /// Executable heap allocation sites.
    pub(crate) allocation_sites: Vec<AllocationSite>,
    /// Executable memory access sites.
    pub(crate) memory_sites: Vec<MemorySite>,
    /// Executable call sites.
    pub(crate) call_sites: Vec<CallSite>,
}

/// Source block points in lowered VM block order.
pub(crate) struct SourceBlock {
    /// MIR block represented by this lowered block.
    pub(crate) block: mir::BlockId,
    /// Function-local operation index where this block starts.
    pub(crate) start: u32,
    /// Source instruction point for each VM instruction offset.
    pub(crate) point_by_pc: Vec<u32>,
}

/// One lowered VM block and its executable site rows.
struct LoweredBlock {
    /// Lowered VM instructions.
    instructions: Vec<Instruction>,
    /// Source instruction point for each VM instruction offset.
    point_by_pc: Vec<u32>,
    /// Executable heap allocation sites.
    allocation_sites: Vec<AllocationSite>,
    /// Executable memory access sites.
    memory_sites: Vec<MemorySite>,
    /// Executable call sites.
    call_sites: Vec<CallSite>,
}

/// One block-local lowerer.
pub(super) struct BlockLowerer<'a> {
    /// Shared immutable function lowering context.
    pub(super) function: &'a FunctionContext<'a>,
    /// MIR block being lowered.
    block_id: mir::LocalNodeId<mir::Block>,
    /// MIR block payload.
    block: &'a mir::Block,
    /// Function-local operation index where this block starts.
    pub(super) block_start: u32,
}

impl<'a> BlockLowerer<'a> {
    /// Return the current MIR block id.
    pub(super) fn block_id(&self) -> mir::LocalNodeId<mir::Block> {
        self.block_id
    }

    /// Lower the block into program form.
    fn lower(self, pool: &mut Pool<'_, '_>) -> LinkResult<LoweredBlock> {
        let mut instructions = Vec::with_capacity(self.block.instructions.len() + 1);
        let mut point_by_pc = Vec::with_capacity(self.block.instructions.len() + 2);
        let mut allocation_sites = Vec::new();
        let mut memory_sites = Vec::new();
        let mut call_sites = Vec::new();
        point_by_pc.push(0);

        // convert regular instructions
        let mut inst_index = 0usize;
        while inst_index < self.block.instructions.len() {
            let inst_id = self.block.instructions[inst_index];
            let inst = self.function.tree.get(inst_id);
            let pc = instructions.len() as u32;

            if let Some(site) = self.allocation_site_for_instruction(inst, pc)? {
                allocation_sites.push(site);
            }
            self.push_memory_sites_for_instruction(inst, pc, &mut memory_sites)?;
            if let Some(site) = self.call_site_for_instruction(inst, pc)? {
                call_sites.push(site);
            }

            // lower the remaining instruction shape
            let lowered = self.lower_instructions(inst, pool)?;
            let lowered_len = lowered.len();
            instructions.extend(lowered);

            // internal instructions still belong to the current MIR operation
            for _ in 1..lowered_len {
                point_by_pc.push(inst_index as u32);
            }

            inst_index += 1;

            // record the MIR point after the MIR operation is complete
            if lowered_len > 0 {
                point_by_pc.push(inst_index as u32);
            }
        }

        let terminator = self.function.tree.get(self.block.terminator);

        // fuse compare producers into conditional branches
        if let Some(fused) = self.try_fuse_compare_branch(self.block, &mut instructions, pool) {
            instructions.push(fused);
        } else {
            let pc = instructions.len() as u32;
            if let Some(site) = self.allocation_site_for_terminator(terminator, pc)? {
                allocation_sites.push(site);
            }
            if let Some(site) = self.call_site_for_terminator(terminator, pc)? {
                call_sites.push(site);
            }

            let lowered_terminator = self.lower_terminator(terminator, pool)?;
            instructions.push(lowered_terminator);
        }

        point_by_pc.push((self.block.instructions.len() + 1) as u32);

        Ok(LoweredBlock {
            instructions,
            point_by_pc,
            allocation_sites,
            memory_sites,
            call_sites,
        })
    }

    /// Return the lowered use count for one value.
    pub(super) fn value_use_count(&self, value: mir::Value) -> Option<u32> {
        self.function
            .value_use_counts
            .get(value.0 as usize)
            .copied()
    }

    /// Return one call target for one function id.
    pub(super) fn call_target(
        &self,
        function: mir::LocalNodeId<mir::Function>,
    ) -> LinkResult<CallTarget> {
        let function = self.function.program_function(function);
        self.function
            .call_targets
            .get(function.index())
            .and_then(|target| *target)
            .ok_or_else(|| self.function.undefined_function(format!("{function:?}")))
    }

    /// Return one lowered local index for one local id.
    pub(super) fn local_index(&self, local: mir::LocalNodeId<mir::Local>) -> LinkResult<u32> {
        self.function
            .local_index_by_id
            .get(&local)
            .copied()
            .ok_or_else(|| {
                self.function
                    .invalid_input(format!("local index for {local:?}"))
            })
    }

    /// Return one fixed byte offset as an instruction operand.
    pub(super) fn instruction_byte_offset(&self, byte_offset: usize) -> LinkResult<u32> {
        u32::try_from(byte_offset).map_err(|_| self.layout_overflow("instruction byte offset"))
    }

    /// Return one value's frame slot.
    pub(super) fn frame_value_slot(&self, value: mir::Value) -> LinkResult<&FrameSlot> {
        self.function
            .frame_value_slot(value.0)
            .ok_or_else(|| self.invalid_instruction("frame value slot"))
    }

    /// Return one local's frame slot.
    pub(super) fn frame_local_slot(
        &self,
        local: mir::LocalNodeId<mir::Local>,
    ) -> LinkResult<&FrameSlot> {
        let local = self.local_index(local)?;

        self.function
            .frame_local_slot(local)
            .ok_or_else(|| self.invalid_instruction("frame local slot"))
    }

    /// Return one cell value's frame byte offset.
    pub(super) fn cell_offset(&self, value: mir::Value) -> LinkResult<u32> {
        let slot = self.frame_value_slot(value)?;

        if !self.function.slot_is_cell(slot) {
            return Err(self.type_mismatch("cell value", format!("frame-backed value: {value:?}")));
        }

        Ok(slot.offset)
    }

    /// Return one value's frame byte offset.
    pub(super) fn value_offset(&self, value: mir::Value) -> LinkResult<u32> {
        Ok(self.frame_value_slot(value)?.offset)
    }

    /// Return one lowered move slot for one SSA value.
    pub(super) fn move_slot(&self, value: mir::Value) -> LinkResult<MoveSlot> {
        let slot = self.frame_value_slot(value)?;
        let is_cell = self.function.slot_is_cell(slot);

        Ok(MoveSlot::new(
            slot.ty,
            slot.offset,
            slot.byte_len(),
            is_cell,
        ))
    }

    /// Return one invalid lowered input diagnostic.
    pub(super) fn invalid_input(&self, context: impl Into<String>) -> LinkError {
        self.function.invalid_input(context)
    }

    /// Return one link type mismatch diagnostic.
    pub(super) fn type_mismatch(
        &self,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> LinkError {
        self.function.type_mismatch(expected, actual)
    }

    /// Return one invalid instruction diagnostic.
    pub(super) fn invalid_instruction(&self, context: impl Into<String>) -> LinkError {
        self.function.invalid_instruction(context)
    }

    /// Return one invalid cast diagnostic.
    pub(super) fn invalid_cast(&self, context: impl Into<String>) -> LinkError {
        self.function.invalid_cast(context)
    }

    /// Return one invalid field access diagnostic.
    pub(super) fn invalid_field_access(&self, index: u32, field_count: usize) -> LinkError {
        self.function.invalid_field_access(index, field_count)
    }

    /// Return one invalid pointer type diagnostic.
    pub(super) fn invalid_pointer_type(&self, actual: impl Into<String>) -> LinkError {
        self.function.invalid_pointer_type(actual)
    }

    /// Return one unsupported instruction diagnostic.
    pub(super) fn unsupported_instruction(&self, name: impl Into<String>) -> LinkError {
        self.function.unsupported_instruction(name)
    }

    /// Return one layout overflow diagnostic.
    pub(super) fn layout_overflow(&self, context: impl Into<String>) -> LinkError {
        self.function.layout_overflow(context)
    }

    /// Return one internal link diagnostic.
    pub(super) fn internal(&self, message: impl Into<String>) -> LinkError {
        self.function.internal(message)
    }

    /// Return the lowered operand map.
    pub(super) fn operand_map(&self) -> &OperandMap {
        &self.function.operand_map
    }

    /// Return the lowered value types.
    pub(super) fn value_types(&self) -> &[mir::LocalNodeId<mir::Type>] {
        &self.function.value_types
    }

    /// Return the lowered storage layouts.
    pub(super) fn layouts(&self) -> &HashMap<mir::LocalNodeId<mir::Type>, StorageLayout> {
        self.function.layouts
    }
}
