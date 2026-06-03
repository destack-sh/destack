use std::collections::HashMap;

use crate::{Block, BlockReference, ControlFlowGraph, Function, LocalNodeId, Tree};

/// Dense control flow graph used by dominance computation.
#[derive(Debug)]
struct DenseControlFlow {
    /// The dense index for each block.
    block_index: HashMap<LocalNodeId<Block>, usize>,
    /// Successor indices for each block.
    successors: Vec<Vec<usize>>,
    /// Predecessor indices for each block.
    predecessors: Vec<Vec<usize>>,
}

impl DenseControlFlow {
    /// Build one dense control flow graph for one function.
    fn build(function: &Function, tree: &Tree, cfg: &ControlFlowGraph) -> Self {
        let mut block_index = HashMap::new();

        // block indices
        for (index, &block) in function.blocks.iter().enumerate() {
            block_index.insert(block, index);
        }

        let mut successors = vec![Vec::new(); function.blocks.len()];
        let mut predecessors = vec![Vec::new(); function.blocks.len()];

        // edge lists
        for &block in &function.blocks {
            let index = block_index[&block];
            let block_data = tree.get(block);
            let terminator = tree.get(block_data.terminator);

            for successor in terminator.successors() {
                let BlockReference::Block(successor) = successor else {
                    continue;
                };

                let successor_index = block_index[&successor];
                successors[index].push(successor_index);
            }

            for predecessor in cfg.predecessors(block) {
                let predecessor_index = block_index[predecessor];
                predecessors[index].push(predecessor_index);
            }
        }

        Self {
            block_index,
            successors,
            predecessors,
        }
    }
}

/// DFS frame for the Lengauer Tarjan walk.
#[derive(Debug)]
struct DepthFirstFrame {
    /// The dense node being visited.
    node: usize,
    /// The next successor index to explore.
    next_successor: usize,
}

/// Dominator tree for one function.
#[derive(Debug, Clone)]
pub struct DominatorTree {
    /// Immediate dominator for each block.
    immediate_dominators: HashMap<LocalNodeId<Block>, LocalNodeId<Block>>,
    /// Preorder numbers for fast dominance queries.
    preorder: HashMap<LocalNodeId<Block>, u32>,
    /// Maximum preorder number in each subtree.
    preorder_max: HashMap<LocalNodeId<Block>, u32>,
}

impl DominatorTree {
    /// Build the dominator tree for one function.
    pub fn build(function: &Function, tree: &Tree, cfg: &ControlFlowGraph) -> Self {
        let Some(entry) = function.entry else {
            return Self {
                immediate_dominators: HashMap::new(),
                preorder: HashMap::new(),
                preorder_max: HashMap::new(),
            };
        };

        // dense graph
        let dense = DenseControlFlow::build(function, tree, cfg);
        let entry_index = dense.block_index[&entry];
        let result =
            DominatorComputation::compute(&dense.successors, &dense.predecessors, entry_index);

        let mut immediate_dominators = HashMap::new();

        // block dominators
        for (&block, &index) in &dense.block_index {
            if let Some(idom_index) = result.immediate_dominators[index] {
                let idom_block = function.blocks[idom_index];
                immediate_dominators.insert(block, idom_block);
            }
        }

        let (preorder, preorder_max) =
            Self::compute_preorder(&function.blocks, entry, &immediate_dominators);

        Self {
            immediate_dominators,
            preorder,
            preorder_max,
        }
    }

    /// Return the immediate dominator of one block.
    pub fn immediate_dominator(&self, block: LocalNodeId<Block>) -> Option<LocalNodeId<Block>> {
        self.immediate_dominators.get(&block).copied()
    }

    /// Return whether one block dominates another.
    pub fn dominates(&self, a: LocalNodeId<Block>, b: LocalNodeId<Block>) -> bool {
        let Some(a_pre) = self.preorder.get(&a).copied() else {
            return false;
        };
        let Some(a_max) = self.preorder_max.get(&a).copied() else {
            return false;
        };
        let Some(b_pre) = self.preorder.get(&b).copied() else {
            return false;
        };

        a_pre <= b_pre && b_pre <= a_max
    }

    /// Return whether one block strictly dominates another.
    pub fn strictly_dominates(&self, a: LocalNodeId<Block>, b: LocalNodeId<Block>) -> bool {
        a != b && self.dominates(a, b)
    }

    /// Compute preorder ranges for the dominator tree.
    fn compute_preorder(
        blocks: &[LocalNodeId<Block>],
        entry: LocalNodeId<Block>,
        immediate_dominators: &HashMap<LocalNodeId<Block>, LocalNodeId<Block>>,
    ) -> (
        HashMap<LocalNodeId<Block>, u32>,
        HashMap<LocalNodeId<Block>, u32>,
    ) {
        let mut children = HashMap::new();

        // child lists
        for &block in blocks {
            children.insert(block, Vec::new());
        }

        // dominator edges
        for (&block, &idom) in immediate_dominators {
            children
                .get_mut(&idom)
                .unwrap_or_else(|| {
                    unreachable!("missing dominator-tree children for block: {idom:?}")
                })
                .push(block);
        }

        let mut preorder = HashMap::new();
        let mut preorder_max = HashMap::new();
        let mut counter = 0u32;

        // subtree ranges
        Self::fill_preorder(
            entry,
            &children,
            &mut preorder,
            &mut preorder_max,
            &mut counter,
        );

        (preorder, preorder_max)
    }

