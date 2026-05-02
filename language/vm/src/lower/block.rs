use std::collections::{HashMap, HashSet};

use {destack_engine as engine, destack_heap as heap, destack_mir as mir};

use crate::program::{CallTarget, Layout};
use crate::{Error, Result};

use super::value::ValueLayoutMap;

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
                    queue.push((target.block).block().ok_or_else(|| {
                        Error::MissingRepresentation {
                            context: "jump target".to_string(),
                        }
                    })?);
                }
                mir::Terminator::Branch {
                    then_target,
                    else_target,
                    ..
                } => {
                    queue.push((then_target.block).block().ok_or_else(|| {
                        Error::MissingRepresentation {
                            context: "branch then target".to_string(),
                        }
                    })?);
                    queue.push((else_target.block).block().ok_or_else(|| {
                        Error::MissingRepresentation {
                            context: "branch else target".to_string(),
                        }
                    })?);
                }
                mir::Terminator::Check {
                    success, failure, ..
                } => {
                    queue.push((success.block).block().ok_or_else(|| {
                        Error::MissingRepresentation {
                            context: "check success target".to_string(),
                        }
                    })?);
                    queue.push((failure.block).block().ok_or_else(|| {
                        Error::MissingRepresentation {
                            context: "check failure target".to_string(),
                        }
                    })?);
                }
                mir::Terminator::Switch { cases, default, .. } => {
                    for case in cases {
                        queue.push((case.target.block).block().ok_or_else(|| {
                            Error::MissingRepresentation {
                                context: "switch case target".to_string(),
                            }
                        })?);
                    }
                    queue.push((default.block).block().ok_or_else(|| {
                        Error::MissingRepresentation {
                            context: "switch default target".to_string(),
                        }
                    })?);
                }
                mir::Terminator::Yield { resume, .. } => {
                    queue.push((resume.block).block().ok_or_else(|| {
                        Error::MissingRepresentation {
                            context: "yield resume target".to_string(),
                        }
                    })?);
                }
                mir::Terminator::Invoke {
                    normal_target,
                    unwind_target,
                    ..
                }
                | mir::Terminator::InvokeIndirect {
                    normal_target,
                    unwind_target,
                    ..
                }
                | mir::Terminator::InvokeVirtual {
                    normal_target,
                    unwind_target,
                    ..
                }
                | mir::Terminator::InvokeInterface {
                    normal_target,
                    unwind_target,
                    ..
                } => {
                    queue.push((normal_target.block).block().ok_or_else(|| {
                        Error::MissingRepresentation {
                            context: "invoke normal target".to_string(),
                        }
                    })?);
                    queue.push((unwind_target.block).block().ok_or_else(|| {
                        Error::MissingRepresentation {
                            context: "invoke unwind target".to_string(),
                        }
                    })?);
                }
                mir::Terminator::Error => {
                    return Err(Error::MissingRepresentation {
                        context: "terminator".to_string(),
                    });
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

        Ok(Self { index_by_id, block })
    }
}

/// One shared function-scoped lowering context.
pub(super) struct FunctionContext<'a> {
    /// The MIR tree.
    pub(super) tree: &'a mir::Tree,
    /// The current MIR function id.
    pub(super) function_id: mir::LocalNodeId<mir::Function>,
    /// The lowered entry block index.
    pub(super) entry_block: u32,
    /// The lowered yield frame state by MIR block id.
    pub(super) yield_frame_states: &'a HashMap<mir::LocalNodeId<mir::Block>, engine::FrameStateId>,
    /// The lowered exceptional call frame states by MIR block id.
    pub(super) exceptional_call_frame_states:
        &'a HashMap<mir::LocalNodeId<mir::Block>, (engine::FrameStateId, engine::FrameStateId)>,
    /// The call target by MIR function id.
    pub(super) call_targets: &'a HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
    /// The byte layout for this lowered function frame.
    pub(super) frame_layout: &'a engine::FrameLayout,
    /// The lowered value layout by SSA value id.
    pub(super) value_layout_map: ValueLayoutMap,
    /// The lowered value type by SSA value id.
    pub(super) value_type: Vec<mir::LocalNodeId<mir::Type>>,
    /// The lowered VM layout by MIR type id.
    pub(super) layouts: &'a HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    /// The worker-local heap allocation geometry.
    pub(super) heap_options: &'a heap::HeapOptions,
    /// The runtime-shared heap allocation geometry.
    pub(super) shared_heap_options: &'a heap::HeapOptions,
    /// The lowered block index by MIR block id.
    pub(super) block_index_by_id: HashMap<mir::LocalNodeId<mir::Block>, usize>,
    /// The lowered block parameter values by block index.
    pub(super) block_parameter: Vec<Vec<mir::Value>>,
    /// The lowered local index by MIR local id.
    pub(super) local_index_by_id: HashMap<mir::LocalNodeId<mir::Local>, u32>,
    /// The lowered SSA value use count by SSA value id.
    pub(super) value_use_count: Vec<u32>,
}
