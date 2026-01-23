use std::collections::{HashMap, HashSet};

use destack_mir as mir;

use crate::optimize::common::compute_immediate_dominators;
use crate::optimize::{Analysis, AnalysisId, FunctionAnalyses, FunctionAnalysis};

use super::ControlFlowGraph;

/// Postdominator tree for a function.
///
/// A block A postdominates block B if every path from B to a function exit
/// goes through A. The immediate postdominator of B is the closest such block.
#[derive(Debug)]
pub struct PostDominatorTree {
    /// Immediate postdominator for each block. Exit blocks have no ipdom.
    immediate_postdominators: HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
    /// Preorder numbers for fast postdominance queries.
    preorder: HashMap<mir::LocalNodeId<mir::Block>, u32>,
    /// Maximum preorder number in subtree for fast postdominance queries.
    preorder_max: HashMap<mir::LocalNodeId<mir::Block>, u32>,
}

impl PostDominatorTree {
    /// Build a postdominator tree using the Lengauer Tarjan algorithm.
    fn build(function: &mir::Function, tree: &mir::NodeTree, cfg: &ControlFlowGraph) -> Self {
        // handle functions without bodies
        let Some(entry) = function.entry else {
            return Self {
                immediate_postdominators: HashMap::new(),
                preorder: HashMap::new(),
                preorder_max: HashMap::new(),
            };
        };

        // collect exit blocks
        let mut exit_blocks = Self::collect_exit_blocks(function, tree);
        let has_exits = !exit_blocks.is_empty();

        // handle exitless functions by treating all blocks as exits
        if !has_exits {
            exit_blocks = function.blocks.iter().copied().collect();
        }

        // map blocks to dense indices
        let mut block_index = HashMap::new();
        for (index, &block) in function.blocks.iter().enumerate() {
            block_index.insert(block, index);
        }

        // build reverse graph adjacency lists with a virtual exit root
        let virtual_root = function.blocks.len();
        let mut successors = vec![Vec::new(); function.blocks.len() + 1];
        let mut predecessors = vec![Vec::new(); function.blocks.len() + 1];

        // add reverse edges from each block to its predecessors
        for &block in &function.blocks {
            let index = block_index[&block];
            for predecessor in cfg.predecessors(block) {
                let predecessor_index = block_index[predecessor];
                successors[index].push(predecessor_index);
                predecessors[predecessor_index].push(index);
            }
        }

        // connect virtual root to exits
        for exit_block in exit_blocks {
            let exit_index = block_index[&exit_block];
            successors[virtual_root].push(exit_index);
            predecessors[exit_index].push(virtual_root);
        }

        // compute immediate postdominators for reachable exits
        let mut immediate_postdominators = HashMap::new();

        // exitless functions skip ipdom computation to avoid vacuous postdominators
        if has_exits {
            let result = compute_immediate_dominators(&successors, &predecessors, virtual_root);

            // assign immediate postdominators, skipping the virtual root
            for (&block, &index) in &block_index {
                if let Some(idom_index) = result.immediate_dominators[index] {
                    // ignore edges to the virtual root
                    if idom_index == virtual_root {
                        continue;
                    }

                    let ipdom_block = function.blocks[idom_index];
                    immediate_postdominators.insert(block, ipdom_block);
                }
            }
        }

        // compute preorder numbers for O(1) queries
        let (preorder, preorder_max) =
            Self::compute_preorder(&function.blocks, &immediate_postdominators, entry);

        Self {
            immediate_postdominators,
            preorder,
            preorder_max,
        }
    }

    /// Collect blocks that exit the function (return or unreachable).
    fn collect_exit_blocks(
        function: &mir::Function,
        tree: &mir::NodeTree,
    ) -> HashSet<mir::LocalNodeId<mir::Block>> {
        // gather blocks that terminate without successors
        let mut exits = HashSet::new();

        // scan blocks for empty successor sets
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            let successors = block.terminator.successors();

            // record terminators with no successors
            if successors.is_empty() {
                exits.insert(block_id);
            }
        }

