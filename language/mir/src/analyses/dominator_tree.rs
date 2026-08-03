use super::{Analysis, AnalysisId, FunctionAnalysis, FunctionAnalysisCache, Mutation};
use crate::{Block, ControlFlowGraph, Function, LocalNodeId, NodeTable, Tree};

/// Dense control flow graph used by dominance computation.
#[derive(Debug)]
struct DenseControlFlow {
    /// The dense index for each block.
    block_index: NodeTable<Block, Option<usize>>,
    /// Successor indices for each block.
    successors: Vec<Vec<usize>>,
    /// Predecessor indices for each block.
    predecessors: Vec<Vec<usize>>,
}

impl DenseControlFlow {
    /// Build one dense control flow graph for one function.
    fn build(function: &Function, tree: &Tree, cfg: &ControlFlowGraph) -> Self {
        let mut block_index = NodeTable::from_nodes(function.blocks(), || None);

        // block indices
        for (index, &block) in function.blocks().iter().enumerate() {
            *block_index.get_mut(block) = Some(index);
        }

        let mut successors = vec![Vec::new(); function.blocks().len()];
        let mut predecessors = vec![Vec::new(); function.blocks().len()];

        // edge lists
        for &block in function.blocks() {
            let index = block_index.expect(block);
            let block_data = tree.get(block);
            let terminator = tree.get(block_data.terminator);

            for successor in terminator.successors(tree) {
                let successor_index = block_index.expect(successor);
                successors[index].push(successor_index);
            }

            for predecessor in cfg.predecessors(block) {
                let predecessor_index = block_index.expect(*predecessor);
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

impl DenseControlFlow {
    /// Return the dense graph index for one block.
    fn index_of(&self, block: LocalNodeId<Block>) -> usize {
        self.block_index.expect(block)
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
    /// Immediate dominator indexed by block id.
    immediate_dominators: NodeTable<Block, Option<LocalNodeId<Block>>>,
    /// Preorder numbers indexed by block id.
    preorder: NodeTable<Block, Option<u32>>,
    /// Maximum preorder number in each subtree indexed by block id.
    preorder_max: NodeTable<Block, Option<u32>>,
}

impl DominatorTree {
    /// Build the dominator tree for one function.
    pub fn build(function: &Function, tree: &Tree, cfg: &ControlFlowGraph) -> Self {
        let Some(entry) = function.entry() else {
            return Self {
                immediate_dominators: NodeTable::new(),
                preorder: NodeTable::new(),
                preorder_max: NodeTable::new(),
            };
        };

        // dense graph
        let dense = DenseControlFlow::build(function, tree, cfg);
        let entry_index = dense.index_of(entry);
        let result =
            DominatorComputation::compute(&dense.successors, &dense.predecessors, entry_index);

        let mut immediate_dominators = NodeTable::from_nodes(function.blocks(), || None);

        // block dominators
        for &block in function.blocks() {
            let index = dense.index_of(block);
            if let Some(idom_index) = result.immediate_dominators[index] {
                let idom_block = function.block(idom_index);
                *immediate_dominators.get_mut(block) = Some(idom_block);
            }
        }

        let (preorder, preorder_max) =
            Self::compute_preorder(function.blocks(), entry, &immediate_dominators);

        Self {
            immediate_dominators,
            preorder,
            preorder_max,
        }
    }

    /// Return the immediate dominator of one block.
    pub fn immediate_dominator(&self, block: LocalNodeId<Block>) -> Option<LocalNodeId<Block>> {
        *self.immediate_dominators.get(block)
    }

    /// Return whether one block dominates another.
    pub fn dominates(&self, a: LocalNodeId<Block>, b: LocalNodeId<Block>) -> bool {
        let Some(a_pre) = *self.preorder.get(a) else {
            return false;
        };
        let Some(a_max) = *self.preorder_max.get(a) else {
            return false;
        };
        let Some(b_pre) = *self.preorder.get(b) else {
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
        immediate_dominators: &NodeTable<Block, Option<LocalNodeId<Block>>>,
    ) -> (NodeTable<Block, Option<u32>>, NodeTable<Block, Option<u32>>) {
        let mut children = NodeTable::from_nodes(blocks, Vec::new);
        let mut preorder = NodeTable::from_nodes(blocks, || None);
        let mut preorder_max = NodeTable::from_nodes(blocks, || None);

        // dominator edges
        for &block in blocks {
            if let Some(idom) = *immediate_dominators.get(block) {
                children.get_mut(idom).push(block);
            }
        }

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
        children: &NodeTable<Block, Vec<LocalNodeId<Block>>>,
        preorder: &mut NodeTable<Block, Option<u32>>,
        preorder_max: &mut NodeTable<Block, Option<u32>>,
        counter: &mut u32,
    ) {
        *counter += 1;
        *preorder.get_mut(block) = Some(*counter);

        let mut max = *counter;

        // subtree walk
        for &child in children.get(block) {
            Self::fill_preorder(child, children, preorder, preorder_max, counter);
            max = max.max(preorder_max.get(child).unwrap_or_else(|| {
                unreachable!("missing dominator preorder max for child: {child:?}")
            }));
        }

        *preorder_max.get_mut(block) = Some(max);
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
        let node_count = successors.len();

        // validate internal graph shape
        if successors.len() != predecessors.len() {
            panic!("dominator graph has mismatched successor and predecessor tables");
        }
        if root >= node_count {
            panic!("dominator root is outside dense graph: {root}");
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

impl Analysis for DominatorTree {
    const ID: AnalysisId = AnalysisId("domtree");
    const INVALIDATED_BY: Mutation = Mutation::CONTROL;
}

impl FunctionAnalysis for DominatorTree {
    fn compute(function: &Function, tree: &Tree, analyses: &FunctionAnalysisCache) -> Self {
        let cfg = analyses.get::<ControlFlowGraph>(function, tree);

        Self::build(function, tree, &cfg)
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
        let domtree = DominatorTree::build(function, &tree, &cfg);

        let block0 = function.block(0);
        let block1 = function.block(1);
        let block2 = function.block(2);

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
entry(v0: boolean):
    branch v0 => b1 | b2

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
        let domtree = DominatorTree::build(function, &tree, &cfg);

        let block0 = function.block(0);
        let block1 = function.block(1);
        let block2 = function.block(2);
        let block3 = function.block(3);

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
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    return

b2:
    return

b3:
    return
}
"#,
        );

        let function = tree.get(function_id);
        let cfg = ControlFlowGraph::build(function, &tree);
        let domtree = DominatorTree::build(function, &tree, &cfg);

        let block0 = function.block(0);
        let block3 = function.block(3);

        assert_eq!(domtree.immediate_dominator(block3), None);
        assert!(!domtree.dominates(block0, block3));
    }

    #[test]
    fn test_check_strict_dominance() {
        let (tree, function_id) = parse_test_function(
            r#"
function simple(): void {
entry:
    jump b1

b1:
    return
}
"#,
        );

        let function = tree.get(function_id);
        let cfg = ControlFlowGraph::build(function, &tree);
        let domtree = DominatorTree::build(function, &tree, &cfg);

        let block0 = function.block(0);
        let block1 = function.block(1);

        assert!(domtree.dominates(block0, block0));
        assert!(!domtree.strictly_dominates(block0, block0));
        assert!(domtree.strictly_dominates(block0, block1));
    }
}
