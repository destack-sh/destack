use std::collections::VecDeque;

use crate as mir;
use crate::{ControlTable, NodeTable};

/// Lattice for merging dataflow states.
pub trait Lattice: Clone + PartialEq {
    /// Return the greatest lower bound of two states.
    fn meet(&self, other: &Self) -> Self;
}

/// One forward dataflow transfer.
#[derive(Debug)]
pub enum ForwardTransfer<'a> {
    /// Transfer one complete block.
    Block(mir::LocalNodeId<mir::Block>),
    /// Transfer one control-flow edge.
    Edge {
        /// The transferred edge.
        edge: mir::Edge,
        /// The target carried by the edge.
        target: &'a mir::BlockTarget,
    },
}

/// Entry and exit states from one dataflow analysis.
#[derive(Debug, Clone)]
pub struct DataflowTable<S> {
    /// Entry state for each block.
    block_entry: NodeTable<mir::Block, Option<S>>,
    /// Exit state for each block.
    block_exit: NodeTable<mir::Block, Option<S>>,
}

impl<S> DataflowTable<S> {
    /// Create an empty result.
    pub fn new() -> Self {
        Self {
            block_entry: NodeTable::new(),
            block_exit: NodeTable::new(),
        }
    }

    /// Create a result large enough for one function.
    pub fn for_function(function: &mir::Function) -> Self {
        Self {
            block_entry: NodeTable::from_nodes(function.blocks(), || None),
            block_exit: NodeTable::from_nodes(function.blocks(), || None),
        }
    }

    /// Return the state at entry to a block.
    pub fn entry(&self, block: mir::LocalNodeId<mir::Block>) -> Option<&S> {
        self.block_entry.get(block).as_ref()
    }

    /// Return the state at exit of a block.
    pub fn exit(&self, block: mir::LocalNodeId<mir::Block>) -> Option<&S> {
        self.block_exit.get(block).as_ref()
    }

    /// Set the state at entry to a block.
    pub fn set_entry(&mut self, block: mir::LocalNodeId<mir::Block>, state: S) {
        *self.block_entry.get_mut(block) = Some(state);
    }

    /// Set the state at exit of a block.
    pub fn set_exit(&mut self, block: mir::LocalNodeId<mir::Block>, state: S) {
        *self.block_exit.get_mut(block) = Some(state);
    }
}

impl<S> Default for DataflowTable<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> DataflowTable<S>
where
    S: Lattice,
{
    /// Run a forward dataflow analysis using a worklist algorithm.
    pub fn forward<F>(
        function: mir::FunctionId,
        tree: &mir::Tree,
        cfg: &ControlTable,
        entry_state: S,
        mut transfer: F,
    ) -> Self
    where
        F: for<'a> FnMut(ForwardTransfer<'a>, S, &mir::Tree) -> S,
    {
        let entry = match tree.get(function).entry() {
            Some(entry) => entry,
            None => return Self::new(),
        };

        let mut result = Self::for_function(tree.get(function));

        // seed the worklist with the entry block
        let mut worklist: VecDeque<mir::LocalNodeId<mir::Block>> = VecDeque::new();
        let mut in_worklist = NodeTable::from_nodes(tree.get(function).blocks(), || false);

        worklist.push_back(entry);
        *in_worklist.get_mut(entry) = true;

        // reuse edge storage while transfers instantiate types
        let mut incoming = Vec::new();
        while let Some(block_id) = worklist.pop_front() {
            *in_worklist.get_mut(block_id) = false;

            // merge the function entry state and every reached incoming edge
            let mut merged = (block_id == entry).then(|| entry_state.clone());
            incoming.clear();
            incoming.extend(
                cfg.incoming_edges(block_id, tree)
                    .map(|(edge, target)| (edge, target.clone())),
            );
            for (edge, target) in &incoming {
                let Some(predecessor_exit) = result.exit(edge.source) else {
                    continue;
                };

                // transfer and merge this exact edge's state
                let state = transfer(
                    ForwardTransfer::Edge {
                        edge: *edge,
                        target,
                    },
                    predecessor_exit.clone(),
                    tree,
                );
                merged = Some(match merged {
                    Some(merged) => merged.meet(&state),
                    None => state,
                });
            }
            let Some(new_entry) = merged else {
                continue;
            };

            // skip blocks whose entry is already stable
            let entry_changed = result
                .entry(block_id)
                .map(|old| old != &new_entry)
                .unwrap_or(true);
            if !entry_changed {
                continue;
            }

            result.set_entry(block_id, new_entry.clone());

            // apply transfer from entry state to exit state
            let exit_state = transfer(ForwardTransfer::Block(block_id), new_entry, tree);
            let exit_changed = result
                .exit(block_id)
                .map(|old| old != &exit_state)
                .unwrap_or(true);
            if !exit_changed {
                continue;
            }

            result.set_exit(block_id, exit_state);

            // enqueue successors that may observe the changed exit
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);
            for successor in terminator.successors(tree) {
                if !*in_worklist.get(successor) {
                    worklist.push_back(successor);
                    *in_worklist.get_mut(successor) = true;
                }
            }
        }

        result
    }
}
