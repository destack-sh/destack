use std::collections::VecDeque;

use crate as mir;
use crate::NodeTable;
use destack_core::FxIndexSet;

use crate::ControlTable;

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
pub struct Dataflow<S> {
    /// Entry state for each block.
    block_entry: NodeTable<mir::Block, Option<S>>,
    /// Exit state for each block.
    block_exit: NodeTable<mir::Block, Option<S>>,
}

impl<S> Dataflow<S> {
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

impl<S> Default for Dataflow<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Dataflow<S>
where
    S: Lattice,
{
    /// Run a forward dataflow analysis using a worklist algorithm.
    pub fn forward<F>(
        function: &mir::Function,
        tree: &mir::Tree,
        cfg: &ControlTable,
        entry_state: S,
        mut transfer: F,
    ) -> Self
    where
        F: for<'a> FnMut(ForwardTransfer<'a>, S, &mir::Tree) -> S,
    {
        let entry = match function.entry() {
            Some(entry) => entry,
            None => return Self::new(),
        };

        let mut result = Self::for_function(function);

        // seed the worklist with the entry block
        let mut worklist: VecDeque<mir::LocalNodeId<mir::Block>> = VecDeque::new();
        let mut in_worklist = NodeTable::from_nodes(function.blocks(), || false);

        worklist.push_back(entry);
        *in_worklist.get_mut(entry) = true;

        while let Some(block_id) = worklist.pop_front() {
            *in_worklist.get_mut(block_id) = false;

            // merge the function entry state and every reached incoming edge
            let mut merged = (block_id == entry).then(|| entry_state.clone());
            for &predecessor in cfg.predecessors(block_id) {
                let Some(predecessor_exit) = result.exit(predecessor) else {
                    continue;
                };
                let predecessor_block = tree.get(predecessor);
                let terminator = tree.get(predecessor_block.terminator);
                for (edge, target) in terminator
                    .targets(tree, predecessor)
                    .into_iter()
                    .filter(|(_, target)| target.block == block_id)
                {
                    let state = transfer(
                        ForwardTransfer::Edge { edge, target },
                        predecessor_exit.clone(),
                        tree,
                    );
                    merged = Some(match merged {
                        Some(merged) => merged.meet(&state),
                        None => state,
                    });
                }
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

    /// Run a backward dataflow analysis using a worklist algorithm.
    pub fn backward<F>(
        function: &mir::Function,
        tree: &mir::Tree,
        cfg: &ControlTable,
        exit_state: S,
        mut transfer: F,
    ) -> Self
    where
        F: FnMut(mir::LocalNodeId<mir::Block>, S, &mir::Tree) -> S,
    {
        if function.entry().is_none() {
            return Self::new();
        }

        let mut result = Self::for_function(function);

        // seed terminal blocks with the caller-provided exit state
        for &block_id in function.blocks() {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);
            if matches!(
                terminator,
                mir::Terminator::Return { .. }
                    | mir::Terminator::Abort { .. }
                    | mir::Terminator::Panic { .. }
                    | mir::Terminator::Unreachable
                    | mir::Terminator::TailCall { .. }
            ) {
                result.set_exit(block_id, exit_state.clone());
            }
        }

        // seed the worklist from every terminal block
        let mut worklist: VecDeque<mir::LocalNodeId<mir::Block>> = VecDeque::new();
        let mut in_worklist = NodeTable::from_nodes(function.blocks(), || false);

        for &block_id in function.blocks() {
            if result.exit(block_id).is_some() {
                worklist.push_back(block_id);
                *in_worklist.get_mut(block_id) = true;
            }
        }

        while let Some(block_id) = worklist.pop_front() {
            *in_worklist.get_mut(block_id) = false;

            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);

            // merge successor entries into the block exit
            let new_exit = if let Some(exit) = result.exit(block_id) {
                let mut merged = exit.clone();
                for successor in terminator.successors(tree) {
                    if let Some(successor_entry) = result.entry(successor) {
                        merged = merged.meet(successor_entry);
                    }
                }

                merged
            } else {
                let successors = terminator.successors(tree);
                if successors.is_empty() {
                    continue;
                }

                let mut successor_entries = successors
                    .into_iter()
                    .filter_map(|successor| result.entry(successor));
                let Some(first_entry) = successor_entries.next() else {
                    continue;
                };

                let mut merged = first_entry.clone();
                for successor_entry in successor_entries {
                    merged = merged.meet(successor_entry);
                }

                merged
            };

            // skip blocks whose exit is already stable
            let exit_changed = result
                .exit(block_id)
                .map(|old| old != &new_exit)
                .unwrap_or(true);
            if !exit_changed {
                continue;
            }

            result.set_exit(block_id, new_exit.clone());

            // apply transfer from exit state to entry state
            let entry_state = transfer(block_id, new_exit, tree);
            let entry_changed = result
                .entry(block_id)
                .map(|old| old != &entry_state)
                .unwrap_or(true);
            if !entry_changed {
                continue;
            }

            result.set_entry(block_id, entry_state);

            // enqueue predecessors that may observe the changed entry
            for &predecessor in cfg.predecessors(block_id) {
                if !*in_worklist.get(predecessor) {
                    worklist.push_back(predecessor);
                    *in_worklist.get_mut(predecessor) = true;
                }
            }
        }

        result
    }
}

/// Set lattice whose meet operation is union.
impl<T: Clone + Eq + std::hash::Hash> Lattice for FxIndexSet<T> {
    fn meet(&self, other: &Self) -> Self {
        self.union(other).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestProgram;

    /// Forward dataflow merges backedges into the function entry state.
    #[test]
    fn test_forward_dataflow_entry_backedge() {
        let program = TestProgram::new(
            r#"
function test(v0: boolean): void {
entry(v0: boolean):
    branch v0 => entry(v0) | exit

exit:
    return
}
"#,
        );

        let function_id = program.first_function_id();
        let function = program.tree.get(function_id);
        let entry = function.entry().expect("missing entry");
        let control = ControlTable::analyse(function, &program.tree);
        let state = FxIndexSet::from_iter([0_u8]);
        let result = Dataflow::forward(
            function,
            &program.tree,
            &control,
            state,
            |transfer, mut state, _| {
                if matches!(transfer, ForwardTransfer::Edge { .. }) {
                    state.insert(1);
                }

                state
            },
        );

        assert_eq!(result.entry(entry), Some(&FxIndexSet::from_iter([0, 1])));
    }

    /// Forward dataflow visits every block when the first predecessor is unreachable.
    #[test]
    fn test_forward_dataflow_unreachable_predecessor_order() {
        let mut program = TestProgram::new(
            r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    jump b2

b1:
    v2: int32 = 2
    jump b2

b2:
    return v0
}
"#,
        );

        let function_id = program.first_function_id();
        let (block0, block1, block2) = {
            let mut function = program.tree.get(function_id).clone();
            let entry = function.entry().expect("missing entry");

            let block0 = entry;
            let block1 = function.block(1);
            let block2 = function.block(2);

            // reorder blocks so the unreachable predecessor is first
            function.replace_blocks(vec![block1, block0, block2], &program.tree);
            program.tree.set(function_id, function);

            (block0, block1, block2)
        };

        let function = program.tree.get(function_id);
        let cfg = ControlTable::analyse(function, &program.tree);
        let entry_state: FxIndexSet<mir::LocalNodeId<mir::Block>> = [block0].into_iter().collect();
        let result = Dataflow::forward(
            function,
            &program.tree,
            &cfg,
            entry_state,
            |transfer, mut state, _| {
                if let ForwardTransfer::Block(block) = transfer {
                    state.insert(block);
                }

                state
            },
        );

        let entry_block2 = result.entry(block2).expect("missing block2 entry");
        assert!(entry_block2.contains(&block0));
        assert!(!entry_block2.contains(&block1));
    }

