use std::collections::{HashMap, HashSet};

use crate::{Block, BlockReference, Function, LocalNodeId, NodeTree};

/// Control flow graph for one function.
#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    /// Predecessors for each block.
    predecessors: HashMap<LocalNodeId<Block>, Vec<LocalNodeId<Block>>>,
}

impl ControlFlowGraph {
    /// Build the control flow graph for one function.
    pub fn build(function: &Function, tree: &NodeTree) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::test_support::parse_test_function;

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
