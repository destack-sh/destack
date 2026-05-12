use std::collections::{HashMap, HashSet, VecDeque};

use destack_mir as mir;

use crate::optimize::ControlFlowGraph;

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

/// Result of a dataflow analysis.
///
/// Contains the computed state at entry and exit of each block.
#[derive(Debug, Clone)]
pub struct DataflowResult<S> {
    /// State at entry to each block (after merging predecessors).
    pub block_entry: HashMap<mir::LocalNodeId<mir::Block>, S>,
    /// State at exit of each block (after processing instructions).
    pub block_exit: HashMap<mir::LocalNodeId<mir::Block>, S>,
}

impl<S> DataflowResult<S> {
    /// Create an empty result.
    pub fn new() -> Self {
        Self {
            block_entry: HashMap::new(),
            block_exit: HashMap::new(),
        }
    }

    /// Get the state at entry to a block.
    pub fn entry(&self, block: mir::LocalNodeId<mir::Block>) -> Option<&S> {
        self.block_entry.get(&block)
    }

    /// Get the state at exit of a block.
    pub fn exit(&self, block: mir::LocalNodeId<mir::Block>) -> Option<&S> {
        self.block_exit.get(&block)
    }
}

impl<S> Default for DataflowResult<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// Run a forward dataflow analysis using a worklist algorithm.
///
/// Forward dataflow propagates information from entry to exit, following
/// control flow edges. At join points (blocks with multiple predecessors),
/// states are merged using the lattice meet operation.
///
/// # Parameters
///
/// - `function`: The function to analyze
/// - `tree`: The MIR tree
/// - `cfg`: Control flow graph (for predecessor information)
/// - `entry_state`: Initial state at function entry
/// - `transfer`: Transfer function that processes a block and returns the exit state.
///   Takes `(block_id, entry_state, tree)` and returns `exit_state`.
///
/// # Returns
///
/// A `DataflowResult` containing the computed states at entry and exit of each block.
///
/// # Algorithm
///
/// Uses a worklist algorithm:
/// 1. Initialize entry block with `entry_state`
/// 2. For each block in worklist:
///    - Compute entry state by meeting predecessor exit states
///    - Apply transfer function to get exit state
///    - If exit state changed, add successors to worklist
/// 3. Iterate until fixed point
pub fn forward_dataflow<S, F>(
    function: &mir::Function,
    tree: &mir::Tree,
    cfg: &ControlFlowGraph,
    entry_state: S,
    mut transfer: F,
) -> DataflowResult<S>
where
    S: Lattice,
    F: FnMut(mir::LocalNodeId<mir::Block>, S, &mir::Tree) -> S,
{
    let entry = match function.entry {
        Some(e) => e,
        None => return DataflowResult::new(), // no body (extern function)
    };

    let mut result = DataflowResult::new();

    // initialize entry block
    result.block_entry.insert(entry, entry_state.clone());

    // worklist algorithm
    let mut worklist: VecDeque<mir::LocalNodeId<mir::Block>> = VecDeque::new();
    let mut in_worklist: HashSet<mir::LocalNodeId<mir::Block>> = HashSet::new();

    worklist.push_back(entry);
    in_worklist.insert(entry);

    while let Some(block_id) = worklist.pop_front() {
        in_worklist.remove(&block_id);

        // compute entry state by merging predecessor exits
        let new_entry = if block_id == entry {
            result
                .block_entry
                .get(&entry)
                .cloned()
                .unwrap_or_else(|| entry_state.clone())
        } else {
            let predecessors = cfg.predecessors(block_id);
            if predecessors.is_empty() {
                continue; // unreachable block
            }

            let mut merged: Option<S> = None;
            for &pred in predecessors {
                if let Some(pred_exit) = result.block_exit.get(&pred) {
                    merged = Some(match merged {
                        Some(state) => state.meet(pred_exit),
                        None => pred_exit.clone(),
                    });
                }
            }

            let Some(merged) = merged else {
                continue;
            };

            merged
        };

        // check if entry state changed
        let entry_changed = result
            .block_entry
            .get(&block_id)
            .map(|old| old != &new_entry)
            .unwrap_or(true);

        if entry_changed || block_id == entry {
            result.block_entry.insert(block_id, new_entry.clone());

            // apply transfer function
            let exit_state = transfer(block_id, new_entry, tree);

            // check if exit state changed
            let exit_changed = result
                .block_exit
                .get(&block_id)
                .map(|old| old != &exit_state)
                .unwrap_or(true);

            if exit_changed {
                result.block_exit.insert(block_id, exit_state);

                // add successors to worklist
                let block = tree.get(block_id);
                let terminator = tree.get(block.terminator);
                for successor in terminator.successors() {
                    let Some(successor) = successor.block() else {
                        continue;
                    };

                    if !in_worklist.contains(&successor) {
                        worklist.push_back(successor);
                        in_worklist.insert(successor);
                    }
                }
            }
        }
    }

    result
}

