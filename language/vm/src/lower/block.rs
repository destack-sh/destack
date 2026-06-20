use std::collections::{HashMap, HashSet};

use destack_heap as heap;
use destack_mir as mir;

use crate::{Error, Result};
use destack_program::vm::CallTarget;
use destack_program::{FunctionId, ProgramIndex, TypeTable};

use super::layout::ValueLayout;
use super::value::ValueShapeMap;

/// One lowered block traversal order.
pub(super) struct BlockOrder {
    /// The lowered block index by MIR block id.
    pub(super) index_by_id: HashMap<mir::LocalNodeId<mir::Block>, usize>,
    /// The MIR blocks in lowered traversal order.
    pub(super) block: Vec<mir::LocalNodeId<mir::Block>>,
}

impl BlockOrder {
    /// Build one lowered block traversal order from the entry block.
    pub(super) fn new(tree: &mir::Tree, entry_block: mir::LocalNodeId<mir::Block>) -> Result<Self> {
        let mut index_by_id = HashMap::new();
        let mut block = Vec::new();
        let mut queue = vec![entry_block];
        let mut visited = HashSet::new();

        while let Some(block_id) = queue.pop() {
            // skip visited blocks
            if visited.contains(&block_id) {
                continue;
            }

            // record block index
            visited.insert(block_id);
            let block_index = block.len();
            index_by_id.insert(block_id, block_index);
            block.push(block_id);

            // enqueue successor blocks
            let mir_block = tree.get(block_id);
            let terminator = tree.get(mir_block.terminator);
            match terminator {
                mir::Terminator::Jump { target, .. } => {
                    queue.push(target.block);
                }
                mir::Terminator::Branch {
                    then_target,
                    else_target,
                    ..
                } => {
                    queue.push(then_target.block);
                    queue.push(else_target.block);
                }
                mir::Terminator::Check {
                    success, failure, ..
                } => {
                    queue.push(success.block);
                    queue.push(failure.block);
                }
                mir::Terminator::NewZeroedTry {
                    success, failure, ..
                }
                | mir::Terminator::NewUninitTry {
                    success, failure, ..
                }
                | mir::Terminator::NewSliceZeroedTry {
                    success, failure, ..
                }
                | mir::Terminator::NewSliceUninitTry {
                    success, failure, ..
                } => {
                    queue.push(success.block);
                    queue.push(failure.block);
                }
                mir::Terminator::Switch { cases, default, .. } => {
                    for case in tree.get_switch_cases(*cases) {
                        queue.push(case.target.block);
                    }
                    queue.push(default.block);
                }
                mir::Terminator::Yield { resume, .. } => {
                    queue.push(resume.block);
                }
                mir::Terminator::Call { target, unwind, .. }
                | mir::Terminator::CallIndirect { target, unwind, .. }
                | mir::Terminator::CallVirtual { target, unwind, .. }
                | mir::Terminator::CallDynamic { target, unwind, .. } => {
                    queue.push(target.block);
                    if let Some(unwind) = unwind {
                        queue.push(unwind.block);
                    }
                }
                mir::Terminator::Error => {
                    return Err(Error::invalid_program("terminator"));
                }
                mir::Terminator::Return { .. }
                | mir::Terminator::Panic { .. }
                | mir::Terminator::UnwindResume
                | mir::Terminator::Trap { .. }
                | mir::Terminator::Unreachable
                | mir::Terminator::TailCall { .. }
                | mir::Terminator::TailCallIndirect { .. }
                | mir::Terminator::TailCallVirtual { .. }
                | mir::Terminator::TailCallDynamic { .. } => {}
            }
        }

        debug_assert!(
            block.len() <= u32::MAX as usize,
            "too many blocks for lowered block indices"
        );

        Ok(Self { index_by_id, block })
    }
}

