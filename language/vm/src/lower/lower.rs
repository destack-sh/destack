use std::collections::HashMap;
use std::ops::Deref;

use {destack_engine as engine, destack_heap as heap, destack_mir as mir};

use crate::program::{Block, BlockCode, CallTarget, Function, Layout, SideTableBuilder};
use crate::{Error, Result};

use super::block::{BlockOrder, FunctionContext};
use super::pool::{Pool, lookup_call_target};
use super::tree::ValueType;
use super::value::{ValueLayoutMap, ValueLayoutMapBuilder};

/// One whole-function lowering session.
struct FunctionLowerer<'a, 'table> {
    context: FunctionContext<'a>,
    func: &'a mir::Function,
    frame_layout: &'a engine::FrameLayout,
    pool: Pool<'a, 'table>,
}

impl<'a, 'table> FunctionLowerer<'a, 'table> {
    /// Create one function lowerer for the given MIR function.
    fn new(
        tree: &'a mir::Tree,
        func_id: mir::LocalNodeId<mir::Function>,
        frame_layout: &'a engine::FrameLayout,
        yield_frame_states: &'a HashMap<mir::LocalNodeId<mir::Block>, engine::FrameStateId>,
        call_frame_states: &'a HashMap<mir::LocalNodeId<mir::Block>, engine::FrameStateId>,
        call_targets: &'a HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
        layouts: &'a HashMap<mir::LocalNodeId<mir::Type>, Layout>,
        layout_id_by_type: &'a HashMap<mir::LocalNodeId<mir::Type>, mir::LayoutId>,
        heap_options: &'a heap::HeapOptions,
        shared_heap_options: &'a heap::HeapOptions,
        value_types: &'a [ValueType],
        side_table: &'table mut SideTableBuilder,
    ) -> Result<Option<Self>> {
        let func = tree.get(func_id);

        if func.is_import() {
            return Ok(None);
        }

        let Some(entry) = func.entry else {
            return Ok(None);
        };

        let block_order = BlockOrder::new(tree, entry)?;
        let value_type = value_types
            .iter()
            .map(|value_type| value_type.ty)
            .collect::<Vec<_>>();
        let value_count = value_types.len();
        let value_layout_map =
            ValueLayoutMapBuilder::new(tree, func, &block_order.block, &value_type, value_count)
                .build();
        let value_use_count = compute_value_use_counts(tree, &block_order.block, value_count)?;
        let local_index_by_id = Self::local_index_map(func);
        let block_parameter = Self::block_parameter(tree, &block_order.block)?;
        let entry_block = block_order.index_by_id[&entry] as u32;
        let context = FunctionContext {
            tree,
            function_id: func_id,
            entry_block,
            yield_frame_states,
            call_frame_states,
            call_targets,
            frame_layout,
            value_layout_map,
            value_type,
            layouts,
            layout_id_by_type,
            heap_options,
            shared_heap_options,
            block_index_by_id: block_order.index_by_id,
            block_parameter,
            local_index_by_id,
            value_use_count,
        };

        Ok(Some(Self {
            context,
            func,
            frame_layout,
            pool: Pool::new(side_table, frame_layout),
        }))
    }