/// Run a backward dataflow analysis using a worklist algorithm.
///
/// Backward dataflow propagates information from exit to entry, following
/// control flow edges in reverse. At join points (blocks with multiple successors),
/// states are merged using the lattice meet operation.
///
/// # Parameters
///
/// - `function`: The function to analyze
/// - `tree`: The MIR tree
/// - `cfg`: Control flow graph (for predecessor information)
/// - `exit_state`: Initial state at function exits (return/unreachable)
/// - `transfer`: Transfer function that processes a block and returns the entry state.
///   Takes `(block_id, exit_state, tree)` and returns `entry_state`.
///
/// # Returns
///
/// A `DataflowResult` containing the computed states at entry and exit of each block.
pub fn backward_dataflow<S, F>(
    function: &mir::Function,
    tree: &mir::Tree,
    cfg: &ControlFlowGraph,
    exit_state: S,
    mut transfer: F,
) -> DataflowResult<S>
where
    S: Lattice,
    F: FnMut(mir::LocalNodeId<mir::Block>, S, &mir::Tree) -> S,
{
    if function.entry.is_none() {
        return DataflowResult::new();
    }

    let mut result = DataflowResult::new();

    // initialize exit blocks (return/unreachable terminators)
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        if matches!(
            terminator,
            mir::Terminator::Return { .. }
                | mir::Terminator::Trap { .. }
                | mir::Terminator::Unreachable
                | mir::Terminator::TailCall { .. }
                | mir::Terminator::TailCallClass { .. }
                | mir::Terminator::TailCallInterface { .. }
                | mir::Terminator::TailCallIndirect { .. }
        ) {
            result.block_exit.insert(block_id, exit_state.clone());
        }
    }

    // worklist algorithm (process in reverse order for faster convergence)
    let mut worklist: VecDeque<mir::LocalNodeId<mir::Block>> = VecDeque::new();
    let mut in_worklist: HashSet<mir::LocalNodeId<mir::Block>> = HashSet::new();

    // start with all blocks that have exit states
    for &block_id in &function.blocks {
        if result.block_exit.contains_key(&block_id) {
            worklist.push_back(block_id);
            in_worklist.insert(block_id);
        }
    }

    while let Some(block_id) = worklist.pop_front() {
        in_worklist.remove(&block_id);

        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);

        // compute exit state by merging successor entries
        let new_exit = if result.block_exit.contains_key(&block_id) {
            // use existing exit state (for exit blocks)
            let mut merged = result.block_exit.get(&block_id).unwrap().clone();

            // also merge with successor entries
            for successor in terminator.successors() {
                let Some(successor) = successor.block() else {
                    continue;
                };

                if let Some(succ_entry) = result.block_entry.get(&successor) {
                    merged = merged.meet(succ_entry);
                }
            }
            merged
        }
        // compute from successors only
        else {
            let successors: Vec<_> = terminator
                .successors()
                .into_iter()
                .filter_map(|successor| successor.block())
                .collect();
            if successors.is_empty() {
                continue;
            }

            let mut merged = match result.block_entry.get(&successors[0]) {
                Some(s) => s.clone(),
                None => continue,
            };

            for &succ in &successors[1..] {
                if let Some(succ_entry) = result.block_entry.get(&succ) {
                    merged = merged.meet(succ_entry);
                }
            }

            merged
        };

        // check if exit state changed
        let exit_changed = result
            .block_exit
            .get(&block_id)
            .map(|old| old != &new_exit)
            .unwrap_or(true);

        if exit_changed {
            result.block_exit.insert(block_id, new_exit.clone());

            // apply transfer function (backward: exit to entry)
            let entry_state = transfer(block_id, new_exit, tree);

            // check if entry state changed
            let entry_changed = result
                .block_entry
                .get(&block_id)
                .map(|old| old != &entry_state)
                .unwrap_or(true);

            if entry_changed {
                result.block_entry.insert(block_id, entry_state);

                // add predecessors to worklist
                for &pred_id in cfg.predecessors(block_id) {
                    if !in_worklist.contains(&pred_id) {
                        worklist.push_back(pred_id);
                        in_worklist.insert(pred_id);
                    }
                }
            }
        }
    }

    result
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
    use crate::optimize::common::tests::TestProgram;

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
b0(v0: int32):
    v1: int32 = 1int32
    jump b2
b1:
    v2: int32 = 2int32
    jump b2
b2:
    return v0
}"#,
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
        let result = forward_dataflow(
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
b0(v0: int32):
    return v0
}
function test(v0: int32): int32 {
b0(v0: int32):
    tailCall callee(v0): (int32) -> int32
}"#,
        );

        let function_id = program.function_id_by_name("test");
        let function = program.tree.get(function_id);
        let cfg = ControlFlowGraph::build(function, &program.tree);
        let entry = function.entry.expect("missing entry");

        let exit_state: HashSet<mir::LocalNodeId<mir::Block>> = [entry].into_iter().collect();
        let result = backward_dataflow(
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
b0(v0: ref<void, managed, readonly>):
    trap.panic v0
}"#,
        );

        let function_id = program.function_id_by_name("test");
        let function = program.tree.get(function_id);
        let cfg = ControlFlowGraph::build(function, &program.tree);
        let entry = function.entry.expect("missing entry");

        let exit_state: HashSet<mir::LocalNodeId<mir::Block>> = [entry].into_iter().collect();
        let result = backward_dataflow(
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
