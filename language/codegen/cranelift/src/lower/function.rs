use std::collections::HashMap;
use std::sync::Arc;

use cranelift_codegen::ir::{
    Block as CraneliftBlock, Function as CraneliftFunction, StackSlot, Value as CraneliftValue,
};
use cranelift_codegen::isa::TargetIsa;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::FuncId;
use destack_mir::{Block, Function, Local, LocalNodeId, NodeTree, Value};
use destack_source::ImmutableStringPool;

use super::types::lower_type;
use super::{lower_instruction, lower_terminator};
use crate::CraneliftError;

/// Lowers a single MIR function to Cranelift IR.
pub(crate) struct FunctionLowerer<'a> {
    /// The MIR node tree.
    tree: &'a NodeTree,
    /// String pool for resolving names.
    #[allow(dead_code)]
    strings: &'a ImmutableStringPool,
    /// The MIR function being lowered.
    function: &'a Function,
    /// The target ISA.
    #[allow(dead_code)]
    isa: &'a Arc<dyn TargetIsa>,
    /// Function id mapping for calls.
    #[allow(dead_code)]
    function_ids: &'a HashMap<LocalNodeId<Function>, FuncId>,
}

impl<'a> FunctionLowerer<'a> {
    /// Create a new function lowerer.
    pub(crate) fn new(
        tree: &'a NodeTree,
        strings: &'a ImmutableStringPool,
        mir_function: &'a Function,
        isa: &'a Arc<dyn TargetIsa>,
        function_ids: &'a HashMap<LocalNodeId<Function>, FuncId>,
    ) -> Self {
        Self {
            tree,
            strings,
            function: mir_function,
            isa,
            function_ids,
        }
    }

    /// Lower the function.
    pub(crate) fn lower(self, cl_function: &mut CraneliftFunction) -> Result<(), CraneliftError> {
        let mut builder_context = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(cl_function, &mut builder_context);

        // state for lowering
        let mut value_map: HashMap<Value, CraneliftValue> = HashMap::new();
        let mut block_map: HashMap<LocalNodeId<Block>, CraneliftBlock> = HashMap::new();
        let mut local_map: HashMap<LocalNodeId<Local>, StackSlot> = HashMap::new();

        // create stack slots for locals
        self.create_locals(&mut builder, &mut local_map)?;

        // create all blocks first
        self.create_blocks(&mut builder, &mut value_map, &mut block_map)?;

        // lower each block
        for &block_id in &self.function.blocks {
            self.lower_block(
                &mut builder,
                block_id,
                &mut value_map,
                &block_map,
                &local_map,
            )?;
        }

        builder.finalize();
        Ok(())
    }

    /// Create Cranelift stack slots for MIR locals.
    fn create_locals(
        &self,
        builder: &mut FunctionBuilder<'_>,
        local_map: &mut HashMap<LocalNodeId<Local>, StackSlot>,
    ) -> Result<(), CraneliftError> {
        for &local_id in &self.function.locals {
            let local = self.tree.get(local_id);
            let ty = lower_type(self.tree, local.ty)?;
            let size = ty.bytes();

            let slot = builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                size,
                0, // align_shift
            ));

            local_map.insert(local_id, slot);
        }

        Ok(())
    }

    /// Create Cranelift blocks for all MIR blocks.
    fn create_blocks(
        &self,
        builder: &mut FunctionBuilder<'_>,
        value_map: &mut HashMap<Value, CraneliftValue>,
        block_map: &mut HashMap<LocalNodeId<Block>, CraneliftBlock>,
    ) -> Result<(), CraneliftError> {
        // first, create all blocks
        for &block_id in &self.function.blocks {
            let cl_block = builder.create_block();
            block_map.insert(block_id, cl_block);
        }

        // set entry block and add function parameters as entry block parameters
        let entry_block = block_map[&self.function.entry];

        // function parameters become entry block parameters in Cranelift
        for param in &self.function.parameters {
            let ty = lower_type(self.tree, param.ty)?;
            let cl_value = builder.append_block_param(entry_block, ty);
            value_map.insert(param.value, cl_value);
        }

        // now add MIR block parameters for non-entry blocks
        for &block_id in &self.function.blocks {
            let mir_block = self.tree.get(block_id);
            let cl_block = block_map[&block_id];

            // add block parameters (these are for phi nodes / join points)
            for param in &mir_block.parameters {
                let ty = lower_type(self.tree, param.ty)?;
                let cl_value = builder.append_block_param(cl_block, ty);
                value_map.insert(param.value, cl_value);
            }
        }

        // switch to entry block
        builder.switch_to_block(entry_block);

        // seal entry block (no predecessors)
        builder.seal_block(entry_block);

        Ok(())
    }

    /// Lower a single block.
    fn lower_block(
        &self,
        builder: &mut FunctionBuilder<'_>,
        block_id: LocalNodeId<Block>,
        value_map: &mut HashMap<Value, CraneliftValue>,
        block_map: &HashMap<LocalNodeId<Block>, CraneliftBlock>,
        local_map: &HashMap<LocalNodeId<Local>, StackSlot>,
    ) -> Result<(), CraneliftError> {
        let mir_block = self.tree.get(block_id);
        let cl_block = block_map[&block_id];

        // switch to this block (may already be current for entry)
        if builder.current_block() != Some(cl_block) {
            builder.switch_to_block(cl_block);
        }

        // lower instructions
        for &instruction_id in &mir_block.instructions {
            let instruction = self.tree.get(instruction_id);
            lower_instruction(self.tree, instruction, builder, value_map, local_map)?;
        }

        // lower terminator
        lower_terminator(&mir_block.terminator, builder, value_map, block_map)?;

        // seal the block (all predecessors are now known)
        builder.seal_block(cl_block);

        Ok(())
    }
}