    /// Lower the function into program form.
    fn lower(mut self) -> Result<Function> {
        let parameter_value = self
            .func
            .parameters
            .iter()
            .map(|parameter| {
                (parameter.value)
                    .value()
                    .ok_or_else(|| Error::MissingRepresentation {
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
        let mut lowered_blocks = Vec::with_capacity(mir_block.len());

        for (_, mir_block) in mir_block {
            lowered_blocks.push(self.lower_block(mir_block)?);
        }

        let (argument_pool, move_pool) = self.pool.finish();
        let mut code = Vec::new();
        let mut blocks = Vec::with_capacity(lowered_blocks.len());

        for block in lowered_blocks {
            let start = code.len() as u32;
            let len = block.instructions.len() as u32;

            code.extend(block.instructions);
            blocks.push(Block {
                mir_block: block.mir_block,
                start,
                len,
                mir_point_by_pc: block.mir_point_by_pc,
            });
        }

        Ok(Function {
            mir_function: self.context.function_id,
            frame_layout: self.frame_layout.id,
            parameters: parameter,
            entry: self.context.entry_block,
            code,
            blocks,
            argument_pool,
            move_pool,
        })
    }

    /// Lower one MIR block into program form.
    fn lower_block(&mut self, mir_block: mir::LocalNodeId<mir::Block>) -> Result<BlockCode> {
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
        tree: &mir::Tree,
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
                        .ok_or_else(|| Error::MissingRepresentation {
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
    tree: &mir::Tree,
    func_id: mir::LocalNodeId<mir::Function>,
    frame_layout: &engine::FrameLayout,
    yield_frame_states: &HashMap<mir::LocalNodeId<mir::Block>, engine::FrameStateId>,
    call_frame_states: &HashMap<mir::LocalNodeId<mir::Block>, engine::FrameStateId>,
    call_targets: &HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    layout_id_by_type: &HashMap<mir::LocalNodeId<mir::Type>, mir::LayoutId>,
    heap_options: &heap::HeapOptions,
    shared_heap_options: &heap::HeapOptions,
    value_types: &[ValueType],
    side_table: &mut SideTableBuilder,
) -> Result<Option<Function>> {
    let lowerer = FunctionLowerer::new(
        tree,
        func_id,
        frame_layout,
        yield_frame_states,
        call_frame_states,
        call_targets,
        layouts,
        layout_id_by_type,
        heap_options,
        shared_heap_options,
        value_types,
        side_table,
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

    /// Lower the block into program form.
    fn lower(self, pool: &mut Pool<'_, '_>) -> Result<BlockCode> {
        let mut instructions = Vec::with_capacity(self.block.instructions.len() + 1);
        let mut mir_point_by_pc = Vec::with_capacity(self.block.instructions.len() + 2);
        mir_point_by_pc.push(0);

        // convert regular instructions
        let mut inst_index = 0usize;
        while inst_index < self.block.instructions.len() {
            let inst_id = self.block.instructions[inst_index];
            let inst = self.tree.get(inst_id);

            // lower the remaining instruction shape
            let lowered = self.lower_instructions(inst, pool)?;
            let lowered_len = lowered.len();
            instructions.extend(lowered);

            // internal instructions still belong to the current MIR operation
            for _ in 1..lowered_len {
                mir_point_by_pc.push(inst_index as u32);
            }

            inst_index += 1;

            // record the MIR point after the MIR operation is complete
            if lowered_len > 0 {
                mir_point_by_pc.push(inst_index as u32);
            }
        }

        let terminator = self.tree.get(self.block.terminator);

        // fuse compare producers into conditional branches
        if let Some(fused) = self.try_fuse_compare_branch(self.block, &mut instructions, pool) {
            instructions.push(fused);
        } else {
            let lowered_terminator = self.lower_terminator(terminator, pool)?;
            instructions.push(lowered_terminator);
        }

        mir_point_by_pc.push((self.block.instructions.len() + 1) as u32);

        Ok(BlockCode {
            mir_block: self.mir_block,
            instructions,
            mir_point_by_pc,
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

    /// Return the lowered value layout map.
    pub(super) fn value_layout_map(&self) -> &ValueLayoutMap {
        &self.function.value_layout_map
    }

    /// Return the lowered value types.
    pub(super) fn value_type(&self) -> &[mir::LocalNodeId<mir::Type>] {
        &self.function.value_type
    }

    /// Return the lowered VM layouts.
    pub(super) fn layouts(&self) -> &HashMap<mir::LocalNodeId<mir::Type>, Layout> {
        self.function.layouts
    }

    /// Return the MIR layout id for one type.
    pub(super) fn layout_id_for_type(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Result<mir::LayoutId> {
        self.function
            .layout_id_by_type
            .get(&ty)
            .copied()
            .ok_or_else(|| Error::InvariantViolation {
                context: format!("missing layout id for type: {ty:?}"),
            })
    }
}

/// Compute SSA value use counts across the function.
fn compute_value_use_counts(
    tree: &mir::Tree,
    mir_blocks: &[mir::LocalNodeId<mir::Block>],
    value_count: usize,
) -> Result<Vec<u32>> {
    // allocate use counters
    let mut uses = vec![0u32; value_count];

    // record a single use safely
    let mut record_use = |value: mir::Value| {
        if let Some(count) = uses.get_mut(value.0 as usize) {
            *count += 1;
        }
    };

    // scan instructions and terminators for value uses
    for block_id in mir_blocks {
        let block = tree.get(*block_id);
        let terminator = tree.get(block.terminator);
        for inst_id in &block.instructions {
            let inst = tree.get(*inst_id);
            for value in inst.uses() {
                record_use(
                    (value)
                        .value()
                        .ok_or_else(|| Error::MissingRepresentation {
                            context: "instruction use".to_string(),
                        })?,
                );
            }
            if let Some(args) = inst.argument_slice() {
                for arg in tree.get_arguments(args) {
                    record_use((*arg).value().ok_or_else(|| Error::MissingRepresentation {
                        context: "instruction argument".to_string(),
                    })?);
                }
            }
        }
        for value in terminator.uses() {
            record_use(
                (value)
                    .value()
                    .ok_or_else(|| Error::MissingRepresentation {
                        context: "terminator use".to_string(),
                    })?,
            );
        }
    }

    // return the use table
    Ok(uses)
}
