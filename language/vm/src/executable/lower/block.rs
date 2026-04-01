use std::collections::{HashMap, HashSet};

use destack_mir as mir;

use super::super::layout::Layout;
use super::kind::ValueKindMap;
use super::tree::BlockParameterMap;

/// One lowered block traversal order.
pub(super) struct BlockOrder {
    /// The lowered block index by MIR block id.
    pub(super) index_by_id: HashMap<mir::LocalNodeId<mir::Block>, usize>,
    /// The MIR blocks in lowered traversal order.
    pub(super) block: Vec<mir::LocalNodeId<mir::Block>>,
}

impl BlockOrder {
    /// Build one lowered block traversal order from the entry block.
    pub(super) fn new(tree: &mir::NodeTree, entry_block: mir::LocalNodeId<mir::Block>) -> Self {
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
            match &mir_block.terminator {
                mir::Terminator::Jump { target, .. } => {
                    queue.push(*target);
                }
                mir::Terminator::Branch {
                    then_target,
                    else_target,
                    ..
                } => {
                    queue.push(*then_target);
                    queue.push(*else_target);
                }
                mir::Terminator::Check {
                    success, failure, ..
                } => {
                    queue.push(success.target);
                    queue.push(failure.target);
                }
                mir::Terminator::Switch { cases, default, .. } => {
                    for case in cases {
                        queue.push(case.target);
                    }
                    queue.push(*default);
                }
                mir::Terminator::Yield { resume, .. } => {
                    queue.push(*resume);
                }
                mir::Terminator::Call {
                    normal_target,
                    unwind_target,
                    ..
                }
                | mir::Terminator::CallIndirect {
                    normal_target,
                    unwind_target,
                    ..
                }
                | mir::Terminator::CallVirtual {
                    normal_target,
                    unwind_target,
                    ..
                }
                | mir::Terminator::CallInterface {
                    normal_target,
                    unwind_target,
                    ..
                } => {
                    queue.push(*normal_target);
                    queue.push(*unwind_target);
                }
                mir::Terminator::Return { .. }
                | mir::Terminator::Throw { .. }
                | mir::Terminator::Trap { .. }
                | mir::Terminator::Unreachable
                | mir::Terminator::TailCall { .. }
                | mir::Terminator::TailCallIndirect { .. }
                | mir::Terminator::TailCallVirtual { .. }
                | mir::Terminator::TailCallInterface { .. } => {}
            }
        }

        debug_assert!(
            block.len() <= u32::MAX as usize,
            "too many blocks for lowered block indices"
        );

        Self { index_by_id, block }
    }
}

/// One shared function-scoped lowering context.
pub(super) struct FunctionContext<'a> {
    /// The MIR node tree.
    pub(super) tree: &'a mir::NodeTree,
    /// The current MIR function id.
    pub(super) function_id: mir::LocalNodeId<mir::Function>,
    /// The lowered entry block index.
    pub(super) entry_block: u32,
    /// The lowered yield resume point by MIR block id.
    pub(super) yield_resume_points:
        &'a HashMap<mir::LocalNodeId<mir::Block>, destack_engine::ResumePointId>,
    /// The lowered exceptional call resume points by MIR block id.
    pub(super) exceptional_call_resume_points: &'a HashMap<
        mir::LocalNodeId<mir::Block>,
        (destack_engine::ResumePointId, destack_engine::ResumePointId),
    >,
    /// The lowered function index by MIR function id.
    pub(super) function_indices: &'a HashMap<mir::LocalNodeId<mir::Function>, u32>,
    /// The lowered value kind by SSA value id.
    pub(super) value_kind_map: ValueKindMap,
    /// The lowered value type by SSA value id.
    pub(super) value_type: Vec<mir::LocalNodeId<mir::Type>>,
    /// The lowered VM layout by MIR type id.
    pub(super) layouts: &'a HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    /// The lowered block index by MIR block id.
    pub(super) block_index_by_id: HashMap<mir::LocalNodeId<mir::Block>, usize>,
    /// The lowered block parameter values by block index.
    pub(super) block_parameter: Vec<Vec<mir::Value>>,
    /// The lowered local index by MIR local id.
    pub(super) local_index_by_id: HashMap<mir::LocalNodeId<mir::Local>, u32>,
    /// The lowered SSA value use count by SSA value id.
    pub(super) value_use_count: Vec<u32>,
    /// The block parameter decomposition metadata.
    pub(super) block_parameter_map: &'a BlockParameterMap,
}