    /// Fill preorder ranges for one dominator subtree.
    fn fill_preorder(
        block: LocalNodeId<Block>,
        children: &HashMap<LocalNodeId<Block>, Vec<LocalNodeId<Block>>>,
        preorder: &mut HashMap<LocalNodeId<Block>, u32>,
        preorder_max: &mut HashMap<LocalNodeId<Block>, u32>,
        counter: &mut u32,
    ) {
        *counter += 1;
        preorder.insert(block, *counter);

        let mut max = *counter;

        // subtree walk
        if let Some(block_children) = children.get(&block) {
            for &child in block_children {
                Self::fill_preorder(child, children, preorder, preorder_max, counter);
                max = max.max(*preorder_max.get(&child).unwrap_or_else(|| {
                    unreachable!("missing dominator preorder max for child: {child:?}")
                }));
            }
        }

        preorder_max.insert(block, max);
    }
}

/// Immediate dominators for one dense graph.
#[derive(Debug)]
pub(super) struct DominatorResult {
    /// Immediate dominator index for each node in the graph.
    pub(super) immediate_dominators: Vec<Option<usize>>,
}

/// Lengauer Tarjan dominator computation over one dense graph.
#[derive(Debug)]
pub(super) struct DominatorComputation {
    /// DFS index by dense node.
    dfs_index: Vec<usize>,
    /// Dense node by DFS index.
    vertex: Vec<usize>,
    /// DFS parent by DFS index.
    parent: Vec<usize>,
    /// Semidominator by DFS index.
    semi: Vec<usize>,
    /// Best ancestor label by DFS index.
    label: Vec<usize>,
    /// Union-find ancestor by DFS index.
    ancestor: Vec<usize>,
    /// Immediate dominator by DFS index.
    idom: Vec<usize>,
    /// Deferred buckets by semidominator.
    bucket: Vec<Vec<usize>>,
}

impl DominatorComputation {
    /// Compute immediate dominators with the Lengauer Tarjan algorithm.
    pub(super) fn compute(
        successors: &[Vec<usize>],
        predecessors: &[Vec<usize>],
        root: usize,
    ) -> DominatorResult {
        // invalid graph shape
        if successors.len() != predecessors.len() {
            return DominatorResult {
                immediate_dominators: vec![None; successors.len()],
            };
        }

        let node_count = successors.len();

        // invalid root
        if root >= node_count {
            return DominatorResult {
                immediate_dominators: vec![None; node_count],
            };
        }

        let mut computation = Self {
            dfs_index: vec![0; node_count],
            vertex: vec![0; node_count + 1],
            parent: vec![0; node_count + 1],
            semi: vec![0; node_count + 1],
            label: vec![0; node_count + 1],
            ancestor: vec![0; node_count + 1],
            idom: vec![0; node_count + 1],
            bucket: vec![Vec::new(); node_count + 1],
        };

        computation.run(successors, predecessors, root)
    }

    /// Run the dominator computation for one root.
    fn run(
        &mut self,
        successors: &[Vec<usize>],
        predecessors: &[Vec<usize>],
        root: usize,
    ) -> DominatorResult {
        let counter = self.compute_depth_first_order(successors, root);

        // initialize semidominator state
        for index in 1..=counter {
            self.semi[index] = index;
            self.label[index] = index;
        }

        // reverse DFS order
        for index in (2..=counter).rev() {
            let node_index = self.vertex[index];

            for &predecessor in &predecessors[node_index] {
                let predecessor_index = self.dfs_index[predecessor];
                if predecessor_index == 0 {
                    continue;
                }

                let representative = self.eval(predecessor_index);
                if self.semi[representative] < self.semi[index] {
                    self.semi[index] = self.semi[representative];
                }
            }

            self.bucket[self.semi[index]].push(index);
            self.ancestor[index] = self.parent[index];

            let parent_index = self.parent[index];
            if parent_index == 0 {
                continue;
            }

            let deferred_nodes: Vec<_> = self.bucket[parent_index].drain(..).collect();
            for deferred in deferred_nodes {
                let representative = self.eval(deferred);
                if self.semi[representative] < self.semi[deferred] {
                    self.idom[deferred] = representative;
                } else {
                    self.idom[deferred] = parent_index;
                }
            }
        }

        // final idoms
        for index in 2..=counter {
            if self.idom[index] != self.semi[index] {
                self.idom[index] = self.idom[self.idom[index]];
            }
        }

        self.build_result(counter)
    }

