use super::{Analysis, AnalysisId, FunctionAnalyses, FunctionAnalysis, Mutation};
use crate::{Block, Function, LocalNodeId, Tree};

/// Control flow graph for one function.
#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    /// Predecessors indexed by block id.
    predecessors: Vec<Vec<LocalNodeId<Block>>>,
}

impl ControlFlowGraph {
    /// Build the control flow graph for one function.
    pub fn build(function: &Function, tree: &Tree) -> Self {
        let block_count = function.block_capacity();
        let mut predecessors = vec![Vec::new(); block_count];

        // compute predecessors from successor edges
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);

            for successor in terminator.successors(tree) {
                if let Some(block_predecessors) = predecessors.get_mut(successor.id as usize) {
                    block_predecessors.push(block_id);
                }
            }
        }

        Self { predecessors }
    }

    /// Return the predecessors of one block.
    pub fn predecessors(&self, block: LocalNodeId<Block>) -> &[LocalNodeId<Block>] {
        self.predecessors
            .get(block.id as usize)
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
        let mut visited = vec![false; self.predecessors.len()];

        // walk backward through predecessors until we find the entry
        while let Some(current) = worklist.pop() {
            let Some(is_visited) = visited.get_mut(current.id as usize) else {
                continue;
            };
            if *is_visited {
                continue;
            }
            *is_visited = true;

            if current == entry {
                return true;
            }

            if let Some(predecessors) = self.predecessors.get(current.id as usize) {
                worklist.extend(predecessors.iter().copied());
            }
        }

        false
    }
}

impl Analysis for ControlFlowGraph {
    const ID: AnalysisId = AnalysisId("cfg");
    const INVALIDATED_BY: Mutation = Mutation::CONTROL;
}

impl FunctionAnalysis for ControlFlowGraph {
    fn compute(function: &Function, tree: &Tree, _analyses: &FunctionAnalyses) -> Self {
        Self::build(function, tree)
    }
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
entry:
    jump b1

b1:
    jump b2

b2:
    return
}
"#,
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
entry(v0: boolean):
    branch v0, b1, b2

b1:
    return

b2:
    return
}
"#,
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
entry(v0: boolean):
    branch v0, b1, b2

b1:
    jump b3

b2:
    jump b3

b3:
    return
}
"#,
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
entry(v0: boolean):
    jump b1(v0)

b1(v1: boolean):
    branch v1, b1(v1), b2

b2:
    return
}
"#,
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
entry(v0: boolean):
    branch v0, b1, b2

b1:
    v1: int32 = 1
    return v1

b2:
    unreachable

b3:
    v2: int32 = 2
    return v2
}
"#,
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