        exits
    }

    /// Compute preorder numbers for the postdominator forest.
    fn compute_preorder(
        blocks: &[mir::LocalNodeId<mir::Block>],
        ipdoms: &HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
        entry: mir::LocalNodeId<mir::Block>,
    ) -> (
        HashMap<mir::LocalNodeId<mir::Block>, u32>,
        HashMap<mir::LocalNodeId<mir::Block>, u32>,
    ) {
        // allocate child lists for the forest
        let mut children: HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>> =
            HashMap::new();

        // initialize empty child lists
        for &block in blocks {
            children.insert(block, Vec::new());
        }

        // populate parent to child edges
        for (&block, &ipdom) in ipdoms {
            children.get_mut(&ipdom).unwrap().push(block);
        }

        // find roots for the postdominator forest
        let mut roots: Vec<_> = blocks
            .iter()
            .copied()
            .filter(|block| !ipdoms.contains_key(block))
            .collect();
        roots.sort();

        // handle degenerate functions by at least walking entry
        if roots.is_empty() {
            roots.push(entry);
        }

        // allocate preorder tracking
        let mut preorder = HashMap::new();
        let mut preorder_max = HashMap::new();
        let mut counter = 0u32;

        // depth first traversal for each root
        fn dfs(
            block: mir::LocalNodeId<mir::Block>,
            children: &HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>>,
            preorder: &mut HashMap<mir::LocalNodeId<mir::Block>, u32>,
            preorder_max: &mut HashMap<mir::LocalNodeId<mir::Block>, u32>,
            counter: &mut u32,
        ) {
            // assign preorder for the current node
            *counter += 1;
            preorder.insert(block, *counter);

            // traverse children and track subtree max
            let mut max = *counter;
            if let Some(block_children) = children.get(&block) {
                for &child in block_children {
                    dfs(child, children, preorder, preorder_max, counter);
                    max = max.max(*preorder_max.get(&child).unwrap());
                }
            }

            preorder_max.insert(block, max);
        }

        // walk each root once
        for root in roots {
            // skip roots already walked by a previous traversal
            if preorder.contains_key(&root) {
                continue;
            }

            dfs(
                root,
                &children,
                &mut preorder,
                &mut preorder_max,
                &mut counter,
            );
        }

        (preorder, preorder_max)
    }

    /// Get the immediate postdominator of a block.
    pub fn immediate_postdominator(
        &self,
        block: mir::LocalNodeId<mir::Block>,
    ) -> Option<mir::LocalNodeId<mir::Block>> {
        self.immediate_postdominators.get(&block).copied()
    }

    /// Check if block A postdominates block B.
    ///
    /// Uses preorder numbers for O(1) queries.
    /// In exitless functions, only identical blocks are treated as postdominators.
    pub fn postdominates(
        &self,
        a: mir::LocalNodeId<mir::Block>,
        b: mir::LocalNodeId<mir::Block>,
    ) -> bool {
        // identical blocks always postdominate
        if a == b {
            return true;
        }

        // read preorder data for both nodes
        let Some(a_pre) = self.preorder.get(&a).copied() else {
            return false;
        };
        let Some(b_pre) = self.preorder.get(&b).copied() else {
            return false;
        };
        let Some(a_max) = self.preorder_max.get(&a).copied() else {
            return false;
        };

        // check subtree containment
        a_pre <= b_pre && a_max >= b_pre
    }
}

impl Analysis for PostDominatorTree {
    const ID: AnalysisId = AnalysisId("postdomtree");
    const DEPENDENCIES: &'static [AnalysisId] = &[ControlFlowGraph::ID];
}