/// One shared function-scoped lowerer context.
pub(super) struct FunctionContext<'a> {
    /// The MIR tree.
    pub(super) tree: &'a mir::Tree,
    /// The current MIR function id.
    pub(super) function_id: mir::LocalNodeId<mir::Function>,
    /// The lowered entry block index.
    pub(super) entry_block: u32,
    /// The lowered yield frame state by MIR block id.
    pub(super) yield_frame_states: &'a HashMap<mir::LocalNodeId<mir::Block>, mir::FrameStateId>,
    /// The lowered call terminator frame state by MIR block id.
    pub(super) call_frame_states: &'a HashMap<mir::LocalNodeId<mir::Block>, mir::FrameStateId>,
    /// The call target by program function id.
    pub(super) call_targets: &'a HashMap<FunctionId, CallTarget>,
    /// Dense program id index.
    pub(super) index: &'a ProgramIndex,
    /// The lowered runtime type table.
    pub(super) types: &'a TypeTable,
    /// Pointer byte width used by pointer-sized runtime values.
    pub(super) pointer_bytes: u8,
    /// The byte layout for this lowered function frame.
    pub(super) frame_layout: &'a mir::FrameLayout,
    /// The lowered value shape by SSA value id.
    pub(super) value_shape_map: ValueShapeMap,
    /// The lowered value type by SSA value id.
    pub(super) value_type: Vec<mir::LocalNodeId<mir::Type>>,
    /// The lowered VM layout by MIR type id.
    pub(super) layouts: &'a HashMap<mir::LocalNodeId<mir::Type>, ValueLayout>,
    /// The worker-local heap allocation geometry.
    pub(super) heap_options: &'a heap::HeapOptions,
    /// The runtime-shared heap allocation geometry.
    pub(super) shared_heap_options: &'a heap::SharedHeapOptions,
    /// The lowered block index by MIR block id.
    pub(super) block_index_by_id: HashMap<mir::LocalNodeId<mir::Block>, usize>,
    /// The lowered block parameter values by block index.
    pub(super) block_parameter: Vec<Vec<mir::Value>>,
    /// The lowered local index by MIR local id.
    pub(super) local_index_by_id: HashMap<mir::LocalNodeId<mir::Local>, u32>,
    /// The lowered SSA value use count by SSA value id.
    pub(super) value_use_count: Vec<u32>,
}

impl<'a> FunctionContext<'a> {
    /// Return whether one frame slot is lowered as one VM cell.
    pub(super) fn slot_is_cell(&self, slot: &mir::FrameSlot) -> bool {
        self.layouts.get(&slot.ty).is_some_and(ValueLayout::is_cell)
    }

    /// Return the program function id for one MIR function.
    pub(super) fn program_function(&self, function: mir::LocalNodeId<mir::Function>) -> FunctionId {
        self.index.function_id(function)
    }

    /// Return the runtime shape for one MIR type.
    pub(super) fn value_shape_for_type(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Option<destack_program::vm::ValueShape> {
        self.types.value_shape_for_mir(ty, self.pointer_bytes)
    }

    /// Return values stored in one MIR value slice.
    #[inline]
    pub(super) fn values(&self, slice: mir::ValueSlice) -> &'a [mir::Value] {
        self.tree.get_values(slice)
    }

    /// Return indices stored in one MIR index slice.
    #[inline]
    pub(super) fn indices(&self, slice: mir::IndexSlice) -> &'a [u32] {
        self.tree.get_indices(slice)
    }

    /// Return switch cases stored in one MIR switch slice.
    #[inline]
    pub(super) fn switch_cases(&self, slice: mir::SwitchCaseSlice) -> &'a [mir::SwitchCase] {
        self.tree.get_switch_cases(slice)
    }

    /// Return arguments stored on one MIR block target.
    #[inline]
    pub(super) fn target_arguments(&self, target: &mir::BlockTarget) -> &'a [mir::Value] {
        target.arguments(self.tree)
    }
}
