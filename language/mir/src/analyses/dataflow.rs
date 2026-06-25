use std::collections::{HashSet, VecDeque};

use crate as mir;
use crate::NodeTable;

use super::ControlFlowGraph;

/// Trait for types that form a lattice for dataflow analysis.
///
/// A lattice provides a partial order with a meet (greatest lower bound) operation.
/// Dataflow analyses use lattices to merge states at control flow join points.
///
/// # Laws
///
/// Implementations must satisfy:
/// - Commutativity: `a.meet(b) == b.meet(a)`
/// - Associativity: `a.meet(b.meet(c)) == a.meet(b).meet(c)`
/// - Idempotency: `a.meet(a) == a`
pub trait Lattice: Clone + PartialEq {
    /// Compute the meet (greatest lower bound) of two lattice elements.
    ///
    /// The meet represents the most precise state that is valid for both inputs.
    /// At control flow join points, states from all predecessors are merged using meet.
    fn meet(&self, other: &Self) -> Self;
}

/// Dense result of a dataflow analysis.
#[derive(Debug, Clone)]
pub struct DataflowResult<S> {
    /// State at entry indexed by block id.
    block_entry: NodeTable<mir::Block, Option<S>>,
    /// State at exit indexed by block id.
    block_exit: NodeTable<mir::Block, Option<S>>,
}

impl<S> DataflowResult<S> {
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
            block_entry: NodeTable::from_nodes(&function.blocks, || None),
            block_exit: NodeTable::from_nodes(&function.blocks, || None),
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

    /// Split into entry and exit tables.
    pub(crate) fn into_parts(
        self,
    ) -> (
        NodeTable<mir::Block, Option<S>>,
        NodeTable<mir::Block, Option<S>>,
    ) {
        (self.block_entry, self.block_exit)
    }
}

impl<S> Default for DataflowResult<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> DataflowResult<S>
where
    S: Lattice,
{
    /// Run a forward dataflow analysis using a worklist algorithm.
    pub fn forward<F>(
        function: &mir::Function,
        tree: &mir::Tree,
        cfg: &ControlFlowGraph,
        entry_state: S,
        mut transfer: F,
    ) -> Self
    where
        F: FnMut(mir::LocalNodeId<mir::Block>, S, &mir::Tree) -> S,
    {
        let entry = match function.entry {
            Some(entry) => entry,
            None => return Self::new(),
        };

        let mut result = Self::for_function(function);

        // initialize entry block
        result.set_entry(entry, entry_state.clone());

        // seed the worklist with the entry block
        let mut worklist: VecDeque<mir::LocalNodeId<mir::Block>> = VecDeque::new();
        let mut in_worklist: HashSet<mir::LocalNodeId<mir::Block>> = HashSet::new();

        worklist.push_back(entry);
        in_worklist.insert(entry);

        while let Some(block_id) = worklist.pop_front() {
            in_worklist.remove(&block_id);

            // merge predecessor exits into the block entry
            let new_entry = if block_id == entry {
                result
                    .entry(entry)
                    .cloned()
                    .unwrap_or_else(|| panic!("missing entry state for dataflow root: {entry:?}"))
            } else {
                let predecessors = cfg.predecessors(block_id);
                if predecessors.is_empty() {
                    continue;
                }

                let mut merged: Option<S> = None;
                for &predecessor in predecessors {
                    if let Some(predecessor_exit) = result.exit(predecessor) {
                        merged = Some(match merged {
                            Some(state) => state.meet(predecessor_exit),
                            None => predecessor_exit.clone(),
                        });
                    }
                }

                let Some(merged) = merged else {
                    continue;
                };

                merged
            };

            // skip blocks whose entry is already stable
            let entry_changed = result
                .entry(block_id)
                .map(|old| old != &new_entry)
                .unwrap_or(true);
            if !entry_changed && block_id != entry {
                continue;
            }

            result.set_entry(block_id, new_entry.clone());

            // apply transfer from entry state to exit state
            let exit_state = transfer(block_id, new_entry, tree);
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
                if !in_worklist.contains(&successor) {
                    worklist.push_back(successor);
                    in_worklist.insert(successor);
                }
            }
        }

        result
    }

    /// Run a backward dataflow analysis using a worklist algorithm.
    pub fn backward<F>(
        function: &mir::Function,
        tree: &mir::Tree,
        cfg: &ControlFlowGraph,
        exit_state: S,
        mut transfer: F,
    ) -> Self
    where
        F: FnMut(mir::LocalNodeId<mir::Block>, S, &mir::Tree) -> S,
    {
        if function.entry.is_none() {
            return Self::new();
        }

        let mut result = Self::for_function(function);

        // seed terminal blocks with the caller-provided exit state
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);
            if matches!(
                terminator,
                mir::Terminator::Return { .. }
                    | mir::Terminator::Trap { .. }
                    | mir::Terminator::Panic { .. }
                    | mir::Terminator::Unreachable
                    | mir::Terminator::TailCall { .. }
                    | mir::Terminator::TailCallVirtual { .. }
                    | mir::Terminator::TailCallDynamic { .. }
                    | mir::Terminator::TailCallIndirect { .. }
            ) {
                result.set_exit(block_id, exit_state.clone());
            }
        }

        // seed the worklist from every terminal block
        let mut worklist: VecDeque<mir::LocalNodeId<mir::Block>> = VecDeque::new();
        let mut in_worklist: HashSet<mir::LocalNodeId<mir::Block>> = HashSet::new();

        for &block_id in &function.blocks {
            if result.exit(block_id).is_some() {
                worklist.push_back(block_id);
                in_worklist.insert(block_id);
            }
        }

        while let Some(block_id) = worklist.pop_front() {
            in_worklist.remove(&block_id);

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
                if !in_worklist.contains(&predecessor) {
                    worklist.push_back(predecessor);
                    in_worklist.insert(predecessor);
                }
            }
        }

        result
    }
}

