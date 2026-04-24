use std::collections::HashMap;
use std::ops::Deref;

use destack_mir as mir;

use crate::module::{Block, CallTarget, Function, Layout};
use crate::{Error, Result};

use super::block::{BlockOrder, FunctionContext};
use super::kind::{KindMapBuilder, ValueKindMap};
use super::pool::{Pool, lookup_call_target};
use super::tree::ValueSlot;

/// One whole-function lowering session.
struct FunctionLowerer<'a> {
    context: FunctionContext<'a>,
    func: &'a mir::Function,
    frame_layout: destack_engine::FrameLayoutId,
    pool: Pool,
}

impl<'a> FunctionLowerer<'a> {
    /// Create one function lowerer for the given MIR function.
    fn new(
        tree: &'a mir::NodeTree,
        func_id: mir::LocalNodeId<mir::Function>,
        frame_layout: destack_engine::FrameLayoutId,
        yield_resume_points: &'a HashMap<
            mir::LocalNodeId<mir::Block>,
            destack_engine::ResumePointId,
        >,
        exceptional_call_resume_points: &'a HashMap<
            mir::LocalNodeId<mir::Block>,
            (destack_engine::ResumePointId, destack_engine::ResumePointId),
        >,
        call_targets: &'a HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
        layouts: &'a HashMap<mir::LocalNodeId<mir::Type>, Layout>,
        value_slots: &'a [ValueSlot],
    ) -> Result<Option<Self>> {
        let func = tree.get(func_id);

        if func.is_import() {
            return Ok(None);
        }

        let Some(entry) = func.entry else {
            return Ok(None);
        };

        let block_order = BlockOrder::new(tree, entry)?;
        let value_type = value_slots.iter().map(|slot| slot.ty).collect::<Vec<_>>();
        let value_count = value_slots.len();
        let value_kind_map =
            KindMapBuilder::new(tree, func, &block_order.block, &value_type, value_count).build();
        let value_use_count = compute_value_use_counts(tree, &block_order.block, value_count)?;
        let local_index_by_id = Self::local_index_map(func);
        let block_parameter = Self::block_parameter(tree, &block_order.block)?;
        let entry_block = block_order.index_by_id[&entry] as u32;
        let context = FunctionContext {
            tree,
            function_id: func_id,
            entry_block,
            yield_resume_points,
            exceptional_call_resume_points,
            call_targets,
            value_kind_map,
            value_type,
            layouts,
            block_index_by_id: block_order.index_by_id,
            block_parameter,
            local_index_by_id,
            value_use_count,
        };

        Ok(Some(Self {
            context,
            func,
            frame_layout,
            pool: Pool::new(),
        }))
    }

    /// Lower the function into module form.
    fn lower(mut self) -> Result<Function> {
        let parameter_value = self
            .func
            .parameters
            .iter()
            .map(|parameter| {
                (parameter.value)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "function parameter value".to_string(),
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        let parameter = self.pool.argument_range(&parameter_value);
        let mut mir_block = self
            .context
            .block_index_by_id
            .iter()
            .map(|(block_id, index)| (*index, *block_id))
            .collect::<Vec<_>>();
        mir_block.sort_unstable_by_key(|(index, _)| *index);
        let mut block = Vec::with_capacity(mir_block.len());

        for (_, mir_block) in mir_block {
            block.push(self.lower_block(mir_block)?);
        }

        let (argument_pool, switch_case_pool, copy_pool) = self.pool.into_parts();

        Ok(Function {
            frame_layout: self.frame_layout,
            parameters: parameter,
            entry: self.context.entry_block,
            blocks: block,
            argument_pool,
            switch_case_pool,
            copy_pool,
            value_count: self.context.value_type.len(),
            local_count: self.func.locals.len(),
        })
    }

    /// Lower one MIR block into module form.
    fn lower_block(&mut self, mir_block: mir::LocalNodeId<mir::Block>) -> Result<Block> {
        let lowerer = BlockLowerer {
            function: &self.context,
            mir_block,
            block: self.context.tree.get(mir_block),
        };

        lowerer.lower(&mut self.pool)
    }

    /// Build the lowered local index map for the function.
    fn local_index_map(func: &mir::Function) -> HashMap<mir::LocalNodeId<mir::Local>, u32> {
        debug_assert!(
            func.locals.len() <= u32::MAX as usize,
            "too many locals for lowered function indices"
        );

        let mut local_index_by_id = HashMap::with_capacity(func.locals.len());

        for (index, local) in func.locals.iter().enumerate() {
            local_index_by_id.insert(*local, index as u32);
        }

        local_index_by_id
    }

    /// Collect block parameter values in lowered block order.
    fn block_parameter(
        tree: &mir::NodeTree,
        mir_block: &[mir::LocalNodeId<mir::Block>],
    ) -> Result<Vec<Vec<mir::Value>>> {
        let mut block_parameter = Vec::with_capacity(mir_block.len());

        for block_id in mir_block {
            let block = tree.get(*block_id);
            let parameter = block
                .parameters
                .iter()
                .map(|parameter| {
                    (parameter.value)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "block parameter value".to_string(),
                        })
                })
                .collect::<Result<Vec<_>>>()?;
            block_parameter.push(parameter);
        }

        Ok(block_parameter)
    }
}

