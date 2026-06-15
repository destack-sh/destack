use std::collections::{HashMap, HashSet};

use super::{Analysis, AnalysisId, FunctionAnalyses, FunctionAnalysis, Mutation};
use crate as mir;
use crate::{Block, BlockReference, Function, LocalNodeId, Tree};

/// Control flow graph for one function.
#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    /// Predecessors for each block.
    predecessors: HashMap<LocalNodeId<Block>, Vec<LocalNodeId<Block>>>,
}

impl ControlFlowGraph {
    /// Build the control flow graph for one function.
    pub fn build(function: &Function, tree: &Tree) -> Self {
        let mut predecessors = HashMap::new();

        // initialize predecessor lists
        for &block_id in &function.blocks {
            predecessors.insert(block_id, Vec::new());
        }

        // compute predecessors from successor edges
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);

            for successor in terminator.successors() {
                let BlockReference::Block(successor) = successor else {
                    continue;
                };

                if let Some(block_predecessors) = predecessors.get_mut(&successor) {
                    block_predecessors.push(block_id);
                }
            }
        }

        Self { predecessors }
    }

    /// Return the predecessors of one block.
    pub fn predecessors(&self, block: LocalNodeId<Block>) -> &[LocalNodeId<Block>] {
        self.predecessors
            .get(&block)
            .map(|predecessors| predecessors.as_slice())
            .unwrap_or(&[])
    }

    /// Return whether one block is reachable from the entry block.
    pub fn is_reachable(&self, block: LocalNodeId<Block>, entry: LocalNodeId<Block>) -> bool {
        // the entry block is always reachable from itself
        if block == entry {
            return true;
        }

        let mut worklist = vec![block];
        let mut visited = HashSet::new();

        // walk backward through predecessors until we find the entry
        while let Some(current) = worklist.pop() {
            if !visited.insert(current) {
                continue;
            }

            if current == entry {
                return true;
            }

            if let Some(predecessors) = self.predecessors.get(&current) {
                worklist.extend(predecessors.iter().copied());
            }
        }

        false
    }
}

impl Analysis for ControlFlowGraph {
    const ID: AnalysisId = AnalysisId("cfg");
    const INVALIDATED_BY: Mutation = Mutation::CONTROL_FLOW;
}

impl FunctionAnalysis for ControlFlowGraph {
    fn compute(function: &Function, tree: &Tree, _analyses: &FunctionAnalyses) -> Self {
        Self::build(function, tree)
    }
}

/// Successor selected by one MIR terminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Successor {
    /// The target of an unconditional jump.
    Jump,
    /// The return continuation of a call terminator.
    CallReturn,
    /// The unwind continuation of a call terminator.
    CallUnwind,
    /// The then target of a branch terminator.
    BranchThen,
    /// The else target of a branch terminator.
    BranchElse,
    /// The success target of a check terminator.
    CheckSuccess,
    /// The failure target of a check terminator.
    CheckFailure,
    /// The success target of a fallible terminator.
    TrySuccess,
    /// The failure target of a fallible terminator.
    TryFailure,
    /// One switch case target.
    SwitchCase { value: i128 },
    /// The default target of a switch terminator.
    SwitchDefault,
    /// The resume target of a yield terminator.
    YieldResume,
    /// The unwind target of a yield terminator.
    YieldUnwind,
}

/// Control flow edge selected by a terminator successor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Edge {
    /// The source block.
    pub source: LocalNodeId<Block>,
    /// The successor field selected from the source terminator.
    pub successor: Successor,
    /// The target block.
    pub target: LocalNodeId<Block>,
}

impl Edge {
    /// Create one control flow edge.
    pub fn new(
        source: LocalNodeId<Block>,
        successor: Successor,
        target: LocalNodeId<Block>,
    ) -> Self {
        Self {
            source,
            successor,
            target,
        }
    }
}

/// Enumerate the control flow edges leaving one terminator.
pub fn terminator_edges(
    source: mir::LocalNodeId<mir::Block>,
    terminator: &mir::Terminator,
) -> Vec<(mir::Edge, mir::LocalNodeId<mir::Block>)> {
    terminator_targets(source, terminator)
        .into_iter()
        .map(|(edge, _)| (edge, edge.target))
        .collect()
}