/// A set lattice where meet is union.
///
/// Useful for analyses that collect facts (e.g., reaching definitions).
impl<T: Clone + Eq + std::hash::Hash> Lattice for HashSet<T> {
    fn meet(&self, other: &Self) -> Self {
        self.union(other).cloned().collect()
    }
}

/// Option as a lattice where None is top (unknown) and Some is a known value.
///
/// Meet of two different Some values could be handled differently depending
/// on the inner type; this implementation takes the first value.
impl<T: Clone + PartialEq> Lattice for Option<T> {
    fn meet(&self, other: &Self) -> Self {
        match (self, other) {
            (None, x) | (x, None) => x.clone(),
            (Some(a), Some(b)) if a == b => Some(a.clone()),
            _ => None, // conflict, return top
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestProgram;

    /// HashSet lattice uses union for meet.
    #[test]
    fn test_hashset_lattice() {
        let a: HashSet<i32> = [1, 2, 3].into_iter().collect();
        let b: HashSet<i32> = [2, 3, 4].into_iter().collect();
        let meet = a.meet(&b);
        let expected: HashSet<i32> = [1, 2, 3, 4].into_iter().collect();

        assert_eq!(meet, expected);
    }

    /// Option lattice handles None and Some correctly.
    #[test]
    fn test_option_lattice() {
        let a: Option<i32> = Some(42);
        let b: Option<i32> = None;
        let c: Option<i32> = Some(42);
        let d: Option<i32> = Some(99);

        assert_eq!(a.meet(&b), Some(42));
        assert_eq!(b.meet(&a), Some(42));
        assert_eq!(a.meet(&c), Some(42));
        assert_eq!(a.meet(&d), None); // Conflict
    }
    /// Forward dataflow should not skip blocks when the first predecessor is unreachable.
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
            let function = program.tree.get_mut(function_id);
            let entry = function.entry.expect("missing entry");

            let block0 = entry;
            let block1 = function.blocks[1];
            let block2 = function.blocks[2];

            // reorder blocks so the unreachable predecessor is first
            function.blocks = vec![block1, block0, block2];

            (block0, block1, block2)
        };

        let function = program.tree.get(function_id);
        let cfg = ControlFlowGraph::build(function, &program.tree);
        let entry_state: HashSet<mir::LocalNodeId<mir::Block>> = [block0].into_iter().collect();
        let result = DataflowResult::forward(
            function,
            &program.tree,
            &cfg,
            entry_state,
            |block_id, mut state, _| {
                state.insert(block_id);
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
    tail.call callee(v0)
}
"#,
        );

        let function_id = program.function_id_by_name("test");
        let function = program.tree.get(function_id);
        let cfg = ControlFlowGraph::build(function, &program.tree);
        let entry = function.entry.expect("missing entry");

        let exit_state: HashSet<mir::LocalNodeId<mir::Block>> = [entry].into_iter().collect();
        let result = DataflowResult::backward(
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

    /// Trap terminators act as backward dataflow exits.
    #[test]
    fn test_backward_dataflow_trap_exit() {
        let program = TestProgram::new(
            r#"
function test(v0: ref<void, managed, readonly>): void {
entry(v0: ref<void, managed, readonly>):
    panic v0
}
"#,
        );

        let function_id = program.function_id_by_name("test");
        let function = program.tree.get(function_id);
        let cfg = ControlFlowGraph::build(function, &program.tree);
        let entry = function.entry.expect("missing entry");

        let exit_state: HashSet<mir::LocalNodeId<mir::Block>> = [entry].into_iter().collect();
        let result = DataflowResult::backward(
            function,
            &program.tree,
            &cfg,
            exit_state,
            |block_id, mut state, _| {
                state.insert(block_id);
                state
            },
        );

        let exit = result.exit(entry).expect("missing trap exit");
        assert!(exit.contains(&entry));
    }
}