    /// Backward dataflow seeds tailcall blocks as exits.
    #[test]
    fn test_backward_dataflow_tailcall_exit() {
        let program = TestProgram::new(
            r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    tail.call callee(v0): (int32) => int32
}
"#,
        );

        let function_id = program.function_id_by_name("test");
        let function = program.tree.get(function_id);
        let cfg = ControlTable::analyse(function, &program.tree);
        let entry = function.entry().expect("missing entry");

        let exit_state: FxIndexSet<mir::LocalNodeId<mir::Block>> = [entry].into_iter().collect();
        let result = Dataflow::backward(
            function,
            &program.tree,
            &cfg,
            exit_state,
            |block_id, mut state, _| {
                state.insert(block_id);
                state
            },
        );

        let exit = result.exit(entry).expect("missing tailcall exit");
        assert!(exit.contains(&entry));
    }

    /// Panic terminators act as backward dataflow exits.
    #[test]
    fn test_backward_dataflow_panic_exit() {
        let program = TestProgram::new(
            r#"
function test(v0: ref<void, managed, readonly, local>): void {
entry(v0: ref<void, managed, readonly, local>):
    panic v0
}
"#,
        );

        let function_id = program.function_id_by_name("test");
        let function = program.tree.get(function_id);
        let cfg = ControlTable::analyse(function, &program.tree);
        let entry = function.entry().expect("missing entry");

        let exit_state: FxIndexSet<mir::LocalNodeId<mir::Block>> = [entry].into_iter().collect();
        let result = Dataflow::backward(
            function,
            &program.tree,
            &cfg,
            exit_state,
            |block_id, mut state, _| {
                state.insert(block_id);
                state
            },
        );

        let exit = result.exit(entry).expect("missing panic exit");
        assert!(exit.contains(&entry));
    }
}