impl FunctionAnalysis for PostDominatorTree {
    fn compute(
        function: &mir::Function,
        tree: &mir::NodeTree,
        analyses: &FunctionAnalyses<'_>,
    ) -> Self {
        let cfg = analyses.get::<ControlFlowGraph>();
        Self::build(function, tree, &cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Linear flow postdominators follow the chain to the exit.
    #[test]
    fn test_postdom_linear_flow() {
        let test = TestProgram::new(
            r#"function @linear() -> void {
block0:
    jump block1
block1:
    jump block2
block2:
    return
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let postdom = analyses.get::<PostDominatorTree>();

        let block0 = function.blocks[0];
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];

        assert_eq!(postdom.immediate_postdominator(block2), None);
        assert_eq!(postdom.immediate_postdominator(block1), Some(block2));
        assert_eq!(postdom.immediate_postdominator(block0), Some(block1));
        assert!(postdom.postdominates(block2, block0));
    }

    /// Diamond control flow postdominates through the merge block.
    #[test]
    fn test_postdom_diamond() {
        let test = TestProgram::new(
            r#"function @diamond(v0: bool) -> void {
block0(v0: bool):
    branch v0, block1, block2
block1:
    jump block3
block2:
    jump block3
block3:
    return
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let postdom = analyses.get::<PostDominatorTree>();

        let block0 = function.blocks[0];
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];
        let block3 = function.blocks[3];

        assert_eq!(postdom.immediate_postdominator(block1), Some(block3));
        assert_eq!(postdom.immediate_postdominator(block2), Some(block3));
        assert_eq!(postdom.immediate_postdominator(block0), Some(block3));
        assert!(postdom.postdominates(block3, block0));
    }

    /// Multiple exits yield no shared postdominator for the branch block.
    #[test]
    fn test_postdom_multiple_exits() {
        let test = TestProgram::new(
            r#"function @multi_exit(v0: bool) -> void {
block0(v0: bool):
    branch v0, block1, block2
block1:
    return
block2:
    return
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let postdom = analyses.get::<PostDominatorTree>();

        let block0 = function.blocks[0];
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];

        assert_eq!(postdom.immediate_postdominator(block1), None);
        assert_eq!(postdom.immediate_postdominator(block2), None);
        assert_eq!(postdom.immediate_postdominator(block0), None);
        assert!(!postdom.postdominates(block1, block0));
        assert!(!postdom.postdominates(block2, block0));
    }

    /// Loops postdominate through the unique exit block.
    #[test]
    fn test_postdom_loop() {
        let test = TestProgram::new(
            r#"function @loop(v0: i32) -> void {
block0(v0: i32):
    v1: i32 = iconst 0i32
    jump block1(v1)
block1(v2: i32):
    v3: i32 = iconst 1i32
    v4: i32 = iadd v2, v3
    v5: bool = icmp_slt v4, v0
    branch v5, block2(v4), block3
block2(v6: i32):
    jump block1(v6)
block3:
    return
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let postdom = analyses.get::<PostDominatorTree>();

        let block0 = function.blocks[0];
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];
        let block3 = function.blocks[3];

        assert_eq!(postdom.immediate_postdominator(block2), Some(block1));
        assert_eq!(postdom.immediate_postdominator(block1), Some(block3));
        assert_eq!(postdom.immediate_postdominator(block0), Some(block1));
        assert!(postdom.postdominates(block3, block2));
    }

    /// Exitless loops do not assign postdominators beyond self.
    #[test]
    fn test_postdom_no_exit() {
        let test = TestProgram::new(
            r#"function @no_exit() -> void {
block0:
    jump block1
block1:
    jump block0
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let postdom = analyses.get::<PostDominatorTree>();

        let block0 = function.blocks[0];
        let block1 = function.blocks[1];

        assert_eq!(postdom.immediate_postdominator(block0), None);
        assert_eq!(postdom.immediate_postdominator(block1), None);
        assert!(!postdom.postdominates(block0, block1));
        assert!(!postdom.postdominates(block1, block0));
    }
}