/// Lower a MIR function into the interpreter function form.
pub(crate) fn lower_function(
    tree: &mir::NodeTree,
    func_id: mir::LocalNodeId<mir::Function>,
    frame_layout: destack_engine::FrameLayoutId,
    yield_resume_points: &HashMap<mir::LocalNodeId<mir::Block>, destack_engine::ResumePointId>,
    exceptional_call_resume_points: &HashMap<
        mir::LocalNodeId<mir::Block>,
        (destack_engine::ResumePointId, destack_engine::ResumePointId),
    >,
    call_targets: &HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    value_slots: &[ValueSlot],
) -> Result<Option<Function>> {
    let lowerer = FunctionLowerer::new(
        tree,
        func_id,
        frame_layout,
        yield_resume_points,
        exceptional_call_resume_points,
        call_targets,
        layouts,
        value_slots,
    )?;

    lowerer.map(FunctionLowerer::lower).transpose()
}

/// One block-local lowering session.
pub(super) struct BlockLowerer<'a> {
    function: &'a FunctionContext<'a>,
    mir_block: mir::LocalNodeId<mir::Block>,
    block: &'a mir::Block,
}

impl<'a> Deref for BlockLowerer<'a> {
    type Target = FunctionContext<'a>;

    fn deref(&self) -> &Self::Target {
        self.function
    }
}

impl<'a> BlockLowerer<'a> {
    /// Return the current MIR block id.
    pub(super) fn block_id(&self) -> mir::LocalNodeId<mir::Block> {
        self.mir_block
    }

    /// Lower the block into module form.
    fn lower(self, pool: &mut Pool) -> Result<Block> {
        let mut instructions = Vec::with_capacity(self.block.instructions.len() + 1);
        let mut mir_instruction_offsets = Vec::with_capacity(self.block.instructions.len() + 2);
        mir_instruction_offsets.push(0);

        // convert regular instructions
        let mut inst_index = 0usize;
        while inst_index < self.block.instructions.len() {
            let inst_id = self.block.instructions[inst_index];
            let inst = self.tree.get(inst_id);

            if let Some((instruction, skip)) = self
                .try_fuse_addr_access(inst, self.block.instructions.get(inst_index + 1).copied())
            {
                instructions.push(instruction);
                inst_index += skip;
                mir_instruction_offsets.push(inst_index as u32);
                continue;
            }

            if let Some((instruction, skip)) = self
                .try_fuse_const_binary(inst, self.block.instructions.get(inst_index + 1).copied())
            {
                instructions.push(instruction);
                inst_index += skip;
                mir_instruction_offsets.push(inst_index as u32);
                continue;
            }

            let instruction = self.lower_instruction(inst, pool)?;
            instructions.push(instruction);
            inst_index += 1;
            mir_instruction_offsets.push(inst_index as u32);
        }

        let terminator = self.tree.get(self.block.terminator);

        if let Some(fused) = self.try_fuse_compare_branch(self.block, &mut instructions, pool) {
            instructions.push(fused);
        } else {
            let lowered_terminator = self.lower_terminator(terminator, pool)?;
            instructions.push(lowered_terminator);
        }

        mir_instruction_offsets.push((self.block.instructions.len() + 1) as u32);
        let mir_instruction_count = (self.block.instructions.len() + 1) as u32;

        Ok(Block {
            mir_block: self.mir_block,
            instructions,
            mir_instruction_offsets,
            mir_instruction_count,
        })
    }

    /// Return the lowered use count for one value.
    pub(super) fn value_use_count(&self, value: mir::Value) -> Option<u32> {
        self.function.value_use_count.get(value.0 as usize).copied()
    }

    /// Return one call target for one function id.
    pub(super) fn call_target(
        &self,
        function: mir::LocalNodeId<mir::Function>,
    ) -> Result<CallTarget> {
        lookup_call_target(self.call_targets, function).ok_or_else(|| Error::InvariantViolation {
            context: format!("missing call target for function: {function:?}"),
        })
    }

    /// Return one lowered local index for one local id.
    pub(super) fn local_index(&self, local: mir::LocalNodeId<mir::Local>) -> Result<u32> {
        self.local_index_by_id
            .get(&local)
            .copied()
            .ok_or_else(|| Error::InvariantViolation {
                context: format!("missing local index for {local:?}"),
            })
    }

    /// Return the lowered value kind map.
    pub(super) fn value_kind_map(&self) -> &ValueKindMap {
        &self.function.value_kind_map
    }

    /// Return the lowered value types.
    pub(super) fn value_type(&self) -> &[mir::LocalNodeId<mir::Type>] {
        &self.function.value_type
    }

    /// Return the lowered VM layouts.
    pub(super) fn layouts(&self) -> &HashMap<mir::LocalNodeId<mir::Type>, Layout> {
        self.function.layouts
    }
}

/// Compute SSA value use counts across the function.
fn compute_value_use_counts(
    tree: &mir::NodeTree,
    mir_blocks: &[mir::LocalNodeId<mir::Block>],
    value_count: usize,
) -> Result<Vec<u32>> {
    // allocate use counters
    let mut uses = vec![0u32; value_count];

    // record a single use safely
    let mut record_use = |value: mir::Value| {
        if let Some(slot) = uses.get_mut(value.0 as usize) {
            *slot = slot.saturating_add(1);
        }
    };

    // scan instructions and terminators for value uses
    for block_id in mir_blocks {
        let block = tree.get(*block_id);
        let terminator = tree.get(block.terminator);
        for inst_id in &block.instructions {
            let inst = tree.get(*inst_id);
            for value in inst.uses() {
                record_use((value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "instruction use".to_string(),
                })?);
            }
            if let Some(args) = inst.argument_slice() {
                for arg in tree.get_arguments(args) {
                    record_use((*arg).value().ok_or_else(|| Error::ConcreteMirRequired {
                        context: "instruction argument".to_string(),
                    })?);
                }
            }
        }
        for value in terminator.uses() {
            record_use((value).value().ok_or_else(|| Error::ConcreteMirRequired {
                context: "terminator use".to_string(),
            })?);
        }
    }

    // return the use table
    Ok(uses)
}