/// Enumerate the control flow edges leaving one terminator with their targets.
pub fn terminator_targets(
    source: mir::LocalNodeId<mir::Block>,
    terminator: &mir::Terminator,
) -> Vec<(mir::Edge, &mir::BlockTarget)> {
    match terminator {
        mir::Terminator::Error => Vec::new(),
        mir::Terminator::Jump { target, .. } => block_edge(source, mir::Successor::Jump, target)
            .into_iter()
            .collect(),
        mir::Terminator::Branch {
            then_target,
            else_target,
            ..
        } => [
            block_edge(source, mir::Successor::BranchThen, then_target),
            block_edge(source, mir::Successor::BranchElse, else_target),
        ]
        .into_iter()
        .flatten()
        .collect(),
        mir::Terminator::Check {
            success, failure, ..
        } => [
            block_edge(source, mir::Successor::CheckSuccess, success),
            block_edge(source, mir::Successor::CheckFailure, failure),
        ]
        .into_iter()
        .flatten()
        .collect(),
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
        } => [
            block_edge(source, mir::Successor::TrySuccess, success),
            block_edge(source, mir::Successor::TryFailure, failure),
        ]
        .into_iter()
        .flatten()
        .collect(),
        mir::Terminator::Switch { default, cases, .. } => {
            let mut edges = Vec::with_capacity(cases.len() + 1);

            // default edge
            edges.extend(block_edge(source, mir::Successor::SwitchDefault, default));

            // case edges
            for case in cases {
                let Some(value) = case.value.integer() else {
                    continue;
                };
                edges.extend(block_edge(
                    source,
                    mir::Successor::SwitchCase { value },
                    &case.target,
                ));
            }

            edges
        }
        mir::Terminator::Yield { resume, unwind, .. } => {
            let mut edges = Vec::with_capacity(2);

            edges.extend(block_edge(source, mir::Successor::YieldResume, resume));
            if let Some(unwind) = unwind {
                edges.extend(block_edge(source, mir::Successor::YieldUnwind, unwind));
            }

            edges
        }
        mir::Terminator::Call { target, unwind, .. }
        | mir::Terminator::CallIndirect { target, unwind, .. }
        | mir::Terminator::CallVirtual { target, unwind, .. }
        | mir::Terminator::CallDynamic { target, unwind, .. } => {
            let mut edges = Vec::with_capacity(2);

            edges.extend(block_edge(source, mir::Successor::CallReturn, target));
            if let Some(unwind) = unwind {
                edges.extend(block_edge(source, mir::Successor::CallUnwind, unwind));
            }

            edges
        }
        mir::Terminator::Return { .. }
        | mir::Terminator::Panic { .. }
        | mir::Terminator::ResumeUnwind
        | mir::Terminator::Trap { .. }
        | mir::Terminator::Unreachable
        | mir::Terminator::TailCall { .. }
        | mir::Terminator::TailCallVirtual { .. }
        | mir::Terminator::TailCallDynamic { .. }
        | mir::Terminator::TailCallIndirect { .. } => Vec::new(),
    }
}

/// Pair one block target with its edge when it resolves to a concrete block.
fn block_edge(
    source: mir::LocalNodeId<mir::Block>,
    successor: mir::Successor,
    target: &mir::BlockTarget,
) -> Option<(mir::Edge, &mir::BlockTarget)> {
    target
        .block
        .block()
        .map(|block| (mir::Edge::new(source, successor, block), target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::parse_test_function;

    #[test]
    fn test_build_predecessors_for_linear_flow() {
        let (tree, function_id) = parse_test_function(
            r#"
function linear(): void {
b0:
    jump b1
b1:
    jump b2
b2:
    return
}"#,
        );

        let function = tree.get(function_id);
        let cfg = ControlFlowGraph::build(function, &tree);

        let block0 = function.entry.expect("missing entry");
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];

        assert!(cfg.predecessors(block0).is_empty());
        assert_eq!(cfg.predecessors(block1).len(), 1);
        assert_eq!(cfg.predecessors(block2).len(), 1);
    }

    #[test]
    fn test_build_predecessors_for_branch() {
        let (tree, function_id) = parse_test_function(
            r#"
function testBranch(v0: boolean): void {
b0(v0: boolean):
    branch v0, b1, b2
b1:
    return
b2:
    return
}"#,
        );

        let function = tree.get(function_id);
        let cfg = ControlFlowGraph::build(function, &tree);

        let block1 = function.blocks[1];
        let block2 = function.blocks[2];

        assert_eq!(cfg.predecessors(block1).len(), 1);
        assert_eq!(cfg.predecessors(block2).len(), 1);
    }

    #[test]
    fn test_build_predecessors_for_diamond() {
        let (tree, function_id) = parse_test_function(
            r#"
function diamond(v0: boolean): void {
b0(v0: boolean):
    branch v0, b1, b2
b1:
    jump b3
b2:
    jump b3
b3:
    return
}"#,
        );

        let function = tree.get(function_id);
        let cfg = ControlFlowGraph::build(function, &tree);

        let block3 = function.blocks[3];

        assert_eq!(cfg.predecessors(block3).len(), 2);
    }

    #[test]
    fn test_build_predecessors_for_loop() {
        let (tree, function_id) = parse_test_function(
            r#"
function loop(v0: boolean): void {
b0(v0: boolean):
    jump b1(v0)
b1(v1: boolean):
    branch v1, b1(v1), b2
b2:
    return
}"#,
        );

        let function = tree.get(function_id);
        let cfg = ControlFlowGraph::build(function, &tree);

        let block1 = function.blocks[1];

        assert_eq!(cfg.predecessors(block1).len(), 2);
    }

    #[test]
    fn test_check_reachability() {
        let (tree, function_id) = parse_test_function(
            r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b1, b2
b1:
    v1: int32 = 1int32
    return v1
b2:
    unreachable
b3:
    v2: int32 = 2int32
    return v2
}"#,
        );

        let function = tree.get(function_id);
        let cfg = ControlFlowGraph::build(function, &tree);
        let entry = function.entry.expect("missing entry");

        let reachable_block = function.blocks[1];
        let unreachable_block = function.blocks[3];

        assert!(cfg.is_reachable(reachable_block, entry));
        assert!(!cfg.is_reachable(unreachable_block, entry));
    }
}