    /// Compute the DFS order for one root.
    fn compute_depth_first_order(&mut self, successors: &[Vec<usize>], root: usize) -> usize {
        let mut counter = 1usize;
        self.dfs_index[root] = counter;
        self.vertex[counter] = root;
        self.parent[counter] = 0;

        let mut stack = vec![DepthFirstFrame {
            node: root,
            next_successor: 0,
        }];

        // iterative DFS
        while let Some(frame) = stack.last_mut() {
            if frame.next_successor >= successors[frame.node].len() {
                stack.pop();
                continue;
            }

            let successor = successors[frame.node][frame.next_successor];
            frame.next_successor += 1;

            if self.dfs_index[successor] != 0 {
                continue;
            }

            counter += 1;
            self.dfs_index[successor] = counter;
            self.vertex[counter] = successor;
            self.parent[counter] = self.dfs_index[frame.node];

            stack.push(DepthFirstFrame {
                node: successor,
                next_successor: 0,
            });
        }

        counter
    }

    /// Compress the union-find path for one DFS index.
    fn compress(&mut self, index: usize) {
        let ancestor_index = self.ancestor[index];
        let ancestor_parent = self.ancestor[ancestor_index];

        if ancestor_parent == 0 {
            return;
        }

        self.compress(ancestor_index);

        if self.semi[self.label[ancestor_index]] < self.semi[self.label[index]] {
            self.label[index] = self.label[ancestor_index];
        }

        self.ancestor[index] = ancestor_parent;
    }

    /// Evaluate the best semidominator representative for one DFS index.
    fn eval(&mut self, index: usize) -> usize {
        if self.ancestor[index] == 0 {
            return self.label[index];
        }

        self.compress(index);

        let ancestor_label = self.label[self.ancestor[index]];
        if self.semi[ancestor_label] < self.semi[self.label[index]] {
            ancestor_label
        } else {
            self.label[index]
        }
    }

    /// Build the final dominator result in dense node space.
    fn build_result(&self, counter: usize) -> DominatorResult {
        let node_count = self.dfs_index.len();
        let mut immediate_dominators = vec![None; node_count];

        // dense node idoms
        for index in 2..=counter {
            let node = self.vertex[index];
            let idom_index = self.idom[index];
            if idom_index != 0 {
                immediate_dominators[node] = Some(self.vertex[idom_index]);
            }
        }

        DominatorResult {
            immediate_dominators,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::parse_test_function;

    #[test]
    fn test_build_dominators_for_linear_flow() {
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
        let domtree = DominatorTree::build(function, &tree, &cfg);

        let block0 = function.blocks[0];
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];

        assert!(domtree.dominates(block0, block0));
        assert!(domtree.dominates(block0, block1));
        assert!(domtree.dominates(block0, block2));
        assert!(domtree.dominates(block1, block2));
        assert!(!domtree.dominates(block2, block0));
        assert!(!domtree.dominates(block2, block1));
        assert_eq!(domtree.immediate_dominator(block1), Some(block0));
        assert_eq!(domtree.immediate_dominator(block2), Some(block1));
    }

    #[test]
    fn test_build_dominators_for_diamond() {
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
        let domtree = DominatorTree::build(function, &tree, &cfg);

        let block0 = function.blocks[0];
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];
        let block3 = function.blocks[3];

        assert!(domtree.dominates(block0, block0));
        assert!(domtree.dominates(block0, block1));
        assert!(domtree.dominates(block0, block2));
        assert!(domtree.dominates(block0, block3));
        assert!(!domtree.dominates(block1, block3));
        assert!(!domtree.dominates(block2, block3));
        assert_eq!(domtree.immediate_dominator(block3), Some(block0));
    }

    #[test]
    fn test_build_dominators_for_unreachable_block() {
        let (tree, function_id) = parse_test_function(
            r#"
function unreachableBlock(v0: boolean): void {
b0(v0: boolean):
    branch v0, b1, b2
b1:
    return
b2:
    return
b3:
    return
}"#,
        );

        let function = tree.get(function_id);
        let cfg = ControlFlowGraph::build(function, &tree);
        let domtree = DominatorTree::build(function, &tree, &cfg);

        let block0 = function.blocks[0];
        let block3 = function.blocks[3];

        assert_eq!(domtree.immediate_dominator(block3), None);
        assert!(!domtree.dominates(block0, block3));
    }

    #[test]
    fn test_check_strict_dominance() {
        let (tree, function_id) = parse_test_function(
            r#"
function simple(): void {
b0:
    jump b1
b1:
    return
}"#,
        );

        let function = tree.get(function_id);
        let cfg = ControlFlowGraph::build(function, &tree);
        let domtree = DominatorTree::build(function, &tree, &cfg);

        let block0 = function.blocks[0];
        let block1 = function.blocks[1];

        assert!(domtree.dominates(block0, block0));
        assert!(!domtree.strictly_dominates(block0, block0));
        assert!(domtree.strictly_dominates(block0, block1));
    }
}
